use std::path::Path;

use anyhow::Context;
use vb_ui_snapshot::{BASELINE_HEIGHT, BASELINE_WIDTH, tokens::UiTokens};

pub(crate) fn generate_fixture_screenshot(
    path: &Path,
    fixture_name: &str,
    tokens: Option<&UiTokens>,
) -> anyhow::Result<()> {
    let tokens = tokens.cloned().unwrap_or_default();
    let mut img = image::RgbaImage::new(BASELINE_WIDTH, BASELINE_HEIGHT);
    paint_fixture_background(&mut img, &tokens);
    paint_fixture_layout(&mut img, &tokens);
    draw_token_swatches(&mut img, &tokens);
    draw_fixture_marker(&mut img, fixture_name, &tokens);
    img.save(path)
        .with_context(|| format!("Failed to write fixture screenshot: {}", path.display()))
}

fn paint_fixture_background(img: &mut image::RgbaImage, tokens: &UiTokens) {
    let bg = rgba_from_hex(&tokens.background_board);
    let surface = rgba_from_hex(&tokens.surface);
    fill_rect(img, 0, 0, BASELINE_WIDTH, BASELINE_HEIGHT, bg);
    fill_rect(img, 96, 96, 1728, 888, surface);
}

fn paint_fixture_layout(img: &mut image::RgbaImage, tokens: &UiTokens) {
    let shell = rgba_from_hex(&tokens.shell);
    let surface = rgba_from_hex(&tokens.surface);
    fill_rect(img, 0, 0, tokens.sidebar_width, BASELINE_HEIGHT, shell);
    fill_top_bar(img, tokens, surface);
    fill_graph_canvas(img, tokens, surface);
}

fn fill_top_bar(img: &mut image::RgbaImage, tokens: &UiTokens, surface: image::Rgba<u8>) {
    let top_bar = rgba_from_hex(&tokens.surface_muted);
    let top_width = BASELINE_WIDTH.saturating_sub(tokens.sidebar_width);
    fill_rect(img, tokens.sidebar_width, 0, top_width, 72, top_bar);
    let label_x = tokens.sidebar_width.saturating_add(24);
    fill_rect(img, label_x, 20, 240, 32, surface);
}

fn fill_graph_canvas(img: &mut image::RgbaImage, tokens: &UiTokens, surface: image::Rgba<u8>) {
    let canvas = rgba_from_hex(&tokens.line_soft);
    fill_rect(img, 360, 140, 1200, 760, surface);
    fill_rect(img, 400, 220, 420, 8, canvas);
    fill_rect(img, 820, 420, 380, 8, canvas);
    fill_rect(img, 1180, 220, 8, 480, canvas);
}

pub(crate) fn fill_rect(
    img: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: image::Rgba<u8>,
) {
    let max_x = x.saturating_add(width).min(img.width());
    let max_y = y.saturating_add(height).min(img.height());
    for px in x..max_x {
        for py in y..max_y {
            img.put_pixel(px, py, color);
        }
    }
}

pub(crate) fn draw_token_swatches(img: &mut image::RgbaImage, tokens: &UiTokens) {
    let colors = [
        &tokens.surface,
        &tokens.text_primary,
        &tokens.running,
        &tokens.success,
        &tokens.warning,
        &tokens.failure,
        &tokens.active_cyan,
        &tokens.line_soft,
    ];
    let mut x = 320;
    for color in colors {
        fill_rect(img, x, 128, 48, 48, rgba_from_hex(color));
        x = x.saturating_add(96);
    }
}

pub(crate) fn draw_fixture_marker(
    img: &mut image::RgbaImage,
    fixture_name: &str,
    tokens: &UiTokens,
) {
    let color = match fixture_name {
        "incident_failure" => rgba_from_hex(&tokens.failure),
        "verification_certificate" => rgba_from_hex(&tokens.success),
        "replay_theater" => rgba_from_hex(&tokens.active_cyan),
        _ => rgba_from_hex(&tokens.running),
    };
    fill_rect(img, 384, 240, 180, 96, color);
    fill_rect(img, 720, 420, 220, 96, color);
    fill_rect(img, 1120, 620, 260, 96, color);
}

pub(crate) fn rgba_from_hex(hex: &str) -> image::Rgba<u8> {
    let trimmed = hex.strip_prefix('#').unwrap_or(hex);
    let bytes = trimmed.as_bytes();
    let parsed = parse_hex_pair(bytes.get(0..2))
        .zip(parse_hex_pair(bytes.get(2..4)))
        .zip(parse_hex_pair(bytes.get(4..6)))
        .map(|((r, g), b)| [r, g, b, 255]);
    image::Rgba(parsed.unwrap_or([255, 255, 255, 255]))
}

fn parse_hex_pair(bytes: Option<&[u8]>) -> Option<u8> {
    let text = std::str::from_utf8(bytes?).ok()?;
    u8::from_str_radix(text, 16).ok()
}
