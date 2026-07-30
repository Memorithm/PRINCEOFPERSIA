//! Drawing characters as *shapes* rather than as stacks of capsules.
//!
//! A limb built from a tapered capsule is a tube, and a body built from tubes
//! reads as a balloon animal however carefully it is shaded. Two things make a
//! drawn figure read instead:
//!
//! 1. **An authored silhouette.** A real arm is not a cone: it swells at the
//!    deltoid, again at the bicep, pinches at the elbow, swells at the forearm and
//!    narrows to the wrist — and it does so *asymmetrically*, because the bicep is
//!    at the front and the triceps at the back. So each limb here carries a
//!    [`Profile`]: a list of (position along the bone, front half-width, back
//!    half-width). That profile is what the eye actually reads.
//!
//! 2. **Hard-edged shading.** Smooth cylindrical falloff looks like plastic. Cel
//!    shading — a flat base tone, a flat shadow tone with a *crisp* boundary, and
//!    a thin lit rim — is what character art has always used, and it survives
//!    being squeezed into a terminal far better than a gradient does.
//!
//! Everything a character is made of goes through [`Limb`] or [`cel_poly`], so the
//! whole figure shares one lighting model and one set of tones, and [`contour`]
//! puts the drawn line back where two forms of the same colour overlap.
//!
//! All of this works in **device pixels**: callers project through the camera
//! first, so a profile's half-widths are multiplied by an already-scaled girth.

use crate::gfx::color::Rgb;
use crate::gfx::target::{fill_capsule, fill_poly, Target};
use crate::util::{clampf, lerp, v2, V2};

/// A limb's silhouette: `(t, front half-width, back half-width)`, with `t` running
/// from 0 at the start joint to 1 at the end joint. Widths are in girth units.
pub type Profile = &'static [(f32, f32, f32)];

/// Fraction of the width, measured from the shadow edge, that the core shadow
/// covers.
const SHADOW: f32 = 0.44;
/// Fraction covered by the lit rim, measured from the lit edge.
const RIM: f32 = 0.15;
/// Multipliers applied to the base tone.
const SHADE_MUL: f32 = 0.56;
const RIM_MUL: f32 = 1.20;

/// Sample a profile at `t`, returning `(front, back)` half-widths.
fn sample(prof: Profile, t: f32) -> (f32, f32) {
    if prof.is_empty() {
        return (0.0, 0.0);
    }
    if t <= prof[0].0 {
        return (prof[0].1, prof[0].2);
    }
    for w in prof.windows(2) {
        let (t0, f0, b0) = w[0];
        let (t1, f1, b1) = w[1];
        if t <= t1 {
            let k = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
            return (lerp(f0, f1, k), lerp(b0, b1, k));
        }
    }
    let l = prof[prof.len() - 1];
    (l.1, l.2)
}

/// The anatomical *front* of a bone running `a` → `b` on a figure facing
/// `facing`: perpendicular to the bone and rotating with it, so a raised thigh
/// keeps its quadriceps on the correct side.
pub fn front_of(a: V2, b: V2, facing: f32) -> V2 {
    let d = b.sub(a);
    if d.len() < 1e-4 {
        return v2(if facing < 0.0 { -1.0 } else { 1.0 }, 0.0);
    }
    // `perp` turns (0,1) — straight down in device space — into (-1,0), so the
    // sign flip puts the front of a hanging limb on the facing side.
    d.norm().perp().mul(if facing < 0.0 { 1.0 } else { -1.0 })
}

/// One bone's worth of drawable body: a segment plus the silhouette wrapped
/// around it.
#[derive(Clone, Copy)]
pub struct Limb {
    pub a: V2,
    pub b: V2,
    /// Unit vector towards the limb's anatomical front.
    pub front: V2,
    pub prof: Profile,
    /// Half-width multiplier, already in device pixels.
    pub girth: f32,
    /// Samples along the bone. More is smoother and slower; 8 is plenty for an
    /// arm, 10 for a torso.
    pub steps: usize,
    /// Multiplier for the core shadow. A big form turning away from the light —
    /// the back of a torso — needs a deeper shadow than a thin one like a
    /// forearm, or the near limbs have nothing to come forward from.
    pub shade: f32,
}

impl Limb {
    pub fn new(a: V2, b: V2, front: V2, prof: Profile, girth: f32) -> Limb {
        Limb {
            a,
            b,
            front,
            prof,
            girth,
            steps: 8,
            shade: SHADE_MUL,
        }
    }

    pub fn steps(mut self, n: usize) -> Limb {
        self.steps = n.max(3);
        self
    }

    pub fn shade(mut self, k: f32) -> Limb {
        self.shade = k;
        self
    }

    /// A point on the limb's boundary: `t` along the bone, `k` across it (0 at the
    /// back edge, 1 at the front), pushed `pad` pixels further out.
    fn edge_pt(&self, t: f32, k: f32, pad: f32) -> V2 {
        let (fw, bw) = sample(self.prof, t);
        let w = lerp(-(bw * self.girth + pad), fw * self.girth + pad, k);
        self.a.lerp(self.b, t).add(self.front.mul(w))
    }

    /// The band of the limb between two fractions across its width.
    pub fn slice(&self, lo: f32, hi: f32, pad: f32) -> Vec<V2> {
        let n = self.steps;
        let mut pts = Vec::with_capacity(n * 2);
        for i in 0..n {
            pts.push(self.edge_pt(i as f32 / (n - 1) as f32, hi, pad));
        }
        for i in (0..n).rev() {
            pts.push(self.edge_pt(i as f32 / (n - 1) as f32, lo, pad));
        }
        pts
    }

    /// The whole silhouette, optionally expanded by `pad` pixels.
    pub fn outline(&self, pad: f32) -> Vec<V2> {
        self.slice(0.0, 1.0, pad)
    }

    /// Which flank of the limb faces the light?
    fn lit_front(&self, light: V2) -> bool {
        self.front.x * light.x + self.front.y * light.y >= 0.0
    }

    /// A dark shape a little larger than the limb: the drawn line that separates
    /// it from whatever it lies over.
    pub fn edge<T: Target>(&self, t: &mut T, pad: f32, col: Rgb, alpha: f32) {
        fill_poly(t, &self.outline(pad), col, alpha);
    }

    /// Base tone, hard-edged core shadow, lit rim.
    pub fn draw<T: Target>(&self, t: &mut T, base: Rgb, light: V2) {
        fill_poly(t, &self.outline(0.0), base, 1.0);
        let (s_lo, s_hi, r_lo, r_hi) = if self.lit_front(light) {
            (0.0, SHADOW, 1.0 - RIM, 1.0)
        } else {
            (1.0 - SHADOW, 1.0, 0.0, RIM)
        };
        fill_poly(t, &self.slice(s_lo, s_hi, 0.0), base.scale(self.shade), 1.0);
        fill_poly(t, &self.slice(r_lo, r_hi, 0.0), base.scale(RIM_MUL), 1.0);
    }

    /// Point on the lit edge at `t` — where to hang a highlight or start a crease.
    pub fn lit_edge(&self, t: f32, light: V2) -> V2 {
        self.edge_pt(t, if self.lit_front(light) { 1.0 } else { 0.0 }, 0.0)
    }

    /// Point on the shadow edge at `t`.
    pub fn dark_edge(&self, t: f32, light: V2) -> V2 {
        self.edge_pt(t, if self.lit_front(light) { 0.0 } else { 1.0 }, 0.0)
    }
}

/// Cel-shade an arbitrary authored polygon — heads, hands, cloth, hair.
///
/// The shadow is everything beyond a line perpendicular to `light`, placed so it
/// covers `shade_at` of the shape's extent along that axis. A polygon shaded this
/// way sits in the same light as the limbs beside it.
pub fn cel_poly<T: Target>(t: &mut T, pts: &[V2], base: Rgb, light: V2, shade_at: f32) {
    if pts.len() < 3 {
        return;
    }
    fill_poly(t, pts, base, 1.0);
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for p in pts {
        let d = p.x * light.x + p.y * light.y;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    // `light` points towards the light, so "away from it" is the small-`d` side.
    let cut = lerp(lo, hi, clampf(shade_at, 0.03, 0.95));
    let clipped = clip_half(pts, light, cut, true);
    if clipped.len() >= 3 {
        fill_poly(t, &clipped, base.scale(SHADE_MUL), 1.0);
    }
    // The lit band has to stay *narrow*. On a tall shape a generous rim covers
    // half the form and the whole thing goes chalky.
    let cut2 = lerp(lo, hi, 0.90);
    let band = clip_half(pts, light, cut2, false);
    if band.len() >= 3 {
        fill_poly(t, &band, base.scale(RIM_MUL), 1.0);
    }
}

/// Flat fill, no shading — for shapes that are already a single value (an eye, the
/// dark of an open mouth).
pub fn flat<T: Target>(t: &mut T, pts: &[V2], col: Rgb, alpha: f32) {
    fill_poly(t, pts, col, alpha);
}

/// Sutherland–Hodgman clip of a polygon against the half-plane
/// `dot(p, n) <= cut` (or `>=` when `keep_low` is false).
pub fn clip_half(pts: &[V2], n: V2, cut: f32, keep_low: bool) -> Vec<V2> {
    let inside = |p: &V2| {
        let d = p.x * n.x + p.y * n.y;
        if keep_low {
            d <= cut
        } else {
            d >= cut
        }
    };
    let mut out: Vec<V2> = Vec::with_capacity(pts.len() + 4);
    for i in 0..pts.len() {
        let cur = pts[i];
        let prev = pts[(i + pts.len() - 1) % pts.len()];
        let (ci, pi) = (inside(&cur), inside(&prev));
        if ci != pi {
            let dp = prev.x * n.x + prev.y * n.y;
            let dc = cur.x * n.x + cur.y * n.y;
            let k = if (dc - dp).abs() > 1e-6 {
                (cut - dp) / (dc - dp)
            } else {
                0.0
            };
            out.push(prev.lerp(cur, clampf(k, 0.0, 1.0)));
        }
        if ci {
            out.push(cur);
        }
    }
    out
}

/// A thin dark line along a contour — where an arm crosses the chest, where the
/// sash meets the belly, under a pectoral. Line work is what separates one form
/// from another when the two are the same colour, and it is the difference
/// between a drawing and a lump.
pub fn contour<T: Target>(t: &mut T, a: V2, b: V2, w: f32, col: Rgb, alpha: f32) {
    fill_capsule(t, a, b, w, w * 0.7, col, alpha);
}

/// Build a polygon in a local frame: `o` is the origin, `+x` runs along `ex` and
/// `+y` along `ey`. Every authored shape in the figure is written this way, so the
/// numbers below read as a drawing on squared paper.
pub fn frame(o: V2, ex: V2, ey: V2, pts: &[(f32, f32)]) -> Vec<V2> {
    pts.iter()
        .map(|&(x, y)| o.add(ex.mul(x)).add(ey.mul(y)))
        .collect()
}

// ---------------------------------------------------------------- profiles

/// Deltoid, bicep, elbow. Front is the bicep side. The deltoid is nearly twice
/// the width of the elbow — that taper is what joins an arm to a body instead of
/// pegging it on.
pub const UPPER_ARM: Profile = &[
    (0.00, 1.02, 0.98), // the cap, rounded over the joint rather than sawn off
    (0.11, 1.46, 1.40),
    (0.28, 1.48, 1.40),
    (0.54, 1.18, 1.12),
    (0.80, 0.96, 0.94),
    (1.00, 0.84, 0.86),
];

/// Forearm: the brachioradialis swells just below the elbow, then the wrist.
pub const FOREARM: Profile = &[
    (0.00, 0.92, 0.94),
    (0.26, 1.06, 1.02),
    (0.64, 0.80, 0.78),
    (1.00, 0.58, 0.56),
];

/// A short sleeve capping the shoulder: full at the deltoid, hemmed square.
pub const SLEEVE: Profile = &[
    (0.00, 1.46, 1.42),
    (0.35, 1.56, 1.50),
    (0.80, 1.48, 1.46),
    (1.00, 1.50, 1.48),
];

/// Thigh: heavy at the hip, tapering to the knee, fuller at the back.
pub const THIGH: Profile = &[
    (0.00, 1.88, 2.08),
    (0.30, 1.78, 1.94),
    (0.70, 1.50, 1.56),
    (1.00, 1.32, 1.32),
];

/// Calf: the gastrocnemius is at the *back*, and the front of the shin is nearly
/// straight. Getting this the right way round is most of what makes a leg read.
pub const CALF: Profile = &[
    (0.00, 1.32, 1.38),
    (0.26, 1.26, 1.66),
    (0.62, 1.00, 1.12),
    (1.00, 0.70, 0.72),
];

/// Loose Persian trousers: full through the hip and knee, then gathered hard into
/// the boot. The gather is the whole silhouette — without it they are pyjamas.
pub const THIGH_BAGGY: Profile = &[
    (0.00, 1.94, 2.16),
    (0.35, 2.06, 2.28),
    (0.75, 2.10, 2.26),
    (1.00, 2.02, 2.16),
];

pub const CALF_BAGGY: Profile = &[
    (0.00, 2.04, 2.18),
    (0.28, 1.96, 2.20),
    (0.62, 1.52, 1.70),
    (0.88, 0.92, 0.98),
    (1.00, 0.82, 0.84),
];

/// Torso: shoulders, pectoral, the pinch at the waist, the flare at the hips.
/// Widths are multiples of the chest half-depth.
/// The narrow start is the slope of the trapezius running up into the neck: a
/// torso that reaches full width at the shoulder joint has a flat sawn-off top
/// and no shoulders at all.
pub const TORSO: Profile = &[
    (0.00, 0.50, 0.58),
    (0.15, 1.04, 1.16),
    (0.32, 1.10, 1.02),
    (0.58, 0.88, 0.88),
    (0.78, 0.76, 0.86),
    (1.00, 0.96, 1.10),
];

/// A robed body: no waist, just a widening column.
pub const TORSO_ROBE: Profile = &[
    (0.00, 0.52, 0.60),
    (0.15, 1.00, 1.10),
    (0.35, 1.04, 1.10),
    (0.70, 1.10, 1.20),
    (1.00, 1.26, 1.36),
];

/// A heavy body — the fat guard. Belly at the front, no waist at all.
pub const TORSO_HEAVY: Profile = &[
    (0.00, 0.54, 0.62),
    (0.16, 1.00, 1.06),
    (0.34, 1.10, 1.04),
    (0.60, 1.24, 1.08),
    (0.84, 1.34, 1.10),
    (1.00, 1.14, 1.04),
];

/// The neck, drawn from the jaw down: narrow under the ear, flaring hard into the
/// trapezius so the head sits *on* the shoulders instead of on a stalk. The back
/// flares more than the front, which is where the trapezius actually is.
pub const NECK: Profile = &[
    (0.00, 1.05, 1.22),
    (0.45, 1.12, 1.44),
    (1.00, 1.50, 2.05),
];

// ---------------------------------------------------------------- head shapes

/// The head in profile, in units of the head radius, drawn facing +x with +y
/// down. Every character shares this construction and differs only in what is
/// put on top of it: the shape of a skull is not what tells two people apart at
/// twelve pixels tall, but a *drawn* skull instead of an egg is what tells a
/// character from a blob.
pub const HEAD: &[(f32, f32)] = &[
    (-0.10, -1.02), // crown
    (0.38, -0.94),  // front of the crown
    (0.72, -0.64),  // forehead
    (0.84, -0.32),  // brow ridge
    (0.76, -0.14),  // the dip at the bridge of the nose
    (0.92, 0.06),
    (1.12, 0.30), // tip of the nose
    (0.92, 0.38), // under the nose
    (0.94, 0.48), // upper lip
    (0.84, 0.56), // the mouth
    (0.92, 0.64), // lower lip
    (0.80, 0.74), // the crease above the chin
    (0.88, 0.86), // chin
    (0.64, 0.98), // under the chin
    (0.08, 0.98), // jawline
    (-0.46, 0.74), // the angle of the jaw
    (-0.84, 0.34), // behind the ear
    (-1.02, -0.10), // back of the skull
    (-0.96, -0.62),
    (-0.60, -0.94),
];

/// The prince's hair as a solid mass, not a rind: the outer silhouette from the
/// temple over the crown and down to the tail at the nape, closed along the
/// hairline. Drawing it as a band around the skull leaves the cranium bare, which
/// is the difference between hair and a hat brim.
pub const HAIR: &[(f32, f32)] = &[
    (0.86, -0.52),  // front hairline, at the temple
    (0.94, -0.80),  // the mass lifting off the forehead
    (0.58, -1.14),
    (-0.08, -1.34), // crown
    (-0.78, -1.16),
    (-1.20, -0.66),
    (-1.32, -0.08),
    (-1.34, 0.42),
    (-1.08, 0.88), // the tail flicking out at the nape
    (-0.72, 0.48), // underside of the tail
    (-0.44, 0.06), // behind the ear
    (-0.08, -0.30), // the hairline sweeping up over the temple
    (0.44, -0.46),
];

/// A skull: same construction, but the jaw hangs and the cranium is rounder.
pub const SKULL: &[(f32, f32)] = &[
    (-0.04, -1.08),
    (0.48, -0.96),
    (0.82, -0.56),
    (0.88, -0.16),
    (0.74, 0.02),
    (0.92, 0.20),
    (0.98, 0.42), // the bare nasal spine
    (0.80, 0.44),
    (0.86, 0.62), // upper teeth
    (0.88, 0.92), // the jaw, dropped
    (0.36, 1.06),
    (-0.10, 0.92),
    (-0.30, 0.60),
    (-0.72, 0.46),
    (-1.02, 0.02),
    (-1.04, -0.56),
    (-0.62, -1.00),
];
