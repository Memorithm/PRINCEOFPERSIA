//! Anti-aliased drawing primitives, generic over anything you can blend into.
//!
//! Everything here works in *device pixels* of the target. Coverage is computed
//! analytically for round shapes (signed distance) and by sub-scanline sampling
//! for polygons, so edges come out smooth. Since the scene canvas is itself
//! super-sampled 2x before being squeezed into terminal cells, the result reads
//! as genuinely anti-aliased artwork rather than blocky character graphics.

use crate::gfx::color::Rgb;
use crate::util::{clampf, dist_seg, proj_seg, v2, V2};

pub trait Target {
    /// Valid coordinate range as `(x0, y0, x1, y1)`, half-open. Note this is not
    /// necessarily anchored at the origin: a scratch [`Layer`] accepts the same
    /// canvas coordinates as the canvas itself but only covers a sub-rectangle of
    /// it, so primitives must clamp to these bounds rather than to `0..w`.
    ///
    /// [`Layer`]: crate::gfx::layer::Layer
    fn bounds(&self) -> (i32, i32, i32, i32);
    /// Alpha-blend `c` at `(x, y)`; implementations clip out-of-range writes.
    fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32);
}

// ------------------------------------------------------------------ rects

pub fn fill_rect<T: Target>(t: &mut T, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgb, a: f32) {
    if a <= 0.001 || x1 <= x0 || y1 <= y0 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let iy0 = (y0.floor() as i32).max(by0);
    let iy1 = (y1.ceil() as i32).min(by1);
    let ix0 = (x0.floor() as i32).max(bx0);
    let ix1 = (x1.ceil() as i32).min(bx1);
    for y in iy0..iy1 {
        let cy = span_cov(y as f32, y as f32 + 1.0, y0, y1);
        if cy <= 0.0 {
            continue;
        }
        for x in ix0..ix1 {
            let cx = span_cov(x as f32, x as f32 + 1.0, x0, x1);
            if cx > 0.0 {
                t.blend(x, y, c, a * cx * cy);
            }
        }
    }
}

/// Overlap of `[lo, hi]` with a unit pixel `[p0, p1]`, in 0..1.
#[inline]
fn span_cov(p0: f32, p1: f32, lo: f32, hi: f32) -> f32 {
    let l = p0.max(lo);
    let r = p1.min(hi);
    if r <= l {
        0.0
    } else {
        r - l
    }
}

/// Axis-aligned rect with a soft vertical gradient — the workhorse for stone.
#[allow(clippy::too_many_arguments)]
pub fn fill_rect_grad<T: Target>(
    t: &mut T,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    top: Rgb,
    bot: Rgb,
    a: f32,
) {
    if a <= 0.001 || x1 <= x0 || y1 <= y0 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let iy0 = (y0.floor() as i32).max(by0);
    let iy1 = (y1.ceil() as i32).min(by1);
    let ix0 = (x0.floor() as i32).max(bx0);
    let ix1 = (x1.ceil() as i32).min(bx1);
    let inv = 1.0 / (y1 - y0).max(0.001);
    for y in iy0..iy1 {
        let cy = span_cov(y as f32, y as f32 + 1.0, y0, y1);
        if cy <= 0.0 {
            continue;
        }
        let c = top.lerp(bot, clampf((y as f32 + 0.5 - y0) * inv, 0.0, 1.0));
        for x in ix0..ix1 {
            let cx = span_cov(x as f32, x as f32 + 1.0, x0, x1);
            if cx > 0.0 {
                t.blend(x, y, c, a * cx * cy);
            }
        }
    }
}

// ------------------------------------------------------------------ round shapes

pub fn fill_circle<T: Target>(t: &mut T, c: V2, r: f32, col: Rgb, a: f32) {
    if r <= 0.0 || a <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let x0 = ((c.x - r - 1.0).floor() as i32).max(bx0);
    let x1 = ((c.x + r + 1.0).ceil() as i32).min(bx1);
    let y0 = ((c.y - r - 1.0).floor() as i32).max(by0);
    let y1 = ((c.y + r + 1.0).ceil() as i32).min(by1);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = v2(x as f32 + 0.5, y as f32 + 0.5).sub(c).len() - r;
            let cov = clampf(0.5 - d, 0.0, 1.0);
            if cov > 0.0 {
                t.blend(x, y, col, a * cov);
            }
        }
    }
}

/// Axis-aligned ellipse.
pub fn fill_ellipse<T: Target>(t: &mut T, c: V2, rx: f32, ry: f32, col: Rgb, a: f32) {
    if rx <= 0.0 || ry <= 0.0 || a <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let x0 = ((c.x - rx - 1.0).floor() as i32).max(bx0);
    let x1 = ((c.x + rx + 1.0).ceil() as i32).min(bx1);
    let y0 = ((c.y - ry - 1.0).floor() as i32).max(by0);
    let y1 = ((c.y + ry + 1.0).ceil() as i32).min(by1);
    let s = rx.min(ry);
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - c.x) / rx;
            let dy = (y as f32 + 0.5 - c.y) / ry;
            // Scale the normalised distance back to pixels for a stable edge.
            let d = ((dx * dx + dy * dy).sqrt() - 1.0) * s;
            let cov = clampf(0.5 - d, 0.0, 1.0);
            if cov > 0.0 {
                t.blend(x, y, col, a * cov);
            }
        }
    }
}

/// Tapered capsule: a segment from `a` to `b` with radius `ra` fading to `rb`.
/// This is the single most-used primitive — every limb, every sword, every
/// bone is one of these.
pub fn fill_capsule<T: Target>(t: &mut T, p: V2, q: V2, ra: f32, rb: f32, col: Rgb, alpha: f32) {
    let rmax = ra.max(rb);
    if rmax <= 0.0 || alpha <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let x0 = ((p.x.min(q.x) - rmax - 1.0).floor() as i32).max(bx0);
    let x1 = ((p.x.max(q.x) + rmax + 1.0).ceil() as i32).min(bx1);
    let y0 = ((p.y.min(q.y) - rmax - 1.0).floor() as i32).max(by0);
    let y1 = ((p.y.max(q.y) + rmax + 1.0).ceil() as i32).min(by1);
    for y in y0..y1 {
        for x in x0..x1 {
            let pt = v2(x as f32 + 0.5, y as f32 + 0.5);
            let tt = proj_seg(pt, p, q);
            let r = ra + (rb - ra) * tt;
            let d = dist_seg(pt, p, q) - r;
            let cov = clampf(0.5 - d, 0.0, 1.0);
            if cov > 0.0 {
                t.blend(x, y, col, alpha * cov);
            }
        }
    }
}

/// Capsule shaded along its length, from `col_a` at `p` to `col_b` at `q`.
#[allow(clippy::too_many_arguments)]
pub fn fill_capsule_grad<T: Target>(
    t: &mut T,
    p: V2,
    q: V2,
    ra: f32,
    rb: f32,
    col_a: Rgb,
    col_b: Rgb,
    alpha: f32,
) {
    let rmax = ra.max(rb);
    if rmax <= 0.0 || alpha <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let x0 = ((p.x.min(q.x) - rmax - 1.0).floor() as i32).max(bx0);
    let x1 = ((p.x.max(q.x) + rmax + 1.0).ceil() as i32).min(bx1);
    let y0 = ((p.y.min(q.y) - rmax - 1.0).floor() as i32).max(by0);
    let y1 = ((p.y.max(q.y) + rmax + 1.0).ceil() as i32).min(by1);
    for y in y0..y1 {
        for x in x0..x1 {
            let pt = v2(x as f32 + 0.5, y as f32 + 0.5);
            let tt = proj_seg(pt, p, q);
            let r = ra + (rb - ra) * tt;
            let d = dist_seg(pt, p, q) - r;
            let cov = clampf(0.5 - d, 0.0, 1.0);
            if cov > 0.0 {
                t.blend(x, y, col_a.lerp(col_b, tt), alpha * cov);
            }
        }
    }
}

pub fn stroke_line<T: Target>(t: &mut T, p: V2, q: V2, width: f32, col: Rgb, a: f32) {
    fill_capsule(t, p, q, width * 0.5, width * 0.5, col, a);
}

// ------------------------------------------------------------------ polygons

const SUBROWS: i32 = 4;

/// Even-odd fill of a simple polygon with 4x vertical sub-sampling and
/// analytic horizontal coverage.
pub fn fill_poly<T: Target>(t: &mut T, pts: &[V2], col: Rgb, alpha: f32) {
    fill_poly_shaded(t, pts, col, col, alpha);
}

/// Same, but shaded top-to-bottom across the polygon's bounding box.
pub fn fill_poly_shaded<T: Target>(t: &mut T, pts: &[V2], top: Rgb, bot: Rgb, alpha: f32) {
    if pts.len() < 3 || alpha <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let mut ymin = f32::MAX;
    let mut ymax = f32::MIN;
    let mut xmin = f32::MAX;
    let mut xmax = f32::MIN;
    for p in pts {
        ymin = ymin.min(p.y);
        ymax = ymax.max(p.y);
        xmin = xmin.min(p.x);
        xmax = xmax.max(p.x);
    }
    let iy0 = (ymin.floor() as i32).max(by0);
    let iy1 = (ymax.ceil() as i32).min(by1);
    let ix0 = (xmin.floor() as i32).max(bx0);
    let ix1 = (xmax.ceil() as i32).min(bx1);
    if iy1 <= iy0 || ix1 <= ix0 {
        return;
    }
    let span = (ix1 - ix0) as usize;
    let mut cov = vec![0f32; span];
    let mut xs: Vec<f32> = Vec::with_capacity(8);
    let inv_h = 1.0 / (ymax - ymin).max(0.001);
    let wsub = 1.0 / SUBROWS as f32;

    for y in iy0..iy1 {
        for c in cov.iter_mut() {
            *c = 0.0;
        }
        let mut any = false;
        for s in 0..SUBROWS {
            let sy = y as f32 + (s as f32 + 0.5) * wsub;
            xs.clear();
            for i in 0..pts.len() {
                let a = pts[i];
                let b = pts[(i + 1) % pts.len()];
                if (a.y <= sy) != (b.y <= sy) {
                    let f = (sy - a.y) / (b.y - a.y);
                    xs.push(a.x + f * (b.x - a.x));
                }
            }
            if xs.len() < 2 {
                continue;
            }
            xs.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
            let mut i = 0;
            while i + 1 < xs.len() {
                let (l, r) = (xs[i], xs[i + 1]);
                if r > l {
                    any = true;
                    accum_span(&mut cov, ix0, ix1, l, r, wsub);
                }
                i += 2;
            }
        }
        if !any {
            continue;
        }
        let c = top.lerp(bot, clampf((y as f32 + 0.5 - ymin) * inv_h, 0.0, 1.0));
        for (i, &v) in cov.iter().enumerate() {
            if v > 0.002 {
                t.blend(ix0 + i as i32, y, c, alpha * v.min(1.0));
            }
        }
    }
}

#[inline]
fn accum_span(cov: &mut [f32], ix0: i32, ix1: i32, l: f32, r: f32, weight: f32) {
    let l = l.max(ix0 as f32);
    let r = r.min(ix1 as f32);
    if r <= l {
        return;
    }
    let px0 = l.floor() as i32;
    let px1 = (r.ceil() as i32).min(ix1);
    for x in px0..px1 {
        let c = span_cov(x as f32, x as f32 + 1.0, l, r);
        if c > 0.0 {
            let idx = (x - ix0) as usize;
            if idx < cov.len() {
                cov[idx] += c * weight;
            }
        }
    }
}

/// Outline a polygon by stroking its edges.
pub fn stroke_poly<T: Target>(t: &mut T, pts: &[V2], width: f32, col: Rgb, a: f32) {
    for i in 0..pts.len() {
        stroke_line(t, pts[i], pts[(i + 1) % pts.len()], width, col, a);
    }
}

/// Radial glow, additive-friendly: brightest at the centre, quadratic falloff.
pub fn radial_glow<T: Target>(t: &mut T, c: V2, r: f32, col: Rgb, peak: f32) {
    if r <= 0.0 || peak <= 0.001 {
        return;
    }
    let (bx0, by0, bx1, by1) = t.bounds();
    let x0 = ((c.x - r).floor() as i32).max(bx0);
    let x1 = ((c.x + r).ceil() as i32).min(bx1);
    let y0 = ((c.y - r).floor() as i32).max(by0);
    let y1 = ((c.y + r).ceil() as i32).min(by1);
    let inv = 1.0 / r;
    for y in y0..y1 {
        for x in x0..x1 {
            let d = v2(x as f32 + 0.5, y as f32 + 0.5).sub(c).len() * inv;
            if d < 1.0 {
                let f = 1.0 - d;
                t.blend(x, y, col, peak * f * f);
            }
        }
    }
}
