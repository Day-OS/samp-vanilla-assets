use std::time::Duration;

pub const TD_STRING_MAX: usize = 800;
pub const TD_PLAYER_BUDGET: usize = 256;

/// Computes COLOUR_LEVELS from the string budget and grid dimensions.
/// chunk_rows  = (TD_STRING_MAX + 3) / (cols + 3)
/// chunk_count = ceil(rows / chunk_rows)
/// max_layers  = (TD_PLAYER_BUDGET - 1) / chunk_count  (-1 for the background box)
/// colour_levels = (max_layers - 1) / 4               (1 white + 4×G + 4×B + 4×R + 4×K)
pub fn compute_colour_levels(cols: usize, rows: usize, budget: usize) -> usize {
    let chunk_rows = (TD_STRING_MAX + 3) / (cols + 3);
    let chunk_count = (rows + chunk_rows - 1) / chunk_rows;
    let max_layers = (budget.saturating_sub(1)) / chunk_count;
    ((max_layers.saturating_sub(1)) / 4).max(1)
}

pub fn layer_count(colour_levels: usize) -> usize {
    1 + colour_levels * 4
}

#[derive(Clone)]
pub struct TextDrawFrame {
    pub layer_chunks: Vec<Vec<String>>,
}

fn layer_alpha(colour_levels: usize) -> u8 {
    // 85% max stacked coverage: 1 - 0.15^(1/n)
    let a = 1.0_f32 - (0.15_f32).powf(1.0 / colour_levels as f32);
    (a * 255.0).round() as u8
}

pub fn layer_rgba(layer: usize, colour_levels: usize) -> u32 {
    let a = layer_alpha(colour_levels);
    if layer == 0 {
        return 0xFFFFFFFF;
    }
    let l = layer - 1;
    if l < colour_levels {
        u32::from_be_bytes([0x00, 0xFF, 0x00, a])
    } else if l < colour_levels * 2 {
        u32::from_be_bytes([0x00, 0x00, 0xFF, a])
    } else if l < colour_levels * 3 {
        u32::from_be_bytes([0xFF, 0x00, 0x00, a])
    } else {
        u32::from_be_bytes([0x00, 0x00, 0x00, a])
    }
}

fn pixel_on(r: u8, g: u8, b: u8, a: u8, layer: usize, colour_levels: usize) -> bool {
    if a < 128 {
        return false;
    }
    if layer == 0 {
        return true;
    }
    let l = layer - 1;
    let threshold = |level: usize| ((level + 1) * 255 / (colour_levels + 1)) as u8;
    let min_ch = r.min(g).min(b);
    let max_ch = r.max(g).max(b);

    if l < colour_levels * 3 {
        let (channel, level) = if l < colour_levels {
            (g, l)
        } else if l < colour_levels * 2 {
            (b, l - colour_levels)
        } else {
            (r, l - colour_levels * 2)
        };
        let chroma = channel.saturating_sub(min_ch);
        chroma > threshold(level)
    } else {
        let darkness = 255u8.saturating_sub(max_ch);
        darkness > threshold(l - colour_levels * 3)
    }
}

pub fn build_frame(canvas: &[u8], delay: Duration, cols: usize, rows: usize, budget: usize) -> (TextDrawFrame, Duration) {
    let colour_levels = compute_colour_levels(cols, rows, budget);
    let layer_count = layer_count(colour_levels);
    let chunk_rows = (TD_STRING_MAX + 3) / (cols + 3);
    let chunk_count = (rows + chunk_rows - 1) / chunk_rows;

    let mut layer_chunks = Vec::with_capacity(layer_count);
    for layer in 0..layer_count {
        let mut chunks = Vec::with_capacity(chunk_count);
        for chunk in 0..chunk_count {
            let mut text = String::new();
            for row_in_chunk in 0..chunk_rows {
                let grid_row = chunk * chunk_rows + row_in_chunk;
                if grid_row >= rows {
                    break;
                }
                if row_in_chunk > 0 {
                    text.push_str("~n~");
                }
                for col in 0..cols {
                    let idx = (grid_row * cols + col) * 4;
                    let r = canvas[idx];
                    let g = canvas[idx + 1];
                    let b = canvas[idx + 2];
                    let a = canvas[idx + 3];
                    text.push(if pixel_on(r, g, b, a, layer, colour_levels) { 'I' } else { '.' });
                }
            }
            chunks.push(text);
        }
        layer_chunks.push(chunks);
    }

    (TextDrawFrame { layer_chunks }, delay)
}
