use samp::native;
use samp::prelude::*;

use crate::Plugin;
use crate::amx_natives;
use crate::constants::{
    GHOST_PREVIEW_COLOR, GRID_COLS, GRID_FONT, GRID_FONT_SIZE, GRID_ROWS, MATERIAL_SIZE_512X512,
    TRANSPARENT_ARGB,
};
use crate::screen_3d::screen_buffer::ScreenBuffer;

pub fn paint_screen_decoy(amx: &Amx, screen: &ScreenBuffer) -> AmxResult<()> {
    let grid = grid_text();

    for tile in screen.tiles.iter() {
        amx_natives::set_dynamic_object_material_text(
            amx,
            tile.object_id,
            0,
            &grid,
            MATERIAL_SIZE_512X512,
            GRID_FONT,
            GRID_FONT_SIZE,
            1,
            GHOST_PREVIEW_COLOR,
            TRANSPARENT_ARGB,
            0,
        )?;
    }

    Ok(())
}

pub fn create_screen_decoy(
    amx: &Amx,
    world_position: &crate::engine::WorldPosition,
    tile_cols: usize,
    tile_rows: usize,
    player_id: &Option<i32>,
) -> AmxResult<Vec<i32>> {
    let decoy = ScreenBuffer::new(amx, world_position, tile_cols, tile_rows, player_id)?;
    let object_ids: Vec<i32> = decoy.tiles.iter().map(|tile| tile.object_id).collect();
    paint_screen_decoy(amx, &decoy)?;

    Ok(object_ids)
}

pub fn destroy_screen_decoy(amx: &Amx, object_ids: &[i32]) {
    for object_id in object_ids {
        if let Err(err) = amx_natives::destroy_dynamic_object(amx, *object_id) {
            log::info!(
                "destroy_screen_decoy -> failed to destroy object {}: {:?}",
                object_id,
                err
            );
        }
    }
}

impl Plugin {
    /// Spawns a screen-shaped preview painted with a placeholder grid so the
    /// player can drag/rotate the same tile layout that the final screen will
    /// use. Pawn owns the edit flow; Rust keeps the child tile ids so the
    /// whole preview can be destroyed cleanly afterwards.
    #[native(name = "Create3DMediaScreenPreview")]
    pub fn create_3d_media_screen_preview(
        &mut self,
        amx: &Amx,
        x: f32,
        y: f32,
        z: f32,
        rotation_x: f32,
        rotation_y: f32,
        rotation_z: f32,
        tile_cols: i32,
        tile_rows: i32,
        world_id: i32,
        interior_id: i32,
        player: i32,
    ) -> AmxResult<i32> {
        let player_id: Option<i32> = if player >= 0 { Some(player) } else { None };
        let world_position = crate::engine::WorldPosition {
            position_x: x,
            position_y: y,
            position_z: z,
            rotation_x,
            rotation_y,
            rotation_z,
            world_id,
            interior_id,
        };
        let tile_cols = tile_cols.max(1) as usize;
        let tile_rows = tile_rows.max(1) as usize;
        let object_ids =
            create_screen_decoy(amx, &world_position, tile_cols, tile_rows, &player_id)?;

        let anchor_object_id = object_ids[0];
        self.placement_previews.push(object_ids);
        Ok(anchor_object_id)
    }

    #[native(name = "Destroy3DMediaScreenPreview")]
    pub fn destroy_3d_media_screen_preview(
        &mut self,
        amx: &Amx,
        anchor_object_id: i32,
    ) -> AmxResult<i32> {
        let index = match self
            .placement_previews
            .iter()
            .position(|object_ids| object_ids.first() == Some(&anchor_object_id))
        {
            Some(index) => index,
            None => return Ok(0),
        };

        let object_ids = self.placement_previews.remove(index);
        destroy_screen_decoy(amx, &object_ids);

        Ok(1)
    }
}

fn grid_text() -> String {
    let mut text = String::with_capacity(GRID_ROWS * (GRID_COLS + 1));
    for row in 0..GRID_ROWS {
        if row > 0 {
            text.push('\n');
        }
        for col in 0..GRID_COLS {
            text.push(if row % 6 == 0 || col % 6 == 0 {
                'n'
            } else {
                ' '
            });
        }
    }
    text
}
