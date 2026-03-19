pub mod debug;
pub mod font;
pub mod glyph_cache;
pub mod layout;
pub mod vertex;
pub mod renderer;

pub use debug::debug_print_advances;
pub use font::{FontLoader, GlyphOutlines, BandData, process_bands, pick_ttc_face_index, pick_ttc_face_index_with_options, is_font_collection, font_format, FontFormat};
pub use glyph_cache::{GlyphCache, GlyphInfo};
pub use ttf_parser::fonts_in_collection;
pub use layout::layout_text;
pub use renderer::SlugRenderer;
pub use vertex::{SlugVertex, create_glyph_vertices, create_text_vertices};
