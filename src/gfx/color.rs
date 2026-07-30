//! 24-bit colour with the few blend operations the renderer needs.

use crate::util::clampf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

/// `add` here is saturating channel addition for the emissive pass, not a
/// general arithmetic operator, so it stays a named method.
#[allow(clippy::should_implement_trait)]
impl Rgb {
    pub const BLACK: Rgb = rgb(0, 0, 0);
    pub const WHITE: Rgb = rgb(255, 255, 255);

    #[inline]
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        let t = clampf(t, 0.0, 1.0);
        rgb(
            (self.r as f32 + (o.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (o.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (o.b as f32 - self.b as f32) * t) as u8,
        )
    }

    /// Multiply brightness, clamping at white.
    #[inline]
    pub fn scale(self, k: f32) -> Rgb {
        rgb(
            clampf(self.r as f32 * k, 0.0, 255.0) as u8,
            clampf(self.g as f32 * k, 0.0, 255.0) as u8,
            clampf(self.b as f32 * k, 0.0, 255.0) as u8,
        )
    }

    /// Per-channel multiply, used by the lighting pass.
    #[inline]
    pub fn modulate(self, k: [f32; 3]) -> Rgb {
        rgb(
            clampf(self.r as f32 * k[0], 0.0, 255.0) as u8,
            clampf(self.g as f32 * k[1], 0.0, 255.0) as u8,
            clampf(self.b as f32 * k[2], 0.0, 255.0) as u8,
        )
    }

    /// Saturating add — the emissive pass.
    #[inline]
    pub fn add(self, o: Rgb) -> Rgb {
        rgb(
            self.r.saturating_add(o.r),
            self.g.saturating_add(o.g),
            self.b.saturating_add(o.b),
        )
    }

    #[inline]
    pub fn add_scaled(self, o: Rgb, k: f32) -> Rgb {
        rgb(
            clampf(self.r as f32 + o.r as f32 * k, 0.0, 255.0) as u8,
            clampf(self.g as f32 + o.g as f32 * k, 0.0, 255.0) as u8,
            clampf(self.b as f32 + o.b as f32 * k, 0.0, 255.0) as u8,
        )
    }

    /// Rough luminance in 0..255.
    #[inline]
    pub fn luma(self) -> f32 {
        0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32
    }

    /// Desaturate towards grey.
    pub fn desaturate(self, amount: f32) -> Rgb {
        let l = self.luma();
        self.lerp(rgb(l as u8, l as u8, l as u8), amount)
    }
}

/// Gamma-ish accumulator for box-filtered downscaling. Averaging in a squared
/// space (approximately linear light) avoids the muddy, too-dark look you get
/// from naively averaging sRGB bytes — it matters a lot when a 560-pixel-wide
/// canvas is squeezed into a 120-column terminal.
#[derive(Clone, Copy, Default)]
pub struct Accum {
    r: f32,
    g: f32,
    b: f32,
    n: f32,
}

impl Accum {
    #[inline]
    pub fn push(&mut self, c: Rgb, w: f32) {
        let (r, g, b) = (c.r as f32, c.g as f32, c.b as f32);
        self.r += r * r * w;
        self.g += g * g * w;
        self.b += b * b * w;
        self.n += w;
    }

    #[inline]
    pub fn resolve(&self) -> Rgb {
        if self.n <= 0.0 {
            return Rgb::BLACK;
        }
        let k = 1.0 / self.n;
        rgb(
            (self.r * k).sqrt() as u8,
            (self.g * k).sqrt() as u8,
            (self.b * k).sqrt() as u8,
        )
    }
}
