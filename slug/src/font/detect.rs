//! Font file magic-number detection.
//! Distinguishes TTF/TTC (single vs collection) and OTF/OTC before parsing.

/// Magic bytes at offset 0.
const TTCF: [u8; 4] = *b"ttcf"; // TTC/OTC collection
const OTTO: [u8; 4] = *b"OTTO"; // Single OTF (CFF/CFF2)
const TTF: [u8; 4] = [0x00, 0x01, 0x00, 0x00]; // Single TTF (TrueType)

/// Font format identified by magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// Single TrueType font (glyf outlines).
    Ttf,
    /// Single OpenType font with CFF/CFF2 outlines.
    Otf,
    /// TrueType Collection (multiple TrueType faces).
    Ttc,
    /// OpenType Collection (multiple CFF faces).
    Otc,
}

/// Returns true if the bytes represent a font collection (TTC or OTC).
pub fn is_font_collection(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == TTCF
}

/// Identifies the font format from magic bytes.
/// Returns None if the data is too short or unrecognized.
pub fn font_format(bytes: &[u8]) -> Option<FontFormat> {
    if bytes.len() < 4 {
        return None;
    }
    let magic = &bytes[0..4];
    if magic == TTCF {
        // TTC vs OTC: both use ttcf; ttf_parser/rustybuzz handle both.
        // We cannot distinguish without peeking into the collection.
        // Return Otc as generic "collection" - caller uses fonts_in_collection.
        Some(FontFormat::Ttc) // Or Otc - plan says both use ttcf
    } else if magic == OTTO {
        Some(FontFormat::Otf)
    } else if magic == TTF {
        Some(FontFormat::Ttf)
    } else {
        None
    }
}
