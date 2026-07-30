//! Guards: patrol, engage, fence.
//!
//! Skill (0–9, set per guard in the level's link layer) drives how often a guard
//! thrusts, how reliably it parries and how fast it reacts — the same three dials
//! the original used.

use crate::art::anim::{anims, Clip};
use crate::art::skel::{Blade, Pose, Prop, Style};
use crate::game::combat::Melee;
use crate::game::*;
use crate::gfx::color::rgb;
use crate::util::{clampf, v2, V2};
use crate::world::level::Level;
use crate::world::tile::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GState {
    Idle,
    Patrol,
    Ready,
    Advance,
    Retreat,
    Strike,
    Parry,
    Hurt,
    Dead,
    Falling,
}

#[derive(Clone, Copy)]
pub struct Guard {
    pub kind: MobKind,
    pub p: V2,
    pub v: V2,
    pub facing: f32,
    pub hp: i32,
    pub hp_max: i32,
    pub skill: u8,
    pub st: GState,
    pub t: f32,
    pub home: V2,
    pub dir: f32,
    /// Time until the next fencing decision.
    pub cool: f32,
    /// How long this guard has been aware of the prince.
    pub alert: f32,
    pub stagger: f32,
    pub struck: bool,
    pub idle: f32,
    /// Pose held at the moment the state changed, cross-faded out of.
    pub blend_from: Pose,
    pub blend_t: f32,
    /// Eased facing, as for the prince.
    pub facing_vis: f32,
    /// Walk-cycle phase, advanced by distance travelled.
    pub gait: f32,
}

/// Distance covered by one full walk cycle.
const GUARD_STRIDE: f32 = 21.0;
const WALK_CYCLE: f32 = 1.02;

impl Guard {
    pub fn new(m: &MobSpec) -> Guard {
        let at = v2(Level::cx(m.tx), Level::surf(m.ty));
        let hp = match m.kind {
            MobKind::Guard => 3 + (m.skill as i32) / 4,
            MobKind::Fat => 5,
            MobKind::Skeleton => 4,
            MobKind::Shadow => 4,
            MobKind::Jaffar => 9,
        };
        Guard {
            kind: m.kind,
            p: at,
            v: V2::ZERO,
            facing: m.facing,
            hp,
            hp_max: hp,
            skill: m.skill,
            st: GState::Idle,
            t: 0.0,
            home: at,
            dir: m.facing,
            cool: 0.4,
            alert: 0.0,
            stagger: 0.0,
            struck: false,
            idle: 0.0,
            blend_from: Pose::REST,
            blend_t: crate::game::player::BLEND,
            facing_vis: m.facing,
            gait: 0.0,
        }
    }

    /// Switch state, freezing the outgoing pose so the change cross-fades.
    fn enter(&mut self, st: GState) {
        if self.st == st {
            return;
        }
        self.blend_from = self.pose();
        self.blend_t = 0.0;
        self.st = st;
        self.t = 0.0;
    }

    #[inline]
    pub fn foot_tile(&self) -> (i32, i32) {
        (Level::tx_of(self.p.x), Level::ty_of_feet(self.p.y))
    }

    pub fn melee(&self) -> Melee {
        match self.kind {
            MobKind::Fat => Melee::Scimitar,
            MobKind::Jaffar => Melee::Scimitar,
            _ => Melee::Sword,
        }
    }

    pub fn prop(&self) -> Prop {
        match self.kind {
            MobKind::Guard => Prop::PRINCE.scaled(1.02, 1.10),
            MobKind::Fat => Prop::PRINCE.scaled(0.98, 1.55),
            MobKind::Skeleton => Prop::PRINCE.scaled(1.0, 0.74),
            MobKind::Shadow => Prop::PRINCE.scaled(1.0, 1.0),
            MobKind::Jaffar => Prop::PRINCE.scaled(1.06, 1.14),
        }
    }

    pub fn style(&self) -> Style {
        let base = Style::PRINCE;
        match self.kind {
            MobKind::Guard => Style {
                skin: rgb(196, 146, 104),
                skin_dk: rgb(140, 96, 64),
                cloth: rgb(132, 62, 54),
                cloth_dk: rgb(80, 34, 32),
                sash: rgb(64, 58, 70),
                sash_dk: rgb(38, 34, 44),
                hair: rgb(28, 22, 22),
                boot: rgb(92, 60, 36),
                head_wrap: Some(rgb(148, 152, 168)),
                plume: Some(rgb(178, 46, 46)),
                belt: true,
                ..base
            },
            MobKind::Fat => Style {
                skin: rgb(202, 156, 116),
                skin_dk: rgb(146, 100, 70),
                cloth: rgb(112, 92, 62),
                cloth_dk: rgb(66, 54, 38),
                sash: rgb(150, 42, 40),
                sash_dk: rgb(92, 24, 26),
                head_wrap: Some(rgb(214, 206, 186)),
                belt: true,
                ..base
            },
            MobKind::Skeleton => Style {
                bones: true,
                head_wrap: None,
                ..base
            },
            MobKind::Shadow => Style {
                skin: rgb(70, 68, 104),
                skin_dk: rgb(42, 40, 70),
                cloth: rgb(58, 60, 96),
                cloth_dk: rgb(30, 32, 58),
                sash: rgb(88, 52, 110),
                sash_dk: rgb(48, 26, 64),
                hair: rgb(20, 18, 34),
                boot: rgb(38, 34, 52),
                outline: rgb(12, 10, 24),
                ..base
            },
            MobKind::Jaffar => Style {
                skin: rgb(186, 138, 100),
                skin_dk: rgb(128, 86, 58),
                cloth: rgb(58, 44, 92),
                cloth_dk: rgb(30, 22, 54),
                sash: rgb(212, 172, 76),
                sash_dk: rgb(140, 106, 34),
                head_wrap: Some(rgb(34, 26, 58)),
                plume: Some(rgb(206, 58, 58)),
                robe: 1.0,
                ..base
            },
        }
    }

    pub fn clip(&self) -> (&'static Clip, f32) {
        let a = anims();
        match self.st {
            GState::Idle => (&a.stand, 1.0),
            GState::Patrol => (&a.walk, 1.0),
            GState::Ready => (&a.sword_ready, 1.0),
            GState::Advance => (&a.sword_adv, 1.0),
            GState::Retreat => (&a.sword_ret, 1.0),
            GState::Strike => (&a.sword_strike, self.melee().swing()),
            GState::Parry => (&a.sword_parry, 1.0),
            GState::Hurt => (&a.hurt, 1.0),
            GState::Dead => (&a.dead, 1.0),
            GState::Falling => (&a.fall, 1.0),
        }
    }

    pub fn pose(&self) -> Pose {
        let (c, r) = self.clip();
        let t = if self.st == GState::Patrol { self.gait } else { self.t };
        let raw = c.sample(t * r);
        let b = crate::game::player::BLEND;
        if self.blend_t >= b {
            raw
        } else {
            self.blend_from
                .lerp(&raw, crate::util::smoothstep(self.blend_t / b))
        }
    }

    pub fn blade(&self) -> Blade {
        if matches!(
            self.st,
            GState::Ready | GState::Advance | GState::Retreat | GState::Strike | GState::Parry
        ) {
            self.melee().blade()
        } else if self.kind == MobKind::Skeleton {
            Blade::Sword
        } else {
            Blade::None
        }
    }

    fn strike_p(&self) -> f32 {
        0.18 + self.skill as f32 * 0.072
    }
    fn parry_p(&self) -> f32 {
        0.12 + self.skill as f32 * 0.082
    }
    fn react(&self) -> f32 {
        (0.78 - self.skill as f32 * 0.058).max(0.16)
    }
    fn walk_speed(&self) -> f32 {
        match self.kind {
            MobKind::Fat => 26.0,
            MobKind::Jaffar => 46.0,
            MobKind::Skeleton => 40.0,
            _ => 34.0,
        }
    }
}

impl Game {
    pub fn update_guards(&mut self, dt: f32) {
        let pl = self.player;
        let player_alive = !matches!(self.phase, Phase::Dying(_) | Phase::Dead);
        let n = self.guards.len();
        for i in 0..n {
            let mut g = self.guards[i];
            g.t += dt;
            g.blend_t += dt;
            g.cool = (g.cool - dt).max(0.0);
            g.stagger = (g.stagger - dt).max(0.0);
            g.facing_vis = crate::util::approach(g.facing_vis, g.facing, dt / 0.075);
            let clip_total = {
                let (c, r) = g.clip();
                c.total() / r.abs().max(0.01)
            };
            let done = g.t >= clip_total;

            if g.st == GState::Dead {
                // Sink onto the floor and stay there.
                let (tx, ty) = g.foot_tile();
                if !self.supported(tx, ty) {
                    g.v.y += GRAVITY * dt;
                    g.p.y += g.v.y * dt;
                }
                self.guards[i] = g;
                continue;
            }

            // ---- gravity / floors gone from under it ----------------------
            let (tx, ty) = g.foot_tile();
            if !self.supported(tx, ty) {
                if g.st != GState::Falling {
                    g.enter(GState::Falling);
                }
                g.v.y += GRAVITY * dt;
                let y1 = g.p.y + g.v.y * dt;
                let ty1 = Level::ty_of_feet(y1);
                let mut landed = None;
                for t in ty..=ty1.max(ty) {
                    let s = Level::surf(t);
                    if s >= g.p.y - 0.01 && s <= y1 && self.supported(tx, t) {
                        landed = Some((s, t));
                        break;
                    }
                }
                match landed {
                    Some((s, t)) => {
                        g.p.y = s;
                        g.v.y = 0.0;
                        g.enter(GState::Idle);
                        let floor = s;
                        self.fx.dust(v2(g.p.x, s), 8, 1.0, self.lv.theme.slab_face);
                        if self.lethal(tx, t) {
                            g.hp = 0;
                            g.enter(GState::Dead);
                            self.fx.blood(v2(g.p.x, s - 12.0), 1.0, 18, floor);
                        }
                    }
                    None => {
                        g.p.y = y1;
                        if g.p.y > (self.lv.th + 2) as f32 * TILE_H {
                            g.hp = 0;
                            g.st = GState::Dead;
                        }
                    }
                }
                self.guards[i] = g;
                continue;
            }
            if self.lethal(tx, ty) {
                g.hp = 0;
                g.enter(GState::Dead);
                self.dy.set_flag(tx, ty, F_BLOODY, true);
                self.fx.blood(v2(g.p.x, g.p.y - 12.0), 1.0, 20, g.p.y);
                self.guards[i] = g;
                continue;
            }

            // ---- awareness ------------------------------------------------
            let same_row = (g.p.y - pl.p.y).abs() < TILE_H * 0.6;
            let dx = pl.p.x - g.p.x;
            let dist = dx.abs();
            let same_room = Level::room_of(tx, ty) == Level::room_of(pl.foot_tile().0, pl.foot_tile().1);
            let engaged = player_alive && same_row && same_room && dist < TILE_W * 3.4;
            if engaged {
                g.alert = (g.alert + dt).min(3.0);
            } else {
                g.alert = (g.alert - dt * 0.5).max(0.0);
            }

            // ---- the shadow is not an enemy unless you make it one --------
            if g.kind == MobKind::Shadow && !pl.armed && dist < 15.0 && same_row && player_alive {
                self.merge_shadow(i);
                continue;
            }

            if engaged {
                g.facing = if dx >= 0.0 { 1.0 } else { -1.0 };
                let reach = g.melee().reach();
                match g.st {
                    GState::Idle | GState::Patrol | GState::Ready | GState::Falling => {
                        g.st = GState::Ready;
                        if g.cool <= 0.0 {
                            g.cool = g.react() * self.rng.range(0.7, 1.3);
                            if dist > reach + 6.0 {
                                g.enter(GState::Advance);
                            } else if pl.st == crate::game::PState::Strike
                                && self.rng.chance(g.parry_p())
                            {
                                g.enter(GState::Parry);
                            } else if self.rng.chance(g.strike_p()) {
                                g.enter(GState::Strike);
                                g.struck = false;
                            } else if dist < reach * 0.7 && self.rng.chance(0.3) {
                                g.enter(GState::Retreat);
                            }
                        }
                    }
                    GState::Advance | GState::Retreat => {
                        let sign = if g.st == GState::Advance {
                            g.facing
                        } else {
                            -g.facing
                        };
                        let f = (g.t / clip_total).min(1.0);
                        let sp = (f * std::f32::consts::PI).sin() * g.walk_speed() * 1.5;
                        let nx = g.p.x + sign * sp * dt;
                        let ntx = Level::tx_of(nx);
                        if self.supported(ntx, ty) && self.open(ntx, ty) && !self.lethal(ntx, ty) {
                            g.p.x = nx;
                        }
                        if done {
                            g.enter(GState::Ready);
                        }
                    }
                    GState::Strike => {
                        let f = g.t / clip_total;
                        if !g.struck && (0.30..0.62).contains(&f) {
                            g.struck = true;
                            self.guards[i] = g;
                            self.resolve_guard_strike(i);
                            g = self.guards[i];
                        }
                        if done {
                            g.enter(GState::Ready);
                        }
                    }
                    GState::Parry => {
                        if done {
                            g.enter(GState::Ready);
                        }
                    }
                    GState::Hurt => {
                        if g.stagger <= 0.0 && done {
                            g.enter(GState::Ready);
                        }
                        let nx = g.p.x - g.facing * 24.0 * dt;
                        let ntx = Level::tx_of(nx);
                        if self.supported(ntx, ty) && self.open(ntx, ty) {
                            g.p.x = nx;
                        }
                    }
                    GState::Dead => {}
                }
            } else {
                // ---- patrol -----------------------------------------------
                match g.st {
                    GState::Hurt => {
                        if done {
                            g.enter(GState::Idle);
                        }
                    }
                    GState::Patrol => {
                        let sp = g.walk_speed() * 0.6;
                        let nx = g.p.x + g.dir * sp * dt;
                        let ntx = Level::tx_of(nx + g.dir * BODY_HW);
                        let ok = self.supported(ntx, ty)
                            && self.open(ntx, ty)
                            && !self.lethal(ntx, ty)
                            && (nx - g.home.x).abs() < TILE_W * 1.8;
                        if ok {
                            g.gait += (nx - g.p.x).abs() / GUARD_STRIDE * WALK_CYCLE;
                            g.p.x = nx;
                            g.facing = g.dir;
                        } else {
                            g.dir = -g.dir;
                            g.enter(GState::Idle);
                            g.idle = self.rng.range(0.8, 2.4);
                        }
                    }
                    _ => {
                        g.st = GState::Idle;
                        g.idle -= dt;
                        if g.idle <= 0.0 {
                            g.idle = self.rng.range(1.4, 3.6);
                            if self.kind_patrols(g.kind) {
                                g.enter(GState::Patrol);
                                g.dir = if self.rng.chance(0.5) { -1.0 } else { 1.0 };
                            }
                        }
                    }
                }
            }
            self.guards[i] = g;
        }

        // Clear guards that fell out of the world.
        self.guards.retain(|g| g.p.y < (self.lv.th + 4) as f32 * TILE_H);
    }

    fn kind_patrols(&self, k: MobKind) -> bool {
        !matches!(k, MobKind::Jaffar | MobKind::Shadow)
    }

    fn merge_shadow(&mut self, i: usize) {
        if i >= self.guards.len() {
            return;
        }
        let at = self.guards[i].p;
        self.guards[i].st = GState::Dead;
        self.guards[i].hp = 0;
        self.guards[i].p.y = -9999.0;
        self.player.hp_max += 1;
        self.player.hp = self.player.hp_max;
        self.carry = self.player.carry();
        self.flash = (1.0, rgb(70, 60, 130));
        self.fx.sparks(v2(at.x, at.y - 18.0), 40, 1.6);
        self.say("Vous n'êtes qu'un. Ta vigueur grandit.", 4.0, false);
    }

    fn resolve_guard_strike(&mut self, gi: usize) {
        let g = self.guards[gi];
        let pl = self.player;
        if matches!(self.phase, Phase::Dying(_) | Phase::Dead) {
            return;
        }
        let reach = g.melee().reach();
        let dx = (pl.p.x - g.p.x) * g.facing;
        let tip = v2(g.p.x + g.facing * reach, g.p.y - 18.0);
        crate::game::render::mark_slash(self, tip, g.facing);
        if !(dx > 0.0 && dx < reach + 8.0) || (pl.p.y - g.p.y).abs() > TILE_H * 0.6 {
            return;
        }
        // Parry: an explicit block always works if you face the blow; a buckler
        // gives you a chance even when you are not blocking.
        let facing_it = pl.facing != g.facing;
        let blocked = if pl.st == crate::game::PState::Parry && facing_it {
            true
        } else if pl.buckler && facing_it {
            self.rng.chance(0.4)
        } else {
            false
        };
        if blocked && !self.rng.chance(g.melee().pierce()) {
            let at = v2((pl.p.x + g.p.x) * 0.5, pl.p.y - 19.0);
            self.fx.sparks(at, 20, 1.7);
            self.cam.shake = 0.4;
            self.guards[gi].cool = 0.22;
            self.say("Paré !", 0.8, false);
            return;
        }
        self.hurt_player(g.melee().damage(), g.facing);
    }

    /// Health bar fraction for the HUD.
    pub fn boss(&self) -> Option<(&'static str, f32)> {
        for g in &self.guards {
            if g.st == GState::Dead {
                continue;
            }
            if matches!(g.kind, MobKind::Jaffar) {
                return Some((
                    g.kind.name(),
                    clampf(g.hp as f32 / g.hp_max as f32, 0.0, 1.0),
                ));
            }
        }
        None
    }
}
