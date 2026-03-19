//! Debug utilities for font shaping and advances.

use rustybuzz::{UnicodeBuffer, shape};
use crate::font::FontLoader;

/// Returns the character at the given UTF-8 byte offset, if valid.
fn char_at_byte_offset(text: &str, byte_offset: usize) -> Option<char> {
    let mut pos = 0;
    for c in text.chars() {
        if byte_offset >= pos && byte_offset < pos + c.len_utf8() {
            return Some(c);
        }
        pos += c.len_utf8();
    }
    None
}

/// Shapes text and prints per-glyph advance diagnostics. Use with `--debug` to confirm
/// Latin overlap (~36 = LSB) or CJK offscreen (~65535 = u16::MAX).
pub fn debug_print_advances(loader: &FontLoader, text: &str, font_name: &str) {
    let upem = loader.units_per_em() as f32;

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();

    let glyph_buffer = shape(loader.face(), &[], buffer);
    let infos = glyph_buffer.glyph_infos();
    let positions = glyph_buffer.glyph_positions();

    println!("=== ADVANCE DIAGNOSTICS: {} ===\n", font_name);
    println!("{:>4}  {:>6}  {:^8}  {:>10}  {:>10}  {:>12}", "i", "cluster", "char", "x_advance", "y_advance", "adv_em");
    println!("{}", "-".repeat(58));

    for (i, (info, pos)) in infos.iter().zip(positions.iter()).enumerate() {
        let cluster = info.cluster as usize;
        let ch = char_at_byte_offset(text, cluster)
            .map(|c| format!("{:?}", c))
            .unwrap_or_else(|| "?".to_string());
        let adv_em = pos.x_advance as f32 / upem;
        println!(
            "{:>4}  {:>6}  {:^8}  {:>10}  {:>10}  {:>12.4}",
            i,
            cluster,
            ch,
            pos.x_advance,
            pos.y_advance,
            adv_em
        );
    }
    println!();
}
