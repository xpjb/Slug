//! Band division and curve sorting for efficient pixel shader early-out.
//! Paper: divide glyph into horizontal and vertical bands; sort curves by max coord.

use crate::font::curves::{QuadraticCurve, GlyphOutlines};
use std::cmp::Ordering;

const BAND_TEXTURE_WIDTH: u32 = 4096;
const LOG_BAND_TEXTURE_WIDTH: u32 = 12;
const MAX_BANDS: usize = 16;

/// Curve index in the curve texture plus its max x or max y for sorting.
struct CurveInBand {
    curve_idx: usize,
    max_coord: f32,
}

/// Result of band processing: curve texture data and band texture data.
pub struct BandData {
    /// Curve texture: RGBA16Float. Each curve = 2 texels: (p1,p2) and (p3,_).
    pub curve_texels: Vec<[f32; 4]>,
    /// Curve texture dimensions (width, height). Width = 4096.
    pub curve_width: u32,
    pub curve_height: u32,
    /// Band texture: each texel = (count, offset) for uint. Packed as u32 pairs.
    pub band_texels: Vec<[u32; 4]>,
    pub band_width: u32,
    pub band_height: u32,
    /// Start location of this glyph's data in curve texture (x, y).
    pub curve_start: (u32, u32),
    /// Start location of this glyph's data in band texture (x, y).
    pub band_start: (u32, u32),
    /// Max band indexes (band_max_x, band_max_y).
    pub band_max: (u32, u32),
    /// Bounding box in em-space (min_x, min_y, max_x, max_y).
    pub bbox: (f32, f32, f32, f32),
}

/// Process glyph outlines into curve texture and band texture data.
pub fn process_bands(outlines: &GlyphOutlines) -> BandData {
    let curves = &outlines.curves;
    let (min_x, min_y, max_x, max_y) = outlines.bounding_box();
    let bbox = (min_x, min_y, max_x, max_y);

    // Number of bands: proportional to curve count, up to 16
    let num_bands = (curves.len() / 4).max(1).min(MAX_BANDS);
    let band_width_x = if max_x > min_x {
        (max_x - min_x) / num_bands as f32
    } else {
        1.0
    };
    let band_width_y = if max_y > min_y {
        (max_y - min_y) / num_bands as f32
    } else {
        1.0
    };

    // Horizontal bands: curves that intersect each horizontal strip. Sort by descending max_x.
    let mut h_bands: Vec<Vec<usize>> = vec![Vec::new(); num_bands];
    for (idx, c) in curves.iter().enumerate() {
        let cy_min = c.min_y();
        let cy_max = c.max_y();
        for b in 0..num_bands {
            let band_min = min_y + b as f32 * band_width_y;
            let band_max_y = min_y + (b + 1) as f32 * band_width_y;
            if cy_max >= band_min && cy_min <= band_max_y {
                h_bands[b].push(idx);
            }
        }
    }

    // Sort horizontal band curves by descending max_x
    for band in &mut h_bands {
        band.sort_by(|a, b| {
            curves[*b].max_x().partial_cmp(&curves[*a].max_x()).unwrap_or(Ordering::Equal)
        });
    }

    // Vertical bands: curves that intersect each vertical strip. Sort by descending max_y.
    let mut v_bands: Vec<Vec<usize>> = vec![Vec::new(); num_bands];
    for (idx, c) in curves.iter().enumerate() {
        let cx_min = c.min_x();
        let cx_max = c.max_x();
        for b in 0..num_bands {
            let band_min = min_x + b as f32 * band_width_x;
            let band_max_x = min_x + (b + 1) as f32 * band_width_x;
            if cx_max >= band_min && cx_min <= band_max_x {
                v_bands[b].push(idx);
            }
        }
    }
    for band in &mut v_bands {
        band.sort_by(|a, b| {
            curves[*b].max_y().partial_cmp(&curves[*a].max_y()).unwrap_or(Ordering::Equal)
        });
    }

    // Build curve texture: 2 texels per curve
    let mut curve_texels = Vec::new();
    for c in curves {
        curve_texels.push([c.p1.0, c.p1.1, c.p2.0, c.p2.1]);
        curve_texels.push([c.p3.0, c.p3.1, 0.0, 0.0]);
    }
    let curve_rows = (curve_texels.len() as u32 + BAND_TEXTURE_WIDTH - 1) / BAND_TEXTURE_WIDTH;
    let curve_height = curve_rows.max(1);
    // Pad curve texture rows to full width
    let mut padded_curves = Vec::new();
    let mut row_offset = 0;
    for _ in 0..curve_height {
        for col in 0..BAND_TEXTURE_WIDTH {
            let idx = row_offset + col as usize;
            if idx < curve_texels.len() {
                padded_curves.push(curve_texels[idx]);
            } else {
                padded_curves.push([0.0, 0.0, 0.0, 0.0]);
            }
        }
        row_offset += BAND_TEXTURE_WIDTH as usize;
    }

    // Build band texture
    // Layout: for each horizontal band, 1 header (count, offset). Then curve loc list.
    // Then for each vertical band, 1 header. Then curve loc list.
    // Band data stored at (glyphLoc.x + bandIndex.y, glyphLoc.y) for horizontal
    // (glyphLoc.x + bandMax.y + 1 + bandIndex.x, glyphLoc.y) for vertical
    // Curve locations are (x,y) into curve texture. Each curve uses 2 texels, so
    // curve at index i starts at column (i*2) in row 0 of this glyph's curve data.

    let curve_start_x = 0u32; // This glyph's curves start at column 0
    let curve_start_y = 0u32; // and row 0 in its own curve sub-texture

    // We'll pack: H band headers at cols 0..num_bands, V band headers at num_bands..2*num_bands
    // Curve lists follow. Need to compute offsets.
    let mut band_texels: Vec<[u32; 4]> = Vec::new();

    let mut h_band_headers: Vec<(u32, u32)> = Vec::new();
    let mut curve_locations: Vec<(u32, u32)> = Vec::new();
    let mut offset = 0u32;

    for band in &h_bands {
        let count = band.len() as u32;
        h_band_headers.push((count, offset));
        for &ci in band {
            let curve_col = (ci * 2) as u32;
            curve_locations.push((curve_col, curve_start_y));
            offset += 1;
        }
    }

    let v_offset = offset;
    let mut v_band_headers: Vec<(u32, u32)> = Vec::new();
    for band in &v_bands {
        let count = band.len() as u32;
        v_band_headers.push((count, offset));
        for &ci in band {
            let curve_col = (ci * 2) as u32;
            curve_locations.push((curve_col, curve_start_y));
            offset += 1;
        }
    }

    // Band texture layout per glyph (simplified):
    // Rows: one row per glyph. Cols: headers then curve locs.
    // Reference: bandData.Load(glyphLoc.x + bandIndex.y, glyphLoc.y) for H
    // glyphLoc is (band_start_x, band_start_y). So we need columns for:
    // col 0..num_bands: H band headers (count, offset)
    // col num_bands..num_bands+num_bands: V band headers
    // Then curve locations. CalcBandLoc wraps when x exceeds 4096.

    // Simpler: store glyph band data in a contiguous block. Width 4096.
    // Header row: [H0_count, H0_offset, H1_count, H1_offset, ... | V0_count, V0_offset, ...]
    // Data rows: curve (x,y) pairs. Each texel holds (x,y) as rg32ui.
    // Actually the reference uses uint4 - (count, offset) in .xy and possibly more.
    // For H band: hbandData.x = count, hbandData.y = offset.
    // CalcBandLoc: bandLoc = (glyphLoc.x + offset) with wrap. So offset is into the glyph's band block.
    // The curve locations are stored sequentially; offset points to start of that band's list.

    // We need one texel per band for headers. Count and offset.
    // Then texels for curve locations. Each curve loc is (x,y) - 2 u32s.
    // texture is Rgba32Uint. So one texel = (curve_x, curve_y, _, _).
    // Headers: one texel = (count, offset, _, _).

    let total_band_cols = num_bands * 2 + curve_locations.len(); // headers + locs
    // Pack into 4096-wide texture. Each glyph gets a row? Or region?
    // Paper: "the data for a glyph begins with a table of band headers for all of the horizontal bands
    // followed by all of the vertical bands. The header fits into one texel..."
    // So glyph band block: [H0][H1]...[Hn][V0][V1]...[Vn][curve_loc0][curve_loc1]...
    // Column 0: H0 (count, offset), col 1: H1, ... col num_bands-1: H last
    // col num_bands: V0, ... col 2*num_bands-1: V last
    // col 2*num_bands: first curve loc, etc.

    // bandMax is max valid index; num_bands bands means indices 0..num_bands-1
    let band_max_x = (num_bands as u32).saturating_sub(1);
    let band_max_y = (num_bands as u32).saturating_sub(1);

    BandData {
        curve_texels: padded_curves,
        curve_width: BAND_TEXTURE_WIDTH,
        curve_height,
        band_texels: build_band_texture(
            num_bands,
            &h_band_headers,
            &v_band_headers,
            &curve_locations,
        ),
        band_width: BAND_TEXTURE_WIDTH,
        band_height: 1,
        curve_start: (0, 0),
        band_start: (0, 0),
        band_max: (band_max_x, band_max_y),
        bbox,
    }
}

fn build_band_texture(
    num_bands: usize,
    h_headers: &[(u32, u32)],
    v_headers: &[(u32, u32)],
    curve_locs: &[(u32, u32)],
) -> Vec<[u32; 4]> {
    let mut texels = Vec::new();
    let mut curve_offset = 0u32;
    let header_texels = (num_bands * 2) as u32; // H headers + V headers before curve list

    // Headers for H bands: offset = texel index from start of glyph block
    for (count, _) in h_headers {
        let off = header_texels + curve_offset;
        curve_offset += *count;
        texels.push([*count, off, 0, 0]);
    }
    for _ in h_headers.len()..num_bands {
        texels.push([0, header_texels + curve_offset, 0, 0]);
    }

    // Headers for V bands (curve_offset continues from after H curve list)
    for (count, _) in v_headers {
        let off = header_texels + curve_offset;
        curve_offset += *count;
        texels.push([*count, off, 0, 0]);
    }
    for _ in v_headers.len()..num_bands {
        texels.push([0, header_texels + curve_offset, 0, 0]);
    }

    // Curve locations
    for (x, y) in curve_locs {
        texels.push([*x, *y, 0, 0]);
    }

    texels
}
