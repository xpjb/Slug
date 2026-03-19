//! Text layout: shape text using rustybuzz and lay out glyphs.

use rustybuzz::{UnicodeBuffer, shape};
use ttf_parser::GlyphId;
use crate::font::{FontLoader, process_bands};
use crate::glyph_cache::{GlyphCache, GlyphInfo};

/// Lay out text using the given font and cache. Uses rustybuzz for shaping.
/// Returns (GlyphInfo, x, y) items in em-space (glyph origin positions).
pub fn layout_text(
    loader: &FontLoader,
    cache: &mut GlyphCache,
    text: &str,
    start_x: f32,
    start_y: f32,
) -> Vec<(GlyphInfo, f32, f32)> {
    let upem = loader.units_per_em() as f32;

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();

    let glyph_buffer = shape(loader.face(), &[], buffer);

    let infos = glyph_buffer.glyph_infos();
    let positions = glyph_buffer.glyph_positions();

    let mut items = Vec::with_capacity(infos.len());
    let mut x = start_x;
    let mut y = start_y;

    for (info, pos) in infos.iter().zip(positions.iter()) {
        let glyph_id = info.glyph_id;
        let gx = x + pos.x_offset as f32 / upem;
        let gy = y + pos.y_offset as f32 / upem;

        let outlines = match loader.load_glyph(GlyphId(glyph_id as u16)) {
            Some(o) => o,
            None => {
                x += pos.x_advance as f32 / upem;
                y += pos.y_advance as f32 / upem;
                continue;
            }
        };

        let band_data = process_bands(&outlines);
        let glyph_info = cache.add_glyph(glyph_id, band_data);
        items.push((glyph_info, gx, gy));

        x += pos.x_advance as f32 / upem;
        y += pos.y_advance as f32 / upem;
    }

    items
}
