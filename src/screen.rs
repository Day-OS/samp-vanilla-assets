use std::sync::mpsc::{Sender, SyncSender, TrySendError};
use std::time::Duration;

use samp::amx::{Amx, AmxIdent};
use samp::error::AmxResult;

use log::info;

use crate::amx_natives;
use crate::animation::{Animation, ScreenAnimation};
use crate::audio_server;
use crate::content_sources::video::{self, FrameOutcome};
use crate::engine::WorldPosition;
use crate::network_budget::NetworkBudget;

#[derive(Clone)]
pub enum DisplayTargetMethod {
    Player {
        player_id: i32,
        listening: bool,
        audio_range: f32,
    },
    AllPlayers,
    Area {
        area_id: i32,
        listeners: Vec<i32>,
        audio_range: f32,
    },
}

impl DisplayTargetMethod {
    pub fn player(player_id: i32, audio_range: f32) -> Self {
        DisplayTargetMethod::Player {
            player_id,
            listening: false,
            audio_range,
        }
    }

    pub fn area(area_id: i32, listeners: Vec<i32>, audio_range: f32) -> Self {
        DisplayTargetMethod::Area {
            area_id,
            listeners,
            audio_range,
        }
    }

    pub fn area_id(&self) -> Option<i32> {
        match self {
            DisplayTargetMethod::Area { area_id, .. } => Some(*area_id),
            _ => None,
        }
    }

    pub fn start_audio(
        &mut self,
        amx: &Amx,
        url: &str,
        position: (f32, f32, f32),
    ) -> AmxResult<()> {
        match self {
            DisplayTargetMethod::Player {
                player_id,
                listening,
                audio_range,
            } => {
                if !*listening {
                    amx_natives::play_audio_stream_for_player(
                        amx,
                        player_id,
                        url,
                        Some(position),
                        *audio_range,
                    )?;
                    *listening = true;
                }
            }
            DisplayTargetMethod::Area {
                listeners,
                audio_range,
                ..
            } => {
                for player_id in listeners.iter() {
                    amx_natives::play_audio_stream_for_player(
                        amx,
                        player_id,
                        url,
                        Some(position),
                        *audio_range,
                    )?;
                }
            }
            DisplayTargetMethod::AllPlayers => {}
        }

        Ok(())
    }

    pub fn stop_audio(&mut self, amx: &Amx) -> AmxResult<()> {
        match self {
            DisplayTargetMethod::Player {
                player_id,
                listening,
                ..
            } => {
                if *listening {
                    amx_natives::stop_audio_stream_for_player(amx, *player_id)?;
                    *listening = false;
                }
            }
            DisplayTargetMethod::Area { listeners, .. } => {
                for player_id in listeners.drain(..) {
                    amx_natives::stop_audio_stream_for_player(amx, player_id)?;
                }
            }
            DisplayTargetMethod::AllPlayers => {}
        }

        Ok(())
    }

    pub fn add_area_listener(
        &mut self,
        amx: &Amx,
        player_id: i32,
        audio_url: Option<&str>,
        position: (f32, f32, f32),
    ) -> AmxResult<bool> {
        let Some(url) = audio_url else {
            return Ok(true);
        };

        let DisplayTargetMethod::Area {
            listeners,
            audio_range,
            ..
        } = self
        else {
            return Ok(false);
        };

        if listeners.contains(&player_id) {
            return Ok(true);
        }

        listeners.push(player_id);
        amx_natives::play_audio_stream_for_player(
            amx,
            &player_id,
            url,
            Some(position),
            *audio_range,
        )?;

        Ok(true)
    }

    pub fn remove_area_listener(&mut self, amx: &Amx, player_id: i32) -> AmxResult<bool> {
        let DisplayTargetMethod::Area { listeners, .. } = self else {
            return Ok(false);
        };

        let Some(index) = listeners.iter().position(|id| *id == player_id) else {
            return Ok(true);
        };

        listeners.remove(index);
        amx_natives::stop_audio_stream_for_player(amx, player_id)?;
        Ok(true)
    }

    pub fn sync_area_listeners(
        &mut self,
        amx: &Amx,
        audio_url: Option<&str>,
        position: (f32, f32, f32),
    ) -> AmxResult<()> {
        let Some(url) = audio_url else {
            return Ok(());
        };

        let DisplayTargetMethod::Area {
            area_id,
            listeners,
            audio_range,
        } = self
        else {
            return Ok(());
        };

        let players = amx_natives::get_players(amx, 1000).unwrap_or_default();
        let mut inside_players = Vec::new();
        for player_id in players {
            let inside = amx_natives::is_player_in_dynamic_area(amx, player_id, *area_id, 1)
                .map(|result| result != 0)
                .unwrap_or(false);
            if inside {
                inside_players.push(player_id);
            }
        }

        for player_id in inside_players.iter().copied() {
            if !listeners.contains(&player_id) {
                info!(
                    "DisplayTargetMethod::sync_area_listeners -> player {} entered audio area {}",
                    player_id, *area_id
                );
                listeners.push(player_id);
                amx_natives::play_audio_stream_for_player(
                    amx,
                    &player_id,
                    url,
                    Some(position),
                    *audio_range,
                )?;
            }
        }

        let mut index = 0;
        while index < listeners.len() {
            if inside_players.contains(&listeners[index]) {
                index += 1;
            } else {
                let player_id = listeners.remove(index);
                info!(
                    "DisplayTargetMethod::sync_area_listeners -> player {} left audio area {}",
                    player_id, *area_id
                );
                amx_natives::stop_audio_stream_for_player(amx, player_id)?;
            }
        }

        Ok(())
    }
}

pub trait Screen {
    /// Paint-ready content for one decoded moment.
    type Frame: Clone;
    /// Whatever a decode needs besides the URL - e.g. the tile grid for
    /// `Screen3D`'s mosaic. `()` for a screen kind with nothing extra to say.
    type DecodeConfig;

    fn decode_dimensions(config: &Self::DecodeConfig) -> (u32, u32);

    fn build_frame(
        canvas: &[u8],
        delay: Duration,
        config: &Self::DecodeConfig,
    ) -> (Self::Frame, Duration);

    /// Decodes `url` into one `(Frame, Duration)` per frame of the clip.
    fn load_clip(
        url: &str,
        sender: Sender<(Self::Frame, Duration)>,
        config: &Self::DecodeConfig,
    ) -> Result<(), String> {
        let (width, height) = Self::decode_dimensions(config);
        let raw_frames = video::load_frames(url, width, height)?;
        if raw_frames.is_empty() {
            return Err("ffmpeg produced no frames".to_string());
        }

        let frame_count = raw_frames.len();
        for raw in raw_frames {
            if sender
                .send(Self::build_frame(&raw.data, raw.delay, config))
                .is_err()
            {
                info!("Screen::load_clip -> receiver disconnected");
                return Ok(());
            }
        }

        info!(
            "Screen::load_clip -> built and sent {} frame(s)",
            frame_count
        );
        Ok(())
    }

    /// Decodes a live stream at `url`, sending each frame to `sender` as it
    /// arrives, until the stream ends or the receiver disconnects.
    fn stream_clip(
        url: &str,
        sender: SyncSender<(Self::Frame, Duration)>,
        config: &Self::DecodeConfig,
    ) -> Result<(), String> {
        let (width, height) = Self::decode_dimensions(config);
        video::stream_frames(url, width, height, |raw| {
            match sender.try_send(Self::build_frame(&raw.data, raw.delay, config)) {
                Ok(()) => FrameOutcome::Sent,
                Err(TrySendError::Full(_)) => FrameOutcome::Dropped,
                Err(TrySendError::Disconnected(_)) => FrameOutcome::Disconnected,
            }
        })
    }

    fn animation_mut(&mut self) -> &mut Animation<Self::Frame>;

    fn step_paint(&mut self, amx: &Amx, budget: &mut NetworkBudget);
    fn amx_ident(&self) -> AmxIdent;
    fn destroy_screen(&self, amx: &Amx);

    fn destroy(&self, amx: &Amx) {
        self.destroy_screen(amx);
        self.destroy_target_audio(amx);
        self.destroy_area(amx);

        if let Some(path) = self.audio_relay_path() {
            audio_server::stop_live_source(path);
        }
    }

    fn destroy_target_audio(&self, amx: &Amx) {
        let mut target_method = self.get_display_target_method().clone();
        if let Err(err) = target_method.stop_audio(amx) {
            info!(
                "Screen::destroy -> failed to stop listener audio: {:?}",
                err
            );
        }
    }

    fn destroy_area(&self, amx: &Amx) {
        if let Some(area_id) = self.get_display_target_method().area_id() {
            if let Err(err) = amx_natives::destroy_dynamic_area(amx, area_id) {
                info!(
                    "Screen::destroy -> failed to destroy audio area {}: {:?}",
                    area_id, err
                );
            }
        }
    }

    fn handle_area_enter(&mut self, amx: &Amx, player_id: i32, area_id: i32) -> AmxResult<bool> {
        if self.get_display_target_method().area_id() != Some(area_id) {
            return Ok(false);
        }

        let audio_url = self.audio_url().clone();
        let position = self.get_position().position();
        self.display_target_method_mut().add_area_listener(
            amx,
            player_id,
            audio_url.as_deref(),
            position,
        )
    }

    fn handle_area_leave(&mut self, amx: &Amx, player_id: i32, area_id: i32) -> AmxResult<bool> {
        if self.get_display_target_method().area_id() != Some(area_id) {
            return Ok(false);
        }

        self.display_target_method_mut()
            .remove_area_listener(amx, player_id)
    }

    fn get_display_target_method(&self) -> &DisplayTargetMethod;
    fn set_display_target_method(&mut self, method: DisplayTargetMethod);
    fn display_target_method_mut(&mut self) -> &mut DisplayTargetMethod;

    fn audio_url(&self) -> &Option<String>;
    fn set_audio_url(&mut self, url: String);
    fn audio_relay_path(&self) -> Option<&str> {
        None
    }
    fn set_audio_relay_path(&mut self, _path: String) {}

    fn start_audio(&mut self, amx: &Amx) -> AmxResult<()> {
        let audio_url = self.audio_url().clone();
        let position = self.get_position().position();
        if let Some(url) = audio_url.as_deref() {
            self.display_target_method_mut()
                .start_audio(amx, url, position)?;
        }
        Ok(())
    }

    fn before_step_paint(&mut self, _amx: &Amx) -> AmxResult<()> {
        Ok(())
    }

    /// Drives animation playback for one server tick.
    fn tick(&mut self, amx: &Amx, budget: &mut NetworkBudget) -> AmxResult<()> {
        if !self.has_started() {
            self.start_audio(amx)?;
            self.set_started();
        }

        self.before_step_paint(amx)?;
        self.step_paint(amx, budget);
        Ok(())
    }

    fn get_position(&self) -> &WorldPosition;

    fn should_build_next_frame(&mut self) -> bool {
        match self.animation_mut().screen_animation.as_mut() {
            Some(animation) => animation.should_build_next_frame(),
            None => false,
        }
    }

    fn build_animation_frame(&mut self) -> Option<(Self::Frame, Duration)> {
        self.animation_mut()
            .screen_animation
            .as_mut()?
            .build_frame()
    }

    fn set_animation(&mut self, animation: ScreenAnimation<Self::Frame>) {
        self.animation_mut().screen_animation = Some(animation);
    }
    fn has_started(&self) -> bool;
    fn set_started(&mut self);
}
