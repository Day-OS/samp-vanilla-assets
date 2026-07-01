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
use frame::{TD_STRING_MAX, TextDrawFrame, build_frame, compute_colour_levels, layer_count, layer_rgba};

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
    /// [layer_index][chunk_index] = PlayerText id
    textdraw_ids: Vec<Vec<i32>>,
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
        for layer_ids in &self.textdraw_ids {
            for &td_id in layer_ids {
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

    fn get_display_target_method(&self) -> &DisplayTargetMethod {
        &self.target_method
    }
    fn set_display_target_method(&mut self, m: DisplayTargetMethod) {
        self.target_method = m;
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
        let colour_levels = compute_colour_levels(grid_cols, grid_rows, budget);
        let layer_count = layer_count(colour_levels);
        let chunk_rows = (TD_STRING_MAX + 3) / (grid_cols + 3);
        let chunk_count = (grid_rows + chunk_rows - 1) / chunk_rows;
        let placeholder = ".".repeat(grid_cols);
        let mut textdraw_ids = Vec::with_capacity(layer_count);

        // letter_size_y is in "letter units"; textdraw screen coordinates use a different
        // scale. From the Pawn reference (letter_size_y=0.35 → row_y_step=3.0):
        // row_y_step = letter_size_y * (3.0 / 0.35)
        let row_y_step = letter_size_y * (3.0 / 0.35);

        // Solid black background box — created first so it sits below all pixel layers.
        // Box height in SA:MP is driven by number of text lines × letter_size_y, not TextSize.y.
        // We build a text with grid_rows lines (one space per line separated by ~n~) so the box
        // naturally matches the pixel grid height. TextSize.x sets the right edge.
        // X: box_scale tunes the right edge — increase if box is too narrow, decrease if too wide.
        let total_width = grid_cols as f32 * letter_size_x * box_scale;
        let bg_text = std::iter::repeat(" ").take(grid_rows).collect::<Vec<_>>().join("~n~");
        let bg_id = amx_natives::create_player_text_draw(amx, player_id, hud_x, hud_y, &bg_text)?;
        amx_natives::player_text_draw_use_box(amx, player_id, bg_id, 1)?;
        amx_natives::player_text_draw_box_color(amx, player_id, bg_id, 0x000000FFu32 as i32)?;
        amx_natives::player_text_draw_letter_size(amx, player_id, bg_id, letter_size_x, letter_size_y)?;
        amx_natives::player_text_draw_text_size(amx, player_id, bg_id, hud_x + total_width, 0.0)?;
        amx_natives::player_text_draw_show(amx, player_id, bg_id)?;

        for layer in 0..layer_count {
            let color = layer_rgba(layer, colour_levels) as i32;
            let mut layer_ids = Vec::with_capacity(chunk_count);

            for chunk in 0..chunk_count {
                let chunk_y = hud_y + (chunk as f32) * (chunk_rows as f32) * row_y_step;
                let td_id = amx_natives::create_player_text_draw(amx, player_id, hud_x, chunk_y, &placeholder)?;
                amx_natives::player_text_draw_letter_size(amx, player_id, td_id, letter_size_x, letter_size_y)?;
                amx_natives::player_text_draw_text_size(amx, player_id, td_id, (grid_cols as f32) * letter_size_x, 0.0)?;
                amx_natives::player_text_draw_alignment(amx, player_id, td_id, TD_ALIGN_LEFT)?;
                amx_natives::player_text_draw_colour(amx, player_id, td_id, color)?;
                amx_natives::player_text_draw_background_colour(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_set_outline(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_set_shadow(amx, player_id, td_id, 0)?;
                amx_natives::player_text_draw_font(amx, player_id, td_id, TD_FONT)?;
                amx_natives::player_text_draw_show(amx, player_id, td_id)?;
                layer_ids.push(td_id);
            }
            textdraw_ids.push(layer_ids);
        }

        let screen = TextDrawScreen {
            amx_ident: amx.ident(),
            bg_textdraw_id: bg_id,
            player_id,
            textdraw_ids,
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
        for (layer, (layer_ids, layer_chunks)) in self
            .textdraw_ids
            .iter()
            .zip(frame.layer_chunks.iter())
            .enumerate()
        {
            for (chunk, (td_id, text)) in layer_ids.iter().zip(layer_chunks.iter()).enumerate() {
                if let Err(e) = amx_natives::player_text_draw_set_string(amx, self.player_id, *td_id, text) {
                    info!("TextDrawScreen::paint layer={} chunk={} set_string failed: {:?}", layer, chunk, e);
                }
                if let Err(e) = amx_natives::player_text_draw_show(amx, self.player_id, *td_id) {
                    info!("TextDrawScreen::paint layer={} chunk={} show failed: {:?}", layer, chunk, e);
                }
            }
        }
    }
}
