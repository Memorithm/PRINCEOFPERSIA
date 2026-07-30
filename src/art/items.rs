//! Pickups on the floor and the projectiles the bonus weapons throw.

use crate::gfx::canvas::{Blend, Cam, Canvas};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::target::{
    fill_capsule, fill_circle, fill_ellipse, fill_poly, fill_poly_shaded, fill_rect, radial_glow,
};
use crate::util::{v2, V2};
use crate::world::tile::ItemKind;

/// Draw a pickup resting on a floor whose surface is at `base`.
pub fn draw_item(cv: &mut Canvas, cam: &Cam, kind: ItemKind, cx: f32, base: f32, bob: f32) {
    cv.blend = Blend::Alpha;
    // Contact shadow.
    fill_ellipse(
        cv,
        v2(cam.x(cx), cam.y(base - 0.4)),
        cam.l(6.0),
        cam.l(1.7),
        rgb(0, 0, 0),
        0.42,
    );
    match kind {
        k if k.is_potion() => draw_flask(cv, cam, k, cx, base + bob),
        ItemKind::Sword => draw_ground_sword(cv, cam, cx, base + bob, false),
        ItemKind::Scimitar => draw_ground_sword(cv, cam, cx, base + bob, true),
        ItemKind::Daggers => draw_ground_daggers(cv, cam, cx, base + bob),
        ItemKind::Wand => draw_ground_wand(cv, cam, cx, base + bob),
        ItemKind::Buckler => draw_ground_buckler(cv, cam, cx, base + bob),
        _ => {}
    }
}

fn draw_flask(cv: &mut Canvas, cam: &Cam, kind: ItemKind, cx: f32, base: f32) {
    let col = kind.colour();
    let glass = rgb(196, 214, 226);
    let cy = base - 5.6;
    // Body.
    fill_circle(cv, v2(cam.x(cx), cam.y(cy)), cam.l(4.3), glass, 0.42);
    fill_circle(cv, v2(cam.x(cx), cam.y(cy + 0.6)), cam.l(3.5), col, 0.92);
    // A lighter surface line on the liquid.
    fill_ellipse(
        cv,
        v2(cam.x(cx), cam.y(cy - 1.4)),
        cam.l(3.2),
        cam.l(0.9),
        col.scale(1.5),
        0.8,
    );
    // Neck and cork.
    fill_rect(
        cv,
        cam.x(cx - 1.3),
        cam.y(cy - 8.4),
        cam.x(cx + 1.3),
        cam.y(cy - 3.2),
        glass,
        0.6,
    );
    fill_rect(
        cv,
        cam.x(cx - 1.7),
        cam.y(cy - 9.8),
        cam.x(cx + 1.7),
        cam.y(cy - 8.0),
        rgb(126, 90, 54),
        1.0,
    );
    // Specular highlight.
    fill_capsule(
        cv,
        v2(cam.x(cx - 2.3), cam.y(cy - 1.8)),
        v2(cam.x(cx - 2.8), cam.y(cy + 1.4)),
        cam.l(0.8),
        cam.l(0.55),
        Rgb::WHITE,
        0.65,
    );
}

fn draw_ground_sword(cv: &mut Canvas, cam: &Cam, cx: f32, base: f32, curved: bool) {
    let y = base - 2.2;
    let (blade, len) = if curved {
        (rgb(220, 210, 176), 15.0)
    } else {
        (rgb(206, 212, 220), 13.5)
    };
    let dip = if curved { -2.6 } else { 0.0 };
    // Grip.
    fill_capsule(
        cv,
        v2(cam.x(cx - len * 0.5 - 3.4), cam.y(y)),
        v2(cam.x(cx - len * 0.5), cam.y(y)),
        cam.l(1.1),
        cam.l(1.1),
        rgb(92, 60, 36),
        1.0,
    );
    // Cross-guard.
    fill_capsule(
        cv,
        v2(cam.x(cx - len * 0.5), cam.y(y - 2.4)),
        v2(cam.x(cx - len * 0.5), cam.y(y + 2.4)),
        cam.l(0.9),
        cam.l(0.9),
        rgb(184, 152, 78),
        1.0,
    );
    // Blade.
    fill_capsule(
        cv,
        v2(cam.x(cx - len * 0.5), cam.y(y)),
        v2(cam.x(cx + len * 0.5), cam.y(y + dip)),
        cam.l(1.2),
        cam.l(0.3),
        blade,
        1.0,
    );
    fill_capsule(
        cv,
        v2(cam.x(cx - len * 0.4), cam.y(y - 0.5)),
        v2(cam.x(cx + len * 0.42), cam.y(y - 0.4 + dip)),
        cam.l(0.35),
        cam.l(0.15),
        Rgb::WHITE,
        0.75,
    );
}

fn draw_ground_daggers(cv: &mut Canvas, cam: &Cam, cx: f32, base: f32) {
    for (i, off) in [(-4.5f32, -0.4f32), (0.0, -2.0), (4.5, -0.4)].iter().enumerate() {
        let x = cx + off.0;
        let y = base - 2.0 + off.1;
        let ang = (i as f32 - 1.0) * 0.22;
        let d = v2(ang.cos(), ang.sin());
        fill_capsule(
            cv,
            v2(cam.x(x - d.x * 2.4), cam.y(y - d.y * 2.4)),
            v2(cam.x(x - d.x * 0.6), cam.y(y - d.y * 0.6)),
            cam.l(0.85),
            cam.l(0.85),
            rgb(80, 52, 32),
            1.0,
        );
        fill_capsule(
            cv,
            v2(cam.x(x - d.x * 0.6), cam.y(y - d.y * 0.6)),
            v2(cam.x(x + d.x * 4.4), cam.y(y + d.y * 4.4)),
            cam.l(1.0),
            cam.l(0.2),
            rgb(212, 218, 226),
            1.0,
        );
    }
}

fn draw_ground_wand(cv: &mut Canvas, cam: &Cam, cx: f32, base: f32) {
    let y = base - 2.4;
    fill_capsule(
        cv,
        v2(cam.x(cx - 7.5), cam.y(y + 1.6)),
        v2(cam.x(cx + 6.0), cam.y(y - 1.6)),
        cam.l(1.2),
        cam.l(1.0),
        rgb(96, 66, 40),
        1.0,
    );
    fill_circle(cv, v2(cam.x(cx + 7.0), cam.y(y - 2.0)), cam.l(2.4), rgb(255, 172, 60), 1.0);
    fill_circle(cv, v2(cam.x(cx + 7.0), cam.y(y - 2.0)), cam.l(1.1), rgb(255, 244, 210), 1.0);
}

fn draw_ground_buckler(cv: &mut Canvas, cam: &Cam, cx: f32, base: f32) {
    let y = base - 4.2;
    fill_ellipse(
        cv,
        v2(cam.x(cx), cam.y(y)),
        cam.l(6.4),
        cam.l(4.6),
        rgb(140, 96, 52),
        1.0,
    );
    fill_ellipse(
        cv,
        v2(cam.x(cx), cam.y(y)),
        cam.l(5.0),
        cam.l(3.4),
        rgb(172, 122, 68),
        1.0,
    );
    fill_ellipse(
        cv,
        v2(cam.x(cx), cam.y(y)),
        cam.l(2.0),
        cam.l(1.6),
        rgb(216, 220, 228),
        1.0,
    );
    fill_capsule(
        cv,
        v2(cam.x(cx - 3.4), cam.y(y - 2.0)),
        v2(cam.x(cx + 1.0), cam.y(y - 3.0)),
        cam.l(0.6),
        cam.l(0.4),
        Rgb::WHITE,
        0.5,
    );
}

/// Emissive halo so pickups catch the eye in a dark room.
pub fn draw_item_glow(cv: &mut Canvas, cam: &Cam, kind: ItemKind, cx: f32, base: f32, bob: f32, pulse: f32) {
    cv.blend = Blend::Add;
    let (col, r, p) = match kind {
        k if k.is_potion() => (k.colour(), 11.0, 0.30),
        ItemKind::Wand => (rgb(255, 176, 72), 12.0, 0.34),
        ItemKind::Scimitar => (rgb(255, 232, 176), 11.0, 0.24),
        _ => (rgb(190, 210, 240), 9.0, 0.20),
    };
    radial_glow(
        cv,
        v2(cam.x(cx), cam.y(base - 5.0 + bob)),
        cam.l(r * (0.9 + 0.14 * pulse)),
        col,
        p * (0.82 + 0.24 * pulse),
    );
    cv.blend = Blend::Alpha;
}

// ---------------------------------------------------------------- projectiles

pub fn draw_dagger_flight(cv: &mut Canvas, cam: &Cam, p: V2, ang: f32) {
    cv.blend = Blend::Alpha;
    let d = v2(ang.cos(), ang.sin());
    let a = p.sub(d.mul(3.6));
    let b = p.add(d.mul(4.2));
    fill_capsule(
        cv,
        cam.p(a),
        cam.p(a.add(d.mul(2.0))),
        cam.l(0.9),
        cam.l(0.9),
        rgb(80, 52, 32),
        1.0,
    );
    fill_capsule(
        cv,
        cam.p(a.add(d.mul(2.0))),
        cam.p(b),
        cam.l(1.1),
        cam.l(0.2),
        rgb(220, 226, 234),
        1.0,
    );
}

pub fn draw_fireball(cv: &mut Canvas, cam: &Cam, p: V2, r: f32, pulse: f32) {
    cv.blend = Blend::Add;
    let d = cam.p(p);
    radial_glow(cv, d, cam.l(r * 3.4), rgb(255, 140, 40), 0.55);
    fill_circle(cv, d, cam.l(r * (1.0 + 0.12 * pulse)), rgb(255, 180, 70), 0.9);
    fill_circle(cv, d, cam.l(r * 0.55), rgb(255, 248, 214), 1.0);
    cv.blend = Blend::Alpha;
}

/// The shimmering afterimage of the shadow prince.
pub fn draw_shadow_aura(cv: &mut Canvas, cam: &Cam, p: V2, r: f32) {
    cv.blend = Blend::Add;
    radial_glow(cv, cam.p(p), cam.l(r), rgb(70, 60, 130), 0.35);
    cv.blend = Blend::Alpha;
}

/// A small directional slash arc, drawn when a blade connects.
pub fn draw_slash(cv: &mut Canvas, cam: &Cam, at: V2, facing: f32, t: f32) {
    cv.blend = Blend::Add;
    let n = 7;
    for i in 0..n {
        let f = i as f32 / (n - 1) as f32;
        let ang = -0.9 + f * 1.8;
        let rr = 13.0 * (0.6 + 0.4 * t);
        let p = at.add(v2(facing * ang.cos() * rr, ang.sin() * rr * 0.8));
        fill_circle(
            cv,
            cam.p(p),
            cam.l(1.6 * (1.0 - t)),
            rgb(255, 246, 220),
            0.5 * (1.0 - t),
        );
    }
    cv.blend = Blend::Alpha;
}

/// Ripple used for the poison / float potion effect flash.
pub fn draw_ring(cv: &mut Canvas, cam: &Cam, at: V2, r: f32, col: Rgb, a: f32) {
    cv.blend = Blend::Add;
    let n = 28;
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let th = i as f32 / n as f32 * std::f32::consts::TAU;
        pts.push(cam.p(at.add(v2(th.cos() * r, th.sin() * r * 0.72))));
    }
    for i in 0..n {
        let a0 = pts[i];
        let b0 = pts[(i + 1) % n];
        fill_capsule(cv, a0, b0, cam.l(0.9), cam.l(0.9), col, a);
    }
    cv.blend = Blend::Alpha;
}

/// Unused-but-handy: a filled polygon helper re-exported for effects.
pub fn poly(cv: &mut Canvas, pts: &[V2], col: Rgb, a: f32) {
    fill_poly(cv, pts, col, a);
}

/// Shaded polygon helper.
pub fn poly_shaded(cv: &mut Canvas, pts: &[V2], top: Rgb, bot: Rgb, a: f32) {
    fill_poly_shaded(cv, pts, top, bot, a);
}
