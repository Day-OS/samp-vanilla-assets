use log::info;
use samp::amx::Amx;
use samp::error::AmxResult;
use samp::native;
use samp::prelude::*;

use crate::amx_natives;
use crate::commands::start_audio_source_playback_for;
use crate::engine::WorldPosition;
use crate::screen::DisplayTargetMethod;
use crate::{AnyScreen, Plugin};

use super::AudioSource;

impl Plugin {
    /// Same player-targeted-vs-area-based split as `Create3DMediaScreen`,
    /// minus every bit of visual machinery - just a position and a sound.
    #[native(name = "CreateAudioSource")]
    pub fn create_audio_source(
        &mut self,
        amx: &Amx,
        url: AmxString,
        x: f32,
        y: f32,
        z: f32,
        player_id: i32,
        world_id: i32,
        interior_id: i32,
        audio_range: f32,
    ) -> AmxResult<i32> {
        if player_id >= 0 && crate::blacklist::is_audio_blacklisted(player_id) {
            info!(
                "CreateAudioSource -> player {} is blacklisted from audio, skipping",
                player_id
            );
            return Ok(-1);
        }

        let url = url.to_string();
        let world_position = WorldPosition {
            position_x: x,
            position_y: y,
            position_z: z,
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
            world_id,
            interior_id,
        };

        let target_method = if player_id >= 0 {
            DisplayTargetMethod::player(player_id, audio_range)
        } else {
            let area_id = amx_natives::create_dynamic_circle(
                amx,
                x,
                y,
                audio_range,
                world_id,
                interior_id,
                -1,
                0,
            )?;
            info!(
                "CreateAudioSource -> created audio area {} at ({:.2}, {:.2}, {:.2}) range={:.2}",
                area_id, x, y, z, audio_range
            );
            let listeners = amx_natives::players_in_area(amx, area_id);
            DisplayTargetMethod::area(area_id, listeners, audio_range)
        };

        let screen_index = AudioSource::new(amx, &mut self.screens, world_position, target_method);

        match self.screens[screen_index].as_mut() {
            Some(AnyScreen::Audio(screen)) => {
                start_audio_source_playback_for(screen, url, "AudioSource")?;
            }
            _ => {
                info!(
                    "CreateAudioSource -> screen {} vanished before audio playback could start",
                    screen_index
                );
            }
        }

        Ok(screen_index as i32)
    }

    #[native(name = "DestroyAudioSource")]
    pub fn destroy_audio_source(&mut self, amx: &Amx, screen_index: i32) -> AmxResult<i32> {
        if screen_index < 0 {
            return Ok(0);
        }

        match self.screens.get_mut(screen_index as usize) {
            Some(slot) => match slot.take() {
                Some(screen) => {
                    screen.destroy(amx);
                    Ok(1)
                }
                None => Ok(0),
            },
            None => Ok(0),
        }
    }
}
