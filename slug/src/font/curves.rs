//! Quadratic Bézier curve representation and bounding box.

/// A quadratic Bézier curve with control points p1, p2, p3.
/// C(t) = (1-t)^2 p1 + 2t(1-t) p2 + t^2 p3
#[derive(Clone, Debug)]
pub struct QuadraticCurve {
    pub p1: (f32, f32),
    pub p2: (f32, f32),
    pub p3: (f32, f32),
}

impl QuadraticCurve {
    pub fn min_x(&self) -> f32 {
        self.p1.0.min(self.p2.0).min(self.p3.0)
    }
    pub fn max_x(&self) -> f32 {
        self.p1.0.max(self.p2.0).max(self.p3.0)
    }
    pub fn min_y(&self) -> f32 {
        self.p1.1.min(self.p2.1).min(self.p3.1)
    }
    pub fn max_y(&self) -> f32 {
        self.p1.1.max(self.p2.1).max(self.p3.1)
    }
}

/// Glyph outlines: a list of quadratic Bézier curves in em-space (0..1).
#[derive(Clone, Debug)]
pub struct GlyphOutlines {
    pub curves: Vec<QuadraticCurve>,
    pub units_per_em: u16,
}

impl GlyphOutlines {
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for c in &self.curves {
            min_x = min_x.min(c.min_x());
            min_y = min_y.min(c.min_y());
            max_x = max_x.max(c.max_x());
            max_y = max_y.max(c.max_y());
        }
        (min_x, min_y, max_x, max_y)
    }
}
