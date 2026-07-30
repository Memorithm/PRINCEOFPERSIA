//! Small math helpers, a fast deterministic RNG, and hashing for tile variation.

use std::f32::consts::PI;

// ---------------------------------------------------------------- vectors

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

#[inline]
pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

/// `add`/`sub`/`mul` are deliberately plain methods rather than `std::ops`
/// implementations: the rasteriser chains them constantly, and `a.add(b).mul(k)`
/// reads better in that context than a mix of operators and method calls.
#[allow(clippy::should_implement_trait)]
impl V2 {
    pub const ZERO: V2 = V2 { x: 0.0, y: 0.0 };

    #[inline]
    pub fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
    #[inline]
    pub fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
    #[inline]
    pub fn mul(self, k: f32) -> V2 {
        v2(self.x * k, self.y * k)
    }
    #[inline]
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    #[inline]
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 {
            self.mul(1.0 / l)
        } else {
            V2::ZERO
        }
    }
    /// Rotate 90 degrees.
    #[inline]
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    #[inline]
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        v2(lerp(self.x, o.x, t), lerp(self.y, o.y, t))
    }
    /// Mirror horizontally around `x = pivot`.
    #[inline]
    pub fn flip_x(self, pivot: f32) -> V2 {
        v2(2.0 * pivot - self.x, self.y)
    }
}

/// Unit vector for an angle in degrees measured from straight **down**,
/// rotating towards +x as the angle grows. This is the convention used by every
/// joint angle in the animation system: a limb at 0 hangs straight down, and
/// positive angles swing it "forward" (towards the direction the body faces).
#[inline]
pub fn dir_down(deg: f32) -> V2 {
    let r = deg * PI / 180.0;
    v2(r.sin(), r.cos())
}

/// Unit vector for an angle in degrees measured from straight **up**.
#[inline]
pub fn dir_up(deg: f32) -> V2 {
    let r = deg * PI / 180.0;
    v2(r.sin(), -r.cos())
}

// ---------------------------------------------------------------- scalars

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[inline]
pub fn clampi(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[inline]
pub fn smoothstep(t: f32) -> f32 {
    let t = clampf(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Ease that starts fast and settles — used for camera moves.
#[inline]
pub fn ease_out(t: f32) -> f32 {
    let t = clampf(t, 0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

/// Move `cur` towards `target` by at most `step`.
#[inline]
pub fn approach(cur: f32, target: f32, step: f32) -> f32 {
    if cur < target {
        (cur + step).min(target)
    } else {
        (cur - step).max(target)
    }
}

/// Signed distance from point `p` to the segment `a`..`b`.
pub fn dist_seg(p: V2, a: V2, b: V2) -> f32 {
    let ab = b.sub(a);
    let l2 = ab.x * ab.x + ab.y * ab.y;
    let t = if l2 <= 1e-9 {
        0.0
    } else {
        clampf((p.sub(a).x * ab.x + p.sub(a).y * ab.y) / l2, 0.0, 1.0)
    };
    p.sub(a.add(ab.mul(t))).len()
}

/// Parameter along `a`..`b` closest to `p`, clamped to the segment.
pub fn proj_seg(p: V2, a: V2, b: V2) -> f32 {
    let ab = b.sub(a);
    let l2 = ab.x * ab.x + ab.y * ab.y;
    if l2 <= 1e-9 {
        0.0
    } else {
        clampf((p.sub(a).x * ab.x + p.sub(a).y * ab.y) / l2, 0.0, 1.0)
    }
}

// ---------------------------------------------------------------- rects

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }
    pub fn from_size(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        }
    }
    pub fn w(&self) -> f32 {
        self.x1 - self.x0
    }
    pub fn h(&self) -> f32 {
        self.y1 - self.y0
    }
    pub fn cx(&self) -> f32 {
        (self.x0 + self.x1) * 0.5
    }
    pub fn cy(&self) -> f32 {
        (self.y0 + self.y1) * 0.5
    }
    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.x0 && p.x <= self.x1 && p.y >= self.y0 && p.y <= self.y1
    }
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    pub fn inflate(&self, d: f32) -> Rect {
        Rect::new(self.x0 - d, self.y0 - d, self.x1 + d, self.y1 + d)
    }
}

// ---------------------------------------------------------------- rng

/// xorshift64* — tiny, fast, deterministic. Good enough for sparks and dice.
#[derive(Clone)]
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            s: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.s = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    /// Uniform in [-1, 1).
    #[inline]
    pub fn sym(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    #[inline]
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    /// Inclusive integer range.
    pub fn irange(&mut self, a: i32, b: i32) -> i32 {
        if b <= a {
            return a;
        }
        a + (self.next_u32() % ((b - a + 1) as u32)) as i32
    }

    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

/// Stable 2D hash — same tile always gets the same brick pattern.
#[inline]
pub fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E3779B1) ^ (y as u32).wrapping_mul(0x85EBCA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545F491);
    h ^= h >> 13;
    h
}

/// Stable hash producing a float in [0, 1).
#[inline]
pub fn hashf(x: i32, y: i32, salt: i32) -> f32 {
    (hash2(x.wrapping_mul(31) + salt, y) >> 8) as f32 / 16_777_216.0
}

/// Cheap smooth 1D noise, used for torch flicker and camera shake.
pub fn noise1(t: f32, seed: i32) -> f32 {
    let i = t.floor();
    let f = t - i;
    let a = hashf(i as i32, seed, 7) * 2.0 - 1.0;
    let b = hashf(i as i32 + 1, seed, 7) * 2.0 - 1.0;
    lerp(a, b, smoothstep(f))
}
