use std::time::Duration;

pub const MATERIAL_SIZE_512X512: i32 = 140;
pub const TRANSPARENT_ARGB: i32 = 0x00000000;
pub const GRID_FONT: &str = "Wingdings";

pub const TARGET_FPS: u64 = 15;
pub const VIDEO_OUTPUT_FPS: u64 = TARGET_FPS;

pub const AUDIO_START_LATENCY_COMPENSATION: Duration = Duration::from_millis(800);

pub const ANIMATION_RING_SIZE: usize = 8;

pub const GHOST_PREVIEW_COLOR: i32 = 0xFF33FF66u32 as i32;

pub const GRID_ROWS: usize = 54;
pub const GRID_COLS: usize = 36;
pub const GRID_FONT_SIZE: i32 = 10;

pub const CLIP_RESOLUTION_LEVELS: &[(usize, usize, i32)] =
    &[(108, 72, 5), (54, 36, 10), (27, 18, 20), (14, 9, 40)];

// Decoded frames are letterboxed (never stretched/cropped) into this exact
// aspect ratio so downsample_to_argb's row/col -> width/height mapping lines
// up with the screen's own grid shape instead of distorting the source media.
pub const CANVAS_WIDTH: u32 = 240;
pub const CANVAS_HEIGHT: u32 = CANVAS_WIDTH * GRID_COLS as u32 / GRID_ROWS as u32;

pub const LAYERS_PER_BUFFER: usize = 16;

// Relative weights for the shared `NetworkBudget` token bucket.
// SetObjectPos/AttachObjectToObject only carry a handful of floats, so a
// flat per-call weight is fine for those. SetObjectMaterialText's payload
// is a text string whose length varies a lot with how colorful/detailed the
// source frame is (more distinct colors -> more layers/longer text), so its
// real network cost is billed per character actually sent rather than a
// flat per-call guess - see `process_pending_paint` in screen_3d/mod.rs.
// `MATERIAL_PAINT_COST_ESTIMATE` is only used to size how many paints to
// attempt per tick before their real lengths are known.
pub const MATERIAL_PAINT_COST_ESTIMATE: f64 = 5.0;
pub const MATERIAL_PAINT_COST_PER_CHAR: f64 = 0.05;
pub const POSITION_UPDATE_COST: f64 = 1.0;

// Network budget rate/capacity and the screen model/DFF/TXD ids are
// configurable via SVA_Config.toml now - see `config.rs`.

// A single screen tile is already maxed out (16 materials, 512x512 texture,
// ~2000 char budget) - one object physically cannot show more detail. To get
// more resolution, `Create3DMediaScreen` callers pass a tile_cols x tile_rows
// grid of full-budget objects instead, each rendering its own slice of the
// source media at full per-tile quality. Columns run along the wall's
// horizontal (right) axis, rows along its vertical (up) axis.

// Physical size (game units) of one screen tile, used to lay tiles out
// edge-to-edge. This is a guess, not measured from screen.dff - check the
// mosaic in-game and adjust until tiles line up with no gap/overlap.
pub const TILE_WIDTH: f32 = 0.471;
pub const TILE_HEIGHT: f32 = 0.312;
