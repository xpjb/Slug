//! Vertex struct and attribute layout for Slug rendering.
//! 5 attributes × vec4: pos, tex, jac, bnd, col.

use bytemuck::{Pod, Zeroable};
use crate::glyph_cache::GlyphInfo;

/// Vertex format for Slug: 5 × vec4 = 80 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SlugVertex {
    /// pos.xy = object-space vertex coords, pos.zw = object-space normal
    pub pos: [f32; 4],
    /// tex.xy = em-space sample coords, tex.zw = packed glyph/band data
    pub tex: [f32; 4],
    /// jac = inverse Jacobian (00, 01, 10, 11)
    pub jac: [f32; 4],
    /// bnd = (band_scale_x, band_scale_y, band_offset_x, band_offset_y)
    pub bnd: [f32; 4],
    /// col = RGBA
    pub col: [f32; 4],
}

/// Create 6 vertices for a glyph quad.
/// Position (px, py) is the glyph's origin in em-space.
pub fn create_glyph_vertices(
    info: &GlyphInfo,
    px: f32,
    py: f32,
    color: [f32; 4],
) -> [SlugVertex; 6] {
    let (min_x, min_y, max_x, max_y) = info.bbox;
    let (bx, by) = (px + min_x, py + min_y);
    let (bw, bh) = (max_x - min_x, max_y - min_y);

    let num_bands_x = info.band_max.0 + 1;
    let num_bands_y = info.band_max.1 + 1;
    let scale_x = if bw > 0.0001 {
        num_bands_x as f32 / bw
    } else {
        1.0
    };
    let scale_y = if bh > 0.0001 {
        num_bands_y as f32 / bh
    } else {
        1.0
    };
    // Band offset for glyph-local coords: local (min_x, min_y) -> band_index 0
    let off_x = -min_x * scale_x;
    let off_y = -min_y * scale_y;

    // Pack glyph location: tex.z = (gy << 16) | gx as float bits
    let gx = info.band_start.0 as u32;
    let gy = info.band_start.1 as u32;
    let tex_zw = f32::from_bits((gy << 16) | gx);
    // tex.w = band_max: (band_max_y << 16) | band_max_x for SlugUnpack
    let tex_ww_bits: u32 = ((info.band_max.1 as u32 & 0xFF) << 16)
        | (info.band_max.0 as u32 & 0xFF);
    let tex_ww = f32::from_bits(tex_ww_bits);

    // jac.xy = glyph origin (px, py) for glyph-local texcoord; jac.zw unused when kUseDilate=false
    let jac = [px, py, 0.0, 0.0];
    let bnd = [scale_x, scale_y, off_x, off_y];

    let corners = [
        (bx, by, -1.0, -1.0),
        (bx + bw, by, 1.0, -1.0),
        (bx + bw, by + bh, 1.0, 1.0),
        (bx, by, -1.0, -1.0),
        (bx + bw, by + bh, 1.0, 1.0),
        (bx, by + bh, -1.0, 1.0),
    ];

    let mut out = [SlugVertex {
        pos: [0.0, 0.0, 0.0, 0.0],
        tex: [0.0, 0.0, 0.0, 0.0],
        jac,
        bnd,
        col: color,
    }; 6];

    for (i, (cx, cy, nx, ny)) in corners.iter().enumerate() {
        out[i] = SlugVertex {
            pos: [*cx, *cy, *nx, *ny],
            tex: [*cx, *cy, tex_zw, tex_ww],
            jac,
            bnd,
            col: color,
        };
    }
    out
}

/// Create vertices for a string of glyphs. Returns Vec of SlugVertex.
/// Each item is (GlyphInfo, x, y) - x,y in em-space.
pub fn create_text_vertices(
    items: &[(&GlyphInfo, f32, f32)],
    color: [f32; 4],
) -> Vec<SlugVertex> {
    let mut out = Vec::with_capacity(items.len() * 6);
    for (info, px, py) in items {
        for v in create_glyph_vertices(info, *px, *py, color) {
            out.push(v);
        }
    }
    out
}
