pub mod frame;
pub mod placement;
pub mod screen_buffer;

use std::collections::VecDeque;
use std::sync::mpsc::{Sender, SyncSender, TrySendError};
use std::time::{Duration, Instant};

use log::info;
use samp::amx::{Amx, AmxExt, AmxIdent};
use samp::error::AmxResult;

use crate::AnyScreen;
use crate::animation::Animation;
use crate::constants::{
    CANVAS_HEIGHT, CANVAS_WIDTH, MATERIAL_PAINT_COST_ESTIMATE, MATERIAL_PAINT_COST_PER_CHAR,
    POSITION_UPDATE_COST,
};
use crate::content_sources::video::{self, FrameOutcome};
use crate::engine::WorldPosition;
use crate::network_budget::NetworkBudget;
use crate::screen::{DisplayTargetMethod, Screen};
use crate::screen_3d::frame::{Frame3D, Frame3DMaterial, build_frame3d};
use crate::screen_3d::placement::{create_screen_decoy, destroy_screen_decoy};
use crate::screen_3d::screen_buffer::ScreenBuffer;

const PAINT_BATCH: usize = 6;
/// How long a freshly painted hidden target sits before it's eligible to
/// swap in, giving any in-flight paint operations a moment to actually land.
const SETTLE_DURATION: Duration = Duration::from_millis(30);
/// Minimum time after a swap before the next target starts painting, so the
/// newly visible frame gets a moment on screen first.
const POST_SWAP_GRACE: Duration = Duration::from_millis(80);
/// Floor on how long a frame stays visible before the next swap, regardless
/// of its own (often much shorter) source duration. Without this, a backlog
/// piled up in `ready_queue` drains in a rapid-fire burst of swaps once
/// timing/budget allows it, instead of a smooth, gradual transition -
/// falling behind real time is an acceptable tradeoff for evenly paced swaps.
const MIN_SWAP_INTERVAL: Duration = Duration::from_millis(40);
const AREA_LISTENER_SYNC_INTERVAL: Duration = Duration::from_millis(500);

pub struct Screen3D {
    pub amx_ident: AmxIdent,
    pub buffers: Vec<ScreenBuffer>,
    w_position: WorldPosition,
    hidden_buffer_pos: WorldPosition,
    animation: Animation<Frame3D>,
    painting_target: Option<usize>,
    painting_frame_duration: Duration,
    ready_queue: VecDeque<(usize, Instant, Duration)>,
    next_hidden_index: usize,
    visible_index: usize,
    // This screen's live-audio relay path on `audio_server`, if it's a live
    // stream - so `destroy` can pull the plug on the actual audio source
    // server-side instead of leaving it playing for whoever's already
    // connected to it.
    live_audio_path: Option<String>,
    target_method: DisplayTargetMethod,
    _has_started: bool,
    last_area_listener_sync: Instant,
    loading_decoy: Option<Vec<i32>>,
}

impl Screen for Screen3D {
    type Frame = Frame3D;
    /// `(tile_cols, tile_rows)` for the mosaic.
    type DecodeConfig = (usize, usize);

    fn load_clip(
        url: &str,
        sender: Sender<(Vec<Frame3DMaterial>, Duration)>,
        &(tile_cols, tile_rows): &(usize, usize),
    ) -> Result<(), String> {
        let canvas_width = CANVAS_WIDTH * tile_cols as u32;
        let canvas_height = CANVAS_HEIGHT * tile_rows as u32;

        info!(
            "load_clip -> decoding {} ({}x{}, tiles {}x{})",
            url, canvas_width, canvas_height, tile_cols, tile_rows
        );

        let raw_frames = video::load_frames(url, canvas_width, canvas_height)?;
        info!(
            "load_clip -> ffmpeg produced {} raw frame(s), building Frame3D",
            raw_frames.len()
        );

        let frame_count = raw_frames.len();
        for raw in raw_frames {
            if sender
                .send(build_frame3d(&raw.data, raw.delay, tile_cols, tile_rows))
                .is_err()
            {
                info!("load_clip -> receiver disconnected");
                return Ok(());
            }
        }
        info!(
            "load_clip -> built and sent {} Frame3D entries",
            frame_count
        );
        info!("load_clip -> done");

        Ok(())
    }

    fn stream_clip(
        url: &str,
        sender: SyncSender<(Vec<Frame3DMaterial>, Duration)>,
        &(tile_cols, tile_rows): &(usize, usize),
    ) -> Result<(), String> {
        let canvas_width = CANVAS_WIDTH * tile_cols as u32;
        let canvas_height = CANVAS_HEIGHT * tile_rows as u32;

        video::stream_frames(url, canvas_width, canvas_height, |raw| {
            match sender.try_send(build_frame3d(&raw.data, raw.delay, tile_cols, tile_rows)) {
                Ok(()) => FrameOutcome::Sent,
                Err(TrySendError::Full(_)) => FrameOutcome::Dropped,
                Err(TrySendError::Disconnected(_)) => FrameOutcome::Disconnected,
            }
        })
    }

    fn animation_mut(&mut self) -> &mut Animation<Frame3D> {
        &mut self.animation
    }

    fn step_paint(&mut self, amx: &Amx, budget: &mut NetworkBudget) {
        if self.advance_active_paint(amx, budget) {
            return;
        }

        if self.try_swap_ready(amx, budget) {
            return;
        }

        self.try_start_next_paint(amx, budget);
    }

    fn tick(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> AmxResult<()> {
        if !self._has_started {
            let audio_url = self.live_audio_path.clone();
            if let Some(url) = audio_url.as_deref() {
                self.target_method
                    .start_audio(amx, url, self.w_position.position())?;
            }
            self._has_started = true;
        }

        if self.last_area_listener_sync.elapsed() >= AREA_LISTENER_SYNC_INTERVAL {
            let audio_url = self.live_audio_path.clone();
            self.target_method.sync_area_listeners(
                amx,
                audio_url.as_deref(),
                self.w_position.position(),
            )?;
            self.last_area_listener_sync = Instant::now();
        }

        self.step_paint(amx, budget);
        Ok(())
    }

    fn get_display_target_method(&self) -> &DisplayTargetMethod {
        &self.target_method
    }

    fn set_display_target_method(&mut self, method: crate::screen::DisplayTargetMethod) {
        self.target_method = method;
    }

    fn display_target_method_mut(&mut self) -> &mut DisplayTargetMethod {
        &mut self.target_method
    }

    fn audio_url(&self) -> &Option<String> {
        &self.live_audio_path
    }

    fn set_audio_url(&mut self, url: String) {
        self.live_audio_path = Some(url);
    }

    fn get_position(&self) -> &WorldPosition {
        &self.w_position
    }

    fn has_started(&self) -> bool {
        self._has_started
    }

    fn set_started(&mut self) {
        self._has_started = true;
    }
}

impl Screen3D {
    pub fn new(
        amx: &Amx,
        screens: &mut Vec<Option<AnyScreen>>,
        position: WorldPosition,
        hidden_buffer_pos: WorldPosition,
        ring_size: usize,
        tile_grid: (usize, usize),
        target_method: DisplayTargetMethod,
    ) -> AmxResult<usize> {
        let amx_ident = amx.ident();
        let (tile_cols, tile_rows) = tile_grid;

        let mut buffers = Vec::with_capacity(ring_size);

        let player_id = match &target_method {
            DisplayTargetMethod::Player { player_id, .. } => Some(*player_id),
            _ => None,
        };

        
        let loading_decoy = Some(create_screen_decoy(
            amx, &position, tile_cols, tile_rows, &player_id,
        )?);

        for index in 0..ring_size {
            let position = if index == 0 {
                &position
            } else {
                &hidden_buffer_pos
            };
            buffers.push(ScreenBuffer::new(
                amx, position, tile_cols, tile_rows, &player_id,
            )?);
        }


        let frame_duration = Duration::from_millis(100);
        let screen_animation = None;

        let screen_3d = Screen3D {
            amx_ident,
            buffers,
            w_position: position,
            hidden_buffer_pos,
            animation: Animation {
                frame_duration,
                screen_animation,
                since: Instant::now(),
            },
            painting_target: None,
            painting_frame_duration: frame_duration,
            ready_queue: VecDeque::new(),
            next_hidden_index: 1,
            visible_index: 0,
            live_audio_path: None,
            target_method,
            _has_started: false,
            last_area_listener_sync: Instant::now(),
            loading_decoy,
        };

        screens.push(Some(AnyScreen::ThreeD(screen_3d)));

        let screen_index = screens.len() - 1;
        Ok(screen_index)
    }

    /// Processes a batch of work on the in-flight paint target. Returns
    /// `true` if it's still mid-paint and the caller should stop for this
    /// tick; otherwise moves the finished target into the settle queue.
    fn advance_active_paint(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> bool {
        if let Some(target) = self.painting_target {
            if self.has_pending_paint(target) {
                self.process_pending_paint(amx, target, budget);
                if self.has_pending_paint(target) {
                    return true;
                }
            }
        }

        if let Some(target) = self.painting_target.take() {
            let duration = self.painting_frame_duration;
            self.ready_queue
                .push_back((target, Instant::now() + SETTLE_DURATION, duration));
        }

        false
    }

    /// Swaps in the front of the settle queue once it has settled and the
    /// currently visible frame has been on screen for its full duration.
    fn try_swap_ready(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> bool {
        // Only the freshest painted buffer is worth showing - if several
        // piled up while we were waiting to swap, skip straight past the
        // older ones instead of playing through a cluster of near-identical
        // moments.
        while self.ready_queue.len() > 1 {
            self.ready_queue.pop_front();
        }

        let Some(&(target, ready_at, duration)) = self.ready_queue.front() else {
            return false;
        };

        let frame_elapsed = self.animation.since.elapsed();
        let min_visible = self.animation.frame_duration.max(MIN_SWAP_INTERVAL);
        if Instant::now() < ready_at || frame_elapsed < min_visible {
            return false;
        }

        let tile_count = self.buffers[target].tiles.len();
        let swap_cost = 2.0 * tile_count as f64 * POSITION_UPDATE_COST;
        if !budget.try_spend(swap_cost) {
            return false;
        }

        self.ready_queue.pop_front();
        if let Err(err) = self.swap_visible(amx, target) {
            info!("failed to swap visible paint target: {:?}", err);
        }
        self.animation.since = Instant::now();
        self.animation.frame_duration = duration;
        true
    }

    fn try_start_next_paint(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> bool {
        if !self.ready_queue.is_empty() {
            return false;
        }

        if self.painting_target.is_some() || self.animation.since.elapsed() < POST_SWAP_GRACE {
            return false;
        }

        if !self.should_build_next_frame() {
            return false;
        }

        let Some((frame, duration)) = self.build_animation_frame() else {
            return false;
        };

        self.start_frame(frame, duration);
        if let Some(target) = self.painting_target {
            self.process_pending_paint(amx, target, budget);
        }
        true
    }

    fn start_frame(&mut self, frame: Frame3D, duration: Duration) {
        let target = self.allocate_paint_target();
        self.painting_target = Some(target);
        self.painting_frame_duration = duration;

        self.stage_paint(target, &frame);

        if !self.has_pending_paint(target) {
            self.painting_target = None;
            self.ready_queue
                .push_back((target, Instant::now() + SETTLE_DURATION, duration));
        }
    }

    fn allocate_paint_target(&mut self) -> usize {
        loop {
            let index = self.next_hidden_index % self.buffers.len();
            self.next_hidden_index = index + 1;
            if index != self.visible_index {
                return index;
            }
        }
    }

    fn stage_paint(&mut self, target: usize, frame: &Frame3D) {
        self.buffers[target].stage_paint(frame);
    }

    fn has_pending_paint(&self, target: usize) -> bool {
        self.buffers[target].has_pending()
    }

    fn process_pending_paint(&mut self, amx: &Amx, target: usize, budget: &mut NetworkBudget) {
        let affordable = budget.affordable(MATERIAL_PAINT_COST_ESTIMATE, PAINT_BATCH);
        let (_, chars_sent) = self.buffers[target].process_pending(amx, affordable);
        budget.spend(chars_sent as f64 * MATERIAL_PAINT_COST_PER_CHAR);
    }

    fn swap_visible(&mut self, amx: &Amx, target: usize) -> AmxResult<()> {
        if let Some(object_ids) = self.loading_decoy.take() {
            destroy_screen_decoy(amx, &object_ids);
        }
        self.buffers[target].set_position(amx, self.w_position.position())?;
        self.buffers[self.visible_index].set_position(amx, self.hidden_buffer_pos.position())?;
        self.visible_index = target;

        info!(
            "Swap -> target_index={} new_visible_object={}",
            target, self.buffers[target].tiles[0].object_id
        );
        Ok(())
    }

    /// Destroys every ring-buffer object backing this screen - visible and
    /// hidden alike - and stops its live audio relay, if it has one.
    /// Whatever's mid-paint/mid-swap is simply abandoned; there's no "finish
    /// gracefully" step worth doing for a screen that's about to stop
    /// existing entirely.
    pub fn destroy(&self, amx: &Amx) {
        for buffer in &self.buffers {
            buffer.destroy(amx);
        }
        if let Some(object_ids) = &self.loading_decoy {
            destroy_screen_decoy(amx, object_ids);
        }

        let mut target_method = self.target_method.clone();
        if let Err(err) = target_method.stop_audio(amx) {
            info!(
                "Screen3D::destroy -> failed to stop listener audio: {:?}",
                err
            );
        }

        if let Some(area_id) = self.target_method.area_id() {
            if let Err(err) = crate::amx_natives::destroy_dynamic_area(amx, area_id) {
                info!(
                    "Screen3D::destroy -> failed to destroy audio area {}: {:?}",
                    area_id, err
                );
            }
        }

        match &self.live_audio_path {
            Some(path) => {
                info!("Screen3D::destroy -> stopping live audio source {}", path);
                crate::audio_server::stop_live_source(path);
            }
            None => info!(
                "Screen3D::destroy -> no live_audio_path set (not a live screen, or never got that far)"
            ),
        }
    }

    pub fn handle_area_enter(
        &mut self,
        amx: &Amx,
        player_id: i32,
        area_id: i32,
    ) -> AmxResult<bool> {
        if self.target_method.area_id() != Some(area_id) {
            return Ok(false);
        }

        let audio_url = self.live_audio_path.as_deref();
        self.target_method
            .add_area_listener(amx, player_id, audio_url, self.w_position.position())
    }

    pub fn handle_area_leave(
        &mut self,
        amx: &Amx,
        player_id: i32,
        area_id: i32,
    ) -> AmxResult<bool> {
        if self.target_method.area_id() != Some(area_id) {
            return Ok(false);
        }

        self.target_method.remove_area_listener(amx, player_id)
    }
}
