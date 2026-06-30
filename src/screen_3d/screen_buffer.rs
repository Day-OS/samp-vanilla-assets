use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use log::info;
use samp::amx::Amx;
use samp::error::AmxResult;

use crate::amx_natives;
use crate::constants::{
    CUSTOM_SCREEN_BASE_MODEL, CUSTOM_SCREEN_DFF, CUSTOM_SCREEN_MODEL, CUSTOM_SCREEN_TXD, GRID_FONT,
    GRID_FONT_SIZE, LAYERS_PER_BUFFER, MATERIAL_SIZE_512X512, TILE_HEIGHT, TILE_WIDTH,
    TRANSPARENT_ARGB, CUSTOM_SCREEN_SHADOW_BASE_MODEL, CUSTOM_SCREEN_SHADOW_DFF,
    CUSTOM_SCREEN_SHADOW_MODEL, CUSTOM_SCREEN_SHADOW_TXD,
};
use crate::engine::WorldPosition;
use crate::screen_3d::frame::{Frame3DMaterial, Frame3DMaterialLayer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenModel {
	Standard = 0,
	Shadow = 1,
}

impl ScreenModel {
	pub fn from_id(id: i32) -> Option<Self> {
		match id {
			0 => Some(ScreenModel::Standard),
			1 => Some(ScreenModel::Shadow),
			_ => None,
		}
	}

	pub fn to_model_id(&self) -> i32 {
		match self {
			ScreenModel::Standard => CUSTOM_SCREEN_MODEL,
			ScreenModel::Shadow => CUSTOM_SCREEN_SHADOW_MODEL,
		}
	}
}

static SCREEN_MODEL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn ensure_screen_model_registered(amx: &Amx) {
    if SCREEN_MODEL_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Err(err) = amx_natives::add_simple_model(
        amx,
        -1, // virtualWorld: all
        CUSTOM_SCREEN_BASE_MODEL,
        CUSTOM_SCREEN_MODEL,
        CUSTOM_SCREEN_DFF,
        CUSTOM_SCREEN_TXD,
    ) {
        info!("failed to register custom screen model: {:?}", err);
    }
    if let Err(err) = amx_natives::add_simple_model(
        amx,
        -1, // virtualWorld: all
        CUSTOM_SCREEN_SHADOW_BASE_MODEL,
        CUSTOM_SCREEN_SHADOW_MODEL,
        CUSTOM_SCREEN_SHADOW_DFF,
        CUSTOM_SCREEN_SHADOW_TXD,
    ) {
        info!("failed to register custom screen-shadow model: {:?}", err);
    }
}

const STREAMER_OBJECT_SD: f32 = 300.0;
const STREAMER_OBJECT_DD: f32 = 0.;

pub fn create_wall(
    amx: &Amx,
    world_position: &WorldPosition,
    player_id: &Option<i32>,
    model: ScreenModel,
) -> AmxResult<i32> {
    amx_natives::create_dynamic_object(
        amx,
        model.to_model_id(),
        world_position.position_x,
        world_position.position_y,
        world_position.position_z,
        world_position.rotation_x,
        world_position.rotation_y,
        world_position.rotation_z,
        world_position.world_id,
        world_position.interior_id,
        player_id.unwrap_or(-1),
        STREAMER_OBJECT_SD,
        STREAMER_OBJECT_DD,
        -1,
        0,
    )
}

/// One physical wall object that is part of a [`ScreenBuffer`]. Owns its own
/// paint queue and executes it against its own `object_id` - nothing else
/// needs to carry that id around to get a layer painted.
pub struct ScreenBufferTile {
    pub object_id: i32,
    pub last_painted_layers: usize,
    pending: VecDeque<Frame3DMaterialLayer>,
}

impl ScreenBufferTile {
    pub fn new(object_id: i32) -> Self {
        ScreenBufferTile {
            object_id,
            last_painted_layers: LAYERS_PER_BUFFER,
            pending: VecDeque::new(),
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Queues paint operations for this tile's new layers, plus blanks for
    /// any materials that were painted last time but aren't used anymore.
    pub fn stage_paint(&mut self, layers: &[Frame3DMaterialLayer]) {
        let new_layer_count = layers.len();

        for layer in layers {
            self.pending.push_back(layer.clone());
        }

        for material_index in new_layer_count..self.last_painted_layers {
            self.pending.push_back(Frame3DMaterialLayer {
                material_index: material_index as i32,
                text: " ".to_string(),
                font_size: GRID_FONT_SIZE,
            });
        }

        self.last_painted_layers = new_layer_count;
    }

    /// Executes up to `budget` queued paints against this tile's own
    /// object, returning how many were actually processed and the total
    /// length of text actually sent (the real driver of each call's network
    /// cost, since it varies with the source frame's color complexity).
    pub fn process_pending(&mut self, amx: &Amx, budget: usize) -> (usize, usize) {
        let mut processed = 0;
        let mut chars_sent = 0;

        while processed < budget {
            if let Some(layer) = self.pending.pop_front() {
                chars_sent += layer.text.len();
                if let Err(err) = self.paint(amx, &layer) {
                    info!(
                        "failed to paint pending object {} materialIndex {}: {:?}",
                        self.object_id, layer.material_index, err
                    );
                }
                processed += 1;
            } else {
                break;
            }
        }

        (processed, chars_sent)
    }
    pub fn paint(&self, amx: &Amx, material: &Frame3DMaterialLayer) -> AmxResult<i32> {
        amx_natives::set_dynamic_object_material_text(
            amx,
            self.object_id,
            material.material_index,
            &material.text,
            MATERIAL_SIZE_512X512,
            GRID_FONT,
            material.font_size,
            1,
            TRANSPARENT_ARGB,
            TRANSPARENT_ARGB,
            0,
        )
    }
}

/// A group of [`ScreenBufferTiles`]s objects that together make up a
/// single screen's worth
pub struct ScreenBuffer {
    pub tiles: Vec<ScreenBufferTile>,
    pub tile_cols: usize,
}

impl ScreenBuffer {
    pub fn new(
        amx: &Amx,
        world_position: &WorldPosition,
        tile_cols: usize,
        tile_rows: usize,
        player_id: &Option<i32>,
        model: ScreenModel,
    ) -> AmxResult<ScreenBuffer> {
        let tile_count = tile_rows * tile_cols;
        let mut tiles = Vec::with_capacity(tile_count);

        let anchor_object = create_wall(amx, world_position, player_id, model)?;
        tiles.push(ScreenBufferTile::new(anchor_object));

        for tile_row in 0..tile_rows {
            for tile_col in 0..tile_cols {
                if tile_row == 0 && tile_col == 0 {
                    continue;
                }

                let offset_y = tile_col as f32 * TILE_WIDTH;
                let offset_z = tile_row as f32 * TILE_HEIGHT;

                let object_id = create_wall(amx, world_position, player_id, model)?;
                const SYNC_ROTATION: i32 = 1;
                amx_natives::attach_object_to_object(
                    amx,
                    object_id,
                    anchor_object,
                    0.0,
                    offset_y,
                    offset_z,
                    0.0,
                    0.0,
                    0.0,
                    SYNC_ROTATION,
                )?;

                tiles.push(ScreenBufferTile::new(object_id));
            }
        }

        Ok(ScreenBuffer { tiles, tile_cols })
    }
    pub fn set_position(&self, amx: &Amx, position: (f32, f32, f32)) -> AmxResult<()> {
        let tile_cols = self.tile_cols;

        let anchor_object_id = self.tiles[0].object_id;
        for (tile_index, tile) in self.tiles.iter().enumerate() {
            if tile_index == 0 {
                amx_natives::set_object_pos(
                    amx,
                    tile.object_id,
                    position.0,
                    position.1,
                    position.2,
                )?;
            } else {
                let offset_y = (tile_index % tile_cols) as f32 * TILE_WIDTH;
                let offset_z = (tile_index / tile_cols) as f32 * TILE_HEIGHT;
                amx_natives::attach_object_to_object(
                    amx,
                    tile.object_id,
                    anchor_object_id,
                    0.0,
                    offset_y,
                    offset_z,
                    0.0,
                    0.0,
                    0.0,
                    1,
                )?;
            }
        }
        Ok(())
    }

    /// Stages paint operations for every tile in this buffer - `frame_tiles[i]`
    /// are the layers for `self.tiles[i]`.
    pub fn stage_paint(&mut self, frame_tiles: &Vec<Frame3DMaterial>) {
        for (tile, material) in self.tiles.iter_mut().zip(frame_tiles.iter()) {
            tile.stage_paint(&material.layers);
        }
    }

    pub fn has_pending(&self) -> bool {
        self.tiles.iter().any(ScreenBufferTile::has_pending)
    }

    /// Destroys every tile object that makes up this buffer.
    pub fn destroy(&self, amx: &Amx) {
        for tile in &self.tiles {
            if let Err(err) = amx_natives::destroy_object(amx, tile.object_id) {
                info!("failed to destroy object {}: {:?}", tile.object_id, err);
            }
        }
    }

    /// Spreads up to `budget` queued paints across this buffer's tiles,
    /// returning how many were actually processed and the total length of
    /// text actually sent across all of them.
    pub fn process_pending(&mut self, amx: &Amx, budget: usize) -> (usize, usize) {
        let mut remaining = budget;
        let mut chars_sent = 0;

        for tile in self.tiles.iter_mut() {
            if remaining == 0 {
                break;
            }
            let (processed, tile_chars) = tile.process_pending(amx, remaining);
            remaining -= processed;
            chars_sent += tile_chars;
        }

        (budget - remaining, chars_sent)
    }
}
