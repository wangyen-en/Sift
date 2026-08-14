// ============================================================
// Sift - Quality Analysis Command
// Technical quality assessment (no ML model, pure image statistics):
//   - Sharpness  : Laplacian variance (low value => blur)
//   - Exposure   : luminance histogram + clipped pixel ratio
//   - Noise      : mean absolute residual after a 3x3 box blur
// All scores are normalized to 0-100 (higher = better).
// ============================================================

use crate::models::photo::QualityData;
use image::imageops::FilterType;
use image::GenericImageView;

/// Maximum long edge used for analysis (downscaled for speed)
const MAX_ANALYSIS_DIM: u32 = 1024;

#[tauri::command]
pub async fn analyze_quality(jpg_path: String) -> Result<QualityData, String> {
    tokio::task::spawn_blocking(move || analyze_quality_sync(&jpg_path))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

fn analyze_quality_sync(path: &str) -> Result<QualityData, String> {
    let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;

    // Downscale to keep analysis fast (analysis is resolution-tolerant)
    let (w, h) = img.dimensions();
    let max_dim = w.max(h);
    let img = if max_dim > MAX_ANALYSIS_DIM {
        let scale = MAX_ANALYSIS_DIM as f32 / max_dim as f32;
        let nw = ((w as f32 * scale).round().max(1.0)) as u32;
        let nh = ((h as f32 * scale).round().max(1.0)) as u32;
        img.resize_exact(nw, nh, FilterType::Triangle)
    } else {
        img
    };

    let gray = img.to_luma8();
    let (gw, gh) = gray.dimensions();
    let g = gray.as_raw();
    let width = gw as usize;
    let height = gh as usize;

    // --- Sharpness: variance of the Laplacian ---
    let laplacian_var = compute_laplacian_variance(g, width, height);

    // --- Exposure: mean luminance + clipped pixel ratio ---
    let (overexposed_pct, underexposed_pct, brightness) = compute_exposure(g, width, height);

    // --- Noise: mean absolute residual after 3x3 box blur ---
    let noise_residual = compute_noise_residual(g, width, height);

    // --- Normalize to 0-100 scores ---
    let sharpness = clamp(sharpness_to_score(laplacian_var), 0.0, 100.0);
    let exposure = clamp(
        exposure_to_score(brightness, overexposed_pct, underexposed_pct),
        0.0,
        100.0,
    );
    let noise = clamp(noise_to_score(noise_residual), 0.0, 100.0);

    // --- Overall: weighted average ---
    let overall = clamp(sharpness * 0.4 + exposure * 0.3 + noise * 0.3, 0.0, 100.0);

    Ok(QualityData {
        sharpness: round1(sharpness),
        exposure: round1(exposure),
        noise: round1(noise),
        overall: round1(overall),
        brightness: round1(brightness),
        overexposed_pct: round1(overexposed_pct),
        underexposed_pct: round1(underexposed_pct),
    })
}

/// Variance of the 4-neighbour Laplacian over the whole image.
/// High variance => strong edges => sharp. Low => blurry.
fn compute_laplacian_variance(g: &[u8], w: usize, h: usize) -> f64 {
    if w < 3 || h < 3 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut count = 0u64;
    for y in 1..h - 1 {
        let row = y * w;
        for x in 1..w - 1 {
            let idx = row + x;
            let c = g[idx] as f64;
            let lap = 4.0 * c
                - g[idx - 1] as f64
                - g[idx + 1] as f64
                - g[idx - w] as f64
                - g[idx + w] as f64;
            sum += lap;
            sum_sq += lap * lap;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let mean = sum / count as f64;
    (sum_sq / count as f64) - mean * mean
}

/// Mean luminance and clipped-pixel percentages.
fn compute_exposure(g: &[u8], _w: usize, _h: usize) -> (f64, f64, f64) {
    let total = g.len() as f64;
    if total == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let mut sum = 0.0f64;
    let mut over = 0u64; // > 250
    let mut under = 0u64; // < 5
    for &p in g {
        sum += p as f64;
        if p > 250 {
            over += 1;
        } else if p < 5 {
            under += 1;
        }
    }
    (
        over as f64 / total * 100.0,
        under as f64 / total * 100.0,
        sum / total,
    )
}

/// Mean absolute residual between each pixel and its 3x3 box-blur.
/// High residual => high-frequency content (fine detail or noise).
fn compute_noise_residual(g: &[u8], w: usize, h: usize) -> f64 {
    if w < 3 || h < 3 {
        return 0.0;
    }
    let mut residual_sum = 0.0f64;
    let mut count = 0u64;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            let c = g[idx] as f64;
            let mut s = 0u32;
            for dy in 0..3usize {
                let row = (y + dy - 1) * w;
                for dx in 0..3usize {
                    s += g[row + x + dx - 1] as u32;
                }
            }
            let mean = s as f64 / 9.0;
            residual_sum += (c - mean).abs();
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    residual_sum / count as f64
}

/// Map Laplacian variance to a 0-100 sharpness score.
fn sharpness_to_score(v: f64) -> f64 {
    // Saturation curve: ~150 => 63, ~300 => 86, ~50 => 28, ~20 => 12
    100.0 * (1.0 - (-v / 150.0).exp())
}

/// Map exposure stats to a 0-100 balance score.
fn exposure_to_score(mean: f64, over: f64, under: f64) -> f64 {
    let mut score = 100.0;
    // Penalize under/over-exposed mean luminance
    if mean < 60.0 {
        score -= (60.0 - mean) * 1.5;
    } else if mean > 190.0 {
        score -= (mean - 190.0) * 1.5;
    }
    // Penalize clipped areas
    score -= over * 1.5;
    score -= under * 1.5;
    score.max(0.0)
}

/// Map noise residual to a 0-100 cleanliness score (higher = cleaner).
fn noise_to_score(residual: f64) -> f64 {
    // Clean ~3-8, noisy ~15-40. Linear penalty, ~25 => 0.
    (100.0 - residual * 4.0).max(0.0)
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
