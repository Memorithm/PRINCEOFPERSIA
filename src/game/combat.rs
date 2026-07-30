//! Weapons, projectiles and damage resolution.

use crate::art::skel::Blade;
use crate::game::{Game, Phase};
use crate::gfx::color::{rgb, Rgb};
use crate::util::{v2, V2};
use crate::world::level::Level;
use crate::world::tile::*;

/// The melee weapon currently in hand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Melee {
    /// No blade — you can only run.
    None,
    Sword,
    /// Bonus weapon: slower, but cleaves through a guard's guard.
    Scimitar,
}

impl Melee {
    pub fn damage(self) -> i32 {
        match self {
            Melee::None => 0,
            Melee::Sword => 1,
            Melee::Scimitar => 2,
        }
    }

    /// How far in front of the hips the tip reaches.
    pub fn reach(self) -> f32 {
        match self {
            Melee::None => 0.0,
            Melee::Sword => 25.0,
            Melee::Scimitar => 29.0,
        }
    }

    /// Multiplier on the strike animation's speed.
    pub fn swing(self) -> f32 {
        match self {
            Melee::Scimitar => 0.78,
            _ => 1.0,
        }
    }

    /// Chance a defender's parry fails against this weapon.
    pub fn pierce(self) -> f32 {
        match self {
            Melee::Scimitar => 0.35,
            _ => 0.0,
        }
    }

    pub fn blade(self) -> Blade {
        match self {
            Melee::None => Blade::None,
            Melee::Sword => Blade::Sword,
            Melee::Scimitar => Blade::Scimitar,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Melee::None => "à mains nues",
            Melee::Sword => "épée",
            Melee::Scimitar => "cimeterre",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    Dagger,
    Fireball,
}

#[derive(Clone, Copy)]
pub struct Shot {
    pub kind: ShotKind,
    pub p: V2,
    pub v: V2,
    pub life: f32,
    pub spin: f32,
    /// True if the prince threw it.
    pub friendly: bool,
}

impl ShotKind {
    pub fn damage(self) -> i32 {
        match self {
            ShotKind::Dagger => 1,
            ShotKind::Fireball => 2,
        }
    }
    pub fn speed(self) -> f32 {
        match self {
            ShotKind::Dagger => 210.0,
            ShotKind::Fireball => 150.0,
        }
    }
    pub fn radius(self) -> f32 {
        match self {
            ShotKind::Dagger => 3.0,
            ShotKind::Fireball => 4.6,
        }
    }
    pub fn colour(self) -> Rgb {
        match self {
            ShotKind::Dagger => rgb(220, 226, 234),
            ShotKind::Fireball => rgb(255, 168, 60),
        }
    }
}

impl Game {
    pub fn spawn_shot(&mut self, kind: ShotKind, from: V2, dir: f32, friendly: bool) {
        self.shots.push(Shot {
            kind,
            p: from,
            v: v2(dir * kind.speed(), if kind == ShotKind::Dagger { -8.0 } else { 0.0 }),
            life: 2.6,
            spin: 0.0,
            friendly,
        });
    }

    pub fn update_shots(&mut self, dt: f32) {
        let mut hits: Vec<(usize, Option<usize>, V2)> = Vec::new();
        let mut player_hits: Vec<(usize, V2, f32)> = Vec::new();

        for s in self.shots.iter_mut() {
            s.life -= dt;
            s.spin += dt * 22.0 * s.v.x.signum();
            if s.kind == ShotKind::Dagger {
                s.v.y += 120.0 * dt;
            }
            s.p.x += s.v.x * dt;
            s.p.y += s.v.y * dt;
        }

        // Terrain and character collisions.
        let shots = self.shots.clone();
        for (i, s) in shots.iter().enumerate() {
            let tx = Level::tx_of(s.p.x);
            let ty = Level::ty_of(s.p.y);
            let solid = !self.open(tx, ty);
            if solid || s.life <= 0.0 {
                hits.push((i, None, s.p));
                continue;
            }
            if s.friendly {
                for (gi, g) in self.guards.iter().enumerate() {
                    if g.st == crate::game::GState::Dead {
                        continue;
                    }
                    if (g.p.x - s.p.x).abs() < 11.0 && (g.p.y - 15.0 - s.p.y).abs() < 17.0 {
                        hits.push((i, Some(gi), s.p));
                        break;
                    }
                }
            } else {
                let pl = &self.player;
                if (pl.p.x - s.p.x).abs() < 10.0 && (pl.p.y - 15.0 - s.p.y).abs() < 16.0 {
                    player_hits.push((i, s.p, s.v.x.signum()));
                }
            }
        }

        // Resolve, from the end so indices stay valid.
        let mut dead: Vec<usize> = Vec::new();
        for (i, target, at) in hits {
            let kind = self.shots[i].kind;
            dead.push(i);
            match kind {
                ShotKind::Fireball => {
                    self.fx.sparks(at, 16, 1.5);
                    self.cam.shake = 0.4;
                }
                ShotKind::Dagger => {
                    self.fx.sparks(at, 5, 0.7);
                }
            }
            if let Some(gi) = target {
                let d = kind.damage();
                self.damage_guard(gi, d, self.shots[i].v.x.signum(), true);
            }
        }
        for (i, at, dir) in player_hits {
            let kind = self.shots[i].kind;
            dead.push(i);
            self.fx.sparks(at, 8, 1.0);
            // A buckler turns projectiles aside outright.
            if self.player.buckler && self.player.facing != dir {
                self.say("Le bouclier dévie le trait !", 1.6, false);
            } else {
                self.hurt_player(kind.damage(), dir);
            }
        }
        dead.sort_unstable();
        dead.dedup();
        for i in dead.into_iter().rev() {
            if i < self.shots.len() {
                self.shots.remove(i);
            }
        }
        // Fireballs trail flame.
        let trails: Vec<(V2, Rgb)> = self
            .shots
            .iter()
            .filter(|s| s.kind == ShotKind::Fireball)
            .map(|s| (s.p, rgb(255, 150, 50)))
            .collect();
        for (p, c) in trails {
            self.fx.flame(p, 6.0, 0.8, c);
        }
    }

    /// Apply damage to a guard; `throw` marks ranged hits, which cannot be parried.
    pub fn damage_guard(&mut self, gi: usize, dmg: i32, dir: f32, throw: bool) {
        if gi >= self.guards.len() {
            return;
        }
        let floor = {
            let g = self.guards[gi];
            g.p.y
        };
        let g = &mut self.guards[gi];
        if g.st == crate::game::GState::Dead {
            return;
        }
        g.hp -= dmg;
        g.stagger = 0.34;
        g.facing = -dir;
        let at = v2(g.p.x, g.p.y - 17.0);
        let bones = g.kind == MobKind::Skeleton;
        if g.hp <= 0 {
            g.st = crate::game::GState::Dead;
            g.t = 0.0;
            self.kills += 1;
            let name = g.kind.name();
            self.say(&format!("{} vaincu !", name), 1.8, false);
        } else {
            g.st = crate::game::GState::Hurt;
            g.t = 0.0;
        }
        if bones {
            self.fx.debris(at, 8, rgb(222, 216, 196), floor);
        } else {
            self.fx.blood(at, -dir, if dmg > 1 { 16 } else { 10 }, floor);
        }
        self.fx.sparks(at, 6, 0.8);
        self.cam.shake = 0.3;
        let _ = throw;
    }

    pub fn hurt_player(&mut self, dmg: i32, dir: f32) {
        if self.player.invuln > 0.0 || self.phase != Phase::Play {
            return;
        }
        self.player.hp -= dmg;
        self.player.invuln = 0.9;
        self.player.facing = -dir;
        let at = v2(self.player.p.x, self.player.p.y - 17.0);
        let floor = self.player.p.y;
        self.fx.blood(at, -dir, 12, floor);
        self.cam.shake = 0.6;
        self.flash = (0.7, rgb(150, 20, 24));
        if self.player.hp <= 0 {
            self.kill_player("Tu es tombé sous la lame.");
        } else {
            self.player.st = crate::game::PState::Hurt;
            self.player.t = 0.0;
            self.player.v.x = -dir * 40.0;
        }
    }
}
