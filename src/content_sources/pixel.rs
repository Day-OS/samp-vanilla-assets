/// Copies a `region_w x region_h` rectangle out of a larger raw RGBA canvas.
pub fn extract_region(
    canvas: &[u8],
    canvas_width: usize,
    region_x: usize,
    region_y: usize,
    region_w: usize,
    region_h: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; region_w * region_h * 4];

    for row in 0..region_h {
        let src_start = ((region_y + row) * canvas_width + region_x) * 4;
        let dst_start = row * region_w * 4;
        out[dst_start..dst_start + region_w * 4]
            .copy_from_slice(&canvas[src_start..src_start + region_w * 4]);
    }

    out
}

pub fn quantize_pixels(pixels: &[Vec<i32>], step: i32) -> Vec<Vec<i32>> {
    pixels
        .iter()
        .map(|row| {
            row.iter()
                .map(|&pixel| quantize_pixel(pixel, step))
                .collect()
        })
        .collect()
}

fn quantize_pixel(pixel: i32, step: i32) -> i32 {
    let alpha = pixel & !0x00FF_FFFF;
    let r = (pixel >> 16) & 0xFF;
    let g = (pixel >> 8) & 0xFF;
    let b = pixel & 0xFF;

    let r = (r / step) * step;
    let g = (g / step) * step;
    let b = (b / step) * step;

    alpha | (r << 16) | (g << 8) | b
}

pub fn downsample_to_argb(
    canvas: &[u8],
    width: usize,
    height: usize,
    rows: usize,
    cols: usize,
) -> Vec<Vec<i32>> {
    let mut pixels = vec![vec![0i32; cols]; rows];

    for row in 0..rows {
        let src_x = (row * width / rows).min(width.saturating_sub(1));
        for col in 0..cols {
            let forward_y = (col * height / cols).min(height.saturating_sub(1));
            let src_y = height - 1 - forward_y;
            let pixel_index = (src_y * width + src_x) * 4;
            let r = canvas[pixel_index] as u32;
            let g = canvas[pixel_index + 1] as u32;
            let b = canvas[pixel_index + 2] as u32;
            let a = canvas[pixel_index + 3] as u32;
            pixels[row][col] = ((a << 24) | (r << 16) | (g << 8) | b) as i32;
        }
    }

    pixels
}
