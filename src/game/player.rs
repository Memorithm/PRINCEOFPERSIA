//! The prince: his state machine, physics and interactions.
//!
//! The moveset follows the original closely — run, skid to a halt, turn on the
//! spot, standing and running jumps, crouch, careful step, hang from a ledge,
//! pull up, climb down, and a fencing stance with advance, retreat, thrust and
//! parry — with the bonus weapons layered on top.

use crate::art::anim::{anims, Clip};
use crate::art::skel::Pose;
use crate::game::combat::{Melee, ShotKind};
use crate::game::*;
use crate::gfx::color::rgb;
use crate::input::Input;
use crate::util::{clampf, ease_out, v2, V2};
use crate::world::level::Level;
use crate::world::tile::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PState {
    Stand,
    Turn,
    RunStart,
    Run,
    RunStop,
    Step,
    CrouchIn,
    Crouch,
    CrouchOut,
    JumpUp,
    JumpRun,
    Fall,
    Land,
    Hang,
    Climb,
    ClimbDown,
    Ready,
    Advance,
    Retreat,
    Strike,
    Parry,
    Hurt,
    Dead,
    Drink,
    Throw,
    Cast,
    Leaving,
}

impl PState {
    pub fn airborne(self) -> bool {
        matches!(self, PState::JumpUp | PState::JumpRun | PState::Fall)
    }
    /// States during which the prince cannot start something else.
    pub fn locked(self) -> bool {
        matches!(
            self,
            PState::Turn
                | PState::RunStart
                | PState::RunStop
                | PState::Step
                | PState::CrouchIn
                | PState::CrouchOut
                | PState::Land
                | PState::Climb
                | PState::ClimbDown
                | PState::Strike
                | PState::Advance
                | PState::Retreat
                | PState::Hurt
                | PState::Dead
                | PState::Drink
                | PState::Throw
                | PState::Cast
                | PState::Leaving
        )
    }
    /// Sword drawn in this pose?
    pub fn fencing(self) -> bool {
        matches!(
            self,
            PState::Ready | PState::Advance | PState::Retreat | PState::Strike | PState::Parry
        )
    }
}

#[derive(Clone, Copy)]
pub struct Player {
    pub p: V2,
    pub v: V2,
    pub facing: f32,
    pub st: PState,
    /// Time spent in the current state.
    pub t: f32,
    pub hp: i32,
    pub hp_max: i32,
    /// Sword drawn.
    pub armed: bool,
    pub melee: Melee,
    pub sword: bool,
    pub scimitar: bool,
    pub buckler: bool,
    pub wand: bool,
    pub daggers: i32,
    pub charges: i32,
    /// Height at which the current fall began.
    pub fall_from: f32,
    pub float_t: f32,
    pub swift_t: f32,
    pub invuln: f32,
    /// Ledge being hung from, and the anchor for a climb.
    pub ledge: (i32, i32),
    pub anchor: V2,
    /// This strike has already been resolved.
    pub struck: bool,
    pub step_to: f32,
    /// Cooldown between thrown weapons.
    pub throw_cd: f32,
    pub cause: Option<&'static str>,

    // ---- presentation and feel ------------------------------------------
    /// Pose held at the moment the state changed, cross-faded out of so that no
    /// transition ever snaps.
    pub blend_from: Pose,
    pub blend_t: f32,
    /// Eased facing. The sign mirrors the figure and the magnitude compresses it,
    /// so a turn passes through an edge-on pose instead of flipping.
    pub facing_vis: f32,
    /// Run-cycle phase, advanced by *distance travelled* rather than by time, so
    /// the feet never skate however fast he happens to be moving.
    pub gait: f32,
    /// Buffered button presses, so an action pressed during a locked animation
    /// still happens the moment the animation lets go.
    pub buf_jump: f32,
    pub buf_attack: f32,
}

/// How long a pose cross-fade takes.
pub const BLEND: f32 = 0.085;
/// Distance covered by one full six-key run cycle.
pub const STRIDE_PX: f32 = 36.0;
/// Duration of that cycle in the clip.
pub const RUN_CYCLE: f32 = 0.45;
/// How long a button press stays queued.
pub const BUFFER: f32 = 0.18;

impl Player {
    pub fn new(at: V2, facing: f32, carry: &Carry) -> Player {
        Player {
            p: at,
            v: V2::ZERO,
            facing,
            st: PState::Stand,
            t: 0.0,
            hp: carry.hp_max,
            hp_max: carry.hp_max,
            armed: false,
            melee: if carry.scimitar {
                Melee::Scimitar
            } else if carry.sword {
                Melee::Sword
            } else {
                Melee::None
            },
            sword: carry.sword,
            scimitar: carry.scimitar,
            buckler: carry.buckler,
            wand: carry.wand,
            daggers: carry.daggers,
            charges: if carry.wand { 8 } else { 0 },
            fall_from: at.y,
            float_t: 0.0,
            swift_t: 0.0,
            invuln: 0.0,
            ledge: (0, 0),
            anchor: at,
            struck: false,
            step_to: at.x,
            throw_cd: 0.0,
            cause: None,
            blend_from: Pose::REST,
            blend_t: BLEND,
            facing_vis: facing,
            gait: 0.0,
            buf_jump: 0.0,
            buf_attack: 0.0,
        }
    }

    #[inline]
    pub fn foot_tile(&self) -> (i32, i32) {
        (Level::tx_of(self.p.x), Level::ty_of_feet(self.p.y))
    }

    pub fn speed(&self) -> f32 {
        RUN_SPEED * if self.swift_t > 0.0 { 1.35 } else { 1.0 }
    }

    pub fn carry(&self) -> Carry {
        Carry {
            hp_max: self.hp_max,
            sword: self.sword,
            scimitar: self.scimitar,
            buckler: self.buckler,
            wand: self.wand,
            daggers: self.daggers,
        }
    }

    /// The clip that drives the current state, and how fast to play it.
    pub fn clip(&self) -> (&'static Clip, f32) {
        let a = anims();
        match self.st {
            PState::Stand => (&a.stand, 1.0),
            PState::Turn => (&a.turn, 1.0),
            PState::RunStart => (&a.run_start, 1.0),
            PState::Run => (&a.run, if self.swift_t > 0.0 { 1.3 } else { 1.0 }),
            PState::RunStop => (&a.run_stop, 1.0),
            PState::Step => (&a.step, 1.0),
            PState::CrouchIn => (&a.crouch_in, 1.0),
            PState::Crouch => (&a.crouch, 1.0),
            PState::CrouchOut => (&a.crouch_in, -1.0),
            PState::JumpUp => (&a.jump_up, 1.0),
            PState::JumpRun => (&a.jump_run, 1.0),
            PState::Fall => (&a.fall, 1.0),
            PState::Land => (&a.land, 1.0),
            PState::Hang => (&a.hang, 1.0),
            PState::Climb => (&a.climb, 1.0),
            PState::ClimbDown => (&a.climb, -1.0),
            PState::Ready => (&a.sword_ready, 1.0),
            PState::Advance => (&a.sword_adv, 1.0),
            PState::Retreat => (&a.sword_ret, 1.0),
            PState::Strike => (&a.sword_strike, self.melee.swing()),
            PState::Parry => (&a.sword_parry, 1.0),
            PState::Hurt => (&a.hurt, 1.0),
            PState::Dead => (&a.dead, 1.0),
            PState::Drink => (&a.drink, 1.0),
            PState::Throw => (&a.throw, 1.0),
            PState::Cast => (&a.cast, 1.0),
            PState::Leaving => (&a.bow, 1.0),
        }
    }

    /// Where in its clip the current state is. Cycles are driven by distance
    /// travelled; everything else by time in state.
    fn clip_time(&self) -> f32 {
        match self.st {
            PState::Run => self.gait,
            _ => self.t,
        }
    }

    /// The clip's own pose, before cross-fading.
    fn raw_pose(&self) -> Pose {
        let (clip, rate) = self.clip();
        if rate < 0.0 {
            let total = clip.total();
            clip.sample((total - self.clip_time() * -rate).max(0.0))
        } else {
            clip.sample(self.clip_time() * rate)
        }
    }

    /// Sample the pose for the current moment, cross-faded out of the pose the
    /// previous state ended on.
    pub fn pose(&self) -> Pose {
        let p = self.raw_pose();
        if self.blend_t >= BLEND {
            p
        } else {
            self.blend_from
                .lerp(&p, crate::util::smoothstep(self.blend_t / BLEND))
        }
    }

    pub fn blade(&self) -> crate::art::skel::Blade {
        if self.st == PState::Throw {
            crate::art::skel::Blade::Dagger
        } else if self.st == PState::Cast {
            crate::art::skel::Blade::Wand
        } else if self.armed {
            self.melee.blade()
        } else {
            crate::art::skel::Blade::None
        }
    }
}

// ---------------------------------------------------------------- simulation

impl Game {
    fn enter(&mut self, st: PState) {
        if self.player.st == st {
            return;
        }
        // Freeze the pose we are leaving and fade out of it.
        let from = self.player.pose();
        let pl = &mut self.player;
        pl.blend_from = from;
        pl.blend_t = 0.0;
        pl.st = st;
        pl.t = 0.0;
        pl.struck = false;
    }

    /// Slide `x` horizontally, stopping against masonry and closed gates.
    fn slide_x(&self, x: f32, y: f32, dx: f32) -> f32 {
        if dx == 0.0 {
            return x;
        }
        let dir = if dx > 0.0 { 1.0 } else { -1.0 };
        let nx = x + dx;
        let edge = nx + dir * BODY_HW;
        let tx = Level::tx_of(edge);
        let blocked = [5.0f32, 15.0, 25.0]
            .iter()
            .any(|h| !self.open(tx, Level::ty_of(y - h)));
        if !blocked {
            return nx;
        }
        if dir > 0.0 {
            let bound = tx as f32 * TILE_W - BODY_HW - 0.02;
            nx.min(bound).max(x.min(bound))
        } else {
            let bound = (tx + 1) as f32 * TILE_W + BODY_HW + 0.02;
            nx.max(bound).min(x.max(bound))
        }
    }

    /// The ledge one level up in front of the prince, if he can pull onto it.
    fn climb_target(&self, pl: &Player) -> Option<(i32, i32)> {
        let (tx, br) = pl.foot_tile();
        let front = Level::tx_of(pl.p.x + pl.facing * (BODY_HW + 4.0));
        if front == tx {
            return None;
        }
        if !self.supported(front, br - 1) {
            return None;
        }
        if !self.open(tx, br - 1) {
            return None;
        }
        Some((front, br - 1))
    }

    /// A ledge to grab while airborne.
    ///
    /// The ledge ahead is tried first, then the one *behind* — that second case
    /// is the original's signature save: run off a brink with the grab key held
    /// and catch the lip you just left, turning to face it as you do.
    fn grab_target(&self, pl: &Player) -> Option<(i32, i32, bool)> {
        if pl.p.y < 0.0 {
            return None;
        }
        let br = Level::ty_of_feet(pl.p.y);
        let ly = br - 1;
        let hang_y = Level::surf(ly) + HANG_DROP;
        if (pl.p.y - hang_y).abs() > 15.0 {
            return None;
        }
        for (dir, turn) in [(pl.facing, false), (-pl.facing, true)] {
            let lx = Level::tx_of(pl.p.x + dir * (BODY_HW + 4.0));
            // Something to grip, and room for the body to hang below it.
            if self.supported(lx, ly) && self.open(lx, br) {
                return Some((lx, ly, turn));
            }
        }
        None
    }

    fn start_hang(&mut self, ledge: (i32, i32, bool)) {
        let (lx, ly, turn) = ledge;
        let mut pl = self.player;
        if turn {
            // You end up facing the lip you caught.
            pl.facing = -pl.facing;
        }
        pl.ledge = (lx, ly);
        pl.p.y = Level::surf(ly) + HANG_DROP;
        // Hands on the lip: line the body up with the edge of the ledge tile.
        pl.p.x = if pl.facing > 0.0 {
            lx as f32 * TILE_W - 2.0
        } else {
            (lx + 1) as f32 * TILE_W + 2.0
        };
        pl.v = V2::ZERO;
        pl.fall_from = pl.p.y;
        self.player = pl;
        self.enter(PState::Hang);
        let at = v2(self.player.p.x, self.player.p.y - HANG_DROP + 2.0);
        self.fx.dust(at, 4, 0.6, self.lv.theme.slab_face);
    }

    fn begin_fall(&mut self) {
        if self.player.st.airborne() {
            return;
        }
        self.player.fall_from = self.player.p.y;
        self.enter(PState::Fall);
    }

    fn land(&mut self, surf: f32) {
        let mut pl = self.player;
        let drop = surf - pl.fall_from;
        pl.p.y = surf;
        pl.v = V2::ZERO;
        self.player = pl;
        let (tx, ty) = self.player.foot_tile();
        let hard = drop > FALL_SAFE;
        self.fx.dust(
            v2(self.player.p.x, surf - 1.0),
            if hard { 14 } else { 7 },
            if hard { 1.5 } else { 0.8 },
            self.lv.theme.slab_face.scale(0.85),
        );
        if hard {
            self.cam.shake = 0.5;
        }
        if self.lethal(tx, ty) {
            self.kill_player("Empalé.");
            return;
        }
        if drop >= FALL_LETHAL && self.player.float_t <= 0.0 {
            self.kill_player("La chute était trop longue.");
            return;
        }
        if drop > FALL_SAFE && self.player.float_t <= 0.0 {
            self.player.hp -= 1;
            self.flash = (0.6, rgb(140, 20, 20));
            if self.player.hp <= 0 {
                self.kill_player("Brisé par la chute.");
                return;
            }
        }
        self.enter(PState::Land);
    }

    pub fn kill_player(&mut self, cause: &'static str) {
        if matches!(self.phase, Phase::Dying(_) | Phase::Dead) {
            return;
        }
        self.player.cause = Some(cause);
        self.player.st = PState::Dead;
        self.player.t = 0.0;
        self.player.v = V2::ZERO;
        self.deaths += 1;
        self.phase = Phase::Dying(2.1);
        self.flash = (1.0, rgb(120, 10, 12));
        self.cam.shake = 0.8;
        let at = v2(self.player.p.x, self.player.p.y - 16.0);
        let floor = self.player.p.y;
        self.fx.blood(at, self.player.facing, 26, floor);
        self.say(cause, 3.0, true);
    }

    pub fn update_player_dead(&mut self, dt: f32) {
        self.player.t += dt;
        // Let the body settle onto the floor.
        let (tx, ty) = self.player.foot_tile();
        if !self.supported(tx, ty) {
            self.player.v.y += GRAVITY * dt;
            let y1 = self.player.p.y + self.player.v.y * dt;
            let ty1 = Level::ty_of_feet(y1);
            let mut landed = None;
            for t in ty..=ty1.max(ty) {
                if Level::surf(t) <= y1 && self.supported(tx, t) {
                    landed = Some(Level::surf(t));
                    break;
                }
            }
            match landed {
                Some(s) => {
                    self.player.p.y = s;
                    self.player.v.y = 0.0;
                }
                None => self.player.p.y = y1,
            }
        }
    }

    pub fn update_player(&mut self, dt: f32, inp: &Input) {
        let mut pl = self.player;
        pl.t += dt;
        pl.blend_t += dt;
        pl.invuln = (pl.invuln - dt).max(0.0);
        pl.float_t = (pl.float_t - dt).max(0.0);
        pl.swift_t = (pl.swift_t - dt).max(0.0);
        pl.throw_cd = (pl.throw_cd - dt).max(0.0);
        pl.buf_jump = (pl.buf_jump - dt).max(0.0);
        pl.buf_attack = (pl.buf_attack - dt).max(0.0);
        if inp.up_edge {
            pl.buf_jump = BUFFER;
        }
        if inp.attack {
            pl.buf_attack = BUFFER;
        }
        // Ease the visible facing so turning is a movement, not a mirror flip.
        pl.facing_vis = crate::util::approach(pl.facing_vis, pl.facing, dt / 0.075);
        self.player = pl;

        // ---- ranged weapons work from almost any footing -----------------
        if !self.player.st.locked() && !self.player.st.airborne() {
            if inp.throw && self.player.daggers > 0 && self.player.throw_cd <= 0.0 {
                self.player.daggers -= 1;
                self.player.throw_cd = 0.42;
                let from = v2(
                    self.player.p.x + self.player.facing * 10.0,
                    self.player.p.y - 19.0,
                );
                let dir = self.player.facing;
                self.spawn_shot(ShotKind::Dagger, from, dir, true);
                self.enter(PState::Throw);
            } else if inp.cast && self.player.wand && self.player.charges > 0 {
                self.player.charges -= 1;
                let from = v2(
                    self.player.p.x + self.player.facing * 12.0,
                    self.player.p.y - 18.0,
                );
                let dir = self.player.facing;
                self.spawn_shot(ShotKind::Fireball, from, dir, true);
                self.enter(PState::Cast);
                self.flash = (0.35, rgb(90, 50, 10));
            } else if inp.cast && self.player.wand {
                self.say("Le bâton est éteint.", 1.4, true);
            } else if inp.throw && self.player.sword && self.player.daggers == 0 {
                self.say("Plus de dagues.", 1.2, true);
            }
        }

        let clip_total = {
            let (c, r) = self.player.clip();
            c.total() / r.abs().max(0.01)
        };
        let done = self.player.t >= clip_total;
        let enemy = self.nearest_enemy();

        match self.player.st {
            // ---------------------------------------------------------- idle
            PState::Stand | PState::Ready => {
                let (tx, ty) = self.player.foot_tile();
                if !self.supported(tx, ty) {
                    self.begin_fall();
                } else if inp.sheathe && self.player.armed {
                    self.player.armed = false;
                    self.enter(PState::Stand);
                } else if inp.attack || self.player.buf_attack > 0.0 {
                    self.player.buf_attack = 0.0;
                    if self.player.armed {
                        self.enter(PState::Strike);
                    } else if self.player.melee != Melee::None {
                        self.player.armed = true;
                        self.enter(PState::Ready);
                        self.say("Épée au clair.", 1.2, false);
                    } else {
                        self.say("Tu n'as pas d'arme.", 1.4, true);
                    }
                } else if inp.parry && self.player.armed {
                    self.enter(PState::Parry);
                } else if inp.up || self.player.buf_jump > 0.0 {
                    self.player.buf_jump = 0.0;
                    match self.climb_target(&self.player) {
                        Some(l) => self.begin_climb(l),
                        None => {
                            self.player.v.y = JUMP_UP_VY;
                            self.player.fall_from = self.player.p.y;
                            self.enter(PState::JumpUp);
                        }
                    }
                } else if inp.down {
                    if inp.careful || inp.down_edge {
                        if let Some(l) = self.climb_down_target() {
                            self.begin_climb_down(l);
                        } else {
                            self.enter(PState::CrouchIn);
                        }
                    } else {
                        self.enter(PState::CrouchIn);
                    }
                } else if inp.any_dir() {
                    let d = inp.dir();
                    if d != self.player.facing {
                        if self.player.armed && enemy.is_some() {
                            // Fencing: stepping back does not turn your back.
                            self.enter(PState::Retreat);
                        } else {
                            self.enter(PState::Turn);
                        }
                    } else if inp.careful {
                        self.begin_step();
                    } else if self.player.armed && enemy.is_some() {
                        self.enter(PState::Advance);
                    } else {
                        self.enter(PState::RunStart);
                    }
                } else {
                    // Settle into the right idle stance.
                    let want = if self.player.armed && enemy.is_some() {
                        PState::Ready
                    } else {
                        PState::Stand
                    };
                    if self.player.st != want {
                        self.enter(want);
                    }
                }
            }

            // ---------------------------------------------------------- turn
            PState::Turn => {
                if self.player.t >= clip_total * 0.55 && !self.player.struck {
                    self.player.facing = -self.player.facing;
                    self.player.struck = true;
                }
                if done {
                    self.enter(PState::Stand);
                }
            }

            // ---------------------------------------------------------- running
            PState::RunStart => {
                let sp = self.player.speed() * (self.player.t / clip_total).min(1.0);
                self.step_run(dt, sp);
                if done {
                    self.enter(PState::Run);
                }
                if inp.dir() != self.player.facing && inp.any_dir() {
                    self.enter(PState::RunStop);
                }
            }
            PState::Run => {
                let sp = self.player.speed();
                let before = self.player.p.x;
                self.step_run(dt, sp);
                let stuck = (self.player.p.x - before).abs() < sp * dt * 0.3;
                if inp.up {
                    self.player.v.y = JUMP_RUN_VY;
                    self.player.v.x = self.player.facing * JUMP_RUN_VX;
                    self.player.fall_from = self.player.p.y;
                    self.enter(PState::JumpRun);
                } else if inp.down {
                    self.enter(PState::CrouchIn);
                } else if !inp.any_dir() || inp.dir() != self.player.facing || stuck {
                    self.enter(PState::RunStop);
                }
            }
            PState::RunStop => {
                let f = 1.0 - (self.player.t / clip_total).min(1.0);
                self.step_run(dt, self.player.speed() * f * 0.8);
                // A jump queued mid-skid fires as soon as the feet are under him,
                // rather than being swallowed.
                let early = self.player.buf_jump > 0.0 && self.player.t > clip_total * 0.45;
                if done || early {
                    self.enter(PState::Stand);
                }
            }
            PState::Step => {
                let f = (self.player.t / clip_total).min(1.0);
                let target = self.player.step_to;
                let x0 = self.player.anchor.x;
                let nx = x0 + (target - x0) * ease_out(f);
                let dx = nx - self.player.p.x;
                let y = self.player.p.y;
                self.player.p.x = self.slide_x(self.player.p.x, y, dx);
                if done {
                    self.enter(PState::Stand);
                }
            }

            // ---------------------------------------------------------- crouch
            PState::CrouchIn => {
                if done {
                    self.enter(PState::Crouch);
                }
            }
            PState::Crouch => {
                if !inp.down {
                    self.enter(PState::CrouchOut);
                } else if inp.down_edge {
                    if let Some(l) = self.climb_down_target() {
                        self.begin_climb_down(l);
                    }
                }
            }
            PState::CrouchOut => {
                if done {
                    self.enter(PState::Stand);
                }
            }

            // ---------------------------------------------------------- airborne
            PState::JumpUp | PState::JumpRun | PState::Fall => {
                self.airborne_step(dt, inp);
            }
            PState::Land => {
                let early = (self.player.buf_jump > 0.0 || inp.any_dir())
                    && self.player.t > clip_total * 0.5;
                if done || early {
                    self.enter(PState::Stand);
                }
            }

            // ---------------------------------------------------------- ledges
            PState::Hang => {
                if inp.up {
                    let l = self.player.ledge;
                    self.begin_climb_from_hang(l);
                } else if inp.down {
                    self.player.fall_from = self.player.p.y;
                    self.enter(PState::Fall);
                }
            }
            PState::Climb => {
                let f = ease_out((self.player.t / clip_total).min(1.0));
                let a = self.player.anchor;
                let (lx, ly) = self.player.ledge;
                let ty = Level::surf(ly);
                let tgt_x = Level::cx(lx) - self.player.facing * 4.0;
                self.player.p.y = a.y + (ty - a.y) * f;
                self.player.p.x = a.x + (tgt_x - a.x) * clampf(f * 1.6 - 0.4, 0.0, 1.0);
                if done {
                    self.player.p.y = ty;
                    self.player.p.x = tgt_x;
                    self.player.fall_from = ty;
                    self.enter(PState::Stand);
                }
            }
            PState::ClimbDown => {
                let f = (self.player.t / clip_total).min(1.0);
                let a = self.player.anchor;
                let (lx, ly) = self.player.ledge;
                let hy = Level::surf(ly) + HANG_DROP;
                let hx = if self.player.facing > 0.0 {
                    lx as f32 * TILE_W - 2.0
                } else {
                    (lx + 1) as f32 * TILE_W + 2.0
                };
                self.player.p.y = a.y + (hy - a.y) * ease_out(f);
                self.player.p.x = a.x + (hx - a.x) * f;
                if done {
                    self.player.p.y = hy;
                    self.player.p.x = hx;
                    self.player.fall_from = hy;
                    self.enter(PState::Hang);
                }
            }

            // ---------------------------------------------------------- fencing
            PState::Advance | PState::Retreat => {
                let sign = if self.player.st == PState::Advance {
                    self.player.facing
                } else {
                    -self.player.facing
                };
                let f = (self.player.t / clip_total).min(1.0);
                // Bell-shaped step so it reads as a fencing lunge.
                let sp = (f * std::f32::consts::PI).sin() * 46.0;
                let y = self.player.p.y;
                let nx = self.slide_x(self.player.p.x, y, sign * sp * dt);
                // Never step off the ledge you are fighting on.
                let ntx = Level::tx_of(nx);
                let (_, ty) = self.player.foot_tile();
                if self.supported(ntx, ty) {
                    self.player.p.x = nx;
                }
                if done {
                    self.enter(PState::Ready);
                }
            }
            PState::Strike => {
                let f = self.player.t / clip_total;
                if !self.player.struck && (0.30..0.62).contains(&f) {
                    self.player.struck = true;
                    self.resolve_player_strike();
                }
                if done {
                    self.enter(PState::Ready);
                }
            }
            PState::Parry => {
                if !inp.parry {
                    self.enter(PState::Ready);
                }
            }

            // ---------------------------------------------------------- misc
            PState::Hurt => {
                let y = self.player.p.y;
                let dx = self.player.v.x * dt;
                self.player.p.x = self.slide_x(self.player.p.x, y, dx);
                self.player.v.x *= 1.0 - 6.0 * dt;
                if done {
                    self.enter(if self.player.armed {
                        PState::Ready
                    } else {
                        PState::Stand
                    });
                }
            }
            PState::Drink | PState::Throw | PState::Cast => {
                if done {
                    self.enter(if self.player.armed {
                        PState::Ready
                    } else {
                        PState::Stand
                    });
                }
            }
            PState::Dead | PState::Leaving => {}
        }

        // ---- hazards under foot ------------------------------------------
        if !matches!(self.player.st, PState::Dead | PState::Hang | PState::Leaving)
            && !self.player.st.airborne()
        {
            let (tx, ty) = self.player.foot_tile();
            if self.lethal(tx, ty) {
                let t = self.lv.tile(tx, ty);
                self.dy.set_flag(tx, ty, F_BLOODY, true);
                self.kill_player(if t == Tile::Spikes {
                    "Les pointes ont jailli."
                } else {
                    "Les lames se sont refermées."
                });
            }
        }

        // ---- the way out --------------------------------------------------
        if self.phase == Phase::Play && !self.player.st.airborne() {
            let (tx, ty) = self.player.foot_tile();
            if self.lv.tile(tx, ty) == Tile::Exit && self.dy.a(tx, ty) > 0.55 {
                self.player.st = PState::Leaving;
                self.player.t = 0.0;
                self.phase = Phase::Leaving(1.4);
                self.say("La voie est libre.", 2.0, false);
            }
        }

        // Run out of a room's floor while sprinting? Keep the dust going.
        if self.player.st == PState::Run && self.rng.chance(dt * 12.0) {
            let at = v2(self.player.p.x - self.player.facing * 5.0, self.player.p.y);
            self.fx.dust(at, 1, 0.35, self.lv.theme.slab_face.scale(0.8));
        }
    }

    fn begin_step(&mut self) {
        let mut pl = self.player;
        pl.anchor = pl.p;
        let want = pl.p.x + pl.facing * (TILE_W * 0.42);
        // A careful step stops at the brink instead of walking off it.
        let ty = Level::ty_of_feet(pl.p.y);
        let ntx = Level::tx_of(want + pl.facing * BODY_HW);
        let safe = if self.supported(ntx, ty) || !self.open(ntx, ty) {
            want
        } else {
            let edge = if pl.facing > 0.0 {
                ntx as f32 * TILE_W - BODY_HW - 1.0
            } else {
                (ntx + 1) as f32 * TILE_W + BODY_HW + 1.0
            };
            if pl.facing > 0.0 {
                want.min(edge)
            } else {
                want.max(edge)
            }
        };
        pl.step_to = safe;
        self.player = pl;
        self.enter(PState::Step);
    }

    fn begin_climb(&mut self, ledge: (i32, i32)) {
        let mut pl = self.player;
        pl.anchor = pl.p;
        pl.ledge = ledge;
        self.player = pl;
        self.enter(PState::Climb);
    }

    fn begin_climb_from_hang(&mut self, ledge: (i32, i32)) {
        let mut pl = self.player;
        pl.anchor = pl.p;
        pl.ledge = ledge;
        self.player = pl;
        self.enter(PState::Climb);
    }

    /// The ledge the prince would hang from if he climbed down here.
    fn climb_down_target(&self) -> Option<(i32, i32)> {
        let pl = &self.player;
        let (tx, ty) = pl.foot_tile();
        // He lets himself over the edge in the direction he faces.
        let front = Level::tx_of(pl.p.x + pl.facing * (BODY_HW + 4.0));
        for (lx, cand) in [(front, front), (tx, tx)] {
            let _ = cand;
            if self.supported(lx, ty) {
                continue;
            }
            if !self.open(lx, ty + 1) {
                continue;
            }
            // Need something under our own feet to have been standing on.
            if self.supported(tx, ty) {
                return Some((tx, ty));
            }
        }
        None
    }

    fn begin_climb_down(&mut self, ledge: (i32, i32)) {
        let mut pl = self.player;
        pl.anchor = pl.p;
        pl.ledge = ledge;
        self.player = pl;
        self.enter(PState::ClimbDown);
    }

    /// Horizontal running motion plus the "walk off the edge" check. The run
    /// cycle is advanced by how far he actually moved, which is what stops the
    /// feet skating when he accelerates or drinks a potion of celerity.
    fn step_run(&mut self, dt: f32, sp: f32) {
        let y = self.player.p.y;
        let dx = self.player.facing * sp * dt;
        let x0 = self.player.p.x;
        self.player.p.x = self.slide_x(x0, y, dx);
        let moved = (self.player.p.x - x0).abs();
        self.player.gait += moved / STRIDE_PX * RUN_CYCLE;
        let (tx, ty) = self.player.foot_tile();
        if !self.supported(tx, ty) {
            self.begin_fall();
        }
    }

    fn airborne_step(&mut self, dt: f32, inp: &Input) {
        let base = if self.player.st == PState::JumpRun {
            GRAVITY_JUMP
        } else {
            GRAVITY
        };
        let g = if self.player.float_t > 0.0 { base * 0.45 } else { base };
        self.player.v.y += g * dt;
        // A little air steering, as in the original.
        if inp.any_dir() && self.player.st != PState::JumpUp {
            let want = inp.dir() * JUMP_RUN_VX;
            self.player.v.x += (want - self.player.v.x).signum() * 60.0 * dt;
        }
        let y0 = self.player.p.y;
        let y1 = y0 + self.player.v.y * dt;
        let x = self.slide_x(self.player.p.x, y0, self.player.v.x * dt);
        self.player.p.x = x;

        // Grab a ledge on the way past.
        if (inp.up || inp.careful || self.player.st == PState::JumpUp) && self.player.v.y > -60.0 {
            let mut probe = self.player;
            probe.p.y = y1;
            if let Some(l) = self.grab_target(&probe) {
                self.start_hang(l);
                return;
            }
        }

        if self.player.v.y < 0.0 {
            // Bonk the ceiling.
            let head = y1 - 27.0;
            let tx = Level::tx_of(self.player.p.x);
            if !self.open(tx, Level::ty_of(head)) {
                self.player.v.y = 20.0;
                self.player.p.y = (Level::ty_of(head) + 1) as f32 * TILE_H + 27.5;
                return;
            }
            self.player.p.y = y1;
            return;
        }

        let tx = Level::tx_of(self.player.p.x);
        let ty0 = Level::ty_of_feet(y0);
        let ty1 = Level::ty_of_feet(y1);
        for t in ty0..=ty1.max(ty0) {
            let s = Level::surf(t);
            if s >= y0 - 0.01 && s <= y1 && self.supported(tx, t) {
                self.land(s);
                return;
            }
        }
        self.player.p.y = y1;
        if self.player.st != PState::Fall && self.player.v.y > 40.0 {
            self.player.st = PState::Fall;
            self.player.t = 0.0;
        }
        // Fell out of the world.
        if self.player.p.y > (self.lv.th + 2) as f32 * TILE_H {
            self.kill_player("Englouti par le vide.");
        }
    }

    // ------------------------------------------------------------ items

    pub fn check_items(&mut self) {
        if self.phase != Phase::Play || self.player.st.airborne() {
            return;
        }
        let (tx, ty) = self.player.foot_tile();
        let mut got: Option<(usize, ItemKind)> = None;
        for (i, it) in self.items.iter().enumerate() {
            if !it.taken && it.tx == tx && it.ty == ty {
                got = Some((i, it.kind));
                break;
            }
        }
        let Some((i, kind)) = got else { return };
        self.items[i].taken = true;
        let label = kind.label();
        match kind {
            ItemKind::PotionHeal => {
                self.player.hp = (self.player.hp + 1).min(self.player.hp_max);
                self.flash = (0.7, rgb(150, 30, 40));
                self.enter(PState::Drink);
            }
            ItemKind::PotionLife => {
                self.player.hp_max += 1;
                self.player.hp = self.player.hp_max;
                self.flash = (0.9, rgb(180, 60, 110));
                self.enter(PState::Drink);
            }
            ItemKind::PotionFloat => {
                self.player.float_t = 24.0;
                self.flash = (0.7, rgb(40, 120, 170));
                self.enter(PState::Drink);
            }
            ItemKind::PotionSwift => {
                self.player.swift_t = 22.0;
                self.flash = (0.7, rgb(170, 150, 40));
                self.enter(PState::Drink);
            }
            ItemKind::PotionPoison => {
                self.flash = (0.9, rgb(60, 150, 50));
                self.enter(PState::Drink);
                self.player.hp -= 1;
                if self.player.hp <= 0 {
                    self.kill_player("Le poison t'emporte.");
                    return;
                }
            }
            ItemKind::Sword => {
                self.player.sword = true;
                if self.player.melee == Melee::None {
                    self.player.melee = Melee::Sword;
                }
                self.player.armed = true;
                self.enter(PState::Ready);
            }
            ItemKind::Scimitar => {
                self.player.scimitar = true;
                self.player.melee = Melee::Scimitar;
                self.player.armed = true;
                self.enter(PState::Ready);
            }
            ItemKind::Daggers => {
                self.player.daggers = (self.player.daggers + 5).min(12);
            }
            ItemKind::Wand => {
                self.player.wand = true;
                self.player.charges = (self.player.charges + 8).min(12);
            }
            ItemKind::Buckler => {
                self.player.buckler = true;
            }
        }
        let at = v2(Level::cx(tx), Level::surf(ty) - 6.0);
        self.fx.sparks(at, 12, 0.9);
        self.say(label, 2.4, kind == ItemKind::PotionPoison);
        self.carry = self.player.carry();
    }

    // ------------------------------------------------------------ combat

    /// Closest living guard that matters right now.
    pub fn nearest_enemy(&self) -> Option<usize> {
        let pl = &self.player;
        let mut best: Option<(usize, f32)> = None;
        for (i, g) in self.guards.iter().enumerate() {
            if g.st == GState::Dead || !g.hostile() {
                continue;
            }
            if (g.p.y - pl.p.y).abs() > TILE_H * 0.6 {
                continue;
            }
            let d = (g.p.x - pl.p.x).abs();
            if d > TILE_W * 3.2 {
                continue;
            }
            if best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    fn resolve_player_strike(&mut self) {
        let pl = self.player;
        let reach = pl.melee.reach();
        let tip = v2(pl.p.x + pl.facing * reach, pl.p.y - 18.0);
        self.fx.sparks(tip, 3, 0.5);
        crate::game::render::mark_slash(self, tip, pl.facing);
        let mut hit: Option<usize> = None;
        for (i, g) in self.guards.iter().enumerate() {
            if g.st == GState::Dead || !g.hostile() {
                continue;
            }
            if (g.p.y - pl.p.y).abs() > TILE_H * 0.6 {
                continue;
            }
            let dx = (g.p.x - pl.p.x) * pl.facing;
            if dx > 2.0 && dx < reach + 8.0 {
                hit = Some(i);
                break;
            }
        }
        let Some(gi) = hit else { return };
        let parrying = self.guards[gi].st == GState::Parry && self.guards[gi].facing != pl.facing;
        let pierce = pl.melee.pierce();
        if parrying && !self.rng.chance(pierce) {
            let at = v2(
                (pl.p.x + self.guards[gi].p.x) * 0.5,
                pl.p.y - 19.0,
            );
            self.fx.sparks(at, 18, 1.6);
            self.cam.shake = 0.35;
            self.guards[gi].cool = 0.16;
            return;
        }
        let dmg = pl.melee.damage();
        self.damage_guard(gi, dmg, pl.facing, false);
    }
}
