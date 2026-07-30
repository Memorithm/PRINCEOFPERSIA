//! Articulated figure rendering.
//!
//! Rather than storing a few dozen hand-drawn sprite sheets, every character in
//! the game is a small skeleton — hip, torso, head, two arms, two legs — drawn
//! with anti-aliased tapered capsules and polygons. Animations are keyframed
//! joint angles, which is what makes it practical to have running, jumping,
//! climbing, hanging, drinking, three sword stances and five different body
//! types all look consistent.
//!
//! Angle convention, used everywhere: **degrees measured from straight down,
//! positive rotating towards the direction the character faces.** A limb at 0
//! hangs straight down. Knees and elbows are given as bend amounts, positive
//! meaning the natural direction (heel towards the back, forearm towards the
//! front).

use crate::gfx::canvas::Cam;
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::layer::Layer;
use crate::gfx::target::{
    fill_capsule, fill_capsule_grad, fill_circle, fill_ellipse, fill_poly_shaded,
};
use crate::util::{clampf, dir_down, dir_up, lerp, v2, V2};

// ---------------------------------------------------------------- pose

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// Hip height above the feet plane, in art pixels.
    pub hip: f32,
    /// Hip offset along the facing direction.
    pub lean: f32,
    /// Torso lean, degrees forward from vertical.
    pub torso: f32,
    /// Head angle relative to the torso.
    pub head: f32,
    /// `[near, far]` x `[shoulder, elbow]`.
    pub arm: [[f32; 2]; 2],
    /// `[near, far]` x `[hip, knee]`.
    pub leg: [[f32; 2]; 2],
    /// Blade angle relative to the near forearm.
    pub sword: f32,
    /// How far the sash tail trails behind.
    pub tail: f32,
    /// Squash factor, 1.0 = neutral (used on landings).
    pub squash: f32,
}

impl Pose {
    pub const REST: Pose = Pose {
        hip: 13.2,
        lean: 0.0,
        torso: 0.0,
        head: 0.0,
        arm: [[0.0, 8.0], [0.0, 8.0]],
        leg: [[0.0, 2.0], [0.0, 2.0]],
        sword: 0.0,
        tail: 1.0,
        squash: 1.0,
    };

    /// Swap the near and far limbs — turns one half of a walk cycle into the
    /// other.
    pub fn mirrored(mut self) -> Pose {
        self.arm.swap(0, 1);
        self.leg.swap(0, 1);
        self
    }

    pub fn with_sword(mut self, a: f32) -> Pose {
        self.sword = a;
        self
    }

    pub fn lerp(&self, o: &Pose, t: f32) -> Pose {
        let mut p = *self;
        p.hip = lerp(self.hip, o.hip, t);
        p.lean = lerp(self.lean, o.lean, t);
        p.torso = lerp(self.torso, o.torso, t);
        p.head = lerp(self.head, o.head, t);
        p.sword = lerp(self.sword, o.sword, t);
        p.tail = lerp(self.tail, o.tail, t);
        p.squash = lerp(self.squash, o.squash, t);
        for i in 0..2 {
            for j in 0..2 {
                p.arm[i][j] = lerp(self.arm[i][j], o.arm[i][j], t);
                p.leg[i][j] = lerp(self.leg[i][j], o.leg[i][j], t);
            }
        }
        p
    }
}

/// Convenience constructor: `ps(hip, lean, torso, head, near_arm, far_arm, near_leg, far_leg)`.
#[allow(clippy::too_many_arguments)]
pub fn ps(
    hip: f32,
    lean: f32,
    torso: f32,
    head: f32,
    an: (f32, f32),
    af: (f32, f32),
    ln: (f32, f32),
    lf: (f32, f32),
) -> Pose {
    Pose {
        hip,
        lean,
        torso,
        head,
        arm: [[an.0, an.1], [af.0, af.1]],
        leg: [[ln.0, ln.1], [lf.0, lf.1]],
        ..Pose::REST
    }
}

// ---------------------------------------------------------------- proportions

#[derive(Clone, Copy)]
pub struct Prop {
    pub thigh: f32,
    pub shin: f32,
    pub foot: f32,
    pub torso: f32,
    pub neck: f32,
    pub head_r: f32,
    pub upper: f32,
    pub fore: f32,
    pub hand: f32,
    /// Overall scale multiplier.
    pub scale: f32,
    /// Limb and body thickness multiplier.
    pub girth: f32,
}

impl Prop {
    pub const PRINCE: Prop = Prop {
        thigh: 7.0,
        shin: 7.0,
        foot: 3.6,
        torso: 8.6,
        neck: 1.7,
        head_r: 2.95,
        upper: 5.6,
        fore: 5.2,
        hand: 1.5,
        scale: 1.0,
        girth: 1.0,
    };

    pub fn scaled(mut self, s: f32, g: f32) -> Prop {
        self.scale = s;
        self.girth = g;
        self
    }
}

// ---------------------------------------------------------------- solved skeleton

#[derive(Clone, Copy)]
pub struct Figure {
    pub hip: V2,
    pub knee: [V2; 2],
    pub ankle: [V2; 2],
    pub toe: [V2; 2],
    pub shoulder: V2,
    pub elbow: [V2; 2],
    pub hand: [V2; 2],
    pub neck: V2,
    pub head: V2,
    pub head_r: f32,
    /// +1 facing right, -1 facing left.
    pub facing: f32,
    /// Direction the near forearm points, for attaching a weapon.
    pub hand_dir: V2,
    pub prop: Prop,
    pub squash: f32,
}

/// Forward kinematics. `feet` is the world point at the character's base,
/// centred horizontally.
pub fn solve(pose: &Pose, prop: &Prop, feet: V2, facing: f32) -> Figure {
    let s = prop.scale;
    let sq = pose.squash;
    // Vertical squash pivots on the feet, horizontal stretch keeps volume.
    let vs = s * sq;
    let hs = s * (2.0 - sq).max(0.6);

    let hip = v2(feet.x + pose.lean * hs, feet.y - pose.hip * vs);

    let mut knee = [V2::ZERO; 2];
    let mut ankle = [V2::ZERO; 2];
    let mut toe = [V2::ZERO; 2];
    for i in 0..2 {
        let h = pose.leg[i][0];
        let k = pose.leg[i][1];
        let d1 = dir_down(h);
        knee[i] = v2(hip.x + d1.x * prop.thigh * hs, hip.y + d1.y * prop.thigh * vs);
        let d2 = dir_down(h - k);
        ankle[i] = v2(
            knee[i].x + d2.x * prop.shin * hs,
            knee[i].y + d2.y * prop.shin * vs,
        );
        // Keep the foot roughly perpendicular to the shin.
        let d3 = dir_down(h - k + 96.0);
        toe[i] = v2(
            ankle[i].x + d3.x * prop.foot * hs,
            ankle[i].y + d3.y * prop.foot * vs,
        );
    }

    let du = dir_up(pose.torso);
    let shoulder = v2(
        hip.x + du.x * prop.torso * hs,
        hip.y + du.y * prop.torso * vs,
    );
    let dn = dir_up(pose.torso + pose.head);
    let neck = v2(
        shoulder.x + dn.x * prop.neck * hs,
        shoulder.y + dn.y * prop.neck * vs,
    );
    let head = v2(
        neck.x + dn.x * prop.head_r * hs,
        neck.y + dn.y * prop.head_r * vs,
    );

    let mut elbow = [V2::ZERO; 2];
    let mut hand = [V2::ZERO; 2];
    let mut hand_dir = v2(1.0, 0.0);
    for i in 0..2 {
        let a = pose.arm[i][0] + pose.torso;
        let e = pose.arm[i][1];
        let d1 = dir_down(a);
        elbow[i] = v2(
            shoulder.x + d1.x * prop.upper * hs,
            shoulder.y + d1.y * prop.upper * vs,
        );
        let d2 = dir_down(a + e);
        hand[i] = v2(
            elbow[i].x + d2.x * prop.fore * hs,
            elbow[i].y + d2.y * prop.fore * vs,
        );
        if i == 0 {
            hand_dir = d2;
        }
    }

    let mut f = Figure {
        hip,
        knee,
        ankle,
        toe,
        shoulder,
        elbow,
        hand,
        neck,
        head,
        head_r: prop.head_r * s,
        facing: if facing < 0.0 { -1.0 } else { 1.0 },
        hand_dir,
        prop: *prop,
        squash: sq,
    };

    if facing < 0.0 {
        let px = feet.x;
        f.hip = f.hip.flip_x(px);
        f.shoulder = f.shoulder.flip_x(px);
        f.neck = f.neck.flip_x(px);
        f.head = f.head.flip_x(px);
        for i in 0..2 {
            f.knee[i] = f.knee[i].flip_x(px);
            f.ankle[i] = f.ankle[i].flip_x(px);
            f.toe[i] = f.toe[i].flip_x(px);
            f.elbow[i] = f.elbow[i].flip_x(px);
            f.hand[i] = f.hand[i].flip_x(px);
        }
        f.hand_dir = v2(-f.hand_dir.x, f.hand_dir.y);
    }
    f
}

// ---------------------------------------------------------------- style

/// What kind of blade, if any, the near hand is holding.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Blade {
    None,
    Sword,
    Scimitar,
    Dagger,
    Wand,
}

#[derive(Clone, Copy)]
pub struct Style {
    pub skin: Rgb,
    pub skin_dk: Rgb,
    pub cloth: Rgb,
    pub cloth_dk: Rgb,
    pub sash: Rgb,
    pub sash_dk: Rgb,
    pub hair: Rgb,
    pub boot: Rgb,
    pub metal: Rgb,
    pub outline: Rgb,
    /// Turban / helmet colour; `None` means bare hair.
    pub head_wrap: Option<Rgb>,
    /// 0 = short tunic, 1 = full-length robe.
    pub robe: f32,
    /// Draw as a skeleton: thin bones, ribcage, skull.
    pub bones: bool,
    /// Multiplier applied to far-side limbs.
    pub back_shade: f32,
    /// A round shield carried in the far hand.
    pub shield: Option<Rgb>,
    /// Plume on the helmet.
    pub plume: Option<Rgb>,
}

impl Style {
    pub const PRINCE: Style = Style {
        skin: rgb(226, 174, 128),
        skin_dk: rgb(168, 116, 78),
        cloth: rgb(238, 238, 231),
        cloth_dk: rgb(176, 178, 188),
        sash: rgb(196, 46, 46),
        sash_dk: rgb(124, 24, 28),
        hair: rgb(38, 30, 34),
        boot: rgb(124, 82, 46),
        metal: rgb(206, 212, 220),
        outline: rgb(18, 14, 22),
        head_wrap: None,
        robe: 0.0,
        bones: false,
        back_shade: 0.68,
        shield: None,
        plume: None,
    };
}

// ---------------------------------------------------------------- drawing

/// Paint a figure into `layer`. The caller composites the layer afterwards so
/// the whole silhouette gets one clean outline.
pub fn draw_figure(
    layer: &mut Layer,
    cam: &Cam,
    f: &Figure,
    st: &Style,
    pose: &Pose,
    blade: Blade,
) {
    let g = f.prop.girth * f.prop.scale;
    let bs = st.back_shade;

    if st.bones {
        draw_bones(layer, cam, f, st, pose, blade);
        return;
    }

    // --- far leg -------------------------------------------------------
    draw_leg(layer, cam, f, st, 1, g, bs);
    // --- near leg ------------------------------------------------------
    draw_leg(layer, cam, f, st, 0, g, 1.0);

    // --- far arm -------------------------------------------------------
    draw_arm(layer, cam, f, st, 1, g, bs);
    if let Some(sc) = st.shield {
        let c = cam.p(f.hand[1]);
        fill_circle(layer, c, cam.l(4.6 * g), sc.scale(bs * 1.05), 1.0);
        fill_circle(layer, c, cam.l(2.0 * g), st.metal.scale(bs), 1.0);
    }

    // --- torso ---------------------------------------------------------
    let torso_r = 2.95 * g;
    fill_capsule_grad(
        layer,
        cam.p(f.shoulder),
        cam.p(f.hip),
        cam.l(torso_r * 0.95),
        cam.l(torso_r * 1.02),
        st.cloth,
        st.cloth.lerp(st.cloth_dk, 0.45),
        1.0,
    );

    // --- tunic / robe ---------------------------------------------------
    draw_tunic(layer, cam, f, st, pose, g);

    // --- sash ----------------------------------------------------------
    let waist = f.hip.lerp(f.shoulder, 0.18);
    let across = f.shoulder.sub(f.hip).norm().perp().mul(torso_r * 1.0);
    fill_capsule(
        layer,
        cam.p(waist.add(across)),
        cam.p(waist.sub(across)),
        cam.l(1.15 * g),
        cam.l(1.15 * g),
        st.sash,
        1.0,
    );
    // Trailing tail behind the hip.
    let back = -f.facing;
    let tail_a = waist.add(v2(back * 1.8 * g, 0.7));
    let tail_b = tail_a.add(v2(back * 3.0 * pose.tail, 4.6 + pose.tail * 1.0));
    fill_capsule(
        layer,
        cam.p(tail_a),
        cam.p(tail_b),
        cam.l(1.0 * g),
        cam.l(0.4 * g),
        st.sash_dk,
        1.0,
    );

    // --- head ----------------------------------------------------------
    let neck_r = 1.5 * g;
    fill_capsule(
        layer,
        cam.p(f.shoulder),
        cam.p(f.neck),
        cam.l(neck_r),
        cam.l(neck_r * 0.9),
        st.skin_dk,
        1.0,
    );
    let hc = cam.p(f.head);
    let hr = cam.l(f.head_r);
    let fw = f.facing;
    // Skull, slightly egg-shaped and pushed forward at the jaw.
    fill_ellipse(layer, hc, hr * 0.98, hr * 1.06, st.skin, 1.0);
    fill_circle(
        layer,
        v2(hc.x + fw * hr * 0.3, hc.y + hr * 0.35),
        hr * 0.72,
        st.skin,
        1.0,
    );

    // Headgear goes on before the face, so the features stay visible.
    match st.head_wrap {
        None => {
            // Hair: a cap sitting on the back and top of the skull, leaving the
            // brow and cheek clear.
            fill_circle(
                layer,
                v2(hc.x - fw * hr * 0.34, hc.y - hr * 0.34),
                hr * 0.9,
                st.hair,
                1.0,
            );
            fill_capsule(
                layer,
                v2(hc.x - fw * hr * 0.55, hc.y - hr * 0.7),
                v2(hc.x - fw * hr * 1.1, hc.y + hr * 0.95),
                hr * 0.46,
                hr * 0.24,
                st.hair,
                1.0,
            );
            // A lock falling over the temple.
            fill_capsule(
                layer,
                v2(hc.x + fw * hr * 0.15, hc.y - hr * 0.85),
                v2(hc.x + fw * hr * 0.72, hc.y - hr * 0.15),
                hr * 0.3,
                hr * 0.16,
                st.hair,
                1.0,
            );
        }
        Some(wrap) => {
            // Turban / helmet: a dome with a wrapped band.
            fill_ellipse(
                layer,
                v2(hc.x - fw * hr * 0.1, hc.y - hr * 0.62),
                hr * 1.08,
                hr * 0.82,
                wrap,
                1.0,
            );
            fill_capsule(
                layer,
                v2(hc.x - hr * 1.02, hc.y - hr * 0.18),
                v2(hc.x + hr * 1.02, hc.y - hr * 0.3),
                hr * 0.34,
                hr * 0.34,
                wrap.scale(0.76),
                1.0,
            );
            if let Some(pl) = st.plume {
                fill_capsule(
                    layer,
                    v2(hc.x - fw * hr * 0.2, hc.y - hr * 1.3),
                    v2(hc.x - fw * hr * 1.5, hc.y - hr * 2.5),
                    hr * 0.34,
                    hr * 0.1,
                    pl,
                    1.0,
                );
            }
        }
    }

    // ---- face ---------------------------------------------------------
    // Shading on the far cheek.
    fill_circle(
        layer,
        v2(hc.x - fw * hr * 0.5, hc.y + hr * 0.3),
        hr * 0.6,
        st.skin.lerp(st.skin_dk, 0.55),
        0.45,
    );
    // Brow.
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.15, hc.y - hr * 0.36),
        v2(hc.x + fw * hr * 0.78, hc.y - hr * 0.3),
        hr * 0.15,
        hr * 0.11,
        st.hair.lerp(st.skin_dk, 0.35),
        0.8,
    );
    // Eye.
    fill_circle(
        layer,
        v2(hc.x + fw * hr * 0.52, hc.y - hr * 0.06),
        hr * 0.19,
        st.outline,
        0.95,
    );
    // Nose and the line of the mouth.
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.72, hc.y - hr * 0.02),
        v2(hc.x + fw * hr * 0.98, hc.y + hr * 0.26),
        hr * 0.22,
        hr * 0.16,
        st.skin,
        1.0,
    );
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.5, hc.y + hr * 0.52),
        v2(hc.x + fw * hr * 0.82, hc.y + hr * 0.5),
        hr * 0.1,
        hr * 0.08,
        st.skin_dk.scale(0.75),
        0.7,
    );

    // --- near arm ------------------------------------------------------
    draw_arm(layer, cam, f, st, 0, g, 1.0);

    // --- weapon --------------------------------------------------------
    draw_blade(layer, cam, f, st, pose, blade, g);
}

fn draw_leg(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, i: usize, g: f32, shade: f32) {
    let cloth = st.cloth.scale(shade);
    let cloth_dk = st.cloth_dk.scale(shade);
    let boot = st.boot.scale(shade);
    if i == 0 {
        // Rim: the near leg is drawn once oversized in a dark tone, so it reads
        // as a separate limb in front of the body rather than merging with it.
        let rim = st.cloth_dk.scale(0.42);
        fill_capsule(layer, cam.p(f.hip), cam.p(f.knee[i]), cam.l(3.2 * g), cam.l(2.5 * g), rim, 1.0);
        fill_capsule(
            layer,
            cam.p(f.knee[i]),
            cam.p(f.ankle[i]),
            cam.l(2.5 * g),
            cam.l(1.95 * g),
            rim,
            1.0,
        );
        fill_capsule(layer, cam.p(f.ankle[i]), cam.p(f.toe[i]), cam.l(2.3 * g), cam.l(1.7 * g), rim, 1.0);
    }
    fill_capsule_grad(
        layer,
        cam.p(f.hip),
        cam.p(f.knee[i]),
        cam.l(2.6 * g),
        cam.l(2.0 * g),
        cloth,
        cloth_dk,
        1.0,
    );
    fill_capsule_grad(
        layer,
        cam.p(f.knee[i]),
        cam.p(f.ankle[i]),
        cam.l(1.95 * g),
        cam.l(1.45 * g),
        cloth_dk,
        cloth_dk.scale(0.9),
        1.0,
    );
    // Boot.
    fill_capsule(
        layer,
        cam.p(f.ankle[i]),
        cam.p(f.toe[i]),
        cam.l(1.8 * g),
        cam.l(1.25 * g),
        boot,
        1.0,
    );
}

fn draw_arm(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, i: usize, g: f32, shade: f32) {
    let skin = st.skin.scale(shade);
    let skin_dk = st.skin_dk.scale(shade);
    if i == 0 {
        let rim = st.cloth_dk.scale(0.42);
        fill_capsule(layer, cam.p(f.shoulder), cam.p(f.elbow[i]), cam.l(2.7 * g), cam.l(2.0 * g), rim, 1.0);
        fill_capsule(layer, cam.p(f.elbow[i]), cam.p(f.hand[i]), cam.l(2.0 * g), cam.l(1.75 * g), rim, 1.0);
    }
    // Sleeve over the upper arm.
    fill_capsule_grad(
        layer,
        cam.p(f.shoulder),
        cam.p(f.shoulder.lerp(f.elbow[i], 0.55)),
        cam.l(2.1 * g),
        cam.l(1.7 * g),
        st.cloth.scale(shade),
        st.cloth_dk.scale(shade),
        1.0,
    );
    fill_capsule_grad(
        layer,
        cam.p(f.shoulder.lerp(f.elbow[i], 0.5)),
        cam.p(f.elbow[i]),
        cam.l(1.7 * g),
        cam.l(1.45 * g),
        skin,
        skin_dk,
        1.0,
    );
    fill_capsule_grad(
        layer,
        cam.p(f.elbow[i]),
        cam.p(f.hand[i]),
        cam.l(1.45 * g),
        cam.l(1.2 * g),
        skin,
        skin_dk,
        1.0,
    );
    fill_circle(layer, cam.p(f.hand[i]), cam.l(1.5 * g), skin, 1.0);
}

fn draw_tunic(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, pose: &Pose, g: f32) {
    let down = f.hip.sub(f.shoulder).norm();
    let side = down.perp().mul(1.0);
    let hem_len = lerp(5.4, 15.5, clampf(st.robe, 0.0, 1.0)) * f.prop.scale;
    let top = f.hip.lerp(f.shoulder, 0.34);
    let w_top = 3.2 * g;
    let w_bot = lerp(4.7, 6.6, st.robe) * g;
    let hem = f.hip.add(down.mul(hem_len));
    // Skirt sways opposite to the motion of the legs.
    let sway = (pose.leg[0][0] - pose.leg[1][0]) * 0.035 * f.facing;
    let hem_off = v2(sway * g, 0.0);
    let pts = [
        top.add(side.mul(w_top)),
        hem.add(side.mul(w_bot)).add(hem_off),
        hem.add(side.mul(-w_bot)).add(hem_off),
        top.add(side.mul(-w_top)),
    ];
    let dev: Vec<V2> = pts.iter().map(|p| cam.p(*p)).collect();
    fill_poly_shaded(layer, &dev, st.cloth, st.cloth_dk, 1.0);
    // Fold shading down the middle.
    fill_capsule(
        layer,
        cam.p(top),
        cam.p(hem.add(hem_off)),
        cam.l(0.8 * g),
        cam.l(1.1 * g),
        st.cloth_dk,
        0.5,
    );
    // Hem trim.
    fill_capsule(
        layer,
        cam.p(hem.add(side.mul(w_bot * 0.92)).add(hem_off)),
        cam.p(hem.add(side.mul(-w_bot * 0.92)).add(hem_off)),
        cam.l(0.75 * g),
        cam.l(0.75 * g),
        st.cloth_dk.scale(0.8),
        0.85,
    );
}

fn draw_blade(
    layer: &mut Layer,
    cam: &Cam,
    f: &Figure,
    st: &Style,
    pose: &Pose,
    blade: Blade,
    g: f32,
) {
    if blade == Blade::None {
        return;
    }
    // Rotate the forearm direction by the pose's blade angle.
    let base_deg = f.hand_dir.x.atan2(f.hand_dir.y).to_degrees() * f.facing;
    let d = dir_down(base_deg + pose.sword);
    let dir = v2(d.x * f.facing, d.y);
    let hand = f.hand[0];
    let (len, wide, curve, col) = match blade {
        Blade::Sword => (13.5, 1.05, 0.0, st.metal),
        Blade::Scimitar => (14.5, 1.35, 0.22, rgb(216, 208, 176)),
        Blade::Dagger => (6.0, 0.9, 0.0, st.metal),
        Blade::Wand => (9.0, 1.0, 0.0, rgb(96, 66, 40)),
        Blade::None => return,
    };
    let len = len * f.prop.scale;
    let guard_a = hand.sub(dir.mul(1.6 * g));
    let tip = hand.add(dir.mul(len)).add(dir.perp().mul(curve * len));
    let mid = hand.add(dir.mul(len * 0.5)).add(dir.perp().mul(curve * len * 0.35));

    // Grip and pommel.
    fill_capsule(
        layer,
        cam.p(guard_a.sub(dir.mul(1.4 * g))),
        cam.p(hand),
        cam.l(1.0 * g),
        cam.l(1.0 * g),
        rgb(92, 60, 36),
        1.0,
    );
    // Cross-guard.
    let cg = dir.perp().mul(2.5 * g);
    fill_capsule(
        layer,
        cam.p(hand.add(cg)),
        cam.p(hand.sub(cg)),
        cam.l(0.85 * g),
        cam.l(0.85 * g),
        rgb(180, 150, 78),
        1.0,
    );
    if blade == Blade::Wand {
        fill_capsule(
            layer,
            cam.p(hand),
            cam.p(tip),
            cam.l(0.85 * g),
            cam.l(0.7 * g),
            col,
            1.0,
        );
        fill_circle(layer, cam.p(tip), cam.l(1.6 * g), rgb(255, 178, 64), 1.0);
        return;
    }
    // Blade: two segments so a scimitar can curve.
    fill_capsule_grad(
        layer,
        cam.p(hand),
        cam.p(mid),
        cam.l(wide * g),
        cam.l(wide * 0.85 * g),
        col,
        col.scale(0.86),
        1.0,
    );
    fill_capsule_grad(
        layer,
        cam.p(mid),
        cam.p(tip),
        cam.l(wide * 0.85 * g),
        cam.l(0.25 * g),
        col.scale(0.95),
        Rgb::WHITE,
        1.0,
    );
}

fn draw_bones(
    layer: &mut Layer,
    cam: &Cam,
    f: &Figure,
    st: &Style,
    pose: &Pose,
    blade: Blade,
) {
    let g = f.prop.girth * f.prop.scale;
    let bone = rgb(226, 220, 198);
    let bone_dk = rgb(150, 144, 126);
    let shade = st.back_shade;
    for &(i, sh) in &[(1usize, shade), (0usize, 1.0)] {
        let c = bone.scale(sh);
        let cd = bone_dk.scale(sh);
        fill_capsule_grad(
            layer,
            cam.p(f.hip),
            cam.p(f.knee[i]),
            cam.l(1.5 * g),
            cam.l(1.1 * g),
            c,
            cd,
            1.0,
        );
        fill_capsule_grad(
            layer,
            cam.p(f.knee[i]),
            cam.p(f.ankle[i]),
            cam.l(1.15 * g),
            cam.l(0.9 * g),
            c,
            cd,
            1.0,
        );
        fill_capsule(
            layer,
            cam.p(f.ankle[i]),
            cam.p(f.toe[i]),
            cam.l(1.1 * g),
            cam.l(0.8 * g),
            cd,
            1.0,
        );
        fill_capsule_grad(
            layer,
            cam.p(f.shoulder),
            cam.p(f.elbow[i]),
            cam.l(1.2 * g),
            cam.l(0.95 * g),
            c,
            cd,
            1.0,
        );
        fill_capsule_grad(
            layer,
            cam.p(f.elbow[i]),
            cam.p(f.hand[i]),
            cam.l(0.95 * g),
            cam.l(0.85 * g),
            c,
            cd,
            1.0,
        );
    }
    // Spine and pelvis.
    fill_capsule(
        layer,
        cam.p(f.hip),
        cam.p(f.shoulder),
        cam.l(1.3 * g),
        cam.l(1.1 * g),
        bone_dk,
        1.0,
    );
    fill_ellipse_local(layer, cam.p(f.hip), cam.l(3.1 * g), cam.l(2.0 * g), bone);
    // Ribs.
    let down = f.hip.sub(f.shoulder).norm();
    for k in 0..4 {
        let t = 0.16 + k as f32 * 0.17;
        let c = f.shoulder.add(down.mul(f.prop.torso * t * f.prop.scale));
        let w = (2.9 - k as f32 * 0.18) * g;
        let per = down.perp();
        fill_capsule(
            layer,
            cam.p(c.add(per.mul(w))),
            cam.p(c.sub(per.mul(w))),
            cam.l(0.62 * g),
            cam.l(0.62 * g),
            bone,
            1.0,
        );
    }
    // Skull.
    let hc = cam.p(f.head);
    let hr = cam.l(f.head_r);
    fill_circle(layer, hc, hr * 1.05, bone, 1.0);
    fill_capsule(
        layer,
        v2(hc.x + f.facing * hr * 0.3, hc.y + hr * 0.75),
        v2(hc.x + f.facing * hr * 0.85, hc.y + hr * 0.65),
        hr * 0.45,
        hr * 0.38,
        bone.scale(0.9),
        1.0,
    );
    // Eye socket and nasal cavity.
    fill_circle(
        layer,
        v2(hc.x + f.facing * hr * 0.45, hc.y - hr * 0.1),
        hr * 0.34,
        rgb(24, 18, 20),
        1.0,
    );
    fill_circle(
        layer,
        v2(hc.x + f.facing * hr * 0.95, hc.y + hr * 0.3),
        hr * 0.18,
        rgb(24, 18, 20),
        0.85,
    );
    draw_blade(layer, cam, f, st, pose, blade, g);
}

fn fill_ellipse_local(layer: &mut Layer, c: V2, rx: f32, ry: f32, col: Rgb) {
    fill_ellipse(layer, c, rx, ry, col, 1.0);
}

/// Device-space bounding box of a figure, padded for outline and weapon.
pub fn figure_bbox(cam: &Cam, f: &Figure, pad: f32) -> (i32, i32, i32, i32) {
    let mut minx = f32::MAX;
    let mut miny = f32::MAX;
    let mut maxx = f32::MIN;
    let mut maxy = f32::MIN;
    let mut acc = |p: V2| {
        let d = cam.p(p);
        minx = minx.min(d.x);
        miny = miny.min(d.y);
        maxx = maxx.max(d.x);
        maxy = maxy.max(d.y);
    };
    acc(f.hip);
    acc(f.shoulder);
    acc(f.head);
    for i in 0..2 {
        acc(f.knee[i]);
        acc(f.ankle[i]);
        acc(f.toe[i]);
        acc(f.elbow[i]);
        acc(f.hand[i]);
    }
    let pad = cam.l(pad) + 4.0;
    (
        (minx - pad).floor() as i32,
        (miny - pad).floor() as i32,
        (maxx + pad).ceil() as i32,
        (maxy + pad).ceil() as i32,
    )
}
