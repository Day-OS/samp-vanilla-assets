use std::sync::atomic::Ordering;

use log::info;
use samp::native;
use samp::prelude::*;

use crate::amx_natives;
use crate::animation::ScreenAnimation;
use crate::audio_server;
use crate::constants::{ANIMATION_RING_SIZE, AUDIO_BASE_URL};
use crate::content_sources::yt_resolver;
use crate::screen::Screen;
use crate::screen_3d::Screen3D;
use crate::{AUDIO_CLIP_COUNTER, AnyScreen, Plugin};

fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

impl Plugin {
    #[native(name = "Create3DMediaScreen")]
    pub fn create_3d_media_screen(
        &mut self,
        amx: &Amx,
        url: AmxString,
        x: f32,
        y: f32,
        z: f32,
        rotation_x: f32,
        rotation_y: f32,
        rotation_z: f32,
        tile_cols: i32,
        tile_rows: i32,
        player_id: i32,
        world_id: i32,
        interior_id: i32,
        audio_range: f32,
        hidden_x: f32,
        hidden_y: f32,
        hidden_z: f32,
    ) -> AmxResult<i32> {
        let url = url.to_string();
        let tile_grid = (tile_cols as usize, tile_rows as usize);

        let world_position = crate::engine::WorldPosition {
            position_x: x,
            position_y: y,
            position_z: z,
            rotation_x,
            rotation_y,
            rotation_z,
            world_id: world_id,
            interior_id: interior_id,
        };
        let w_hidden_position = crate::engine::WorldPosition {
            position_x: hidden_x,
            position_y: hidden_y,
            position_z: hidden_z,
            rotation_x,
            rotation_y,
            rotation_z,
            world_id,
            interior_id,
        };
        let target_method = if player_id >= 0 {
            crate::screen::DisplayTargetMethod::player(player_id, audio_range)
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
                "Create3DMediaScreen -> created audio area {} at ({:.2}, {:.2}, {:.2}) range={:.2}",
                area_id, x, y, z, audio_range
            );
            let listeners = players_in_area(amx, area_id);
            info!(
                "Create3DMediaScreen -> audio area {} initial listeners={:?}",
                area_id, listeners
            );
            crate::screen::DisplayTargetMethod::area(area_id, listeners, audio_range)
        };

        let screen_index = Screen3D::new(
            amx,
            &mut self.screens,
            world_position,
            w_hidden_position,
            ANIMATION_RING_SIZE,
            tile_grid,
            target_method,
        )?;

        self.start_media_playback(screen_index, url, tile_grid)?;

        Ok(screen_index as i32)
    }

    /// Destroys every object backing the screen at `screen_index` (the value
    /// `Create3DMediaScreen` returned) and frees the slot for reuse-by-index
    /// checks - the index itself stays reserved (never recycled to a
    /// different screen) so any clip still decoding in the background for it
    /// just gets silently dropped instead of attaching to the wrong screen.
    #[native(name = "Destroy3DMediaScreen")]
    pub fn destroy_3d_media_screen(&mut self, amx: &Amx, screen_index: i32) -> AmxResult<i32> {
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

    /// Starts media playback for a screen. Fixed clips keep the predecoded,
    /// time-indexed video path; live sources stream frames into the same
    /// animation state. Audio is always served through the relay path so both
    /// modes share the same playback behavior.
    fn start_media_playback(
        &mut self,
        screen_index: usize,
        url: String,
        tile_grid: (usize, usize),
    ) -> AmxResult<()> {
        let is_live_source = is_youtube_url(&url);

        match self.screens[screen_index].as_ref() {
            Some(AnyScreen::ThreeD(_)) => {}
            None => {
                info!(
                    "start_media_playback -> screen {} vanished before decode could start",
                    screen_index
                );
                return Ok(());
            }
        }

        if is_live_source {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);

            let resolved = match yt_resolver::resolve_stream_url(&url) {
                Ok(resolved) => resolved,
                Err(err) => {
                    log::error!(
                        "Create3DMediaScreen -> failed to resolve youtube url {}: {}",
                        url,
                        err
                    );
                    return Ok(());
                }
            };

            self.attach_audio_if_present(screen_index, &resolved, true);
            log::info!(
                "Create3DMediaScreen -> resolved youtube url {} to {}, starting stream",
                url,
                resolved
            );

            std::thread::spawn(move || {
                if let Err(err) = Screen3D::stream_clip(&resolved, tx, &tile_grid) {
                    info!("Create3DMediaScreen -> video stream ended: {}", err);
                }
            });

            if let Some(AnyScreen::ThreeD(screen)) = self.screens[screen_index].as_mut() {
                screen.set_animation(ScreenAnimation::from_frame_stream(rx, false));
            }
        } else {
            let (tx, rx) = std::sync::mpsc::channel();

            self.attach_audio_if_present(screen_index, &url, true);

            std::thread::spawn(move || {
                if let Err(err) = Screen3D::load_clip(&url, tx, &tile_grid) {
                    info!("Create3DMediaScreen -> clip decode failed: {}", err);
                }

                info!("Create3DMediaScreen -> clip decode thread finished");
            });

            if let Some(AnyScreen::ThreeD(screen)) = self.screens[screen_index].as_mut() {
                screen.set_animation(ScreenAnimation::from_frame_stream(rx, true));
            }
        }

        info!(
            "Create3DMediaScreen -> screen {} created, media starting in background",
            screen_index
        );
        Ok(())
    }

    fn attach_audio_if_present(&mut self, screen_index: usize, source_url: &str, loops: bool) {
        if !audio_server::source_has_audio(source_url) {
            info!("Create3DMediaScreen -> source has no audio stream, skipping audio relay");
            return;
        }

        let audio_id = AUDIO_CLIP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let audio_relay_path = format!("clip_{}.mp3", audio_id);
        let audio_url = format!("{}/{}", AUDIO_BASE_URL, audio_relay_path);

        audio_server::register_live_source(audio_relay_path, source_url.to_string(), loops);

        if let Some(AnyScreen::ThreeD(screen)) = self.screens[screen_index].as_mut() {
            screen.set_audio_url(audio_url);
        }
    }

    #[native(name = "SVA_AreaListenerOnPlayerEnter")]
    pub fn sva_area_listener_on_player_enter(
        &mut self,
        amx: &Amx,
        playerid: i32,
        areaid: i32,
    ) -> AmxResult<i32> {
        info!(
            "SVA_AreaListenerOnPlayerEnter: playerid={}, areaid={}",
            playerid, areaid
        );

        for screen in self.screens.iter_mut().flatten() {
            match screen {
                AnyScreen::ThreeD(screen) => {
                    if screen.handle_area_enter(amx, playerid, areaid)? {
                        return Ok(1);
                    }
                }
            }
        }

        Ok(0)
    }

    #[native(name = "SVA_AreaListenerOnPlayerLeave")]
    pub fn sva_area_listener_on_player_leave(
        &mut self,
        amx: &Amx,
        playerid: i32,
        areaid: i32,
    ) -> AmxResult<i32> {
        info!(
            "SVA_AreaListenerOnPlayerLeave: playerid={}, areaid={}",
            playerid, areaid
        );

        for screen in self.screens.iter_mut().flatten() {
            match screen {
                AnyScreen::ThreeD(screen) => {
                    if screen.handle_area_leave(amx, playerid, areaid)? {
                        return Ok(1);
                    }
                }
            }
        }

        Ok(0)
    }
}

fn players_in_area(amx: &Amx, area_id: i32) -> Vec<i32> {
    let Ok(players) = amx_natives::get_players(amx, 1000) else {
        return Vec::new();
    };

    players
        .into_iter()
        .filter(|player_id| {
            amx_natives::is_player_in_dynamic_area(amx, *player_id, area_id, 1)
                .map(|inside| inside != 0)
                .unwrap_or(false)
        })
        .collect()
}
