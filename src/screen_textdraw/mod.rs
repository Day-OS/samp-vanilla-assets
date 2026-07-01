pub mod frame;
mod natives;

use std::time::{Duration, Instant};

use log::info;
use samp::amx::{Amx, AmxExt, AmxIdent};
use samp::error::AmxResult;

use crate::AnyScreen;
use crate::amx_natives;
use crate::animation::Animation;
use crate::engine::WorldPosition;
use crate::network_budget::NetworkBudget;
use crate::screen::{DisplayTargetMethod, Screen};
use frame::{TD_STRING_MAX, TextDrawFrame, build_frame};

const TD_FONT: i32 = 3;
const TD_ALIGN_LEFT: i32 = 1;

#[derive(Clone)]
pub struct TextDrawDecodeConfig {
    pub cols: usize,
    pub rows: usize,
    pub budget: usize,
}

pub struct TextDrawScreen {
    pub amx_ident: AmxIdent,
    player_id: i32,
    bg_textdraw_id: i32,
    /// [chunk_index][pool_slot] — pool_size slots per chunk, created once, colour+text updated per frame.
    chunk_pools: Vec<Vec<i32>>,
    animation: Animation<TextDrawFrame>,
    pending_frame: Option<TextDrawFrame>,
    target_method: DisplayTargetMethod,
    audio_url: Option<String>,
    audio_relay_path: Option<String>,
    has_started: bool,
    position: WorldPosition,
}

impl Screen for TextDrawScreen {
    type Frame = TextDrawFrame;
    type DecodeConfig = TextDrawDecodeConfig;

    fn decode_dimensions(config: &TextDrawDecodeConfig) -> (u32, u32) {
        (config.cols as u32, config.rows as u32)
    }

    fn build_frame(canvas: &[u8], delay: Duration, config: &TextDrawDecodeConfig) -> (TextDrawFrame, Duration) {
        build_frame(canvas, delay, config.cols, config.rows, config.budget)
    }

    fn animation_mut(&mut self) -> &mut Animation<TextDrawFrame> {
        &mut self.animation
    }

    fn step_paint(&mut self, amx: &Amx, _budget: &mut NetworkBudget) {
        if self.should_build_next_frame() {
            if let Some((frame, duration)) = self.build_animation_frame() {
                self.pending_frame = Some(frame);
                self.animation.frame_duration = duration;
                self.animation.since = Instant::now();
            }
        }

        if let Some(frame) = self.pending_frame.take() {
            self.paint_frame(amx, &frame);
        }
    }

    fn amx_ident(&self) -> AmxIdent {
        self.amx_ident
    }

    fn destroy_screen(&self, amx: &Amx) {
        if let Err(e) = amx_natives::player_text_draw_destroy(amx, self.player_id, self.bg_textdraw_id) {
            info!("TextDrawScreen::destroy -> failed bg td: {:?}", e);
        }
        for pool in &self.chunk_pools {
            for &td_id in pool {
                if let Err(e) = amx_natives::player_text_draw_destroy(amx, self.player_id, td_id) {
                    info!("TextDrawScreen::destroy -> failed td {}: {:?}", td_id, e);
                }
            }
        }
    }

    fn start_audio(&mut self, amx: &Amx) -> AmxResult<()> {
        if let Some(url) = self.audio_url.as_deref() {
            amx_natives::play_audio_stream_for_player(amx, &self.player_id, url, None, 0.0)?;
        }
        Ok(())
    }

    fn get_display_target_method(&self) -> &DisplayTargetMethod { &self.target_method }
    fn set_display_target_method(&mut self, m: DisplayTargetMethod) { self.target_method = m; }
    fn display_target_method_mut(&mut self) -> &mut DisplayTargetMethod { &mut self.target_method }
    fn audio_url(&self) -> &Option<String> { &self.audio_url }
    fn set_audio_url(&mut self, url: String) { self.audio_url = Some(url); }
    fn audio_relay_path(&self) -> Option<&str> { self.audio_relay_path.as_deref() }
    fn set_audio_relay_path(&mut self, path: String) { self.audio_relay_path = Some(path); }
    fn get_position(&self) -> &WorldPosition { &self.position }
    fn has_started(&self) -> bool { self.has_started }
    fn set_started(&mut self) { self.has_started = true; }
}

impl TextDrawScreen {
    pub fn new(
        amx: &Amx,
        screens: &mut Vec<Option<AnyScreen>>,
        player_id: i32,
        grid_cols: usize,
        grid_rows: usize,
        hud_x: f32,
        hud_y: f32,
        letter_size_x: f32,
        letter_size_y: f32,
        box_scale: f32,
        budget: usize,
    ) -> AmxResult<usize> {
        let chunk_rows = (TD_STRING_MAX + 3) / (grid_cols + 3);
        let chunk_count = (grid_rows + chunk_rows - 1) / chunk_rows;
        // Pool size mirrors what build_frame computes: (budget-1)/chunk_count slots per chunk.
        let pool_size = budget.saturating_sub(1) / chunk_count.max(1);
        let pool_size = pool_size.max(1);

        // letter_size_y → textdraw coordinate units: empirically 3.0 per row at letter_size_y=0.35
        let row_y_step = letter_size_y * (3.0 / 0.35);

        // Background box — created first, lowest ID = bottom layer.
        let total_width = grid_cols as f32 * letter_size_x * box_scale;
        let bg_text = std::iter::repeat(" ").take(grid_rows).collect::<Vec<_>>().join("~n~");
        let bg_id = amx_natives::create_player_text_draw(amx, player_id, hud_x, hud_y, &bg_text)?;
        amx_natives::player_text_draw_use_box(amx, player_id, bg_id, 1)?;
        amx_natives::player_text_draw_box_color(amx, player_id, bg_id, 0x000000FFu32 as i32)?;
        amx_natives::player_text_draw_letter_size(amx, player_id, bg_id, letter_size_x, letter_size_y)?;
        amx_natives::player_text_draw_text_size(amx, player_id, bg_id, hud_x + total_width, 0.0)?;
        amx_natives::player_text_draw_show(amx, player_id, bg_id)?;

        // Create pool_size textdraws per chunk, all at the same Y position for that chunk.
        // Colour and content are set adaptively each frame in paint_frame().
        let placeholder = ".".repeat(grid_cols);
        let mut chunk_pools = Vec::with_capacity(chunk_count);

        for chunk in 0..chunk_count {
            let chunk_y = hud_y + (chunk as f32) * (chunk_rows as f32) * row_y_step;
            let mut pool = Vec::with_capacity(pool_size);

            for _ in 0..pool_size {
                let td_id = amx_natives::create_player_text_draw(amx, player_id, hud_x, chunk_y, &placeholder)?;
                amx_natives::player_text_draw_letter_size(amx, player_id, td_id, letter_size_x, letter_size_y)?;
                amx_natives::player_text_draw_text_size(amx, player_id, td_id, (grid_cols as f32) * letter_size_x, 0.0)?;
                amx_natives::player_text_draw_alignment(amx, player_id, td_id, TD_ALIGN_LEFT)?;
                amx_natives::player_text_draw_colour(amx, player_id, td_id, 0x00000000u32 as i32)?;
                amx_natives::player_text_draw_background_colour(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_set_outline(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_set_shadow(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_font(amx, player_id, td_id, TD_FONT)?;
                pool.push(td_id);
            }
            chunk_pools.push(pool);
        }

        let screen = TextDrawScreen {
            amx_ident: amx.ident(),
            bg_textdraw_id: bg_id,
            player_id,
            chunk_pools,
            animation: Animation {
                frame_duration: Duration::from_millis(100),
                screen_animation: None,
                since: Instant::now(),
            },
            pending_frame: None,
            target_method: DisplayTargetMethod::player(player_id, 0.0),
            audio_url: None,
            audio_relay_path: None,
            has_started: false,
            position: WorldPosition {
                position_x: 0.0,
                position_y: 0.0,
                position_z: 0.0,
                rotation_x: 0.0,
                rotation_y: 0.0,
                rotation_z: 0.0,
                world_id: -1,
                interior_id: -1,
            },
        };

        screens.push(Some(AnyScreen::TextDraw(screen)));
        Ok(screens.len() - 1)
    }

    fn paint_frame(&self, amx: &Amx, frame: &TextDrawFrame) {
        for (chunk, (pool, chunk_layers)) in self.chunk_pools.iter().zip(frame.chunk_layers.iter()).enumerate() {
            for (slot, (td_id, layer)) in pool.iter().zip(chunk_layers.iter()).enumerate() {
                if let Err(e) = amx_natives::player_text_draw_colour(amx, self.player_id, *td_id, layer.rgba as i32) {
                    info!("TextDrawScreen::paint chunk={} slot={} colour failed: {:?}", chunk, slot, e);
                }
                if let Err(e) = amx_natives::player_text_draw_set_string(amx, self.player_id, *td_id, &layer.text) {
                    info!("TextDrawScreen::paint chunk={} slot={} set_string failed: {:?}", chunk, slot, e);
                }
                if let Err(e) = amx_natives::player_text_draw_show(amx, self.player_id, *td_id) {
                    info!("TextDrawScreen::paint chunk={} slot={} show failed: {:?}", chunk, slot, e);
                }
            }
        }
    }
}
