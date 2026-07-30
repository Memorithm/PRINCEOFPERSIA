//! A small scratch layer used to draw a character in isolation so it can be
//! given a clean silhouette outline before being composited into the scene.
//!
//! Drawing limbs straight onto the canvas would put outlines *between* limbs.
//! Instead every figure is painted into a local coverage layer, the coverage is
//! dilated by a pixel or two, and the ring that appears outside the figure is
//! filled with the outline colour. That is what gives the sprites their crisp
//! read against a busy brick background.

use crate::gfx::canvas::Canvas;
use crate::gfx::color::Rgb;
use crate::gfx::target::Target;

pub struct Layer {
    pub w: i32,
    pub h: i32,
    /// Device offset of the layer's (0, 0) within the canvas.
    pub ox: i32,
    pub oy: i32,
    pub col: Vec<Rgb>,
    pub cov: Vec<f32>,
    dil: Vec<f32>,
}

impl Layer {
    pub fn new() -> Self {
        Layer {
            w: 0,
            h: 0,
            ox: 0,
            oy: 0,
            col: Vec::new(),
            cov: Vec::new(),
            dil: Vec::new(),
        }
    }

    /// Prepare for a figure whose device-space bounding box is given.
    pub fn begin(&mut self, ox: i32, oy: i32, w: i32, h: i32) {
        self.ox = ox;
        self.oy = oy;
        self.w = w.max(1);
        self.h = h.max(1);
        let n = (self.w * self.h) as usize;
        if self.col.len() != n {
            self.col = vec![Rgb::BLACK; n];
            self.cov = vec![0.0; n];
            self.dil = vec![0.0; n];
        } else {
            for c in self.cov.iter_mut() {
                *c = 0.0;
            }
        }
    }

    /// Composite onto the canvas: outline ring first, then the figure itself.
    pub fn composite(&mut self, cv: &mut Canvas, outline: Rgb, ol_alpha: f32, radius: i32, alpha: f32) {
        if self.w <= 0 || self.h <= 0 {
            return;
        }
        if radius > 0 && ol_alpha > 0.001 {
            self.dilate(radius);
            for y in 0..self.h {
                for x in 0..self.w {
                    let i = (y * self.w + x) as usize;
                    let inside = self.cov[i];
                    let d = self.dil[i];
                    if d > 0.01 && inside < 0.99 {
                        let a = (d - inside).max(0.0) * ol_alpha * alpha;
                        if a > 0.004 {
                            cv.blend(self.ox + x, self.oy + y, outline, a);
                        }
                    }
                }
            }
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let i = (y * self.w + x) as usize;
                let c = self.cov[i];
                if c > 0.004 {
                    cv.blend(self.ox + x, self.oy + y, self.col[i], c.min(1.0) * alpha);
                }
            }
        }
    }

    /// Separable max-filter: dilate coverage by `r` pixels.
    fn dilate(&mut self, r: i32) {
        let (w, h) = (self.w, self.h);
        // Horizontal pass into dil.
        for y in 0..h {
            for x in 0..w {
                let mut m = 0.0f32;
                let x0 = (x - r).max(0);
                let x1 = (x + r).min(w - 1);
                for xx in x0..=x1 {
                    let v = self.cov[(y * w + xx) as usize];
                    if v > m {
                        m = v;
                    }
                }
                self.dil[(y * w + x) as usize] = m;
            }
        }
        // Vertical pass in place (read from a copy of the column window).
        for x in 0..w {
            let mut column = Vec::with_capacity(h as usize);
            for y in 0..h {
                column.push(self.dil[(y * w + x) as usize]);
            }
            for y in 0..h {
                let mut m = 0.0f32;
                let y0 = (y - r).max(0);
                let y1 = (y + r).min(h - 1);
                for yy in y0..=y1 {
                    let v = column[yy as usize];
                    if v > m {
                        m = v;
                    }
                }
                self.dil[(y * w + x) as usize] = m;
            }
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new()
    }
}

impl Target for Layer {
    #[inline]
    fn bounds(&self) -> (i32, i32, i32, i32) {
        (self.ox, self.oy, self.ox + self.w, self.oy + self.h)
    }

    /// `x`/`y` are canvas coordinates; the layer translates them itself so the
    /// same drawing code can target either a canvas or a scratch layer.
    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        let x = x - self.ox;
        let y = y - self.oy;
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let i = (y * self.w + x) as usize;
        let old = self.cov[i];
        let a = a.min(1.0);
        // Painter's order within the figure: later shapes cover earlier ones.
        let new_cov = old + (1.0 - old) * a;
        if new_cov > 0.0 {
            // Blend colour weighted by the contribution of this shape.
            let contrib = a;
            let keep = old * (1.0 - a);
            let total = contrib + keep;
            if total > 0.0 {
                self.col[i] = self.col[i].lerp(c, contrib / total);
            }
        }
        self.cov[i] = new_cov;
    }
}
