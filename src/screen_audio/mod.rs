mod natives;

use std::time::{Duration, Instant};

use samp::amx::{Amx, AmxExt, AmxIdent};
use samp::error::AmxResult;

use crate::AnyScreen;
use crate::animation::Animation;
use crate::engine::WorldPosition;
use crate::network_budget::NetworkBudget;
use crate::screen::{DisplayTargetMethod, Screen};

const AREA_LISTENER_SYNC_INTERVAL: Duration = Duration::from_millis(500);

/// A screen with nothing to paint - same position/target-audience machinery
/// as `Screen3D` (player-targeted or area-based), but it only ever plays
/// audio, so there's no buffer, no tiles, no frames.
pub struct AudioSource {
    amx_ident: AmxIdent,
    w_position: WorldPosition,
    animation: Animation<()>,
    audio_url: Option<String>,
    audio_relay_path: Option<String>,
    target_method: DisplayTargetMethod,
    _has_started: bool,
    last_area_listener_sync: Instant,
}

impl Screen for AudioSource {
    type Frame = ();
    type DecodeConfig = ();

    fn decode_dimensions(_config: &()) -> (u32, u32) {
        (0, 0)
    }

    fn build_frame(_canvas: &[u8], delay: Duration, _config: &()) -> ((), Duration) {
        ((), delay)
    }

    fn animation_mut(&mut self) -> &mut Animation<()> {
        &mut self.animation
    }

    fn step_paint(&mut self, _amx: &Amx, _budget: &mut NetworkBudget) {}

    fn amx_ident(&self) -> AmxIdent {
        self.amx_ident
    }

    fn destroy_screen(&self, _amx: &Amx) {}

    fn before_step_paint(&mut self, amx: &Amx) -> AmxResult<()> {
        if self.last_area_listener_sync.elapsed() >= AREA_LISTENER_SYNC_INTERVAL {
            let audio_url = self.audio_url.clone();
            self.target_method.sync_area_listeners(
                amx,
                audio_url.as_deref(),
                self.w_position.position(),
            )?;
            self.last_area_listener_sync = Instant::now();
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
        &self.w_position
    }

    fn has_started(&self) -> bool {
        self._has_started
    }

    fn set_started(&mut self) {
        self._has_started = true;
    }
}

impl AudioSource {
    pub fn new(
        amx: &Amx,
        screens: &mut Vec<Option<AnyScreen>>,
        position: WorldPosition,
        target_method: DisplayTargetMethod,
    ) -> usize {
        let audio_source = AudioSource {
            amx_ident: amx.ident(),
            w_position: position,
            animation: Animation {
                frame_duration: Duration::from_millis(100),
                screen_animation: None,
                since: Instant::now(),
            },
            audio_url: None,
            audio_relay_path: None,
            target_method,
            _has_started: false,
            last_area_listener_sync: Instant::now(),
        };

        screens.push(Some(AnyScreen::Audio(audio_source)));
        screens.len() - 1
    }
}
