//! Font loading and outline extraction using rustybuzz (ttf-parser) and OutlineBuilder.

use rustybuzz::Face as RustyFace;
use ttf_parser::{Face, OutlineBuilder};
use crate::font::curves::{QuadraticCurve, GlyphOutlines};

/// Pick the best TTC/OTC face index by probing metrics. When hmtx table offsets are wrong
/// (e.g. wrong face index), we read LSB instead of advance (e.g. 36) or garbage (65535).
/// This probes each face and returns the index where Latin advance is sane (not LSB, not garbage).
pub fn pick_ttc_face_index(bytes: &[u8], num_faces: u32) -> u32 {
    // Sane advance for 'l' in font units: 50-2000 (excludes LSB ~36 and garbage 65535)
    const SANE_ADVANCE_MIN: u16 = 50;
    const SANE_ADVANCE_MAX: u16 = 2000;

    for i in 0..num_faces.min(4) {
        if let Ok(face) = Face::parse(bytes, i) {
            if let Some(gid) = face.glyph_index('l') {
                if let Some(adv) = face.glyph_hor_advance(gid) {
                    if (SANE_ADVANCE_MIN..=SANE_ADVANCE_MAX).contains(&adv) {
                        return i;
                    }
                }
            }
        }
    }
    // Heuristic fallback: SC often at index 2 in CJK OTC
    if num_faces >= 3 {
        2
    } else {
        0
    }
}

/// Loads TTF fonts and extracts glyph outlines as quadratic Bézier curves.
/// Holds rustybuzz::Face for shaping; outline extraction uses the underlying ttf_parser Face (deref).
/// The font bytes are leaked to allow the Face to live for 'static.
pub struct FontLoader {
    #[allow(dead_code)]
    data: &'static [u8],
    face: RustyFace<'static>,
    units_per_em: u16,
}

impl FontLoader {
    /// Load a font from bytes. The bytes are leaked to satisfy Face lifetime.
    /// For TTC/OTC collections, use [`from_bytes_with_index`] with the correct face index.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        Self::from_bytes_with_index(bytes, 0)
    }

    /// Load a font from bytes with a specific face index (for TTC/OTC collections).
    /// Use [`ttf_parser::fonts_in_collection`] to get the number of faces; for Noto CJK OTC,
    /// Simplified Chinese (SC) is often at index 2 (JP=0, KR=1, SC=2, TC=3).
    pub fn from_bytes_with_index(bytes: Vec<u8>, face_index: u32) -> Result<Self, String> {
        let leaked = Box::leak(bytes.into_boxed_slice());
        let face = RustyFace::from_slice(leaked, face_index)
            .ok_or_else(|| "Failed to parse font")?;
        let units_per_em = face.units_per_em() as u16;
        Ok(Self {
            data: leaked,
            face,
            units_per_em,
        })
    }

    /// Reference to the rustybuzz Face for text shaping.
    pub fn face(&self) -> &RustyFace<'static> {
        &self.face
    }

    /// Extract outlines for a glyph. Returns curves in em-space (0..1).
    pub fn load_glyph(&self, glyph_id: ttf_parser::GlyphId) -> Option<GlyphOutlines> {
        let mut builder = OutlineCollector::new(self.units_per_em);
        self.face.outline_glyph(glyph_id, &mut builder)
            .map(|_| builder.finish())
    }

    /// Get glyph ID for a codepoint.
    pub fn glyph_index(&self, c: char) -> Option<ttf_parser::GlyphId> {
        self.face.glyph_index(c)
    }

    /// Get advance width in font units.
    pub fn advance_width(&self, glyph_id: ttf_parser::GlyphId) -> Option<u16> {
        self.face.glyph_hor_advance(glyph_id)
    }

    /// Get horizontal metrics.
    pub fn glyph_bounds(&self, glyph_id: ttf_parser::GlyphId) -> Option<ttf_parser::Rect> {
        self.face.glyph_bounding_box(glyph_id)
    }

    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
}

/// Implements OutlineBuilder to collect quadratic Bézier curves.
struct OutlineCollector {
    units_per_em: u16,
    contours: Vec<Vec<QuadraticCurve>>,
    current_contour: Vec<QuadraticCurve>,
    last_point: (f32, f32),
    start_point: (f32, f32),
}

impl OutlineCollector {
    fn new(units_per_em: u16) -> Self {
        Self {
            units_per_em,
            contours: Vec::new(),
            current_contour: Vec::new(),
            last_point: (0.0, 0.0),
            start_point: (0.0, 0.0),
        }
    }

    fn finish(mut self) -> GlyphOutlines {
        if !self.current_contour.is_empty() {
            self.contours.push(std::mem::take(&mut self.current_contour));
        }
        let upem = self.units_per_em as f32;
        let curves: Vec<QuadraticCurve> = self
            .contours
            .into_iter()
            .flatten()
            .map(|c| QuadraticCurve {
                p1: (c.p1.0 / upem, c.p1.1 / upem),
                p2: (c.p2.0 / upem, c.p2.1 / upem),
                p3: (c.p3.0 / upem, c.p3.1 / upem),
            })
            .collect();
        GlyphOutlines {
            curves,
            units_per_em: self.units_per_em,
        }
    }
}

impl OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current_contour.is_empty() {
            self.contours.push(std::mem::take(&mut self.current_contour));
        }
        self.last_point = (x, y);
        self.start_point = (x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        // Convert line to degenerate quadratic: p1 = last, p2 = midpoint, p3 = (x,y)
        let (lx, ly) = self.last_point;
        let mx = (lx + x) / 2.0;
        let my = (ly + y) / 2.0;
        self.current_contour.push(QuadraticCurve {
            p1: (lx, ly),
            p2: (mx, my),
            p3: (x, y),
        });
        self.last_point = (x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (lx, ly) = self.last_point;
        self.current_contour.push(QuadraticCurve {
            p1: (lx, ly),
            p2: (x1, y1),
            p3: (x, y),
        });
        self.last_point = (x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // Convert cubic to quadratic(s) via recursive subdivision (Casteljau)
        let (lx, ly) = self.last_point;
        let cubic = [(lx, ly), (x1, y1), (x2, y2), (x, y)];
        let quads = cubic_to_quadratics(&cubic);
        for q in quads {
            self.current_contour.push(q);
        }
        self.last_point = (x, y);
    }

    fn close(&mut self) {
        if (self.last_point.0 - self.start_point.0).abs() > 0.001
            || (self.last_point.1 - self.start_point.1).abs() > 0.001
        {
            self.line_to(self.start_point.0, self.start_point.1);
        }
    }
}

/// Approximate a cubic Bézier with quadratic Bézier curves using degree elevation.
/// Uses a simple subdivision approach: split at t=0.5 and approximate each half.
fn cubic_to_quadratics(cubic: &[(f32, f32); 4]) -> Vec<QuadraticCurve> {
    let (p0, p1, p2, p3) = (cubic[0], cubic[1], cubic[2], cubic[3]);
    // Midpoint of cubic at t=0.5
    let t = 0.5;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    let mid_x = mt3 * p0.0 + 3.0 * mt2 * t * p1.0 + 3.0 * mt * t2 * p2.0 + t3 * p3.0;
    let mid_y = mt3 * p0.1 + 3.0 * mt2 * t * p1.1 + 3.0 * mt * t2 * p2.1 + t3 * p3.1;
    // Control points for first quadratic (p0 to mid)
    let c1_x = (2.0 * p0.0 + p1.0) / 3.0;
    let c1_y = (2.0 * p0.1 + p1.1) / 3.0;
    let c2_x = (p0.0 + 2.0 * p1.0) / 3.0;
    let c2_y = (p0.1 + 2.0 * p1.1) / 3.0;
    let q1_cp_x = (c1_x + c2_x) / 2.0;
    let q1_cp_y = (c1_y + c2_y) / 2.0;
    // Control for second quadratic (mid to p3)
    let c3_x = (2.0 * p2.0 + p3.0) / 3.0;
    let c3_y = (2.0 * p2.1 + p3.1) / 3.0;
    let c4_x = (p2.0 + 2.0 * p3.0) / 3.0;
    let c4_y = (p2.1 + 2.0 * p3.1) / 3.0;
    let q2_cp_x = (c3_x + c4_x) / 2.0;
    let q2_cp_y = (c3_y + c4_y) / 2.0;
    vec![
        QuadraticCurve {
            p1: p0,
            p2: (q1_cp_x, q1_cp_y),
            p3: (mid_x, mid_y),
        },
        QuadraticCurve {
            p1: (mid_x, mid_y),
            p2: (q2_cp_x, q2_cp_y),
            p3: p3,
        },
    ]
}
