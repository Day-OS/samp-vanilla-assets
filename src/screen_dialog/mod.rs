mod natives;
mod render;

use std::time::{Duration, Instant};

use samp::amx::{Amx, AmxExt, AmxIdent};
use samp::error::AmxResult;

use crate::amx_natives;
use crate::animation::Animation;
use crate::engine::WorldPosition;
use crate::network_budget::NetworkBudget;
use crate::screen::{DisplayTargetMethod, Screen};
use crate::AnyScreen;
use render::build_dialog_body;

const DIALOG_STYLE_MSGBOX: i32 = 0;
const DEFAULT_DIALOG_ID: i32 = 1;
const DIALOG_TITLE: &str = "Image Dialog";
const DIALOG_BUTTON_1: &str = "Close";
const DIALOG_BUTTON_2: &str = "";
const DEFAULT_FRAME_DURATION: Duration = Duration::from_secs(60);

#[derive(Clone, Copy)]
pub struct DialogDecodeConfig {
    pub(crate) cols: usize,
    pub(crate) rows: usize,
}

pub struct DialogScreen {
    pub amx_ident: AmxIdent,
    player_id: i32,
    dialog_id: i32,
    style: i32,
    title: String,
    body: String,
    button1: String,
    button2: String,
    position: WorldPosition,
    animation: Animation<String>,
    target_method: DisplayTargetMethod,
    audio_url: Option<String>,
    audio_relay_path: Option<String>,
    has_started: bool,
    has_shown: bool,
}

impl Screen for DialogScreen {
    type Frame = String;
    type DecodeConfig = DialogDecodeConfig;

    fn decode_dimensions(config: &Self::DecodeConfig) -> (u32, u32) {
        (config.cols as u32, config.rows as u32)
    }

    fn build_frame(
        canvas: &[u8],
        delay: Duration,
        config: &Self::DecodeConfig,
    ) -> (Self::Frame, Duration) {
        let body = build_dialog_body(canvas, config.cols, config.rows);
        log::info!(
            "DialogScreen::build_frame -> built dialog frame {}x{} body_len={}",
            config.cols,
            config.rows,
            body.len()
        );
        (body, delay)
    }

    fn animation_mut(&mut self) -> &mut Animation<Self::Frame> {
        &mut self.animation
    }

    fn step_paint(&mut self, amx: &Amx, _budget: &mut NetworkBudget) {
        if self.should_build_next_frame() {
            if let Some((body, duration)) = self.build_animation_frame() {
                self.body = body;
                self.animation.frame_duration = duration;
                self.animation.since = Instant::now();
                self.has_shown = false;
                log::info!(
                    "DialogScreen::step_paint -> applying dialog media frame body_len={}",
                    self.body.len()
                );
            }
        }

        if self.has_shown {
            return;
        }

        match amx_natives::show_player_dialog(
            amx,
            self.player_id,
            self.dialog_id,
            self.style,
            &self.title,
            &self.body,
            &self.button1,
            &self.button2,
        ) {
            Ok(_) => {
                self.has_shown = true;
            }
            Err(err) => {
                log::error!(
                    "DialogScreen::step_paint -> failed to show dialog {} for player {}: {:?}",
                    self.dialog_id,
                    self.player_id,
                    err
                );
            }
        }
    }

    fn amx_ident(&self) -> AmxIdent {
        self.amx_ident
    }

    fn destroy_screen(&self, amx: &Amx) {
        if let Err(err) = amx_natives::hide_player_dialog(amx, self.player_id) {
            log::error!(
                "DialogScreen::destroy -> failed to hide dialog for player {}: {:?}",
                self.player_id,
                err
            );
        }

        if let Err(err) = amx_natives::stop_audio_stream_for_player(amx, self.player_id) {
            log::error!(
                "DialogScreen::destroy -> failed to stop audio for player {}: {:?}",
                self.player_id,
                err
            );
        }
    }

    fn start_audio(&mut self, amx: &Amx) -> AmxResult<()> {
        if let Some(url) = self.audio_url.as_deref() {
            amx_natives::play_audio_stream_for_player(amx, &self.player_id, url, None, 0.0)?;
        }
        Ok(())
    }

    fn get_display_target_method(&self) -> &DisplayTargetMethod {
        &self.target_method
    }

    fn set_display_target_method(&mut self, method: DisplayTargetMethod) {
        self.target_method = method;
    }

    fn display_target_method_mut(&mut self) -> &mut DisplayTargetMethod {
        &mut self.target_method
    }

    fn audio_url(&self) -> &Option<String> {
        &self.audio_url
    }

    fn set_audio_url(&mut self, url: String) {
        self.audio_url = Some(url);
    }

    fn audio_relay_path(&self) -> Option<&str> {
        self.audio_relay_path.as_deref()
    }

    fn set_audio_relay_path(&mut self, path: String) {
        self.audio_relay_path = Some(path);
    }

    fn get_position(&self) -> &WorldPosition {
        &self.position
    }

    fn has_started(&self) -> bool {
        self.has_started
    }

    fn set_started(&mut self) {
        self.has_started = true;
    }
}

impl DialogScreen {
    pub fn new(
        amx: &Amx,
        screens: &mut Vec<Option<AnyScreen>>,
        player_id: i32,
        url: String,
        cols: usize,
        rows: usize,
    ) -> usize {
        log::info!(
            "DialogScreen::new -> player={} url={} cols={} rows={}",
            player_id,
            url,
            cols,
            rows
        );

        let dialog_screen = DialogScreen {
            amx_ident: amx.ident(),
            player_id,
            dialog_id: DEFAULT_DIALOG_ID,
            style: DIALOG_STYLE_MSGBOX,
            title: DIALOG_TITLE.to_string(),
            body: "Loading image...".to_string(),
            button1: DIALOG_BUTTON_1.to_string(),
            button2: DIALOG_BUTTON_2.to_string(),
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
            animation: Animation {
                frame_duration: DEFAULT_FRAME_DURATION,
                screen_animation: None,
                since: Instant::now(),
            },
            target_method: DisplayTargetMethod::player(player_id, 0.0),
            audio_url: None,
            audio_relay_path: None,
            has_started: false,
            has_shown: false,
        };

        screens.push(Some(AnyScreen::Dialog(dialog_screen)));
        screens.len() - 1
    }
}