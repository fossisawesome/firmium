#![allow(dead_code)]

use image::DynamicImage;

#[derive(serde::Serialize, Clone, Copy, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(serde::Serialize, Clone, Copy, Debug)]
pub struct OrbPalette {
    pub primary: Rgb,
    pub secondary: Rgb,
    pub tertiary: Rgb,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct CoverColorsResult {
    pub orb: OrbPalette,
    pub dominant: Option<Rgb>,
}

const DEFAULT_PRIMARY:   Rgb = Rgb { r: 124, g: 92,  b: 255 };
const DEFAULT_SECONDARY: Rgb = Rgb { r: 170, g: 136, b: 255 };
const DEFAULT_TERTIARY:  Rgb = Rgb { r: 85,  g: 51,  b: 204 };

fn default_palette() -> OrbPalette {
    OrbPalette {
        primary:   Rgb { r: DEFAULT_PRIMARY.r,   g: DEFAULT_PRIMARY.g,   b: DEFAULT_PRIMARY.b   },
        secondary: Rgb { r: DEFAULT_SECONDARY.r, g: DEFAULT_SECONDARY.g, b: DEFAULT_SECONDARY.b },
        tertiary:  Rgb { r: DEFAULT_TERTIARY.r,  g: DEFAULT_TERTIARY.g,  b: DEFAULT_TERTIARY.b  },
    }
}

fn extract_orb(img: &DynamicImage) -> OrbPalette {
    let small = img.resize_exact(64, 64, image::imageops::FilterType::Nearest).to_rgba8();

    #[derive(Default)]
    struct Bucket { count: u32, sum_r: u32, sum_g: u32, sum_b: u32, sat: f32 }

    let mut buckets: std::collections::HashMap<(u8, u8, u8), Bucket> = Default::default();
    for pixel in small.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 { continue; }
        let key = (r >> 4, g >> 4, b >> 4);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let sat = if max > 0 { (max - min) as f32 / max as f32 } else { 0.0 };
        let bkt = buckets.entry(key).or_default();
        if bkt.count == 0 { bkt.sat = sat; }
        bkt.count += 1;
        bkt.sum_r += r as u32;
        bkt.sum_g += g as u32;
        bkt.sum_b += b as u32;
    }

    #[derive(Clone, Copy)]
    struct Color { r: u8, g: u8, b: u8, score: f32 }

    let mut sorted: Vec<Color> = buckets.values()
        .filter(|bkt| bkt.count > 2)
        .map(|bkt| Color {
            r: (bkt.sum_r / bkt.count) as u8,
            g: (bkt.sum_g / bkt.count) as u8,
            b: (bkt.sum_b / bkt.count) as u8,
            score: bkt.sat * bkt.count as f32,
        })
        .collect();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.is_empty() {
        return default_palette();
    }

    let dist = |a: Color, c: Color| -> u32 {
        (a.r as i32 - c.r as i32).unsigned_abs()
            + (a.g as i32 - c.g as i32).unsigned_abs()
            + (a.b as i32 - c.b as i32).unsigned_abs()
    };

    let primary   = sorted[0];
    let secondary = sorted.iter().copied().find(|&c| dist(c, primary) > 80)
        .unwrap_or(sorted[1_usize.min(sorted.len() - 1)]);
    let tertiary  = sorted.iter().copied()
        .find(|&c| dist(c, primary) > 60 && dist(c, secondary) > 60)
        .unwrap_or(sorted[2_usize.min(sorted.len() - 1)]);

    OrbPalette {
        primary:   Rgb { r: primary.r,   g: primary.g,   b: primary.b   },
        secondary: Rgb { r: secondary.r, g: secondary.g, b: secondary.b },
        tertiary:  Rgb { r: tertiary.r,  g: tertiary.g,  b: tertiary.b  },
    }
}

fn extract_dominant(img: &DynamicImage) -> Option<Rgb> {
    let small = img.resize_exact(32, 32, image::imageops::FilterType::Nearest).to_rgba8();

    #[derive(Default)]
    struct Bucket { count: u32, sum_r: u32, sum_g: u32, sum_b: u32 }

    let mut buckets: std::collections::HashMap<(u8, u8, u8), Bucket> = Default::default();
    for pixel in small.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < 128 { continue; }
        let bkt = buckets.entry((r >> 5, g >> 5, b >> 5)).or_default();
        bkt.count += 1;
        bkt.sum_r += r as u32;
        bkt.sum_g += g as u32;
        bkt.sum_b += b as u32;
    }

    buckets.values().max_by_key(|bkt| bkt.count).map(|bkt| Rgb {
        r: (bkt.sum_r / bkt.count) as u8,
        g: (bkt.sum_g / bkt.count) as u8,
        b: (bkt.sum_b / bkt.count) as u8,
    })
}

fn process_image(img: &DynamicImage) -> CoverColorsResult {
    CoverColorsResult {
        orb: extract_orb(img),
        dominant: extract_dominant(img),
    }
}

/// Extract orb palette and dominant color from a cached (or fetchable) cover art image.
/// Calls get_cover_art internally to ensure the image is cached before processing.
pub async fn extract_cover_colors(
    cover_id: String,
    url: String,
) -> Result<CoverColorsResult, String> {
    let path_str = super::cover_cache::get_cover_art(cover_id, url).await?;
    let path = std::path::PathBuf::from(path_str);
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path).map_err(|e| e.to_string())?;
        Ok(process_image(&img))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Extract orb palette and dominant color from a local file path (e.g. local cover art).
pub async fn extract_cover_colors_from_path(path: String) -> Result<CoverColorsResult, String> {
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path).map_err(|e| e.to_string())?;
        Ok(process_image(&img))
    })
    .await
    .map_err(|e| e.to_string())?
}
