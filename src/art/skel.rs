//! Articulated figure rendering.
//!
//! Rather than storing a few dozen hand-drawn sprite sheets, every character in
//! the game is a small skeleton — hip, torso, head, two arms, two legs — that is
//! *drawn* each frame rather than assembled from primitives. Each bone carries an
//! authored [silhouette profile](crate::art::shape::Profile) so an arm swells at
//! the deltoid and pinches at the elbow, each is cel-shaded with a hard-edged
//! core shadow and a thin lit rim, and heads, hands, hair and cloth are authored
//! polygons rather than ellipses. Where two forms of the same colour overlap —
//! the near arm against the chest, the sash against the belly — a drawn contour
//! separates them.
//!
//! That combination is what survives being squeezed down to a dozen terminal
//! pixels: the silhouette stays deliberate and the value structure (bright sash,
//! mid skin, dark hair and boots) still reads.
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

use crate::art::shape::{self, Limb};
use crate::gfx::canvas::Cam;
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::layer::Layer;
use crate::gfx::target::{fill_capsule, fill_circle};
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
        neck: 1.15,
        head_r: 2.10,
        upper: 5.3,
        fore: 4.8,
        hand: 1.5,
        chest: 2.60,
        waist: 2.05,
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
        skin: rgb(226, 172, 124),
        skin_dk: rgb(150, 96, 62),
        cloth: rgb(238, 236, 226),
        cloth_dk: rgb(150, 152, 166),
        sash: rgb(190, 48, 48),
        sash_dk: rgb(104, 20, 26),
        hair: rgb(70, 44, 32),
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
///
/// This has to land *below* the core shadow of the torso, or the depth order
/// reads backwards and the far arm appears to float in front of the chest. That
/// is the single easiest way to make a figure look wrong.
fn recede(c: Rgb) -> Rgb {
    c.scale(0.48).desaturate(0.26).lerp(rgb(26, 24, 42), 0.18)
}

// ---------------------------------------------------------------- device frame

/// The figure projected into device pixels once, at the top of the frame.
///
/// Everything below draws in these coordinates: profiles are scaled by a girth
/// that is already in pixels, and authored polygons are placed in local frames
/// built from `fwd`/`up`. Working this way means a figure four pixels wide and one
/// forty pixels wide go through exactly the same code, and no drawing routine
/// needs to know the camera exists.
struct Dev {
    hip: V2,
    knee: [V2; 2],
    ankle: [V2; 2],
    toe: [V2; 2],
    shoulder: V2,
    elbow: [V2; 2],
    hand: [V2; 2],
    neck: V2,
    head: V2,
    /// Head radius, px.
    hr: f32,
    /// One art pixel of limb girth, px.
    g: f32,
    /// One art pixel of length, px.
    u: f32,
    /// Chest half-depth, px.
    chest: f32,
    /// Hip-to-shoulder distance, px.
    tl: f32,
    up: V2,
    fwd: V2,
    facing: f32,
    hand_dir: V2,
    heavy: bool,
}

fn dev(cam: &Cam, f: &Figure) -> Dev {
    let s = f.prop.scale;
    Dev {
        hip: cam.p(f.hip),
        knee: [cam.p(f.knee[0]), cam.p(f.knee[1])],
        ankle: [cam.p(f.ankle[0]), cam.p(f.ankle[1])],
        toe: [cam.p(f.toe[0]), cam.p(f.toe[1])],
        shoulder: cam.p(f.shoulder),
        elbow: [cam.p(f.elbow[0]), cam.p(f.elbow[1])],
        hand: [cam.p(f.hand[0]), cam.p(f.hand[1])],
        neck: cam.p(f.neck),
        head: cam.p(f.head),
        hr: cam.l(f.head_r),
        g: cam.l(f.prop.girth * s),
        u: cam.l(s),
        chest: cam.l(f.prop.chest * f.prop.girth * s),
        tl: cam.p(f.shoulder).sub(cam.p(f.hip)).len().max(1.0),
        up: f.up,
        fwd: f.fwd,
        facing: f.facing,
        hand_dir: f.hand_dir,
        heavy: f.prop.girth > 1.20,
    }
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
    // Key light from above and a little in front, so the face and the chest catch
    // it whichever way the character faces.
    let light = v2(f.facing * 0.40, -0.92).norm();
    let d = dev(cam, f);

    if st.bones {
        draw_bones(layer, &d, st, pose, blade, light);
        return;
    }

    // --- far side ------------------------------------------------------
    draw_leg(layer, &d, st, 1, light, true);
    draw_arm(layer, &d, st, 1, light, true);
    if let Some(sc) = st.shield {
        draw_shield(layer, &d, st, sc, light);
    }
    draw_scarf(layer, &d, st, pose);
    draw_sash_tail(layer, &d, st, pose);

    // --- near leg, torso, then the garment over the thighs --------------
    draw_leg(layer, &d, st, 0, light, false);
    draw_torso(layer, &d, st, light);
    if st.robe > 0.01 {
        draw_tunic(layer, &d, st, pose, light);
    }
    draw_sash(layer, &d, st, light);

    // --- head ----------------------------------------------------------
    draw_head(layer, &d, st, light);

    // --- near arm and weapon -------------------------------------------
    draw_arm(layer, &d, st, 0, light, false);
    draw_blade(layer, &d, st, pose, blade, light);
}

// ---------------------------------------------------------------- legs

fn draw_leg(layer: &mut Layer, d: &Dev, st: &Style, i: usize, light: V2, far: bool) {
    let g = d.g;
    let tint = |c: Rgb| if far { recede(c) } else { c };
    let trouser = tint(st.trouser);
    let boot = tint(st.boot);

    let (hip, knee, ankle, toe) = (d.hip, d.knee[i], d.ankle[i], d.toe[i]);
    let loose = st.baggy > 0.15;
    let (pt, pc) = if loose {
        (shape::THIGH_BAGGY, shape::CALF_BAGGY)
    } else {
        (shape::THIGH, shape::CALF)
    };
    let thigh = Limb::new(hip, knee, shape::front_of(hip, knee, d.facing), pt, g).steps(9);
    let calf = Limb::new(knee, ankle, shape::front_of(knee, ankle, d.facing), pc, g).steps(9);

    // The near leg gets a drawn edge so it separates from the far one even where
    // the two overlap and the recede tint is not enough on its own.
    if !far {
        let e = 0.34 * g;
        let dk = st.trouser.scale(0.26);
        thigh.edge(layer, e, dk, 1.0);
        calf.edge(layer, e, dk, 1.0);
    }
    thigh.draw(layer, trouser, light);
    calf.draw(layer, trouser, light);

    if loose {
        // Two folds running down the outside of the cloth, and the gather where it
        // is stuffed into the boot. Without them a baggy trouser is a sack.
        for (t0, t1, a) in [(0.18f32, 0.72f32, 0.34f32), (0.42, 0.92, 0.22)] {
            shape::contour(
                layer,
                thigh.dark_edge(t0, light).lerp(thigh.lit_edge(t0, light), 0.34),
                thigh.dark_edge(t1, light).lerp(thigh.lit_edge(t1, light), 0.42),
                0.20 * g,
                st.trouser.scale(0.52),
                a,
            );
        }
        let cf = shape::front_of(knee, ankle, d.facing);
        let cuff = ankle.lerp(knee, 0.12);
        shape::contour(
            layer,
            cuff.add(cf.mul(1.05 * g)),
            cuff.sub(cf.mul(1.05 * g)),
            0.34 * g,
            trouser.scale(0.72),
            1.0,
        );
    } else {
        // A crease behind the knee is what turns two segments into a joint.
        let kf = shape::front_of(hip, knee, d.facing);
        shape::contour(
            layer,
            knee.sub(kf.mul(1.20 * g)),
            knee.sub(kf.mul(0.30 * g)),
            0.20 * g,
            trouser.scale(0.46),
            0.8,
        );
    }

    // --- boot: heel, sole and toe as one authored shape ------------------
    let sole = toe.sub(ankle).norm();
    // Pick the perpendicular that points up the screen, whichever way the foot
    // faces — otherwise a left-facing character wears its boots upside down.
    let up = {
        let p = sole.perp();
        if p.y <= 0.0 {
            p
        } else {
            p.mul(-1.0)
        }
    };
    let heel = ankle.sub(sole.mul(1.15 * g));
    let pts = [
        heel.add(up.mul(1.95 * g)),
        ankle.add(up.mul(2.20 * g)),
        toe.add(up.mul(0.95 * g)),
        toe.add(sole.mul(0.30 * g)).sub(up.mul(0.26 * g)),
        heel.sub(up.mul(0.30 * g)),
        heel.sub(sole.mul(0.25 * g)).add(up.mul(0.70 * g)),
    ];
    shape::cel_poly(layer, &pts, boot, light, 0.40);
    // Sole: a dark strip along the bottom.
    shape::contour(
        layer,
        heel.sub(up.mul(0.16 * g)),
        toe.sub(up.mul(0.18 * g)),
        0.34 * g,
        boot.scale(0.40),
        0.95,
    );
    // Cuff, turned over at the top.
    let cf = sole.mul(0.85 * g);
    shape::contour(
        layer,
        ankle.add(up.mul(2.00 * g)).sub(cf),
        ankle.add(up.mul(1.80 * g)).add(cf),
        0.52 * g,
        boot.scale(1.28),
        1.0,
    );
}

// ---------------------------------------------------------------- arms

fn draw_arm(layer: &mut Layer, d: &Dev, st: &Style, i: usize, light: V2, far: bool) {
    let g = d.g;
    let tint = |c: Rgb| if far { recede(c) } else { c };
    let skin = tint(st.skin);
    let sleeved = st.vest.is_some() || !st.bare_chest;
    let sleeve_col = tint(st.vest.unwrap_or(st.cloth));

    // The two shoulder joints are not in the same place: one is nearer the viewer
    // than the other, and the body between them is thick. Offsetting each arm
    // along the depth axis is what pulls the far arm clear of the back and drops
    // the near arm in front of the chest — without it a profile figure is a slab
    // with one arm and the pose reads as a paper cut-out.
    let off = d.fwd.mul(if far { -1.85 } else { 0.55 } * g);
    let (sh, el, hd) = (
        d.shoulder.add(off),
        d.elbow[i].add(off),
        d.hand[i].add(off),
    );
    let uf = shape::front_of(sh, el, d.facing);
    let ff = shape::front_of(el, hd, d.facing);
    let upper = Limb::new(sh, el, uf, shape::UPPER_ARM, g).steps(7);
    let fore = Limb::new(el, hd, ff, shape::FOREARM, g).steps(7);

    // The near arm crosses the chest, and chest and arm are the same colour on a
    // bare-chested figure. The drawn edge is the only thing that keeps them apart.
    if !far {
        let e = 0.30 * g;
        let dk = if st.bare_chest {
            st.skin_dk.scale(0.42)
        } else {
            st.cloth_dk.scale(0.36)
        };
        upper.edge(layer, e, dk, 1.0);
        fore.edge(layer, e, dk, 1.0);
    }

    upper.draw(layer, skin, light);
    fore.draw(layer, skin, light);
    // Crease inside the elbow.
    shape::contour(
        layer,
        el.add(uf.mul(0.75 * g)),
        el.add(ff.mul(0.75 * g)),
        0.16 * g,
        st.skin_dk.scale(0.80),
        if far { 0.25 } else { 0.5 },
    );

    if sleeved {
        let end = sh.lerp(el, 0.46);
        let sl = Limb::new(sh, end, uf, shape::SLEEVE, g).steps(4);
        sl.draw(layer, sleeve_col, light);
        // Hem, and the shadow the sleeve drops on the arm below it.
        shape::contour(
            layer,
            end.add(uf.mul(1.45 * g)),
            end.sub(uf.mul(1.45 * g)),
            0.24 * g,
            sleeve_col.scale(0.66),
            1.0,
        );
        shape::contour(
            layer,
            end.add(uf.mul(1.00 * g)),
            end.sub(uf.mul(1.00 * g)),
            0.22 * g,
            skin.scale(0.60),
            0.45,
        );
    }

    // --- hand: a mitt with a thumb, not a ball --------------------------
    let along = hd.sub(el).norm();
    let hand = shape::frame(
        hd,
        along.mul(g),
        ff.mul(g),
        &[
            (-0.30, 0.74),
            (0.55, 0.86),
            (1.15, 0.60),
            (1.42, 0.10),
            (1.30, -0.40),
            (0.55, -0.78),
            (-0.30, -0.70),
        ],
    );
    shape::cel_poly(layer, &hand, skin, light, 0.42);
    // Knuckle line, and the wrist.
    shape::contour(
        layer,
        hd.add(along.mul(0.72 * g)).add(ff.mul(0.68 * g)),
        hd.add(along.mul(1.10 * g)).sub(ff.mul(0.34 * g)),
        0.13 * g,
        st.skin_dk.scale(0.78),
        if far { 0.3 } else { 0.6 },
    );
}

fn draw_shield(layer: &mut Layer, d: &Dev, st: &Style, face: Rgb, light: V2) {
    let g = d.g;
    let c = d.hand[1];
    let ex = d.fwd.mul(g);
    let ey = d.up.mul(-g);
    // Seen almost edge-on: a narrow oval with a rim and a boss.
    let disc = shape::frame(
        c,
        ex,
        ey,
        &[
            (0.0, -4.6),
            (2.4, -3.4),
            (3.1, 0.0),
            (2.4, 3.4),
            (0.0, 4.6),
            (-2.4, 3.4),
            (-3.1, 0.0),
            (-2.4, -3.4),
        ],
    );
    shape::cel_poly(layer, &disc, recede(face), light, 0.44);
    shape::contour(
        layer,
        c.add(ey.mul(-4.2)),
        c.add(ey.mul(4.2)),
        0.34 * g,
        recede(face).scale(1.35),
        0.8,
    );
    fill_circle(layer, c, 1.5 * g, recede(st.metal), 1.0);
    fill_circle(layer, c.sub(ey.mul(0.4)), 0.7 * g, recede(st.metal).scale(1.3), 1.0);
}

// ---------------------------------------------------------------- torso

fn draw_torso(layer: &mut Layer, d: &Dev, st: &Style, light: V2) {
    let g = d.g;
    let (sh, hip) = (d.shoulder, d.hip);
    let down = hip.sub(sh).norm();
    let front = shape::front_of(sh, hip, d.facing);
    let prof = if st.robe > 0.7 {
        shape::TORSO_ROBE
    } else if d.heavy {
        shape::TORSO_HEAVY
    } else {
        shape::TORSO
    };
    let body = if st.bare_chest { st.skin } else { st.cloth };
    Limb::new(sh, hip, front, prof, d.chest)
        .steps(10)
        .shade(0.46)
        .draw(layer, body, light);

    // A local frame for the landmarks: +x out through the chest, +y down the
    // spine, both normalised so the numbers read as fractions of the body.
    let ex = front.mul(d.chest);
    let ey = down.mul(d.tl);
    let at = |x: f32, y: f32| sh.add(ex.mul(x)).add(ey.mul(y));

    if st.bare_chest {
        // Pectoral: one plate with a hard lower edge, and nothing else. A bare
        // torso seven pixels wide has room for exactly one landmark — sternum
        // grooves and abdominals at this size are scratches, not anatomy, and
        // they cost the chest the broad clean value that makes it read.
        let pec = shape::frame(
            sh,
            ex,
            ey,
            &[
                (-0.16, 0.04),
                (0.62, 0.06),
                (0.98, 0.18),
                (1.00, 0.34),
                (0.66, 0.44),
                (-0.10, 0.40),
                (-0.42, 0.22),
            ],
        );
        shape::cel_poly(layer, &pec, st.skin.scale(1.10), light, 0.30);
        shape::contour(
            layer,
            at(0.96, 0.34),
            at(-0.06, 0.42),
            0.28 * g,
            st.skin_dk.scale(0.62),
            0.9,
        );
        // The crease where the belly folds over the sash.
        shape::contour(
            layer,
            at(0.86, 0.62),
            at(-0.10, 0.66),
            0.20 * g,
            st.skin_dk.scale(0.76),
            0.42,
        );
    } else {
        // A shoulder seam across the top of the chest, then the V of the collar.
        shape::contour(
            layer,
            at(0.90, 0.02),
            at(-0.86, 0.04),
            0.30 * g,
            st.cloth.scale(1.16),
            0.8,
        );
        let collar = shape::frame(
            sh,
            ex,
            ey,
            &[(1.05, -0.02), (0.98, 0.22), (0.42, 0.10), (-0.55, 0.00), (-0.55, -0.06)],
        );
        shape::flat(layer, &collar, st.skin_dk.scale(0.86), 0.9);
        shape::contour(
            layer,
            at(1.02, 0.20),
            at(0.30, 0.02),
            0.20 * g,
            st.cloth_dk,
            0.9,
        );
        // Folds where the garment gathers into the sash.
        for (x, y) in [(0.72f32, 0.42f32), (0.34, 0.48)] {
            shape::contour(layer, at(x, y), at(x - 0.2, y + 0.26), 0.18 * g, st.cloth_dk, 0.34);
        }
    }

    // Open waistcoat: covers the back and the sides, leaves the chest bare. In a
    // side view that reads as the garment's front edge running down the body.
    if let Some(v) = st.vest {
        let coat = shape::frame(
            sh,
            ex,
            ey,
            &[
                (0.30, -0.05),
                (0.36, 0.30),
                (0.26, 0.72),
                (-0.74, 0.80),
                (-1.06, 0.34),
                (-1.02, -0.04),
            ],
        );
        shape::cel_poly(layer, &coat, v, light, 0.46);
        shape::contour(layer, at(0.32, -0.03), at(0.28, 0.74), 0.24 * g, v.scale(1.35), 0.95);
        shape::contour(layer, at(0.30, 0.72), at(-0.72, 0.80), 0.22 * g, v.scale(0.62), 0.9);
    }
}

fn draw_tunic(layer: &mut Layer, d: &Dev, st: &Style, pose: &Pose, light: V2) {
    let g = d.g;
    let down = d.up.mul(-1.0);
    let side = d.fwd;
    let hem_len = lerp(0.0, 14.6, clampf(st.robe, 0.0, 1.0)) * d.u;
    let top = d.hip.add(d.up.mul(d.tl * 0.34));
    let w_top = d.chest * 0.78;
    let w_bot = lerp(2.6, 5.4, st.robe) * g;
    // The skirt swings against the legs and lifts on the leading side.
    let swing = (pose.leg[0][0] - pose.leg[1][0]) * 0.030;
    let hem = d.hip.add(down.mul(hem_len));
    let off = side.mul(swing * g);
    let lift = swing.abs() * 0.09 * g;

    let hp = |t: f32, drop: f32| -> V2 {
        hem.add(side.mul(lerp(w_bot, -w_bot, t)))
            .add(off)
            .add(down.mul(drop))
    };
    let pts = [
        top.add(side.mul(w_top)),
        d.hip.add(side.mul(w_top * 1.10)),
        hp(0.0, -lift),
        hp(0.28, 0.45 * g),
        hp(0.56, -0.15 * g),
        hp(0.82, 0.36 * g),
        hp(1.0, -lift * 0.6),
        d.hip.sub(side.mul(w_top * 1.10)),
        top.sub(side.mul(w_top)),
    ];
    shape::cel_poly(layer, &pts, st.cloth, light, 0.44);

    // Three folds following the drape, each a hard-edged wedge rather than a
    // smear: cloth creases, it does not blur.
    for (t, w) in [(0.24f32, 0.62f32), (0.50, 0.80), (0.76, 0.55)] {
        let a = d.hip.add(side.mul(lerp(w_top, -w_top, t) * 0.8));
        let b = hp(t, 0.25 * g);
        let fold = [
            a.add(side.mul(w * 0.30 * g)),
            b.add(side.mul(w * 0.75 * g)),
            b.sub(side.mul(w * 0.35 * g)),
            a.sub(side.mul(w * 0.20 * g)),
        ];
        shape::flat(layer, &fold, st.cloth_dk, 0.34);
    }
    // A darker band along the wavy hem.
    for i in 0..6 {
        let t0 = i as f32 / 6.0;
        let t1 = (i + 1) as f32 / 6.0;
        let d0 = 0.45 * g * (t0 * 3.6).sin().abs();
        let d1 = 0.45 * g * (t1 * 3.6).sin().abs();
        shape::contour(layer, hp(t0, d0), hp(t1, d1), 0.36 * g, st.cloth_dk.scale(0.88), 0.8);
    }
}

fn draw_sash(layer: &mut Layer, d: &Dev, st: &Style, light: V2) {
    let g = d.g;
    let waist = d.hip.lerp(d.shoulder, 0.20);
    let front = shape::front_of(d.shoulder, d.hip, d.facing);
    let down = d.hip.sub(d.shoulder).norm();
    let w = d.chest * 0.95;
    // Wound twice, so it has a top edge, a bottom edge and a visible overlap.
    let band = shape::frame(
        waist,
        front.mul(w),
        down.mul(g),
        &[
            (0.10, -1.35),
            (1.08, -1.00),
            (1.16, 0.80),
            (0.10, 1.15),
            (-1.06, 0.85),
            (-1.10, -1.05),
        ],
    );
    shape::cel_poly(layer, &band, st.sash, light, 0.42);
    shape::contour(
        layer,
        waist.add(front.mul(w * 1.05)).sub(down.mul(0.05 * g)),
        waist.sub(front.mul(w * 0.95)).add(down.mul(0.10 * g)),
        0.14 * g,
        st.sash_dk,
        0.75,
    );
    // Knot on the near hip.
    let knot_at = waist.add(front.mul(w * 0.92)).add(down.mul(0.25 * g));
    let knot = shape::frame(
        knot_at,
        front.mul(g),
        down.mul(g),
        &[
            (0.25, -1.00),
            (1.05, -0.30),
            (0.90, 0.80),
            (-0.15, 1.00),
            (-0.80, 0.25),
            (-0.50, -0.80),
        ],
    );
    shape::cel_poly(layer, &knot, st.sash.scale(1.10), light, 0.38);

    if st.belt {
        let b = waist.add(down.mul(1.5 * g));
        let across = front.mul(w);
        shape::contour(
            layer,
            b.add(across),
            b.sub(across),
            0.78 * g,
            st.boot.scale(0.70),
            1.0,
        );
        for k in -1..=1 {
            fill_circle(
                layer,
                b.add(front.mul(k as f32 * 1.5 * g)),
                0.40 * g,
                st.metal.scale(0.85),
                1.0,
            );
        }
    }
}

fn draw_sash_tail(layer: &mut Layer, d: &Dev, st: &Style, pose: &Pose) {
    let g = d.g;
    let waist = d.hip.lerp(d.shoulder, 0.20);
    let back = d.fwd.mul(-1.0);
    let a = waist.add(back.mul(1.3 * g));
    let mid = a.add(back.mul(1.5 * g * pose.tail)).add(d.up.mul(-2.4 * g));
    let end = mid
        .add(back.mul(1.0 * g * pose.tail))
        .add(d.up.mul(-2.6 * g - pose.tail * 0.6 * g));
    fill_capsule(layer, a, mid, 0.72 * g, 0.50 * g, st.sash_dk, 1.0);
    fill_capsule(layer, mid, end, 0.50 * g, 0.18 * g, st.sash_dk.scale(0.85), 1.0);
}

/// A long ribbon streaming back over the shoulder. The shadow prince wears one,
/// and it is most of what makes him read as an apparition rather than a recolour.
fn draw_scarf(layer: &mut Layer, d: &Dev, st: &Style, pose: &Pose) {
    let Some(col) = st.scarf else { return };
    // Distinctly brighter than the body, or it disappears into it.
    let col = col.scale(1.25);
    let g = d.g;
    let back = d.fwd.mul(-1.0);
    let mut p = d.shoulder.add(back.mul(1.2 * g)).add(d.up.mul(0.4 * g));
    let wave = pose.leg[0][0] * 0.02;
    for k in 0..4 {
        let t = k as f32;
        let q = p
            .add(back.mul((3.0 + t * 0.6) * g))
            .add(d.up.mul((0.9 + (t * 1.7 + wave).sin() * 2.2) * g));
        fill_capsule(
            layer,
            p,
            q,
            (1.5 - t * 0.3) * g,
            (1.2 - t * 0.28) * g,
            col.scale(1.0 - t * 0.12),
            1.0,
        );
        p = q;
    }
}

// ---------------------------------------------------------------- head

fn draw_head(layer: &mut Layer, d: &Dev, st: &Style, light: V2) {
    let (hc, hr, fw) = (d.head, d.hr, d.facing);
    // Local frame for every authored head shape: +x forward, +y down, one unit of
    // each is one head radius.
    let ex = v2(fw * hr, 0.0);
    let ey = v2(0.0, hr);
    let p = |x: f32, y: f32| hc.add(ex.mul(x)).add(ey.mul(y));

    // Neck. It runs from inside the jaw down to well below the shoulder line, so
    // the head sits on the shoulders rather than balancing on a stalk.
    let nb = d.shoulder.add(d.up.mul(-1.6 * d.g));
    let nt = hc.add(d.up.mul(-0.35 * hr));
    Limb::new(nt, nb, shape::front_of(nt, nb, fw), shape::NECK, d.g)
        .steps(4)
        .draw(layer, st.skin.scale(0.84), light);

    // The head: forehead, brow, nose, lips, chin, jaw, back of the skull.
    let head = shape::frame(hc, ex, ey, shape::HEAD);
    shape::cel_poly(layer, &head, st.skin, light, 0.44);

    // Ear, tucked at the back of the jaw.
    let ear = shape::frame(
        hc,
        ex,
        ey,
        &[(-0.50, 0.02), (-0.28, -0.02), (-0.20, 0.26), (-0.34, 0.46), (-0.56, 0.38)],
    );
    shape::cel_poly(layer, &ear, st.skin.scale(0.94), light, 0.50);
    shape::contour(layer, p(-0.40, 0.10), p(-0.32, 0.32), hr * 0.07, st.skin_dk.scale(0.7), 0.7);

    match st.head_wrap {
        None => draw_hair(layer, d, st, light),
        Some(wrap) => draw_wrap(layer, d, st, wrap, light),
    }
    if let Some(band) = st.band {
        // Tied across the brow, the knot-ends trailing at the back.
        let strip = shape::frame(
            hc,
            ex,
            ey,
            &[(0.88, -0.50), (0.90, -0.22), (-0.10, -0.34), (-1.06, -0.40), (-1.10, -0.66), (-0.10, -0.60)],
        );
        shape::cel_poly(layer, &strip, band, light, 0.42);
        let tail = shape::frame(
            hc,
            ex,
            ey,
            &[(-0.94, -0.62), (-1.52, -0.16), (-1.44, 0.42), (-1.24, 0.30), (-1.30, -0.10), (-0.90, -0.40)],
        );
        shape::cel_poly(layer, &tail, band.scale(0.84), light, 0.46);
    }

    // ---- face ----------------------------------------------------------
    // A face is two planes: the front, which catches the light, and the side,
    // which does not. The edge between them runs from the temple down past the
    // corner of the mouth to the chin, and drawing that edge is what gives a head
    // structure at a size where no amount of modelling would survive.
    let side = shape::frame(
        hc,
        ex,
        ey,
        &[
            (0.34, -0.36),
            (0.54, 0.08),
            (0.48, 0.56),
            (0.16, 0.90),
            (-0.44, 0.72),
            (-0.86, 0.30),
            (-0.94, -0.32),
        ],
    );
    shape::flat(layer, &side, st.skin_dk.scale(0.96), 0.34);

    // The eye socket: a wedge of shadow under the brow ridge.
    let socket = shape::frame(
        hc,
        ex,
        ey,
        &[(0.30, -0.32), (0.82, -0.24), (0.84, 0.02), (0.56, 0.10), (0.32, -0.04)],
    );
    shape::flat(layer, &socket, st.skin_dk.scale(0.70), 0.62);
    // Eyebrow, in hair colour: at this scale it does more for the read than any
    // amount of modelling.
    shape::contour(layer, p(0.30, -0.38), p(0.80, -0.28), hr * 0.10, st.hair, 0.95);
    // Eye: small, dark, with one catch of light. A big white sclera at this size
    // reads as a googly cartoon eye, not as someone looking at something.
    shape::flat(
        layer,
        &shape::frame(hc, ex, ey, &[(0.50, -0.08), (0.66, -0.15), (0.80, -0.05), (0.66, 0.05), (0.53, 0.01)]),
        rgb(228, 220, 206),
        0.92,
    );
    shape::flat(
        layer,
        &shape::frame(hc, ex, ey, &[(0.60, -0.12), (0.75, -0.05), (0.72, 0.04), (0.59, 0.00)]),
        st.outline,
        1.0,
    );
    // The bridge of the nose catches the light; the underside does not.
    shape::contour(layer, p(0.84, -0.06), p(1.06, 0.22), hr * 0.11, st.skin.scale(1.24), 0.7);
    shape::flat(
        layer,
        &shape::frame(hc, ex, ey, &[(0.78, 0.14), (1.08, 0.30), (0.90, 0.36), (0.78, 0.28)]),
        st.skin_dk.scale(0.74),
        0.6,
    );
    fill_circle(layer, p(0.90, 0.31), hr * 0.065, st.skin_dk.scale(0.50), 0.9);
    // The fold from the nostril to the corner of the mouth, the mouth itself, and
    // the shadow under the lower lip.
    shape::contour(layer, p(0.78, 0.36), p(0.70, 0.54), hr * 0.055, st.skin_dk.scale(0.84), 0.30);
    shape::contour(layer, p(0.64, 0.57), p(0.88, 0.55), hr * 0.07, st.skin_dk.scale(0.44), 0.90);
    shape::contour(layer, p(0.68, 0.71), p(0.86, 0.69), hr * 0.055, st.skin_dk.scale(0.70), 0.45);
    // The jaw, and the shadow the head throws down onto the neck.
    shape::contour(layer, p(-0.34, 0.78), p(0.62, 0.92), hr * 0.13, st.skin_dk.scale(0.60), 0.55);
}

fn draw_hair(layer: &mut Layer, d: &Dev, st: &Style, light: V2) {
    let (hc, hr, fw) = (d.head, d.hr, d.facing);
    let ex = v2(fw * hr, 0.0);
    let ey = v2(0.0, hr);
    // A swept mass: low on the brow at the front, standing proud over the crown,
    // falling to a short tail at the nape. One authored polygon, so the
    // silhouette is a decision rather than an accident.
    let cap = shape::frame(hc, ex, ey, shape::HAIR);
    shape::cel_poly(layer, &cap, st.hair, light, 0.46);
    // Locks following the sweep. Without them the mass reads as a helmet.
    for &((ax, ay), (bx, by), w) in &[
        ((0.58f32, -0.86f32), (-0.28f32, -1.20f32), 0.13f32),
        ((0.04, -1.10), (-0.80, -0.96), 0.11),
        ((-0.68, -0.84), (-1.14, -0.20), 0.10),
    ] {
        shape::contour(
            layer,
            hc.add(ex.mul(ax)).add(ey.mul(ay)),
            hc.add(ex.mul(bx)).add(ey.mul(by)),
            hr * w,
            st.hair.scale(1.34),
            0.75,
        );
    }
    // A lock falling across the temple, over the brow.
    let lock = shape::frame(
        hc,
        ex,
        ey,
        &[(0.16, -1.02), (0.60, -0.86), (0.96, -0.48), (0.78, -0.40), (0.48, -0.70), (0.12, -0.86)],
    );
    shape::cel_poly(layer, &lock, st.hair.scale(1.16), light, 0.50);
}

fn draw_wrap(layer: &mut Layer, d: &Dev, st: &Style, wrap: Rgb, light: V2) {
    let (hc, hr, fw) = (d.head, d.hr, d.facing);
    let ex = v2(fw * hr, 0.0);
    let ey = v2(0.0, hr);
    let p = |x: f32, y: f32| hc.add(ex.mul(x)).add(ey.mul(y));
    // Dome.
    let dome = shape::frame(
        hc,
        ex,
        ey,
        &[
            (0.94, -0.30),
            (0.86, -0.70),
            (0.48, -1.16),
            (-0.10, -1.36),
            (-0.70, -1.20),
            (-1.06, -0.78),
            (-1.18, -0.28),
            (-1.08, 0.00),
        ],
    );
    shape::cel_poly(layer, &dome, wrap, light, 0.44);
    // The wrapped band across the brow, with a visible overlap at the back.
    let band = shape::frame(
        hc,
        ex,
        ey,
        &[
            (0.98, -0.34),
            (0.94, -0.02),
            (0.10, 0.08),
            (-0.88, -0.06),
            (-1.14, -0.40),
            (-0.92, -0.62),
            (0.20, -0.58),
        ],
    );
    shape::cel_poly(layer, &band, wrap.scale(0.88), light, 0.42);
    shape::contour(layer, p(0.90, -0.20), p(-0.90, -0.28), hr * 0.07, wrap.scale(0.58), 0.7);
    // The end of the cloth, tucked down behind the ear.
    let tuck = shape::frame(
        hc,
        ex,
        ey,
        &[(-0.86, -0.62), (-1.20, -0.10), (-1.14, 0.34), (-0.92, 0.24), (-0.96, -0.14), (-0.72, -0.46)],
    );
    shape::cel_poly(layer, &tuck, wrap.scale(0.72), light, 0.46);

    // A plume means this is a helmet, so give it a nasal bar too.
    if let Some(pl) = st.plume {
        shape::contour(layer, p(0.72, -0.32), p(0.82, 0.48), hr * 0.16, st.metal.scale(0.9), 1.0);
        let feather = shape::frame(
            hc,
            ex,
            ey,
            &[(-0.05, -1.40), (-0.55, -2.10), (-1.35, -2.55), (-1.15, -2.15), (-0.60, -1.70), (-0.35, -1.30)],
        );
        shape::cel_poly(layer, &feather, pl, light, 0.44);
    }
}

// ---------------------------------------------------------------- weapons

fn draw_blade(layer: &mut Layer, d: &Dev, st: &Style, pose: &Pose, blade: Blade, light: V2) {
    if blade == Blade::None {
        return;
    }
    let g = d.g;
    // Rotate the forearm direction by the pose's blade angle.
    let base_deg = d.hand_dir.x.atan2(d.hand_dir.y).to_degrees() * d.facing;
    let dd = dir_down(base_deg + pose.sword);
    let dir = v2(dd.x * d.facing, dd.y);
    let per = dir.perp();
    let hand = d.hand[0];
    let (len, wide, curve, col) = match blade {
        Blade::Sword => (14.5, 0.78, 0.0, st.metal),
        Blade::Scimitar => (15.5, 1.05, 0.20, rgb(226, 216, 180)),
        Blade::Dagger => (6.4, 0.70, 0.0, st.metal),
        Blade::Wand => (9.0, 1.0, 0.0, rgb(104, 72, 44)),
        Blade::None => return,
    };
    let len = len * d.u;
    let grip_a = hand.sub(dir.mul(2.6 * g));
    let tip = hand.add(dir.mul(len)).add(per.mul(curve * len));
    let mid = hand.add(dir.mul(len * 0.5)).add(per.mul(curve * len * 0.34));

    // Grip and pommel.
    fill_capsule(
        layer,
        grip_a,
        hand.add(dir.mul(0.6 * g)),
        0.70 * g,
        0.76 * g,
        rgb(96, 62, 36),
        1.0,
    );
    fill_circle(layer, grip_a.sub(dir.mul(0.35 * g)), 0.80 * g, rgb(186, 152, 78), 1.0);
    if blade == Blade::Wand {
        fill_capsule(layer, hand, tip, 0.80 * g, 0.60 * g, col, 1.0);
        fill_circle(layer, tip, 1.5 * g, rgb(255, 178, 64), 1.0);
        return;
    }
    // Cross-guard with quillons.
    let cg = per.mul(1.75 * g);
    fill_capsule(
        layer,
        hand.add(cg).add(dir.mul(0.5 * g)),
        hand.sub(cg).add(dir.mul(0.5 * g)),
        0.42 * g,
        0.42 * g,
        rgb(190, 154, 78),
        1.0,
    );
    // Blade: a tapered diamond with a bright fuller and a dark back edge — two
    // hard values, which is what makes steel read as steel.
    let root = hand.add(dir.mul(1.0 * g));
    let spine = [
        root.add(per.mul(wide * g)),
        mid.add(per.mul(wide * 0.82 * g)),
        tip,
        mid.sub(per.mul(wide * 0.82 * g)),
        root.sub(per.mul(wide * g)),
    ];
    shape::flat(layer, &spine, col.scale(0.62), 1.0);
    let lit = [
        root.add(per.mul(wide * g)),
        mid.add(per.mul(wide * 0.82 * g)),
        tip,
        mid.add(per.mul(wide * 0.05 * g)),
        root.add(per.mul(wide * 0.10 * g)),
    ];
    shape::flat(layer, &lit, col.scale(1.18), 1.0);
    shape::contour(layer, root.add(per.mul(wide * 0.45 * g)), mid.lerp(tip, 0.55), 0.24 * g, Rgb::WHITE, 0.7);
    let _ = light;
}

// ---------------------------------------------------------------- skeletons

/// A long bone: knobbed at both ends, thin through the shaft. That profile *is*
/// the read — a capsule of constant width is a stick, not a bone.
const BONE: shape::Profile = &[
    (0.00, 1.30, 1.30),
    (0.14, 0.84, 0.84),
    (0.50, 0.66, 0.66),
    (0.86, 0.80, 0.80),
    (1.00, 1.14, 1.14),
];

fn draw_bones(layer: &mut Layer, d: &Dev, st: &Style, pose: &Pose, blade: Blade, light: V2) {
    let g = d.g;
    let bone = rgb(232, 226, 204);
    for &(i, far) in &[(1usize, true), (0usize, false)] {
        let c = if far { recede(bone) } else { bone };
        let seg = |a: V2, b: V2, k: f32| Limb::new(a, b, shape::front_of(a, b, d.facing), BONE, g * k);
        seg(d.hip, d.knee[i], 1.05).steps(6).draw(layer, c, light);
        seg(d.knee[i], d.ankle[i], 0.90).steps(6).draw(layer, c, light);
        seg(d.ankle[i], d.toe[i], 0.80).steps(4).draw(layer, c.scale(0.9), light);
        seg(d.shoulder, d.elbow[i], 0.90).steps(6).draw(layer, c, light);
        seg(d.elbow[i], d.hand[i], 0.78).steps(6).draw(layer, c, light);
        // Fingers.
        let dir = d.hand[i].sub(d.elbow[i]).norm();
        for k in -1..=1 {
            let o = dir.perp().mul(k as f32 * 0.7 * g);
            fill_capsule(
                layer,
                d.hand[i].add(o),
                d.hand[i].add(o).add(dir.mul(1.5 * g)),
                0.34 * g,
                0.24 * g,
                c.scale(0.95),
                1.0,
            );
        }
    }
    // Spine, pelvis and ribs.
    let front = shape::front_of(d.shoulder, d.hip, d.facing);
    Limb::new(d.shoulder, d.hip, front, BONE, g * 1.0)
        .steps(6)
        .draw(layer, bone.scale(0.80), light);
    let ex = front.mul(g);
    let ey = d.hip.sub(d.shoulder).norm().mul(g);
    let pelvis = shape::frame(
        d.hip,
        ex,
        ey,
        &[(2.6, -1.2), (2.9, 0.6), (1.2, 1.9), (-1.2, 1.9), (-2.7, 0.5), (-2.4, -1.2)],
    );
    shape::cel_poly(layer, &pelvis, bone, light, 0.44);
    shape::flat(
        layer,
        &shape::frame(d.hip, ex, ey, &[(1.3, -0.2), (1.4, 0.9), (-1.3, 0.9), (-1.4, -0.2)]),
        rgb(38, 30, 34),
        0.8,
    );
    let down = d.up.mul(-1.0);
    for k in 0..4 {
        let t = 0.18 + k as f32 * 0.16;
        let c = d.shoulder.add(down.mul(d.tl * t));
        let w = (2.85 - k as f32 * 0.22) * g;
        shape::contour(
            layer,
            c.add(front.mul(w)),
            c.sub(front.mul(w * 0.75)),
            0.52 * g,
            bone,
            1.0,
        );
    }
    shape::contour(
        layer,
        d.shoulder.add(front.mul(2.4 * g)),
        d.shoulder.sub(front.mul(1.8 * g)),
        0.58 * g,
        bone.scale(1.05),
        1.0,
    );

    // Skull: cranium, socket, nasal spine, and the dropped jaw with its teeth.
    let (hc, hr, fw) = (d.head, d.hr, d.facing);
    let hx = v2(fw * hr, 0.0);
    let hy = v2(0.0, hr);
    let p = |x: f32, y: f32| hc.add(hx.mul(x)).add(hy.mul(y));
    shape::cel_poly(layer, &shape::frame(hc, hx, hy, shape::SKULL), bone, light, 0.44);
    shape::flat(
        layer,
        &shape::frame(hc, hx, hy, &[(0.28, -0.20), (0.74, -0.18), (0.76, 0.16), (0.36, 0.18)]),
        rgb(22, 18, 22),
        1.0,
    );
    shape::flat(
        layer,
        &shape::frame(hc, hx, hy, &[(0.78, 0.16), (0.94, 0.30), (0.80, 0.40)]),
        rgb(22, 18, 22),
        0.9,
    );
    shape::contour(layer, p(0.20, 0.60), p(0.88, 0.56), hr * 0.09, rgb(30, 24, 26), 0.85);
    for k in 0..4 {
        let t = k as f32 / 3.0;
        fill_circle(layer, p(lerp(0.24, 0.84, t), lerp(0.62, 0.54, t)), hr * 0.07, rgb(30, 24, 26), 0.85);
    }
    draw_blade(layer, d, st, pose, blade, light);
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
