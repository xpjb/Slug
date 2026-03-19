mod detect;
mod loader;
mod curves;
mod bands;

pub use detect::{is_font_collection, font_format, FontFormat};
pub use loader::{FontLoader, pick_ttc_face_index, pick_ttc_face_index_with_options};
pub use curves::{QuadraticCurve, GlyphOutlines};
pub use bands::{process_bands, BandData};
