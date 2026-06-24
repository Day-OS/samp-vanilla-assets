use std::ops::Range;
use std::time::Duration;

use crate::constants::{CANVAS_HEIGHT, CANVAS_WIDTH, CLIP_RESOLUTION_LEVELS, LAYERS_PER_BUFFER};
use crate::content_sources::pixel::{downsample_to_argb, extract_region, quantize_pixels};

const MAX_TEXT_LEN: usize = 2048;

pub type Frame3D = Vec<Frame3DMaterial>;

#[derive(Clone)]
pub struct Frame3DMaterial {
    pub layers: Vec<Frame3DMaterialLayer>,
}

#[derive(Clone)]
pub struct Frame3DMaterialLayer {
    pub material_index: i32,
    pub text: String,
    pub font_size: i32,
}

pub fn build_color_embedded_bands(pixels: &[Vec<i32>]) -> Vec<String> {
    let costs = row_costs(pixels);
    pack_bands(&costs)
        .iter()
        .map(|range| render_band(pixels, range))
        .collect()
}

pub fn pack_bands(row_costs: &[usize]) -> Vec<Range<usize>> {
    let row_count = row_costs.len();
    if row_count == 0 {
        return Vec::new();
    }

    let content_budget = MAX_TEXT_LEN.saturating_sub(row_count - 1);

    let mut bands = Vec::new();
    let mut band_start = 0;
    let mut band_cost = 0;

    for (row_index, &cost) in row_costs.iter().enumerate() {
        if band_cost > 0 && band_cost + cost > content_budget {
            bands.push(band_start..row_index);
            band_start = row_index;
            band_cost = 0;
        }
        band_cost += cost;
    }
    bands.push(band_start..row_count);

    bands
}

pub fn row_costs(pixels: &[Vec<i32>]) -> Vec<usize> {
    pixels.iter().map(|row| row_cost(row)).collect()
}

fn row_cost(row: &[i32]) -> usize {
    let mut last_color: Option<i32> = None;
    let mut cost = 0;

    for &pixel in row {
        let alpha = (pixel >> 24) & 0xFF;
        if alpha < 128 {
            cost += 1;
            continue;
        }

        let rgb = pixel & 0x00FF_FFFF;
        if last_color != Some(rgb) {
            cost += 8;
            last_color = Some(rgb);
        }
        cost += 1;
    }

    cost
}

fn render_band(pixels: &[Vec<i32>], range: &Range<usize>) -> String {
    let mut text = String::new();

    for (row_index, row) in pixels.iter().enumerate() {
        if row_index > 0 {
            text.push('\n');
        }

        if !range.contains(&row_index) {
            continue;
        }

        let mut last_color: Option<i32> = None;
        for &pixel in row {
            let alpha = (pixel >> 24) & 0xFF;
            if alpha < 128 {
                text.push(' ');
                continue;
            }

            let rgb = pixel & 0x00FF_FFFF;
            if last_color != Some(rgb) {
                let color_hex = format!("{:06X}", rgb);
                text.push('{');
                text.push_str(&color_hex);
                text.push('}');
                last_color = Some(rgb);
            }
            text.push('n');
        }
    }

    text
}

// Each color change costs ~8 chars (`{RRGGBB}`), so a busy/colorful region
// can blow the text budget at a resolution that a flat-colored region fits
// easily. Before giving up on a resolution level, these steps progressively
// merge nearby colors (0 = full color) to cut down on color-change count
const QUANTIZE_STEPS: &[i32] = &[0, 8, 16, 24, 32, 48, 64, 80, 112];

fn pick_quality(regions: &[Vec<u8>], tile_w: usize, tile_h: usize) -> (usize, usize, i32, i32) {
    let mut worst_case = None;

    for &(rows, cols, font_size) in CLIP_RESOLUTION_LEVELS {
        let pixel_grids: Vec<_> = regions
            .iter()
            .map(|region| downsample_to_argb(region, tile_w, tile_h, rows, cols))
            .collect();

        for &quantize_step in QUANTIZE_STEPS {
            let fits = pixel_grids.iter().all(|pixels| {
                let quantized = if quantize_step == 0 {
                    None
                } else {
                    Some(quantize_pixels(pixels, quantize_step))
                };
                let costs = row_costs(quantized.as_ref().unwrap_or(pixels));
                pack_bands(&costs).len() <= LAYERS_PER_BUFFER
            });

            if fits {
                return (rows, cols, font_size, quantize_step);
            }

            worst_case = Some((rows, cols, font_size, quantize_step));
        }
    }

    // Nothing predicted to fit even at the lowest quality - the caller still
    // builds that last candidate, oversized bands and all.
    worst_case.expect("CLIP_RESOLUTION_LEVELS and QUANTIZE_STEPS must not be empty")
}

pub fn build_frame3d(
    canvas: &[u8],
    delay: Duration,
    tile_cols: usize,
    tile_rows: usize,
) -> (Frame3D, Duration) {
    let tile_w = CANVAS_WIDTH as usize;
    let tile_h = CANVAS_HEIGHT as usize;
    let canvas_width = tile_w * tile_cols;

    let mut regions = Vec::with_capacity(tile_rows * tile_cols);
    for tile_row in 0..tile_rows {
        for tile_col in 0..tile_cols {
            let region_x = tile_col * tile_w;
            let region_y = (tile_rows - 1 - tile_row) * tile_h;
            regions.push(extract_region(
                canvas,
                canvas_width,
                region_x,
                region_y,
                tile_w,
                tile_h,
            ));
        }
    }

    let (rows, cols, font_size, quantize_step) = pick_quality(&regions, tile_w, tile_h);

    let mut region_materials = vec![];
    for region in &regions {
        let pixels = downsample_to_argb(region, tile_w, tile_h, rows, cols);
        let quantized = if quantize_step == 0 {
            None
        } else {
            Some(quantize_pixels(&pixels, quantize_step))
        };
        let bands = build_color_embedded_bands(quantized.as_ref().unwrap_or(&pixels));
        let mut layers: Vec<Frame3DMaterialLayer> = vec![];
        for (material_index, text) in bands.into_iter().enumerate() {
            let layer = Frame3DMaterialLayer {
                material_index: material_index as i32,
                text,
                font_size,
            };
            layers.push(layer);
        }
        region_materials.push(Frame3DMaterial { layers });
    }

    (region_materials, delay)
}
