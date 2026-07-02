use log::info;
use samp::amx::Amx;
use samp::error::AmxResult;
use samp::native;
use samp::prelude::*;

use crate::amx_natives;
use crate::commands::start_media_playback_for;
use crate::constants::ANIMATION_RING_SIZE;
use crate::engine::WorldPosition;
use crate::screen::DisplayTargetMethod;
use crate::{AnyScreen, Plugin};

use super::Screen3D;

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
        model_id: i32,
    ) -> AmxResult<i32> {
        if player_id >= 0 && crate::blacklist::is_screen_3d_blacklisted(player_id) {
            info!(
                "Create3DMediaScreen -> player {} is blacklisted from screen_3d, skipping",
                player_id
            );
            return Ok(-1);
        }

        let model = crate::screen_3d::screen_buffer::ScreenModel::from_id(model_id)
            .unwrap_or(crate::screen_3d::screen_buffer::ScreenModel::Standard);
        let url = url.to_string();
        let tile_grid = (tile_cols as usize, tile_rows as usize);

        let world_position = WorldPosition {
            position_x: x,
            position_y: y,
            position_z: z,
            rotation_x,
            rotation_y,
            rotation_z,
            world_id,
            interior_id,
        };
        let w_hidden_position = WorldPosition {
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
                "Create3DMediaScreen -> created audio area {} at ({:.2}, {:.2}, {:.2}) range={:.2}",
                area_id, x, y, z, audio_range
            );
            let listeners = amx_natives::players_in_area(amx, area_id);
            info!(
                "Create3DMediaScreen -> audio area {} initial listeners={:?}",
                area_id, listeners
            );
            DisplayTargetMethod::area(area_id, listeners, audio_range)
        };

        let screen_index = Screen3D::new(
            amx,
            &mut self.screens,
            world_position,
            w_hidden_position,
            ANIMATION_RING_SIZE,
            tile_grid,
            target_method,
            model,
        )?;

        match self.screens[screen_index].as_mut() {
            Some(AnyScreen::ThreeD(screen)) => {
                crate::blacklist::hide_new_screen_3d_from_blacklisted_players(amx, screen);
                start_media_playback_for(screen, url, tile_grid, "Screen3D")?;
            }
            _ => {
                info!(
                    "Create3DMediaScreen -> screen {} vanished before media playback could start",
                    screen_index
                );
            }
        }

        Ok(screen_index as i32)
    }

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
            if screen.handle_area_enter(amx, playerid, areaid)? {
                return Ok(1);
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
            if screen.handle_area_leave(amx, playerid, areaid)? {
                return Ok(1);
            }
        }

        Ok(0)
    }
}
