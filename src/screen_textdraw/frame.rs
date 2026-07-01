use std::time::Duration;

pub const TD_STRING_MAX: usize = 800;
pub const TD_PLAYER_BUDGET: usize = 256;

// Legacy helpers kept for external references
pub fn compute_colour_levels(cols: usize, rows: usize, budget: usize) -> usize {
    let chunk_rows = (TD_STRING_MAX + 3) / (cols + 3);
    let chunk_count = (rows + chunk_rows - 1) / chunk_rows;
    let max_layers = (budget.saturating_sub(1)) / chunk_count;
    ((max_layers.saturating_sub(1)) / 4).max(1)
}

pub fn layer_count(colour_levels: usize) -> usize {
    1 + colour_levels * 4
}

// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ChunkLayer {
    pub rgba: u32,   // 0xRRGGBBAA for PlayerTextDrawColor
    pub text: String, // I/. rows joined by ~n~
}

#[derive(Clone)]
pub struct TextDrawFrame {
    pub pool_size: usize,
    pub chunk_layers: Vec<Vec<ChunkLayer>>,
}

// ---------------------------------------------------------------------------

pub fn build_frame(
    canvas: &[u8],
    delay: Duration,
    cols: usize,
    rows: usize,
    budget: usize,
) -> (TextDrawFrame, Duration) {
    let chunk_rows = (TD_STRING_MAX + 3) / (cols + 3);
    let chunk_count = (rows + chunk_rows - 1) / chunk_rows;
    let pool_size = (budget.saturating_sub(1) / chunk_count.max(1)).max(1);

    let mut chunk_layers = Vec::with_capacity(chunk_count);
    for chunk in 0..chunk_count {
        chunk_layers.push(build_chunk_layers(canvas, cols, rows, chunk, chunk_rows, pool_size));
    }

    (TextDrawFrame { pool_size, chunk_layers }, delay)
}

// ---------------------------------------------------------------------------

/// Max total visual coverage for colour channels (G/B/R) when all bits are active.
const MAX_COVERAGE: f32 = 1.5;

/// Coverage multiplier for the darkness (K) channel — higher = more aggressive darkening.
/// Can exceed 1.0 to boost individual bit alphas beyond their normal weight (capped at 255).
const DARK_COVERAGE: f32 = 1.3;

/// Opacity of the white base layer (0xFFFFFFAA). Lowering this lets the black
/// background box show through, reducing washout and boosting perceived saturation.
const WHITE_OPACITY: f32 = 1.;

fn build_chunk_layers(
    canvas: &[u8],
    cols: usize,
    rows: usize,
    chunk: usize,
    chunk_rows: usize,
    pool_size: usize,
) -> Vec<ChunkLayer> {
    let actual_rows = chunk_rows.min(rows.saturating_sub(chunk * chunk_rows));
    let n = actual_rows * cols;

    // Binary bits per channel: (pool_size - 1 white) / 4 channels, capped at 8-bit precision.
    let n_bits = ((pool_size.saturating_sub(1)) / 4).clamp(1, 8);
    let levels = (1u16 << n_bits) - 1; // 2^n_bits - 1 (e.g. 255 for n_bits=8)

    // Collect per-pixel chroma and darkness, quantised to n_bits precision
    let mut g_ch = vec![0u8; n];
    let mut b_ch = vec![0u8; n];
    let mut r_ch = vec![0u8; n];
    let mut k_dk = vec![0u8; n];
    let mut opaque = vec![false; n];

    let scale = |v: u8| -> u8 {
        ((v as u16 * levels + 127) / 255) as u8
    };

    for row in 0..actual_rows {
        let grid_row = chunk * chunk_rows + row;
        for col in 0..cols {
            let idx = (grid_row * cols + col) * 4;
            let i = row * cols + col;
            if canvas[idx + 3] >= 128 {
                opaque[i] = true;
                let r = canvas[idx];
                let g = canvas[idx + 1];
                let b = canvas[idx + 2];
                let min_ch = r.min(g).min(b);
                let max_ch = r.max(g).max(b);
                g_ch[i] = scale(g.saturating_sub(min_ch));
                b_ch[i] = scale(b.saturating_sub(min_ch));
                r_ch[i] = scale(r.saturating_sub(min_ch));
                k_dk[i] = scale(255u8.saturating_sub(max_ch));
            }
        }
    }

    // Alpha for each bit: weight = 2^bit, scaled so the sum of all weights
    // (= 2^n_bits - 1 = levels) maps to MAX_COVERAGE.
    // This means pixel with chroma=max fires all bits → MAX_COVERAGE total.
    // Pixel with chroma=50% fires roughly the top half of bits → ~MAX_COVERAGE/2.

    let mut layers = Vec::with_capacity(pool_size);

    // Slot 0: white base — always on for opaque pixels, opacity controlled by WHITE_OPACITY
    let white_alpha = (WHITE_OPACITY * 255.0).round() as u8;
    layers.push(ChunkLayer {
        rgba: u32::from_be_bytes([0xFF, 0xFF, 0xFF, white_alpha]),
        text: make_text(actual_rows, cols, |i| opaque[i]),
    });

    // Binary bit layers for G / B / R / K
    let channels: [(&[u8], [u8; 3], f32); 4] = [
        (&g_ch, [0x00, 0xFF, 0x00], MAX_COVERAGE),
        (&b_ch, [0x00, 0x00, 0xFF], MAX_COVERAGE),
        (&r_ch, [0xFF, 0x00, 0x00], MAX_COVERAGE),
        (&k_dk, [0x00, 0x00, 0x00], DARK_COVERAGE),
    ];

    for (values, rgb, coverage) in &channels {
        for bit in 0..n_bits {
            let weight = 1u16 << bit;
            let a = ((weight as f32 / levels as f32) * coverage).min(1.0);
            let alpha = (a * 255.0).round() as u8;
            let rgba = u32::from_be_bytes([rgb[0], rgb[1], rgb[2], alpha]);
            let text = make_text(actual_rows, cols, |i| opaque[i] && (values[i] >> bit) & 1 == 1);
            layers.push(ChunkLayer { rgba, text });
        }
    }

    // Fill remaining slots with transparent blanks
    let blank = make_text(actual_rows, cols, |_| false);
    while layers.len() < pool_size {
        layers.push(ChunkLayer { rgba: 0x00000000, text: blank.clone() });
    }

    layers
}

// ---------------------------------------------------------------------------

fn make_text<F: Fn(usize) -> bool>(actual_rows: usize, cols: usize, active: F) -> String {
    let mut s = String::with_capacity(actual_rows * (cols + 3));
    for row in 0..actual_rows {
        if row > 0 { s.push_str("~n~"); }
        for col in 0..cols {
            s.push(if active(row * cols + col) { 'I' } else { '.' });
        }
    }
    s
}
