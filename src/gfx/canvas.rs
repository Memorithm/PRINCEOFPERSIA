//! The scene canvas: a plain RGB pixel buffer plus the post-processing passes
//! (lighting, emissive, vignette) and the box-filtered resample that feeds the
//! terminal.

use crate::gfx::color::{rgb, Accum, Rgb};
use crate::gfx::target::Target;
use crate::util::{clampf, v2, V2};

/// How colour written into the canvas combines with what is already there.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Normal alpha compositing.
    Alpha,
    /// Additive — used for flames, sparks and glows after the light pass.
    Add,
}

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Rgb>,
    pub blend: Blend,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Self {
        let w = w.max(1);
        let h = h.max(1);
        Canvas {
            w,
            h,
            px: vec![Rgb::BLACK; (w * h) as usize],
            blend: Blend::Alpha,
        }
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        let w = w.max(1);
        let h = h.max(1);
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            self.px = vec![Rgb::BLACK; (w * h) as usize];
        }
    }

    pub fn clear(&mut self, c: Rgb) {
        for p in self.px.iter_mut() {
            *p = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            Rgb::BLACK
        } else {
            self.px[(y * self.w + x) as usize]
        }
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    /// Vertical gradient over the whole canvas — the base "air" of a room.
    pub fn fill_gradient(&mut self, top: Rgb, bot: Rgb) {
        for y in 0..self.h {
            let c = top.lerp(bot, y as f32 / (self.h - 1).max(1) as f32);
            for x in 0..self.w {
                self.px[(y * self.w + x) as usize] = c;
            }
        }
    }

    /// Darken towards the edges. Cheap, and it does a lot of work selling the
    /// "underground torch-lit dungeon" mood.
    pub fn vignette(&mut self, strength: f32, tint: Rgb) {
        if strength <= 0.001 {
            return;
        }
        let cx = self.w as f32 * 0.5;
        let cy = self.h as f32 * 0.5;
        let inv = 1.0 / (cx * cx + cy * cy).sqrt();
        for y in 0..self.h {
            for x in 0..self.w {
                let d = v2(x as f32 - cx, y as f32 - cy).len() * inv;
                let f = clampf((d - 0.42) / 0.58, 0.0, 1.0);
                let k = f * f * strength;
                if k > 0.002 {
                    let i = (y * self.w + x) as usize;
                    self.px[i] = self.px[i].lerp(tint, k);
                }
            }
        }
    }

    /// Box-filtered resample into a flat pixel grid of `dw` x `dh`.
    /// Averaging happens in a squared (roughly linear-light) space, which keeps
    /// the image from turning muddy when a 560-wide canvas is squeezed into a
    /// 120-column terminal.
    pub fn resample_into(&self, dst: &mut [Rgb], dw: i32, dh: i32) {
        if dw <= 0 || dh <= 0 {
            return;
        }
        let sx = self.w as f32 / dw as f32;
        let sy = self.h as f32 / dh as f32;
        // Fast path: 1:1 or upscale — nearest neighbour keeps edges crisp.
        if sx <= 1.001 && sy <= 1.001 {
            for y in 0..dh {
                let src_y = ((y as f32 + 0.5) * sy) as i32;
                for x in 0..dw {
                    let src_x = ((x as f32 + 0.5) * sx) as i32;
                    dst[(y * dw + x) as usize] = self.get(src_x, src_y);
                }
            }
            return;
        }
        for y in 0..dh {
            let fy0 = y as f32 * sy;
            let fy1 = fy0 + sy;
            let iy0 = fy0.floor() as i32;
            let iy1 = (fy1.ceil() as i32).min(self.h);
            for x in 0..dw {
                let fx0 = x as f32 * sx;
                let fx1 = fx0 + sx;
                let ix0 = fx0.floor() as i32;
                let ix1 = (fx1.ceil() as i32).min(self.w);
                let mut acc = Accum::default();
                for yy in iy0.max(0)..iy1 {
                    let wy = (fy1.min(yy as f32 + 1.0) - fy0.max(yy as f32)).max(0.0);
                    if wy <= 0.0 {
                        continue;
                    }
                    let row = (yy * self.w) as usize;
                    for xx in ix0.max(0)..ix1 {
                        let wx = (fx1.min(xx as f32 + 1.0) - fx0.max(xx as f32)).max(0.0);
                        if wx > 0.0 {
                            acc.push(self.px[row + xx as usize], wx * wy);
                        }
                    }
                }
                dst[(y * dw + x) as usize] = acc.resolve();
            }
        }
    }
}

impl Target for Canvas {
    #[inline]
    fn bounds(&self) -> (i32, i32, i32, i32) {
        (0, 0, self.w, self.h)
    }

    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let i = (y * self.w + x) as usize;
        let d = self.px[i];
        self.px[i] = match self.blend {
            Blend::Alpha => {
                if a >= 0.999 {
                    c
                } else {
                    d.lerp(c, a)
                }
            }
            Blend::Add => d.add_scaled(c, a),
        };
    }
}

// ---------------------------------------------------------------- camera

/// World (art pixels) to device (canvas pixels) transform.
#[derive(Clone, Copy)]
pub struct Cam {
    /// World coordinate mapped to device x = 0.
    pub ox: f32,
    pub oy: f32,
    /// Device pixels per world pixel (the super-sampling factor).
    pub s: f32,
}

impl Cam {
    #[inline]
    pub fn p(&self, w: V2) -> V2 {
        v2((w.x - self.ox) * self.s, (w.y - self.oy) * self.s)
    }
    #[inline]
    pub fn x(&self, wx: f32) -> f32 {
        (wx - self.ox) * self.s
    }
    #[inline]
    pub fn y(&self, wy: f32) -> f32 {
        (wy - self.oy) * self.s
    }
    /// Scale a length.
    #[inline]
    pub fn l(&self, len: f32) -> f32 {
        len * self.s
    }
}

/// A brightness field sampled bilinearly over the canvas. The whole scene is
/// drawn at full brightness first and then multiplied by this in one pass,
/// which means torchlight automatically falls on floors, walls, the prince and
/// the guards alike.
pub struct LightField {
    pub gw: i32,
    pub gh: i32,
    pub step: i32,
    pub v: Vec<[f32; 3]>,
}

impl LightField {
    pub fn new() -> Self {
        LightField {
            gw: 0,
            gh: 0,
            step: 8,
            v: Vec::new(),
        }
    }

    /// Reset the grid to `ambient` for a canvas of the given size.
    pub fn begin(&mut self, w: i32, h: i32, ambient: [f32; 3]) {
        self.step = 8;
        let gw = w / self.step + 2;
        let gh = h / self.step + 2;
        if gw != self.gw || gh != self.gh {
            self.gw = gw;
            self.gh = gh;
            self.v = vec![ambient; (gw * gh) as usize];
        } else {
            for c in self.v.iter_mut() {
                *c = ambient;
            }
        }
    }

    /// Add a point light in device coordinates.
    pub fn add(&mut self, c: V2, radius: f32, col: Rgb, intensity: f32) {
        if radius <= 0.0 || intensity <= 0.0 {
            return;
        }
        let st = self.step as f32;
        let gx0 = (((c.x - radius) / st).floor() as i32).max(0);
        let gx1 = (((c.x + radius) / st).ceil() as i32 + 1).min(self.gw);
        let gy0 = (((c.y - radius) / st).floor() as i32).max(0);
        let gy1 = (((c.y + radius) / st).ceil() as i32 + 1).min(self.gh);
        let inv = 1.0 / radius;
        let cr = col.r as f32 / 255.0;
        let cg = col.g as f32 / 255.0;
        let cb = col.b as f32 / 255.0;
        for gy in gy0..gy1 {
            for gx in gx0..gx1 {
                let p = v2(gx as f32 * st, gy as f32 * st);
                let d = p.sub(c).len() * inv;
                if d >= 1.0 {
                    continue;
                }
                // Smooth quadratic falloff with a bright core.
                let f = (1.0 - d) * (1.0 - d) * intensity;
                let s = &mut self.v[(gy * self.gw + gx) as usize];
                s[0] += cr * f;
                s[1] += cg * f;
                s[2] += cb * f;
            }
        }
    }

    #[inline]
    fn sample(&self, x: f32, y: f32) -> [f32; 3] {
        let fx = x / self.step as f32;
        let fy = y / self.step as f32;
        let ix = fx.floor() as i32;
        let iy = fy.floor() as i32;
        let tx = fx - ix as f32;
        let ty = fy - iy as f32;
        let g = |gx: i32, gy: i32| -> [f32; 3] {
            let gx = gx.clamp(0, self.gw - 1);
            let gy = gy.clamp(0, self.gh - 1);
            self.v[(gy * self.gw + gx) as usize]
        };
        let a = g(ix, iy);
        let b = g(ix + 1, iy);
        let c = g(ix, iy + 1);
        let d = g(ix + 1, iy + 1);
        let mut out = [0f32; 3];
        for i in 0..3 {
            let top = a[i] + (b[i] - a[i]) * tx;
            let bot = c[i] + (d[i] - c[i]) * tx;
            out[i] = top + (bot - top) * ty;
        }
        out
    }

    /// Multiply the canvas by the field.
    pub fn apply(&self, cv: &mut Canvas) {
        if self.v.is_empty() {
            return;
        }
        for y in 0..cv.h {
            for x in 0..cv.w {
                let l = self.sample(x as f32 + 0.5, y as f32 + 0.5);
                let i = (y * cv.w + x) as usize;
                cv.px[i] = cv.px[i].modulate(l);
            }
        }
    }
}

impl Default for LightField {
    fn default() -> Self {
        Self::new()
    }
}

/// Ordered dither applied to the very last pass. It breaks up the flat bands
/// that show up in large dark areas once everything has been multiplied down.
pub fn dither(cv: &mut Canvas, amount: f32) {
    if amount <= 0.001 {
        return;
    }
    const M: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    for y in 0..cv.h {
        for x in 0..cv.w {
            let t = (M[(y & 3) as usize][(x & 3) as usize] as f32 / 15.0 - 0.5) * amount;
            let i = (y * cv.w + x) as usize;
            let c = cv.px[i];
            cv.px[i] = rgb(
                clampf(c.r as f32 + t, 0.0, 255.0) as u8,
                clampf(c.g as f32 + t, 0.0, 255.0) as u8,
                clampf(c.b as f32 + t, 0.0, 255.0) as u8,
            );
        }
    }
}
