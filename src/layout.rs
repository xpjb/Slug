//! Text layout: lay out a string of glyphs using a font loader and glyph cache.

use crate::font::{FontLoader, process_bands};
use crate::glyph_cache::{GlyphCache, GlyphInfo};

/// Lay out text using the given font and cache. Returns (GlyphInfo, x, y) items in em-space.
pub fn layout_text(
    loader: &FontLoader,
    cache: &mut GlyphCache,
    text: &str,
    start_x: f32,
    start_y: f32,
) -> Vec<(GlyphInfo, f32, f32)> {
    let upem = loader.units_per_em() as f32;
    let mut items = Vec::new();
    let mut x = start_x;

    for c in text.chars() {
        let glyph_id = match loader.glyph_index(c) {
            Some(id) => id,
            None => continue,
        };

        // Use hmtx advance directly; no bbox fallback (bbox ignores side bearings and breaks spacing).
        // LSB-as-advance (~0.036 em) and TTC wrong-face issues are fixed at load time via pick_ttc_face_index.
        let advance = loader
            .advance_width(glyph_id)
            .map(|v| v as f32 / upem)
            .unwrap_or(0.5);

        let outlines = match loader.load_glyph(glyph_id) {
            Some(o) => o,
            None => {
                x += advance;
                continue;
            }
        };

        let band_data = process_bands(&outlines);
        let info = cache.add_glyph(glyph_id.0.into(), band_data);
        items.push((info, x, start_y));
        x += advance;
    }

    items
}
