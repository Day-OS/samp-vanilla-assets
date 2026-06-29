const MAX_DIALOG_BODY_LEN: usize = 4096;
const MAX_DIALOG_ROW_LEN: usize = 128;

pub(super) fn build_dialog_body(canvas: &[u8], width: usize, height: usize) -> String {
    let pixels = sample_argb(canvas, width, height, width.max(1), height.max(1));
    let render = render_pixels(&pixels);

    log::info!(
        "DialogScreen::build_dialog_body -> adaptive run budget color_tags={} body_len={} max_row_len={}",
        render.color_tags,
        render.body.len(),
        render.max_row_len
    );

    render.body
}

fn sample_argb(
    canvas: &[u8],
    width: usize,
    height: usize,
    cols: usize,
    rows: usize,
) -> Vec<Vec<i32>> {
    let mut pixels = vec![vec![0i32; cols]; rows];

    for row in 0..rows {
        let src_y = (row * height / rows).min(height.saturating_sub(1));
        for col in 0..cols {
            let src_x = (col * width / cols).min(width.saturating_sub(1));
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

struct RenderedDialog {
    body: String,
    color_tags: usize,
    max_row_len: usize,
}

fn render_pixels(pixels: &[Vec<i32>]) -> RenderedDialog {
    let mut body = String::new();
    let mut color_tags = 0usize;
    let mut row_len = 0usize;
    let mut max_row_len = 0usize;
    let mut last_color: Option<i32> = None;
    let row_plans = allocate_row_plans(pixels);

    for (row_index, row) in pixels.iter().enumerate() {
        if row_index > 0 {
            max_row_len = max_row_len.max(row_len);
            row_len = 0;
            body.push('\n');
        }

        let boundaries = row_plans[row_index].boundaries.clone();
        let mut segment_start = 0usize;

        for segment_end in boundaries.into_iter().chain(std::iter::once(row.len())) {
            let average_rgb = average_segment_rgb(&row[segment_start..segment_end]);
            let mut emitted_segment_color = false;

            for &pixel in &row[segment_start..segment_end] {
                let alpha = (pixel >> 24) & 0xFF;
                if alpha < 128 {
                    body.push(' ');
                    row_len += 1;
                    continue;
                }

                if !emitted_segment_color {
                    if let Some(rgb) = average_rgb {
                        if last_color != Some(rgb) {
                            body.push('{');
                            body.push_str(&format!("{:06X}", rgb));
                            body.push('}');
                            row_len += 8;
                            color_tags += 1;
                            last_color = Some(rgb);
                        }
                    }
                    emitted_segment_color = true;
                }

                body.push('@');
                row_len += 1;
            }

            segment_start = segment_end;
        }
    }
    max_row_len = max_row_len.max(row_len);

    RenderedDialog {
        body,
        color_tags,
        max_row_len,
    }
}

struct RowPlan {
    boundaries: Vec<usize>,
}

struct RowOptimization {
    costs: Vec<f64>,
    boundaries_by_budget: Vec<Vec<usize>>,
    cap: usize,
}

fn allocate_row_plans(pixels: &[Vec<i32>]) -> Vec<RowPlan> {
    let base_body_len = pixels.iter().map(Vec::len).sum::<usize>() + pixels.len().saturating_sub(1);
    let mut remaining_tags = MAX_DIALOG_BODY_LEN.saturating_sub(base_body_len) / 8;
    let row_caps: Vec<usize> = pixels
        .iter()
        .map(|row| {
            let row_limit = MAX_DIALOG_ROW_LEN.saturating_sub(row.len()) / 8;
            row_limit.min(opaque_pixel_count(row))
        })
        .collect();
    let optimizations: Vec<RowOptimization> = pixels
        .iter()
        .zip(&row_caps)
        .map(|(row, &cap)| optimize_row_segments(row, cap))
        .collect();
    let mut budgets = vec![0usize; pixels.len()];

    let mut active_rows: Vec<usize> = row_caps
        .iter()
        .enumerate()
        .filter_map(|(row_index, &cap)| (cap > 0).then_some(row_index))
        .collect();
    active_rows.sort_by(|&left, &right| {
        optimizations[right].costs[1].total_cmp(&optimizations[left].costs[1])
    });

    for &row_index in &active_rows {
        if remaining_tags == 0 {
            break;
        }
        budgets[row_index] = 1;
        remaining_tags -= 1;
    }

    while remaining_tags > 0 {
        let Some(row_index) = best_row_for_extra_tag(&budgets, &optimizations) else {
            break;
        };

        budgets[row_index] += 1;
        remaining_tags -= 1;
    }

    budgets
        .into_iter()
        .zip(optimizations)
        .map(|(budget, optimization)| RowPlan {
            boundaries: optimization
                .boundaries_by_budget
                .get(budget)
                .cloned()
                .unwrap_or_default(),
        })
        .collect()
}

fn best_row_for_extra_tag(budgets: &[usize], optimizations: &[RowOptimization]) -> Option<usize> {
    let mut best_row = None;
    let mut best_improvement = 0.0f64;

    for row_index in 0..budgets.len() {
        let budget = budgets[row_index];
        let optimization = &optimizations[row_index];

        if budget == 0 || budget >= optimization.cap {
            continue;
        }

        let improvement = optimization.costs[budget] - optimization.costs[budget + 1];
        if improvement > best_improvement {
            best_improvement = improvement;
            best_row = Some(row_index);
        }
    }

    best_row
}

fn opaque_pixel_count(row: &[i32]) -> usize {
    row.iter()
        .filter(|&&pixel| ((pixel >> 24) & 0xFF) >= 128)
        .count()
}

fn optimize_row_segments(row: &[i32], cap: usize) -> RowOptimization {
    let width = row.len();
    if cap == 0 || width == 0 {
        return RowOptimization {
            costs: vec![0.0],
            boundaries_by_budget: vec![Vec::new()],
            cap: 0,
        };
    }

    let cap = cap.min(width);
    let segment_costs = segment_cost_table(row);
    let mut dp = vec![vec![f64::INFINITY; width + 1]; cap + 1];
    let mut split = vec![vec![0usize; width + 1]; cap + 1];
    dp[0][0] = 0.0;

    for budget in 1..=cap {
        for end in budget..=width {
            for start in (budget - 1)..end {
                let cost = dp[budget - 1][start] + segment_costs[start][end];
                if cost < dp[budget][end] {
                    dp[budget][end] = cost;
                    split[budget][end] = start;
                }
            }
        }
    }

    let mut costs = vec![f64::INFINITY; cap + 1];
    let mut boundaries_by_budget = vec![Vec::new(); cap + 1];
    for budget in 1..=cap {
        costs[budget] = dp[budget][width];
        boundaries_by_budget[budget] = reconstruct_boundaries(&split, budget, width);
    }

    RowOptimization {
        costs,
        boundaries_by_budget,
        cap,
    }
}

fn reconstruct_boundaries(split: &[Vec<usize>], mut budget: usize, mut end: usize) -> Vec<usize> {
    let mut boundaries = Vec::new();

    while budget > 1 {
        let start = split[budget][end];
        boundaries.push(start);
        end = start;
        budget -= 1;
    }

    boundaries.sort_unstable();
    boundaries
}

fn segment_cost_table(row: &[i32]) -> Vec<Vec<f64>> {
    let width = row.len();
    let mut costs = vec![vec![0.0; width + 1]; width + 1];

    for start in 0..width {
        for end in (start + 1)..=width {
            costs[start][end] = segment_error(&row[start..end]);
        }
    }

    costs
}

fn segment_error(segment: &[i32]) -> f64 {
    let mut count = 0.0f64;
    let mut r_sum = 0.0f64;
    let mut g_sum = 0.0f64;
    let mut b_sum = 0.0f64;
    let mut r2_sum = 0.0f64;
    let mut g2_sum = 0.0f64;
    let mut b2_sum = 0.0f64;

    for &pixel in segment {
        let alpha = (pixel >> 24) & 0xFF;
        if alpha < 128 {
            continue;
        }

        let r = ((pixel >> 16) & 0xFF) as f64;
        let g = ((pixel >> 8) & 0xFF) as f64;
        let b = (pixel & 0xFF) as f64;

        count += 1.0;
        r_sum += r;
        g_sum += g;
        b_sum += b;
        r2_sum += r * r;
        g2_sum += g * g;
        b2_sum += b * b;
    }

    if count == 0.0 {
        return 0.0;
    }

    (r2_sum - (r_sum * r_sum / count))
        + (g2_sum - (g_sum * g_sum / count))
        + (b2_sum - (b_sum * b_sum / count))
}

fn average_segment_rgb(segment: &[i32]) -> Option<i32> {
    let mut len = 0usize;
    let mut r_sum = 0u32;
    let mut g_sum = 0u32;
    let mut b_sum = 0u32;

    for &pixel in segment {
        let alpha = (pixel >> 24) & 0xFF;
        if alpha < 128 {
            continue;
        }

        r_sum += ((pixel >> 16) & 0xFF) as u32;
        g_sum += ((pixel >> 8) & 0xFF) as u32;
        b_sum += (pixel & 0xFF) as u32;
        len += 1;
    }

    (len > 0).then(|| average_rgb(r_sum, g_sum, b_sum, len))
}

fn average_rgb(r_sum: u32, g_sum: u32, b_sum: u32, len: usize) -> i32 {
    let len = len as u32;
    let r = (r_sum + (len / 2)) / len;
    let g = (g_sum + (len / 2)) / len;
    let b = (b_sum + (len / 2)) / len;

    ((r << 16) | (g << 8) | b) as i32
}
