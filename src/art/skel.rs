//! Articulated figure rendering.
//!
//! Rather than storing a few dozen hand-drawn sprite sheets, every character in
//! the game is a small skeleton — hip, torso, head, two arms, two legs — drawn
//! with anti-aliased primitives that are shaded as *solids*: each limb is a
//! cylinder lit from a fixed direction, each head a sphere, each piece of cloth a
//! polygon with a directional gradient. That form lighting is what stops the
//! figures reading as flat lozenges, and it is what survives being squeezed down
//! to a dozen terminal pixels — the silhouette stays clean and the value
//! structure (bright tunic, mid skin, dark hair and boots) still reads.
//!
//! Animations are keyframed joint angles, which is what makes it practical to
//! have running, jumping, climbing, hanging, drinking, three sword stances and
//! five body types all look like the same world.
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
    fill_capsule, fill_circle, fill_ellipse, fill_ellipse_lit, fill_capsule_lit, fill_poly,
    fill_poly_dir,
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
        hip: 13.0,
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
    /// Half-depth of the chest, seen from the side.
    pub chest: f32,
    /// Half-depth at the waist.
    pub waist: f32,
    /// Overall scale multiplier.
    pub scale: f32,
    /// Limb and body thickness multiplier.
    pub girth: f32,
}

impl Prop {
    /// Standing height works out to about 27 art pixels — a shade under the 31
    /// pixels of headroom a tile leaves, and close to the original's
    /// prince-to-floor ratio. The head is a sixth of that: heroic without being a
    /// caricature.
    pub const PRINCE: Prop = Prop {
        thigh: 7.0,
        shin: 6.8,
        foot: 3.2,
        torso: 7.4,
        neck: 1.5,
        head_r: 2.45,
        upper: 5.3,
        fore: 4.8,
        hand: 1.5,
        chest: 3.05,
        waist: 2.15,
        scale: 1.0,
        girth: 1.0,
    };

    pub fn scaled(mut self, s: f32, g: f32) -> Prop {
        self.scale = s;
        self.girth = g;
        self
    }

    /// How far above the feet the hands reach with the arms straight up. This is
    /// what [`crate::world::tile::HANG_DROP`] has to agree with, or the hands
    /// float off the ledge they are supposed to be gripping.
    pub fn reach_up(&self) -> f32 {
        (self.thigh + self.shin + self.torso + self.upper + self.fore) * self.scale
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
    /// Unit vector from hip to shoulder.
    pub up: V2,
    /// Unit vector perpendicular to the torso, pointing the way the body faces.
    pub fwd: V2,
    /// +1 facing right, -1 facing left.
    pub facing: f32,
    /// Direction the near forearm points, for attaching a weapon.
    pub hand_dir: V2,
    pub prop: Prop,
    pub squash: f32,
    /// Horizontal compression in 0..1 — drives the turn-on-the-spot effect.
    pub turn: f32,
}

/// Forward kinematics. `feet` is the world point at the character's base,
/// centred horizontally.
///
/// `facing` carries both direction and magnitude: the sign mirrors the figure and
/// the magnitude compresses it horizontally, so easing `facing` from +1 to -1
/// turns the character through an edge-on pose instead of snapping.
pub fn solve(pose: &Pose, prop: &Prop, feet: V2, facing: f32) -> Figure {
    let s = prop.scale;
    let sq = pose.squash;
    let turn = clampf(facing.abs(), 0.16, 1.0);
    // Vertical squash pivots on the feet; horizontal stretch keeps the volume.
    let vs = s * sq;
    let hs = s * (2.0 - sq).max(0.6) * turn;

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

    let up = shoulder.sub(hip).norm();
    // `up.perp()` points towards +x when the body is upright, which is "forward"
    // in the unmirrored space.
    let fwd = up.perp();

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
        up,
        fwd,
        facing: if facing < 0.0 { -1.0 } else { 1.0 },
        hand_dir,
        prop: *prop,
        squash: sq,
        turn,
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
        f.up = v2(-f.up.x, f.up.y);
        f.fwd = v2(-f.fwd.x, f.fwd.y);
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
    /// Length of the garment below the waist: 0 = none (bare trousers, as the
    /// prince wears), 0.5 = short tunic, 1 = full-length robe.
    pub robe: f32,
    /// Draw as a skeleton: thin bones, ribcage, skull.
    pub bones: bool,
    /// A round shield carried in the far hand.
    pub shield: Option<Rgb>,
    /// Plume on the helmet. Its presence also adds a nasal bar.
    pub plume: Option<Rgb>,
    /// A studded belt over the sash.
    pub belt: bool,

    // ---- costume ---------------------------------------------------------
    /// Bare torso, modelled with pectorals and abdominals in skin rather than
    /// covered in cloth. This is how the prince is drawn.
    pub bare_chest: bool,
    /// Open waistcoat over the torso: covers the back and leaves the chest bare.
    pub vest: Option<Rgb>,
    /// Trouser colour, separate from the upper garment.
    pub trouser: Rgb,
    /// How loose the trousers are: 0 fitted, 1 fully gathered at the ankle.
    pub baggy: f32,
    /// Headband tied over the hair.
    pub band: Option<Rgb>,
    /// A long ribbon trailing behind the shoulders.
    pub scarf: Option<Rgb>,
}

impl Style {
    pub const PRINCE: Style = Style {
        skin: rgb(222, 168, 120),
        skin_dk: rgb(150, 96, 62),
        cloth: rgb(238, 236, 226),
        cloth_dk: rgb(150, 152, 166),
        sash: rgb(186, 46, 46),
        sash_dk: rgb(104, 20, 26),
        hair: rgb(74, 48, 34),
        boot: rgb(126, 78, 44),
        metal: rgb(198, 208, 222),
        outline: rgb(20, 16, 28),
        head_wrap: None,
        robe: 0.0,
        bones: false,
        shield: None,
        plume: None,
        belt: false,
        bare_chest: true,
        vest: None,
        trouser: rgb(126, 158, 164),
        baggy: 0.85,
        band: Some(rgb(150, 40, 44)),
        scarf: None,
    };
}

/// Colour treatment for limbs on the far side of the body: darker, cooler and a
/// little desaturated, so the near limbs come forward without needing an outline
/// drawn between them.
fn recede(c: Rgb) -> Rgb {
    c.scale(0.60).desaturate(0.22).lerp(rgb(26, 24, 42), 0.14)
}

// ---------------------------------------------------------------- drawing

/// Paint a figure into `layer`. The caller composites the layer afterwards so the
/// whole silhouette gets one clean outline.
pub fn draw_figure(
    layer: &mut Layer,
    cam: &Cam,
    f: &Figure,
    st: &Style,
    pose: &Pose,
    blade: Blade,
) {
    // Form light: from above and slightly in front, so the face and chest catch
    // it whichever way the character faces.
    let light = v2(f.facing * 0.42, -0.91).norm();

    if st.bones {
        draw_bones(layer, cam, f, st, pose, blade, light);
        return;
    }

    // --- far side ------------------------------------------------------
    draw_leg(layer, cam, f, st, 1, light, true);
    draw_arm(layer, cam, f, st, 1, light, true);
    if let Some(sc) = st.shield {
        let g = f.prop.girth * f.prop.scale;
        let c = cam.p(f.hand[1]);
        fill_ellipse_lit(layer, c, cam.l(4.8 * g), cam.l(4.4 * g), recede(sc), light, 1.0);
        fill_ellipse_lit(
            layer,
            c,
            cam.l(1.8 * g),
            cam.l(1.7 * g),
            recede(st.metal),
            light,
            1.0,
        );
    }
    draw_scarf(layer, cam, f, st, pose);
    draw_sash_tail(layer, cam, f, st, pose);

    // --- near leg, torso, then the tunic over the thighs ----------------
    draw_leg(layer, cam, f, st, 0, light, false);
    draw_torso(layer, cam, f, st, light);
    if st.robe > 0.01 {
        draw_tunic(layer, cam, f, st, pose, light);
    }
    draw_sash(layer, cam, f, st, light);

    // --- head ----------------------------------------------------------
    draw_head(layer, cam, f, st, light);

    // --- near arm and weapon -------------------------------------------
    draw_arm(layer, cam, f, st, 0, light, false);
    draw_blade(layer, cam, f, st, pose, blade);
}

// ---------------------------------------------------------------- limbs

fn draw_leg(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, i: usize, light: V2, far: bool) {
    let g = f.prop.girth * f.prop.scale;
    let tint = |c: Rgb| if far { recede(c) } else { c };
    // Trousers are the tunic cloth a touch deeper, so the legs separate from the
    // skirt hanging over them.
    let trouser = tint(st.trouser);
    let boot = tint(st.boot);
    // Loose trousers widen towards the knee and gather at the ankle, which is the
    // silhouette the reference art has.
    let b = clampf(st.baggy, 0.0, 1.0);
    let th0 = 1.82 + 0.46 * b;
    let th1 = 1.46 + 1.44 * b;
    let sh0 = 1.36 + 1.62 * b;
    let sh1 = 0.98 + 0.10 * b;
    if !far {
        let rim = st.trouser.scale(0.30);
        let e = 0.42 * g;
        fill_capsule(
            layer,
            cam.p(f.hip),
            cam.p(f.knee[i]),
            cam.l(th0 * g) + e,
            cam.l(th1 * g) + e,
            rim,
            1.0,
        );
        fill_capsule(
            layer,
            cam.p(f.knee[i]),
            cam.p(f.ankle[i]),
            cam.l(sh0 * g) + e,
            cam.l(sh1 * g) + e,
            rim,
            1.0,
        );
    }

    fill_capsule_lit(
        layer,
        cam.p(f.hip),
        cam.p(f.knee[i]),
        cam.l(th0 * g),
        cam.l(th1 * g),
        trouser,
        light,
        1.0,
    );
    // A slight bulge at the knee, so the joint is not a hinge.
    fill_ellipse_lit(
        layer,
        cam.p(f.knee[i]),
        cam.l(th1.max(sh0) * g),
        cam.l(th1.max(sh0) * g),
        trouser,
        light,
        1.0,
    );
    fill_capsule_lit(
        layer,
        cam.p(f.knee[i]),
        cam.p(f.ankle[i]),
        cam.l(sh0 * g),
        cam.l(sh1 * g),
        trouser,
        light,
        1.0,
    );
    // Where loose cloth is gathered into the boot.
    if b > 0.15 {
        let a = f.ankle[i];
        let dirn = a.sub(f.knee[i]).norm();
        let per = dirn.perp();
        let cuff = a.sub(dirn.mul(1.3 * g));
        fill_capsule_lit(
            layer,
            cam.p(cuff.add(per.mul((sh1 + 0.5 * b) * g))),
            cam.p(cuff.sub(per.mul((sh1 + 0.5 * b) * g))),
            cam.l(0.55 * g),
            cam.l(0.55 * g),
            trouser.scale(0.78),
            light,
            1.0,
        );
    }

    // --- boot: heel, sole and toe as one shape -------------------------
    let a = f.ankle[i];
    let t = f.toe[i];
    let sole = t.sub(a).norm();
    let up = sole.perp().mul(-1.0);
    let heel = a.sub(sole.mul(1.15 * g));
    let pts = [
        cam.p(heel.add(up.mul(1.95 * g))),
        cam.p(a.add(up.mul(2.15 * g))),
        cam.p(t.add(up.mul(0.85 * g))),
        cam.p(t.sub(up.mul(0.38 * g))),
        cam.p(heel.sub(up.mul(0.28 * g))),
    ];
    fill_poly_dir(layer, &pts, boot.scale(1.18), boot.scale(0.62), light, 1.0);
    // Sole: a dark strip along the bottom.
    fill_capsule(
        layer,
        cam.p(heel.sub(up.mul(0.15 * g))),
        cam.p(t.sub(up.mul(0.2 * g))),
        cam.l(0.4 * g),
        cam.l(0.33 * g),
        boot.scale(0.42),
        0.9,
    );
    // Cuff at the top of the boot.
    let cf = sole.mul(0.85 * g);
    fill_capsule_lit(
        layer,
        cam.p(a.add(up.mul(2.0 * g)).sub(cf)),
        cam.p(a.add(up.mul(1.8 * g)).add(cf)),
        cam.l(0.62 * g),
        cam.l(0.62 * g),
        boot.scale(1.3),
        light,
        1.0,
    );
}

fn draw_arm(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, i: usize, light: V2, far: bool) {
    let g = f.prop.girth * f.prop.scale;
    let tint = |c: Rgb| if far { recede(c) } else { c };
    let skin = tint(st.skin);
    // A bare-armed figure has no sleeve; a waistcoat gives only a shoulder cap.
    let sleeved = st.vest.is_some() || !st.bare_chest;
    let cloth = tint(st.vest.unwrap_or(if st.bare_chest { st.skin } else { st.cloth }));
    if !far {
        let rim = if st.bare_chest {
            st.skin_dk.scale(0.42)
        } else {
            st.cloth_dk.scale(0.34)
        };
        let e = 0.30 * g;
        fill_capsule(
            layer,
            cam.p(f.shoulder),
            cam.p(f.elbow[i]),
            cam.l(1.22 * g) + e,
            cam.l(0.82 * g) + e,
            rim,
            1.0,
        );
        fill_capsule(
            layer,
            cam.p(f.elbow[i]),
            cam.p(f.hand[i]),
            cam.l(0.86 * g) + e,
            cam.l(0.70 * g) + e,
            rim,
            1.0,
        );
    }

    // Deltoid cap, so the shoulder is round rather than a stump.
    let cap = f
        .shoulder
        .add(f.up.mul(0.15))
        .add(f.fwd.mul(if i == 0 { 0.3 } else { -0.3 }));
    fill_ellipse_lit(
        layer,
        cam.p(cap),
        cam.l(1.28 * g),
        cam.l(1.20 * g),
        cloth,
        light,
        1.0,
    );
    // Short sleeve, with a hem.
    let sleeve_end = f.shoulder.lerp(f.elbow[i], 0.42);
    if sleeved {
        fill_capsule_lit(
            layer,
            cam.p(f.shoulder),
            cam.p(sleeve_end),
            cam.l(1.22 * g),
            cam.l(1.02 * g),
            cloth,
            light,
            1.0,
        );
        fill_capsule(
            layer,
            cam.p(sleeve_end.add(f.fwd.mul(0.95 * g))),
            cam.p(sleeve_end.sub(f.fwd.mul(0.95 * g))),
            cam.l(0.30 * g),
            cam.l(0.30 * g),
            tint(if st.bare_chest { st.skin_dk } else { st.cloth_dk }),
            0.85,
        );
    }
    // Upper arm and forearm.
    fill_capsule_lit(
        layer,
        cam.p(if sleeved {
            f.shoulder.lerp(f.elbow[i], 0.44)
        } else {
            f.shoulder
        }),
        cam.p(f.elbow[i]),
        cam.l(if sleeved { 0.98 } else { 1.16 } * g),
        cam.l(0.82 * g),
        skin,
        light,
        1.0,
    );
    fill_capsule_lit(
        layer,
        cam.p(f.elbow[i]),
        cam.p(f.hand[i]),
        cam.l(0.86 * g),
        cam.l(0.68 * g),
        skin,
        light,
        1.0,
    );
    // Hand: a small wedge rather than a ball.
    let d = f.hand[i].sub(f.elbow[i]).norm();
    let s = d.perp();
    let h = f.hand[i];
    let pts = [
        cam.p(h.sub(d.mul(0.25 * g)).add(s.mul(0.85 * g))),
        cam.p(h.add(d.mul(1.15 * g)).add(s.mul(0.60 * g))),
        cam.p(h.add(d.mul(1.25 * g)).sub(s.mul(0.50 * g))),
        cam.p(h.sub(d.mul(0.25 * g)).sub(s.mul(0.82 * g))),
    ];
    fill_poly_dir(layer, &pts, skin.scale(1.15), skin.scale(0.7), light, 1.0);
}

// ---------------------------------------------------------------- body

fn draw_torso(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, light: V2) {
    let g = f.prop.girth * f.prop.scale;
    let down = f.up.mul(-1.0);
    let body = if st.bare_chest { st.skin } else { st.cloth };
    // One cylinder from shoulders to hips, wide at the chest and narrow at the
    // waist; the cross-axis shading does the rest.
    fill_capsule_lit(
        layer,
        cam.p(f.shoulder),
        cam.p(f.hip),
        cam.l(f.prop.chest * g),
        cam.l(f.prop.waist * g),
        body,
        light,
        1.0,
    );

    if st.bare_chest {
        // Pectoral: a plate on the front of the chest with a shadow under it.
        let pec = f
            .shoulder
            .add(down.mul(1.5 * g))
            .add(f.fwd.mul(f.prop.chest * g * 0.34));
        fill_ellipse_lit(
            layer,
            cam.p(pec),
            cam.l(1.9 * g),
            cam.l(1.45 * g),
            st.skin.scale(1.06),
            light,
            1.0,
        );
        fill_capsule(
            layer,
            cam.p(pec.add(down.mul(1.35 * g)).add(f.fwd.mul(1.5 * g))),
            cam.p(pec.add(down.mul(1.5 * g)).sub(f.fwd.mul(1.0 * g))),
            cam.l(0.42 * g),
            cam.l(0.34 * g),
            st.skin_dk.scale(0.85),
            0.55,
        );
        // Abdominals: two short creases down the front of the belly.
        for k in 0..2 {
            let a = f
                .shoulder
                .add(down.mul((3.6 + k as f32 * 1.5) * g))
                .add(f.fwd.mul(f.prop.chest * g * 0.42));
            fill_capsule(
                layer,
                cam.p(a),
                cam.p(a.sub(f.fwd.mul(1.5 * g))),
                cam.l(0.3 * g),
                cam.l(0.24 * g),
                st.skin_dk.scale(0.9),
                0.4,
            );
        }
        // Collarbone.
        fill_capsule(
            layer,
            cam.p(f.shoulder.add(f.fwd.mul(f.prop.chest * g * 0.85))),
            cam.p(f.shoulder.sub(f.fwd.mul(f.prop.chest * g * 0.5))),
            cam.l(0.34 * g),
            cam.l(0.30 * g),
            st.skin.scale(1.18),
            0.6,
        );
        // The hollow of the throat.
        fill_ellipse(
            layer,
            cam.p(f.shoulder.add(f.up.mul(0.3 * g)).add(f.fwd.mul(0.2 * g))),
            cam.l(0.7 * g),
            cam.l(0.5 * g),
            st.skin_dk.scale(0.8),
            0.5,
        );
    } else {
        // Shoulder line across the top of the chest.
        fill_capsule(
            layer,
            cam.p(f.shoulder.add(f.fwd.mul(f.prop.chest * g * 0.9))),
            cam.p(f.shoulder.sub(f.fwd.mul(f.prop.chest * g * 0.9))),
            cam.l(0.7 * g),
            cam.l(0.7 * g),
            st.cloth.scale(1.16),
            0.75,
        );
        // V-neck: a wedge of shadowed skin at the collar.
        let base = f.shoulder.add(f.up.mul(0.2));
        let pts = [
            cam.p(base.add(f.fwd.mul(1.25 * g))),
            cam.p(base.sub(f.fwd.mul(0.7 * g))),
            cam.p(base.sub(f.up.mul(2.1 * g)).add(f.fwd.mul(0.25 * g))),
        ];
        fill_poly(layer, &pts, st.skin_dk.scale(0.9), 0.9);
        fill_capsule(
            layer,
            cam.p(base.add(f.fwd.mul(1.35 * g))),
            cam.p(base.sub(f.fwd.mul(0.85 * g))),
            cam.l(0.34 * g),
            cam.l(0.34 * g),
            st.cloth_dk,
            0.85,
        );
    }

    // Open waistcoat: covers the back and the sides, leaves the chest bare. In a
    // side view that reads as the garment's front edge running down the body.
    if let Some(v) = st.vest {
        let sh = f.shoulder.add(f.up.mul(0.4 * g));
        let hip = f.hip.add(f.up.mul(1.4 * g));
        let front = f.prop.chest * g * 0.34;
        let back = f.prop.chest * g * 1.02;
        let pts = [
            cam.p(sh.add(f.fwd.mul(front))),
            cam.p(hip.add(f.fwd.mul(front * 0.7))),
            cam.p(hip.sub(f.fwd.mul(back * 0.85))),
            cam.p(sh.sub(f.fwd.mul(back))),
        ];
        fill_poly_dir(layer, &pts, v.scale(1.14), v.scale(0.6), light, 1.0);
        // Front edge and shoulder seam.
        fill_capsule(
            layer,
            cam.p(sh.add(f.fwd.mul(front))),
            cam.p(hip.add(f.fwd.mul(front * 0.7))),
            cam.l(0.36 * g),
            cam.l(0.36 * g),
            v.scale(1.35),
            0.9,
        );
    }
}

fn draw_tunic(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, pose: &Pose, light: V2) {
    let g = f.prop.girth * f.prop.scale;
    let down = f.up.mul(-1.0);
    let side = f.fwd;
    let hem_len = lerp(0.0, 14.6, clampf(st.robe, 0.0, 1.0)) * f.prop.scale;
    let top = f.hip.add(f.up.mul(f.prop.torso * f.prop.scale * 0.34));
    let w_top = (f.prop.waist + 0.15) * g;
    let w_bot = lerp(2.6, 5.4, st.robe) * g;
    // The skirt swings against the legs and lifts on the leading side.
    let swing = (pose.leg[0][0] - pose.leg[1][0]) * 0.030;
    let hem = f.hip.add(down.mul(hem_len));
    let off = side.mul(swing * g);
    let lift = swing.abs() * 0.09 * g;

    // Front to back across the hem, the cloth rising slightly at the edges.
    let hp = |t: f32, drop: f32| -> V2 {
        hem.add(side.mul(lerp(w_bot, -w_bot, t)))
            .add(off)
            .add(down.mul(drop))
    };
    let pts = [
        cam.p(top.add(side.mul(w_top))),
        cam.p(f.hip.add(side.mul(w_top * 1.06))),
        cam.p(hp(0.0, -lift)),
        cam.p(hp(0.28, 0.45 * g)),
        cam.p(hp(0.56, -0.15 * g)),
        cam.p(hp(0.82, 0.36 * g)),
        cam.p(hp(1.0, -lift * 0.6)),
        cam.p(f.hip.sub(side.mul(w_top * 1.06))),
        cam.p(top.sub(side.mul(w_top))),
    ];
    fill_poly_dir(layer, &pts, st.cloth.scale(1.1), st.cloth_dk, light, 1.0);

    // Two creases following the drape.
    for (t, w) in [(0.34f32, 0.75f32), (0.66, 0.55)] {
        let a = f.hip.add(side.mul(lerp(w_top, -w_top, t) * 0.8));
        let b = hp(t, 0.25 * g);
        fill_capsule(
            layer,
            cam.p(a),
            cam.p(b),
            cam.l(w * 0.5 * g),
            cam.l(w * g),
            st.cloth_dk,
            0.40,
        );
    }
    // A darker band along the wavy hem.
    for i in 0..6 {
        let t0 = i as f32 / 6.0;
        let t1 = (i + 1) as f32 / 6.0;
        let d0 = 0.45 * g * (t0 * 3.6).sin().abs();
        let d1 = 0.45 * g * (t1 * 3.6).sin().abs();
        fill_capsule(
            layer,
            cam.p(hp(t0, d0)),
            cam.p(hp(t1, d1)),
            cam.l(0.38 * g),
            cam.l(0.38 * g),
            st.cloth_dk.scale(0.9),
            0.7,
        );
    }
}

fn draw_sash(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, light: V2) {
    let g = f.prop.girth * f.prop.scale;
    let waist = f.hip.lerp(f.shoulder, 0.20);
    let across = f.fwd.mul((f.prop.waist + 0.25) * g);
    fill_capsule_lit(
        layer,
        cam.p(waist.add(across)),
        cam.p(waist.sub(across)),
        cam.l(0.85 * g),
        cam.l(0.78 * g),
        st.sash,
        light,
        1.0,
    );
    // Knot on the near hip.
    let knot = waist.add(f.fwd.mul(1.15 * g)).add(f.up.mul(-0.3 * g));
    fill_ellipse_lit(
        layer,
        cam.p(knot),
        cam.l(0.9 * g),
        cam.l(0.75 * g),
        st.sash.scale(1.12),
        light,
        1.0,
    );
    if st.belt {
        let b = waist.add(f.up.mul(-1.5 * g));
        fill_capsule(
            layer,
            cam.p(b.add(across)),
            cam.p(b.sub(across)),
            cam.l(0.8 * g),
            cam.l(0.8 * g),
            st.boot.scale(0.7),
            1.0,
        );
        for k in -1..=1 {
            fill_circle(
                layer,
                cam.p(b.add(f.fwd.mul(k as f32 * 1.5 * g))),
                cam.l(0.42 * g),
                st.metal.scale(0.85),
                1.0,
            );
        }
    }
}

fn draw_sash_tail(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, pose: &Pose) {
    let g = f.prop.girth * f.prop.scale;
    let waist = f.hip.lerp(f.shoulder, 0.20);
    let back = f.fwd.mul(-1.0);
    let a = waist.add(back.mul(1.3 * g));
    let mid = a.add(back.mul(1.5 * g * pose.tail)).add(f.up.mul(-2.4 * g));
    let end = mid
        .add(back.mul(1.0 * g * pose.tail))
        .add(f.up.mul(-2.6 * g - pose.tail * 0.6 * g));
    fill_capsule(
        layer,
        cam.p(a),
        cam.p(mid),
        cam.l(0.72 * g),
        cam.l(0.5 * g),
        st.sash_dk,
        1.0,
    );
    fill_capsule(
        layer,
        cam.p(mid),
        cam.p(end),
        cam.l(0.5 * g),
        cam.l(0.18 * g),
        st.sash_dk.scale(0.85),
        1.0,
    );
}

/// A long ribbon streaming back over the shoulder. The shadow prince wears one,
/// and it is most of what makes him read as an apparition rather than a recolour.
fn draw_scarf(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, pose: &Pose) {
    let Some(col) = st.scarf else { return };
    // Distinctly brighter than the body, or it disappears into it.
    let col = col.scale(1.25);
    let g = f.prop.girth * f.prop.scale;
    let back = f.fwd.mul(-1.0);
    let mut p = f.shoulder.add(back.mul(1.2 * g)).add(f.up.mul(0.4 * g));
    // Four segments, each swinging a little further and waving with the gait.
    let wave = pose.leg[0][0] * 0.02;
    for k in 0..4 {
        let t = k as f32;
        let q = p
            .add(back.mul((3.0 + t * 0.6) * g))
            .add(f.up.mul((0.9 + (t * 1.7 + wave).sin() * 2.2) * g));
        fill_capsule(
            layer,
            cam.p(p),
            cam.p(q),
            cam.l((1.5 - t * 0.3) * g),
            cam.l((1.2 - t * 0.28) * g),
            col.scale(1.0 - t * 0.12),
            1.0,
        );
        p = q;
    }
}

// ---------------------------------------------------------------- head

fn draw_head(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, light: V2) {
    let g = f.prop.girth * f.prop.scale;
    let hc = cam.p(f.head);
    let hr = cam.l(f.head_r);
    let fw = f.facing;

    // Neck.
    fill_capsule_lit(
        layer,
        cam.p(f.shoulder.add(f.up.mul(-0.4))),
        cam.p(f.neck.add(f.up.mul(0.3))),
        cam.l(1.02 * g),
        cam.l(0.95 * g),
        st.skin_dk.lerp(st.skin, 0.45),
        light,
        1.0,
    );

    // Cranium, then the jaw pushed forward and down.
    fill_ellipse_lit(layer, hc, hr * 1.0, hr * 1.08, st.skin, light, 1.0);
    fill_ellipse_lit(
        layer,
        v2(hc.x + fw * hr * 0.26, hc.y + hr * 0.40),
        hr * 0.76,
        hr * 0.60,
        st.skin,
        light,
        1.0,
    );
    // Ear, tucked at the back of the jaw.
    fill_ellipse_lit(
        layer,
        v2(hc.x - fw * hr * 0.62, hc.y + hr * 0.14),
        hr * 0.30,
        hr * 0.38,
        st.skin.scale(0.9),
        light,
        1.0,
    );

    match st.head_wrap {
        None => draw_hair(layer, hc, hr, fw, st, light),
        Some(wrap) => draw_wrap(layer, hc, hr, fw, st, wrap, light),
    }
    if let Some(band) = st.band {
        // Tied across the brow, with the knot-ends trailing at the back.
        let p = |dx: f32, dy: f32| v2(hc.x + fw * hr * dx, hc.y + hr * dy);
        fill_capsule_lit(
            layer,
            p(0.82, -0.44),
            p(-1.02, -0.56),
            hr * 0.26,
            hr * 0.28,
            band,
            light,
            1.0,
        );
        fill_capsule(
            layer,
            p(-0.92, -0.52),
            p(-1.46, -0.10),
            hr * 0.20,
            hr * 0.09,
            band.scale(0.82),
            1.0,
        );
        fill_capsule(
            layer,
            p(-0.92, -0.44),
            p(-1.38, 0.30),
            hr * 0.17,
            hr * 0.08,
            band.scale(0.7),
            1.0,
        );
    }

    // ---- face ---------------------------------------------------------
    // Brow ridge, casting the eye into shadow.
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.14, hc.y - hr * 0.34),
        v2(hc.x + fw * hr * 0.86, hc.y - hr * 0.24),
        hr * 0.17,
        hr * 0.12,
        st.skin_dk.scale(0.78),
        0.9,
    );
    // Eyebrow, in hair colour: at this scale it does more for the read than any
    // amount of modelling.
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.26, hc.y - hr * 0.40),
        v2(hc.x + fw * hr * 0.82, hc.y - hr * 0.32),
        hr * 0.11,
        hr * 0.08,
        st.hair.scale(1.1),
        0.85,
    );
    // Eye: a dark almond on a pale sclera, kept high-contrast so it still reads
    // when the whole figure is a dozen pixels tall.
    let eye = v2(hc.x + fw * hr * 0.56, hc.y);
    fill_ellipse(layer, eye, hr * 0.30, hr * 0.22, rgb(250, 246, 238), 0.95);
    fill_ellipse(
        layer,
        v2(eye.x + fw * hr * 0.07, eye.y + hr * 0.02),
        hr * 0.17,
        hr * 0.19,
        st.outline,
        1.0,
    );
    // Nose: a wedge with a lit bridge.
    let nose = [
        v2(hc.x + fw * hr * 0.56, hc.y - hr * 0.02),
        v2(hc.x + fw * hr * 1.06, hc.y + hr * 0.34),
        v2(hc.x + fw * hr * 0.58, hc.y + hr * 0.40),
    ];
    fill_poly_dir(layer, &nose, st.skin.scale(1.20), st.skin.scale(0.78), light, 1.0);
    // Mouth, and the shadow under the lip.
    fill_capsule(
        layer,
        v2(hc.x + fw * hr * 0.44, hc.y + hr * 0.64),
        v2(hc.x + fw * hr * 0.86, hc.y + hr * 0.60),
        hr * 0.10,
        hr * 0.08,
        st.skin_dk.scale(0.55),
        0.85,
    );
    // Cheekbone highlight and the shadow of the jaw.
    fill_ellipse(
        layer,
        v2(hc.x + fw * hr * 0.34, hc.y + hr * 0.14),
        hr * 0.32,
        hr * 0.22,
        st.skin.scale(1.16),
        0.5,
    );
    fill_capsule(
        layer,
        v2(hc.x - fw * hr * 0.24, hc.y + hr * 0.80),
        v2(hc.x + fw * hr * 0.66, hc.y + hr * 0.74),
        hr * 0.14,
        hr * 0.11,
        st.skin_dk.scale(0.7),
        0.55,
    );
}

fn draw_hair(layer: &mut Layer, hc: V2, hr: f32, fw: f32, st: &Style, light: V2) {
    // A swept cap: low on the brow at the front, sweeping back and down into a
    // short tail. One polygon, so the silhouette is deliberate.
    let p = |dx: f32, dy: f32| v2(hc.x + fw * hr * dx, hc.y + hr * dy);
    let cap = [
        p(0.66, -0.54),
        p(0.30, -0.94),
        p(-0.20, -1.04),
        p(-0.66, -0.86),
        p(-0.94, -0.30),
        p(-1.00, 0.30),
        p(-0.78, 0.62),
        p(-1.10, 0.54),
        p(-1.16, 0.02),
        p(-1.02, -0.64),
        p(-0.44, -1.12),
        p(0.26, -1.02),
        p(0.76, -0.70),
    ];
    fill_poly_dir(layer, &cap, st.hair.scale(1.30), st.hair.scale(0.62), light, 1.0);
    // Tail at the nape.
    fill_capsule_lit(
        layer,
        p(-0.86, 0.34),
        p(-1.06, 0.94),
        hr * 0.30,
        hr * 0.13,
        st.hair.scale(1.1),
        light,
        1.0,
    );
    // A lock falling across the temple.
    fill_capsule(
        layer,
        p(0.18, -0.98),
        p(0.66, -0.60),
        hr * 0.18,
        hr * 0.10,
        st.hair.scale(1.2),
        1.0,
    );
    // Sheen along the top of the head.
    fill_capsule(
        layer,
        p(-0.38, -0.94),
        p(0.30, -0.82),
        hr * 0.15,
        hr * 0.11,
        st.hair.lerp(rgb(150, 152, 180), 0.45),
        0.6,
    );
}

fn draw_wrap(layer: &mut Layer, hc: V2, hr: f32, fw: f32, st: &Style, wrap: Rgb, light: V2) {
    let p = |dx: f32, dy: f32| v2(hc.x + fw * hr * dx, hc.y + hr * dy);
    // Dome.
    fill_ellipse_lit(
        layer,
        v2(hc.x - fw * hr * 0.08, hc.y - hr * 0.66),
        hr * 1.10,
        hr * 0.88,
        wrap,
        light,
        1.0,
    );
    // Wrapped band across the brow, with a visible overlap at the back.
    fill_capsule_lit(
        layer,
        p(0.98, -0.22),
        p(-1.04, -0.34),
        hr * 0.36,
        hr * 0.40,
        wrap.scale(0.86),
        light,
        1.0,
    );
    fill_capsule(
        layer,
        p(-0.78, -0.62),
        p(-1.12, 0.10),
        hr * 0.26,
        hr * 0.22,
        wrap.scale(0.66),
        0.9,
    );
    // A plume means this is a helmet, so give it a nasal bar too.
    if st.plume.is_some() {
        fill_capsule(
            layer,
            p(0.70, -0.30),
            p(0.80, 0.46),
            hr * 0.18,
            hr * 0.14,
            st.metal.scale(0.9),
            1.0,
        );
    }
    if let Some(pl) = st.plume {
        fill_capsule_lit(
            layer,
            p(-0.10, -1.44),
            p(-1.30, -2.50),
            hr * 0.34,
            hr * 0.10,
            pl,
            light,
            1.0,
        );
    }
}

// ---------------------------------------------------------------- weapons

fn draw_blade(layer: &mut Layer, cam: &Cam, f: &Figure, st: &Style, pose: &Pose, blade: Blade) {
    if blade == Blade::None {
        return;
    }
    let g = f.prop.girth * f.prop.scale;
    let light = v2(f.facing * 0.42, -0.91).norm();
    // Rotate the forearm direction by the pose's blade angle.
    let base_deg = f.hand_dir.x.atan2(f.hand_dir.y).to_degrees() * f.facing;
    let d = dir_down(base_deg + pose.sword);
    let dir = v2(d.x * f.facing, d.y);
    let per = dir.perp();
    let hand = f.hand[0];
    let (len, wide, curve, col) = match blade {
        Blade::Sword => (14.5, 0.78, 0.0, st.metal),
        Blade::Scimitar => (15.5, 1.05, 0.20, rgb(222, 212, 176)),
        Blade::Dagger => (6.4, 0.70, 0.0, st.metal),
        Blade::Wand => (9.0, 1.0, 0.0, rgb(104, 72, 44)),
        Blade::None => return,
    };
    let len = len * f.prop.scale;
    let grip_a = hand.sub(dir.mul(2.6 * g));
    let tip = hand.add(dir.mul(len)).add(per.mul(curve * len));
    let mid = hand.add(dir.mul(len * 0.5)).add(per.mul(curve * len * 0.34));

    // Grip and pommel.
    fill_capsule_lit(
        layer,
        cam.p(grip_a),
        cam.p(hand.add(dir.mul(0.6 * g))),
        cam.l(0.72 * g),
        cam.l(0.76 * g),
        rgb(96, 62, 36),
        light,
        1.0,
    );
    fill_ellipse_lit(
        layer,
        cam.p(grip_a.sub(dir.mul(0.35 * g))),
        cam.l(0.78 * g),
        cam.l(0.78 * g),
        rgb(186, 152, 78),
        light,
        1.0,
    );
    if blade == Blade::Wand {
        fill_capsule_lit(
            layer,
            cam.p(hand),
            cam.p(tip),
            cam.l(0.8 * g),
            cam.l(0.65 * g),
            col,
            light,
            1.0,
        );
        fill_ellipse_lit(
            layer,
            cam.p(tip),
            cam.l(1.5 * g),
            cam.l(1.5 * g),
            rgb(255, 178, 64),
            light,
            1.0,
        );
        return;
    }
    // Cross-guard with quillons.
    let cg = per.mul(1.75 * g);
    fill_capsule_lit(
        layer,
        cam.p(hand.add(cg).add(dir.mul(0.5 * g))),
        cam.p(hand.sub(cg).add(dir.mul(0.5 * g))),
        cam.l(0.42 * g),
        cam.l(0.42 * g),
        rgb(190, 154, 78),
        light,
        1.0,
    );
    // Blade: a tapered diamond, brighter along the fuller.
    let root = hand.add(dir.mul(1.0 * g));
    let pts = [
        cam.p(root.add(per.mul(wide * g))),
        cam.p(mid.add(per.mul(wide * 0.82 * g))),
        cam.p(tip),
        cam.p(mid.sub(per.mul(wide * 0.82 * g))),
        cam.p(root.sub(per.mul(wide * g))),
    ];
    fill_poly_dir(layer, &pts, col.scale(1.2), col.scale(0.55), light, 1.0);
    fill_capsule(
        layer,
        cam.p(root.add(per.mul(wide * 0.2 * g))),
        cam.p(mid.lerp(tip, 0.55)),
        cam.l(0.32 * g),
        cam.l(0.12 * g),
        Rgb::WHITE,
        0.75,
    );
}

// ---------------------------------------------------------------- skeletons

fn draw_bones(
    layer: &mut Layer,
    cam: &Cam,
    f: &Figure,
    st: &Style,
    pose: &Pose,
    blade: Blade,
    light: V2,
) {
    let g = f.prop.girth * f.prop.scale;
    let bone = rgb(232, 226, 204);
    for &(i, far) in &[(1usize, true), (0usize, false)] {
        let c = if far { recede(bone) } else { bone };
        // Femur and tibia, thin with knobbed ends.
        fill_capsule_lit(
            layer,
            cam.p(f.hip),
            cam.p(f.knee[i]),
            cam.l(1.35 * g),
            cam.l(1.0 * g),
            c,
            light,
            1.0,
        );
        fill_ellipse_lit(layer, cam.p(f.knee[i]), cam.l(1.5 * g), cam.l(1.4 * g), c, light, 1.0);
        fill_capsule_lit(
            layer,
            cam.p(f.knee[i]),
            cam.p(f.ankle[i]),
            cam.l(1.05 * g),
            cam.l(0.8 * g),
            c,
            light,
            1.0,
        );
        fill_capsule_lit(
            layer,
            cam.p(f.ankle[i]),
            cam.p(f.toe[i]),
            cam.l(0.95 * g),
            cam.l(0.7 * g),
            c.scale(0.9),
            light,
            1.0,
        );
        fill_capsule_lit(
            layer,
            cam.p(f.shoulder),
            cam.p(f.elbow[i]),
            cam.l(1.1 * g),
            cam.l(0.85 * g),
            c,
            light,
            1.0,
        );
        fill_capsule_lit(
            layer,
            cam.p(f.elbow[i]),
            cam.p(f.hand[i]),
            cam.l(0.85 * g),
            cam.l(0.7 * g),
            c,
            light,
            1.0,
        );
        // Fingers.
        let d = f.hand[i].sub(f.elbow[i]).norm();
        for k in -1..=1 {
            let o = d.perp().mul(k as f32 * 0.7 * g);
            fill_capsule(
                layer,
                cam.p(f.hand[i].add(o)),
                cam.p(f.hand[i].add(o).add(d.mul(1.5 * g))),
                cam.l(0.34 * g),
                cam.l(0.24 * g),
                c.scale(0.95),
                1.0,
            );
        }
    }
    // Spine, pelvis and ribs.
    fill_capsule_lit(
        layer,
        cam.p(f.hip),
        cam.p(f.shoulder),
        cam.l(1.15 * g),
        cam.l(1.0 * g),
        bone.scale(0.78),
        light,
        1.0,
    );
    fill_ellipse_lit(layer, cam.p(f.hip), cam.l(2.9 * g), cam.l(1.9 * g), bone, light, 1.0);
    fill_ellipse(
        layer,
        cam.p(f.hip),
        cam.l(1.5 * g),
        cam.l(0.9 * g),
        rgb(38, 30, 34),
        0.8,
    );
    let down = f.up.mul(-1.0);
    for k in 0..4 {
        let t = 0.18 + k as f32 * 0.16;
        let c = f.shoulder.add(down.mul(f.prop.torso * t * f.prop.scale));
        let w = (2.85 - k as f32 * 0.22) * g;
        fill_capsule_lit(
            layer,
            cam.p(c.add(f.fwd.mul(w))),
            cam.p(c.sub(f.fwd.mul(w * 0.75))),
            cam.l(0.55 * g),
            cam.l(0.5 * g),
            bone,
            light,
            1.0,
        );
    }
    // Clavicle.
    fill_capsule(
        layer,
        cam.p(f.shoulder.add(f.fwd.mul(2.4 * g))),
        cam.p(f.shoulder.sub(f.fwd.mul(1.8 * g))),
        cam.l(0.6 * g),
        cam.l(0.55 * g),
        bone.scale(1.05),
        1.0,
    );

    // Skull: cranium, cheekbone, socket, jaw with teeth.
    let hc = cam.p(f.head);
    let hr = cam.l(f.head_r);
    let fw = f.facing;
    fill_ellipse_lit(layer, hc, hr * 1.04, hr * 1.02, bone, light, 1.0);
    fill_ellipse_lit(
        layer,
        v2(hc.x + fw * hr * 0.46, hc.y + hr * 0.30),
        hr * 0.62,
        hr * 0.50,
        bone.scale(0.95),
        light,
        1.0,
    );
    fill_ellipse(
        layer,
        v2(hc.x + fw * hr * 0.40, hc.y - hr * 0.10),
        hr * 0.34,
        hr * 0.30,
        rgb(22, 18, 22),
        1.0,
    );
    fill_ellipse(
        layer,
        v2(hc.x + fw * hr * 0.90, hc.y + hr * 0.24),
        hr * 0.17,
        hr * 0.22,
        rgb(22, 18, 22),
        0.9,
    );
    fill_capsule_lit(
        layer,
        v2(hc.x + fw * hr * 0.10, hc.y + hr * 0.80),
        v2(hc.x + fw * hr * 0.92, hc.y + hr * 0.66),
        hr * 0.20,
        hr * 0.17,
        bone.scale(0.88),
        light,
        1.0,
    );
    for k in 0..4 {
        let t = k as f32 / 3.0;
        fill_circle(
            layer,
            v2(
                hc.x + fw * hr * lerp(0.20, 0.86, t),
                hc.y + hr * lerp(0.62, 0.50, t),
            ),
            hr * 0.075,
            rgb(30, 24, 26),
            0.85,
        );
    }
    draw_blade(layer, cam, f, st, pose, blade);
}

// ---------------------------------------------------------------- bounds

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
