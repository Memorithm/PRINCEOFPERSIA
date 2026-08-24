//! The top-down adventure: state and simulation.
//!
//! A Zelda-like quest in the Prince of Persia universe. The prince explores
//! fifteen linked worlds — two mirrored realms, wildlands, dungeons and the
//! Vizir's tower — collects hearts, gems, keys and relics, gathers the four
//! fragments of the Seal of Light from the dungeon bosses, cuts through a
//! bestiary inspired by the classics, and finally faces the Vizir Ganar to
//! free Princess Zahra.

pub mod app;
pub mod render;
pub mod world;
pub mod world_data;

use crate::gfx::particles::Particles;
use crate::util::{clampf, noise1, v2, Rng, V2};
use world::{FoeKind, Pickup, Portal, ThemeName, Tile, World, LIGHT_REALM, TILE, WORLD_COUNT};

// ---------------------------------------------------------------- tuning

pub const PLAYER_SPEED: f32 = 92.0;
pub const PLAYER_RADIUS: f32 = 7.0;
/// Duration of a sword stroke, seconds.
pub const ATTACK_TIME: f32 = 0.22;
/// Recovery between strokes.
pub const ATTACK_COOLDOWN: f32 = 0.34;
/// How far in front of him the blade bites.
pub const SWORD_REACH: f32 = 15.0;
/// Radius of the bite.
pub const SWORD_ARC: f32 = 13.0;
/// Invulnerability after taking a hit.
pub const HURT_INVULN: f32 = 1.0;
pub const KNOCKBACK: f32 = 130.0;
pub const SHOT_SPEED_STONE: f32 = 95.0;
pub const SHOT_SPEED_FIRE: f32 = 78.0;

// ---------------------------------------------------------------- data

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Down,
    Up,
    Left,
    Right,
}

impl Dir {
    #[inline]
    pub fn vec(self) -> V2 {
        match self {
            Dir::Down => v2(0.0, 1.0),
            Dir::Up => v2(0.0, -1.0),
            Dir::Left => v2(-1.0, 0.0),
            Dir::Right => v2(1.0, 0.0),
        }
    }
}

/// Everything the prince keeps with him, across worlds and deaths.
#[derive(Clone)]
pub struct Inventory {
    pub hp: i32,
    pub hp_max: i32,
    pub gems: u32,
    pub keys: u32,
    pub mirror: bool,
    pub silver_sword: bool,
    /// Seal fragments pried from the dungeon bosses — 4 break the sanctuary door.
    pub seals: u32,
}

impl Default for Inventory {
    fn default() -> Self {
        Inventory {
            hp: 6,
            hp_max: 6,
            gems: 0,
            keys: 0,
            mirror: false,
            silver_sword: false,
            seals: 0,
        }
    }
}

pub struct Player {
    pub p: V2,
    pub facing: Dir,
    pub moving: bool,
    /// Walk-cycle phase.
    pub anim: f32,
    /// >0 while a stroke is live.
    pub attack_t: f32,
    pub cooldown: f32,
    pub invuln: f32,
    pub knock: V2,
}

pub struct Foe {
    pub kind: FoeKind,
    pub home: V2,
    pub p: V2,
    pub hp: i32,
    /// Facing / travel direction.
    pub dir: V2,
    /// State timer (wander tick, charge, teleport cycle…).
    pub t: f32,
    /// Secondary timer (attack cooldowns).
    pub cd: f32,
    /// Hit-flash timer.
    pub hurt_t: f32,
    pub alive: bool,
    /// Animation phase.
    pub anim: f32,
}

pub struct Shot {
    pub p: V2,
    pub v: V2,
    pub fire: bool,
    pub life: f32,
}

pub struct PickupEnt {
    pub kind: Pickup,
    pub p: V2,
    pub taken: bool,
    pub bob: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Play,
    /// Death animation running.
    Dying(f32),
    Dead,
    Victory,
}

pub struct Game {
    pub worlds: Vec<World>,
    pub cur: usize,
    pub inv: Inventory,
    pub player: Player,
    /// Live foes of the current world (rebuilt on entry).
    pub foes: Vec<Foe>,
    /// Pickups of every world, kept across visits (`taken` persists).
    pub pickups: Vec<Vec<PickupEnt>>,
    pub shots: Vec<Shot>,
    pub fx: Particles,
    pub cam: V2,
    pub phase: Phase,
    pub msg: Option<(String, f32, bool)>,
    pub rng: Rng,
    pub time: f32,
    pub elapsed: f32,
    pub deaths: u32,
    pub kills: u32,
    pub ganar_dead: bool,
    /// Grace period after a teleport before portals/mirrors retrigger.
    pub warp_grace: f32,
    /// Cooldown on repeated "sealed door" messages.
    pub seal_msg_cd: f32,
    pub flash: (f32, crate::gfx::color::Rgb),
    /// View size in art pixels; kept in step with the terminal by the app.
    pub view_w: f32,
    pub view_h: f32,
    /// Erfan's rotating hints.
    pub hint_ix: usize,
    pub hint_cd: f32,
    /// Camera shake magnitude, decays over time.
    pub cam_shake: f32,
}

impl Game {
    // ------------------------------------------------------------ setup

    pub fn new(seed: u64) -> Result<Game, world::ParseError> {
        let mut worlds = Vec::with_capacity(WORLD_COUNT);
        for ix in 0..WORLD_COUNT {
            worlds.push(World::parse(ix)?);
        }
        let pickups = worlds
            .iter()
            .map(|w| {
                w.pickups
                    .iter()
                    .map(|&(k, x, y)| PickupEnt {
                        kind: k,
                        p: v2(x, y),
                        taken: false,
                        bob: 0.0,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut g = Game {
            worlds,
            cur: LIGHT_REALM,
            inv: Inventory::default(),
            player: Player {
                p: v2(0.0, 0.0),
                facing: Dir::Down,
                moving: false,
                anim: 0.0,
                attack_t: 0.0,
                cooldown: 0.0,
                invuln: 0.0,
                knock: V2::ZERO,
            },
            foes: Vec::new(),
            pickups,
            shots: Vec::new(),
            fx: Particles::new(seed ^ 0xBEEF),
            cam: V2::ZERO,
            phase: Phase::Play,
            msg: None,
            rng: Rng::new(seed),
            time: 0.0,
            elapsed: 0.0,
            deaths: 0,
            kills: 0,
            ganar_dead: false,
            warp_grace: 0.0,
            seal_msg_cd: 0.0,
            flash: (0.0, crate::gfx::color::rgb(0, 0, 0)),
            view_w: TILE * 15.0,
            view_h: TILE * 11.0,
            hint_ix: 0,
            hint_cd: 0.0,
            cam_shake: 0.0,
        };
        g.enter_world(LIGHT_REALM, None, true);
        g.say("La Vallée d'Ispahan — retrouve la princesse Zahra !", 5.0, false);
        Ok(g)
    }

    #[inline]
    pub fn world(&self) -> &World {
        &self.worlds[self.cur]
    }

    #[inline]
    pub fn theme(&self) -> ThemeName {
        self.world().theme
    }

    pub fn say(&mut self, text: &str, t: f32, warn: bool) {
        let better = match &self.msg {
            Some((_, mt, mw)) => t > *mt || (*mw && !warn && false),
            None => true,
        };
        if better {
            self.msg = Some((text.to_string(), t, warn));
        }
    }

    /// (Re)spawn into a world: rebuild its living foes, place the prince.
    pub fn enter_world(&mut self, ix: usize, at: Option<V2>, reset_foes: bool) {
        self.cur = ix.min(WORLD_COUNT - 1);
        let w = &self.worlds[self.cur];
        let (sx, sy) = w.start;
        self.player.p = at.unwrap_or(v2(sx, sy));
        self.player.knock = V2::ZERO;
        self.player.invuln = 0.8;
        self.warp_grace = 0.7;
        if reset_foes {
            let spawns = w.spawns.clone();
            self.foes = spawns
                .into_iter()
                .filter(|&(k, _, _)| !(self.ganar_dead && k == FoeKind::Ganar))
                .map(|(kind, x, y)| Foe {
                    kind,
                    home: v2(x, y),
                    p: v2(x, y),
                    hp: kind.max_hp(),
                    dir: v2(0.0, 1.0),
                    t: self.rng.range(0.0, 1.0),
                    cd: self.rng.range(0.4, 1.6),
                    hurt_t: 0.0,
                    alive: true,
                    anim: self.rng.unit() * 10.0,
                })
                .collect();
        }
        self.shots.clear();
        self.snap_camera();
    }

    /// Send the prince back to the current world's start, fully healed.
    pub fn respawn(&mut self) {
        self.inv.hp = self.inv.hp_max;
        self.phase = Phase::Play;
        let ix = self.cur;
        self.enter_world(ix, None, true);
    }

    // ------------------------------------------------------------ tiles

    #[inline]
    fn blocked(&self, tx: i32, ty: i32) -> bool {
        self.world().tile(tx, ty).solid()
    }

    /// Circle-vs-grid collision resolve for one axis; `p` is updated in place.
    fn try_move(&self, p: &mut V2, dx: f32, dy: f32) {
        let r = PLAYER_RADIUS;
        let cand = p.add(v2(dx, dy));
        let (x0, x1) = ((cand.x - r).floor() as i32, (cand.x + r).floor() as i32);
        let (y0, y1) = ((cand.y - r).floor() as i32, (cand.y + r).floor() as i32);
        let mut hit = false;
        'outer: for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.blocked(tx, ty) {
                    hit = true;
                    break 'outer;
                }
            }
        }
        if !hit {
            *p = cand;
            return;
        }
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        // Slide to the wall face on the blocked axis only.
        let nx = if dx != 0.0 {
            if dx > 0.0 {
                (cand.x + r).floor() as f32 - r - 0.01
            } else {
                (cand.x - r).floor() as f32 + 1.0 + r + 0.01
            }
        } else {
            p.x
        };
        let ny = if dy != 0.0 {
            if dy > 0.0 {
                (cand.y + r).floor() as f32 - r - 0.01
            } else {
                (cand.y - r).floor() as f32 + 1.0 + r + 0.01
            }
        } else {
            p.y
        };
        let probe = v2(nx, ny);
        let (bx0, bx1) = ((probe.x - r).floor() as i32, (probe.x + r).floor() as i32);
        let (by0, by1) = ((probe.y - r).floor() as i32, (probe.y + r).floor() as i32);
        let mut bh = false;
        'o2: for ty in by0..=by1 {
            for tx in bx0..=bx1 {
                if self.blocked(tx, ty) {
                    bh = true;
                    break 'o2;
                }
            }
        }
        if !bh {
            *p = probe;
        }
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, dt: f32, input: &crate::input::Input) {
        let dt = dt.min(1.0 / 20.0);
        self.time += dt;
        self.elapsed += dt;

        match self.phase {
            Phase::Play => {
                self.update_player(dt, input);
                self.update_foes(dt);
                self.update_shots(dt);
                self.check_pickups();
                self.check_tiles(input);
                if let Some(win) = self.check_npc(dt) {
                    if win {
                        self.phase = Phase::Victory;
                    }
                }
            }
            Phase::Dying(ref mut t) => {
                *t -= dt;
                if *t <= 0.0 {
                    self.phase = Phase::Dead;
                }
            }
            _ => {}
        }

        self.fx.update(dt);
        self.update_camera(dt);
        self.warp_grace = (self.warp_grace - dt).max(0.0);
        self.seal_msg_cd = (self.seal_msg_cd - dt).max(0.0);
        self.hint_cd = (self.hint_cd - dt).max(0.0);
        if let Some((_, t, _)) = &mut self.msg {
            *t -= dt;
            if *t <= 0.0 {
                self.msg = None;
            }
        }
        self.flash.0 = (self.flash.0 - dt * 2.2).max(0.0);
    }

    fn update_player(&mut self, dt: f32, input: &crate::input::Input) {
        // Take the player out so the world can be borrowed freely.
        let mut pl = std::mem::replace(
            &mut self.player,
            Player {
                p: V2::ZERO,
                facing: Dir::Down,
                moving: false,
                anim: 0.0,
                attack_t: 0.0,
                cooldown: 0.0,
                invuln: 0.0,
                knock: V2::ZERO,
            },
        );

        // Timers.
        pl.attack_t = (pl.attack_t - dt).max(0.0);
        pl.cooldown = (pl.cooldown - dt).max(0.0);
        pl.invuln = (pl.invuln - dt).max(0.0);

        // Knockback decays fast.
        pl.knock = pl.knock.mul((-dt * 9.0).exp());
        if pl.knock.len() > 4.0 {
            let k = pl.knock.mul(dt);
            self.try_move(&mut pl.p, k.x, 0.0);
            self.try_move(&mut pl.p, 0.0, k.y);
        }

        // Movement — 8 directions, facing snaps to the dominant axis.
        let mx = (input.right as i32 - input.left as i32) as f32;
        let my = (input.down as i32 - input.up as i32) as f32;
        let want = v2(mx, my);
        pl.moving = want.len() > 0.01 && pl.attack_t <= ATTACK_TIME * 0.45;
        if pl.moving {
            let d = want.norm().mul(PLAYER_SPEED * dt);
            if mx.abs() >= my.abs() && mx != 0.0 {
                pl.facing = if mx > 0.0 { Dir::Right } else { Dir::Left };
            } else if my != 0.0 {
                pl.facing = if my > 0.0 { Dir::Down } else { Dir::Up };
            }
            self.try_move(&mut pl.p, d.x, 0.0);
            self.try_move(&mut pl.p, 0.0, d.y);
            pl.anim += dt * 9.0;
            // Footstep dust.
            if self.rng.chance(dt * 6.0) {
                let col = match self.theme() {
                    ThemeName::Valley | ThemeName::Shadow | ThemeName::Forest
                    | ThemeName::Swamp | ThemeName::Oasis => {
                        crate::gfx::color::rgb(150, 128, 96)
                    }
                    ThemeName::Desert | ThemeName::Pass => crate::gfx::color::rgb(190, 165, 115),
                    _ => crate::gfx::color::rgb(120, 110, 100),
                };
                self.fx.dust(v2(pl.p.x, pl.p.y + 6.0), 1, 0.35, col);
            }
        }

        let swung = input.attack && pl.cooldown <= 0.0;
        if swung {
            pl.attack_t = ATTACK_TIME;
            pl.cooldown = ATTACK_COOLDOWN;
        }

        // Put him back before anything that needs `&mut self`.
        self.player = pl;
        if swung {
            self.resolve_sword();
        }
    }

    /// One sword stroke against everything in the arc.
    fn resolve_sword(&mut self) {
        let dmg = if self.inv.silver_sword { 2 } else { 1 };
        let f = self.player.facing.vec();
        let tip = self.player.p.add(f.mul(SWORD_REACH));

        // Pass 1 — find what the stroke touches (immutable).
        let mut hits: Vec<(usize, bool)> = Vec::new();
        for (i, foe) in self.foes.iter().enumerate() {
            if !foe.alive {
                continue;
            }
            let d = foe.p.sub(tip).len();
            let near = foe.p.sub(self.player.p).len();
            if d < SWORD_ARC || near < SWORD_ARC + 4.0 {
                // Darknuts parry what comes from straight ahead.
                let frontal = f.x * foe.dir.x + f.y * foe.dir.y < -0.55
                    && matches!(foe.kind, FoeKind::Knight | FoeKind::BossKnight);
                hits.push((i, frontal));
            }
        }

        // Pass 2 — apply.
        for (i, blocked) in hits {
            let foe = &mut self.foes[i];
            if blocked {
                self.fx.sparks(foe.p.add(v2(0.0, -2.0)), 6, 0.7);
                self.fx
                    .dust(foe.p, 3, 0.5, crate::gfx::color::rgb(200, 200, 210));
                continue;
            }
            foe.hp -= dmg;
            foe.hurt_t = 0.18;
            foe.dir = f.mul(-1.0); // stagger backwards
            let heavy = dmg > 1;
            self.fx.sparks(
                foe.p.add(v2(0.0, -4.0)),
                if heavy { 12 } else { 8 },
                1.0,
            );
            if foe.hp <= 0 {
                self.kill_foe(i);
            }
        }
    }

    fn kill_foe(&mut self, ix: usize) {
        let (kind, fp) = {
            let foe = &self.foes[ix];
            (foe.kind, foe.p)
        };
        self.foes[ix].alive = false;
        self.kills += 1;
        self.fx.sparks(fp, 16, 1.4);
        self.fx
            .dust(fp, 8, 1.0, crate::gfx::color::rgb(160, 150, 140));
        match kind {
            FoeKind::Ogre => {
                self.inv.seals = self.inv.seals.max(1);
                self.grow_heart();
                self.say(
                    "La Brute s'effondre — fragment du sceau 1/4 ! (+1 cœur)",
                    4.5,
                    false,
                );
                self.flash = (0.5, crate::gfx::color::rgb(240, 200, 90));
            }
            FoeKind::BossKnight => {
                if self.cur == world::PALACE {
                    self.inv.seals = self.inv.seals.max(2);
                    self.grow_heart();
                    self.say(
                        "Le Garde royal tombe — fragment du sceau 2/4 ! (+1 cœur)",
                        4.5,
                        false,
                    );
                    self.flash = (0.5, crate::gfx::color::rgb(240, 200, 90));
                } else {
                    // Elite guard of the tower: no fragment, just riches.
                    self.grow_heart();
                    self.inv.gems += 10;
                    self.say("Le garde d'élite tombe ! (+1 cœur, +10 gemmes)", 4.0, false);
                }
            }
            FoeKind::CrystalGolem => {
                self.inv.seals = self.inv.seals.max(3);
                self.grow_heart();
                self.say(
                    "Le Golem se fissure — fragment du sceau 3/4 ! (+1 cœur)",
                    4.5,
                    false,
                );
                self.flash = (0.5, crate::gfx::color::rgb(160, 220, 255));
                self.cam_shake = 0.9;
            }
            FoeKind::Pharaoh => {
                self.inv.seals = 4;
                self.grow_heart();
                self.say(
                    "LE SCEAU EST RECOMPOSÉ (4/4) ! Direction le Sanctuaire…",
                    5.0,
                    false,
                );
                self.flash = (0.7, crate::gfx::color::rgb(255, 215, 110));
                self.cam_shake = 0.9;
            }
            FoeKind::Ganar => {
                self.ganar_dead = true;
                self.say(
                    "GANAR EST DÉTRUIT ! La princesse est libérée…",
                    6.0,
                    false,
                );
                self.flash = (0.9, crate::gfx::color::rgb(255, 230, 120));
                self.cam_shake = 1.2;
            }
            _ => {
                if self.rng.chance(0.22) {
                    // Slain foes sometimes leave a heart or a gem behind.
                    let kind = if self.rng.chance(0.5) {
                        Pickup::Heart
                    } else {
                        Pickup::Gem
                    };
                    self.pickups[self.cur].push(PickupEnt {
                        kind,
                        p: fp,
                        taken: false,
                        bob: 0.0,
                    });
                }
            }
        }
    }

    fn grow_heart(&mut self) {
        self.inv.hp_max += 2;
        self.inv.hp = self.inv.hp_max;
    }

    fn update_foes(&mut self, dt: f32) {
        let pl_p = self.player.p;
        let player_invuln = self.player.invuln > 0.0;

        // Snapshot of the solid grid so the AI pass never borrows the world.
        let w = self.world();
        let solid: Vec<bool> = w.tiles.iter().map(|t| t.solid()).collect();
        let stw = w.tw;
        let sth = w.th;
        let solid_at = |tx: i32, ty: i32| -> bool {
            if tx < 0 || ty < 0 || tx >= stw || ty >= sth {
                true
            } else {
                solid[(ty * stw + tx) as usize]
            }
        };
        let free = |p: V2, r: f32| -> bool {
            let c = [
                (p.x - r, p.y - r),
                (p.x + r, p.y - r),
                (p.x - r, p.y + r),
                (p.x + r, p.y + r),
            ];
            c.iter().all(|&(x, y)| !solid_at((x / TILE).floor() as i32, (y / TILE).floor() as i32))
        };

        let mut pending_shots: Vec<Shot> = Vec::new();
        let mut contacts: Vec<(i32, V2)> = Vec::new();
        let mut spawn_bats_at: Vec<V2> = Vec::new();

        for foe in self.foes.iter_mut() {
            if !foe.alive {
                continue;
            }
            foe.anim += dt * (if foe.kind == FoeKind::Bat { 14.0 } else { 6.0 });
            foe.hurt_t = (foe.hurt_t - dt).max(0.0);
            foe.cd -= dt;
            foe.t -= dt;
            let to_pl = pl_p.sub(foe.p);
            let dist = to_pl.len();

            let speed = match foe.kind {
                FoeKind::Crab => 26.0,
                FoeKind::Grunt => 24.0,
                FoeKind::Bat => 46.0,
                FoeKind::Knight => 30.0,
                FoeKind::Ogre => 40.0,
                FoeKind::BossKnight => 26.0,
                FoeKind::CrystalGolem => 17.0,
                FoeKind::Pharaoh => 34.0,
                FoeKind::Djinn | FoeKind::Ganar => 0.0,
            };

            let mut vel = V2::ZERO;
            match foe.kind {
                FoeKind::Crab => {
                    // Wander in cardinal hops; spit when aligned.
                    if foe.t <= 0.0 {
                        foe.t = self.rng.range(0.6, 1.6);
                        let dirs = [v2(1.0, 0.0), v2(-1.0, 0.0), v2(0.0, 1.0), v2(0.0, -1.0)];
                        foe.dir = if self.rng.chance(0.25) {
                            V2::ZERO
                        } else {
                            dirs[self.rng.irange(0, 3) as usize]
                        };
                    }
                    vel = foe.dir.mul(speed);
                    if foe.cd <= 0.0
                        && dist < 150.0
                        && (to_pl.x.abs() < 12.0 || to_pl.y.abs() < 12.0)
                    {
                        foe.cd = self.rng.range(2.0, 3.2);
                        let d = to_pl.norm();
                        pending_shots.push(Shot {
                            p: foe.p.add(d.mul(6.0)),
                            v: d.mul(SHOT_SPEED_STONE),
                            fire: false,
                            life: 2.2,
                        });
                    }
                }
                FoeKind::Grunt | FoeKind::Ogre => {
                    let aggro = dist < if foe.kind == FoeKind::Ogre { 150.0 } else { 110.0 };
                    if aggro {
                        if foe.t <= 0.0 {
                            foe.t = self.rng.range(0.35, 0.6);
                            foe.dir = to_pl.norm();
                        }
                        vel = foe.dir.mul(if foe.kind == FoeKind::Ogre {
                            speed
                        } else {
                            speed * 2.2
                        });
                    } else if foe.t <= 0.0 {
                        foe.t = self.rng.range(0.8, 1.8);
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        foe.dir = v2(a.sin(), a.cos());
                        if self.rng.chance(0.3) {
                            foe.dir = V2::ZERO;
                        }
                        vel = foe.dir.mul(speed);
                    }
                }
                FoeKind::Bat => {
                    // Homing with a sinusoidal wobble.
                    if dist < 190.0 && dist > 1.0 {
                        let wob = foe
                            .p
                            .sub(pl_p)
                            .norm()
                            .perp()
                            .mul((foe.anim * 0.9).sin() * 0.8);
                        vel = to_pl.norm().add(wob).norm().mul(speed);
                    } else if foe.t <= 0.0 {
                        foe.t = self.rng.range(0.7, 1.4);
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        foe.dir = v2(a.sin(), a.cos());
                    }
                    if vel.len() < 1.0 {
                        vel = foe.dir.mul(speed * 0.6);
                    }
                }
                FoeKind::Djinn => {
                    // Anchored in the water; spits fireballs.
                    if foe.cd <= 0.0 && dist < 170.0 {
                        foe.cd = self.rng.range(2.4, 3.4);
                        let d = to_pl.norm();
                        pending_shots.push(Shot {
                            p: foe.p.add(v2(0.0, -6.0)),
                            v: d.mul(SHOT_SPEED_FIRE),
                            fire: true,
                            life: 3.0,
                        });
                    }
                }
                FoeKind::Knight | FoeKind::BossKnight => {
                    // March toward the player, dominant axis first.
                    if dist > 2.0 {
                        if to_pl.x.abs() > to_pl.y.abs() {
                            vel = v2(to_pl.x.signum(), 0.0).mul(speed);
                        } else {
                            vel = v2(0.0, to_pl.y.signum()).mul(speed);
                        }
                        foe.dir = vel.norm();
                    }
                    if foe.kind == FoeKind::BossKnight && foe.cd <= 0.0 && dist < 130.0 {
                        foe.cd = self.rng.range(2.6, 3.6);
                        foe.t = 0.35;
                    }
                    if foe.kind == FoeKind::BossKnight && foe.t > 0.0 {
                        // Lunge!
                        vel = foe.dir.mul(speed * 3.4);
                    }
                }
                FoeKind::CrystalGolem => {
                    // Slow relentless chase; slams crystal shards outward.
                    let aggro = dist < 170.0;
                    if aggro {
                        if foe.t <= 0.0 {
                            foe.t = self.rng.range(0.5, 0.8);
                            foe.dir = to_pl.norm();
                        }
                        vel = foe.dir.mul(speed);
                    }
                    if foe.cd <= 0.0 && dist < 120.0 {
                        foe.cd = self.rng.range(3.2, 4.2);
                        self.cam_shake = 0.6;
                        for k in 0..8 {
                            let a = (k as f32) * std::f32::consts::TAU / 8.0;
                            pending_shots.push(Shot {
                                p: foe.p.add(v2(a.sin(), a.cos()).mul(7.0)),
                                v: v2(a.sin(), a.cos()).mul(SHOT_SPEED_STONE),
                                fire: false,
                                life: 1.6,
                            });
                        }
                    }
                }
                FoeKind::Pharaoh => {
                    // Drifts like a mummy in the draft, blinks, calls bats.
                    let aggro = dist < 180.0;
                    if aggro && foe.t <= 0.0 {
                        foe.t = self.rng.range(0.4, 0.7);
                        foe.dir = to_pl.norm();
                    }
                    if aggro {
                        vel = foe.dir.mul(speed);
                    } else if foe.t <= 0.0 {
                        foe.t = self.rng.range(0.9, 1.6);
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        foe.dir = v2(a.sin(), a.cos());
                    }
                    if foe.cd <= 0.0 && dist < 160.0 {
                        foe.cd = self.rng.range(2.2, 3.0);
                        // Blink closer to his home, then curse a fireball.
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        let target = foe.home.add(v2(
                            a.sin() * TILE * 2.2,
                            a.cos() * TILE * 1.6,
                        ));
                        if free(target, 8.0) {
                            self.fx.sparks(foe.p, 10, 0.8);
                            foe.p = target;
                            self.fx.sparks(target, 10, 0.8);
                        }
                        pending_shots.push(Shot {
                            p: foe.p.add(v2(0.0, -6.0)),
                            v: to_pl.norm().mul(SHOT_SPEED_FIRE),
                            fire: true,
                            life: 2.6,
                        });
                        if self.rng.chance(0.35) {
                            spawn_bats_at.push(foe.p.add(v2(TILE, 0.0)));
                        }
                    }
                }
                FoeKind::Ganar => {
                    if foe.t <= 0.0 {
                        foe.t = 3.0;
                        // Teleport around his throne, then fan the flames.
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        let r = self.rng.range(TILE * 1.5, TILE * 4.0);
                        let target = foe.home.add(v2(a.sin() * r, a.cos() * r * 0.7));
                        if free(target, 8.0) {
                            foe.p = target;
                        }
                        let base = to_pl.norm();
                        for ang in [-0.42f32, 0.0, 0.42] {
                            let ca = ang.cos();
                            let sa = ang.sin();
                            let d = v2(base.x * ca - base.y * sa, base.x * sa + base.y * ca);
                            pending_shots.push(Shot {
                                p: foe.p.add(d.mul(8.0)).add(v2(0.0, -6.0)),
                                v: d.mul(SHOT_SPEED_FIRE),
                                fire: true,
                                life: 3.2,
                            });
                        }
                        if self.rng.chance(0.45) {
                            spawn_bats_at.push(foe.p.add(v2(0.0, -TILE)));
                            spawn_bats_at.push(foe.p.add(v2(0.0, TILE)));
                        }
                    }
                }
            }

            if vel.len() > 0.5 {
                let nx = foe.p.x + vel.x * dt;
                if free(v2(nx, foe.p.y), 6.0) {
                    foe.p.x = nx;
                } else if foe.kind == FoeKind::Bat {
                    foe.dir.x = -foe.dir.x;
                }
                let ny = foe.p.y + vel.y * dt;
                if free(v2(foe.p.x, ny), 6.0) {
                    foe.p.y = ny;
                } else if foe.kind == FoeKind::Bat {
                    foe.dir.y = -foe.dir.y;
                }
            }

            // Contact damage intent.
            if dist < PLAYER_RADIUS + 7.0 && !player_invuln {
                contacts.push((foe.kind.touch_damage(), pl_p.sub(foe.p).norm()));
            }
        }

        // Pass 2 — effects that need `&mut self`.
        self.shots.extend(pending_shots);
        for (dmg, away) in contacts {
            if self.player.invuln <= 0.0 {
                self.hurt_player(dmg, away);
            }
        }
        for at in spawn_bats_at {
            let bats_alive = self
                .foes
                .iter()
                .filter(|f| f.alive && f.kind == FoeKind::Bat)
                .count();
            if bats_alive < 4 {
                self.foes.push(Foe {
                    kind: FoeKind::Bat,
                    home: at,
                    p: at,
                    hp: 1,
                    dir: v2(0.0, 1.0),
                    t: 1.0,
                    cd: 1.0,
                    hurt_t: 0.0,
                    alive: true,
                    anim: 0.0,
                });
                self.fx.sparks(at, 8, 0.9);
            }
        }
    }

    fn hurt_player(&mut self, dmg: i32, away: V2) {
        if self.phase != Phase::Play || self.player.invuln > 0.0 {
            return;
        }
        self.inv.hp -= dmg;
        self.player.invuln = HURT_INVULN;
        self.player.knock = away.mul(KNOCKBACK);
        self.flash = (0.6, crate::gfx::color::rgb(180, 30, 26));
        self.fx.sparks(self.player.p, 10, 1.0);
        self.cam_shake = 0.7;
        if self.inv.hp <= 0 {
            self.inv.hp = 0;
            self.phase = Phase::Dying(1.2);
            self.deaths += 1;
        }
    }

    fn update_shots(&mut self, dt: f32) {
        for s in self.shots.iter_mut() {
            s.life -= dt;
            s.p = s.p.add(s.v.mul(dt));
            if s.fire && self.rng.chance(dt * 30.0) {
                self.fx.flame(s.p, 4.0, 0.5, crate::gfx::color::rgb(255, 160, 60));
            }
        }
        // Walls stop everything.
        let mut dead: Vec<(usize, V2)> = Vec::new();
        for (i, s) in self.shots.iter().enumerate() {
            let t = self.world().tile(world::tx_of(s.p.x), world::ty_of(s.p.y));
            if t.solid() || s.life <= 0.0 {
                dead.push((i, s.p));
            }
        }
        // Player hits.
        let mut player_hits: Vec<V2> = Vec::new();
        for (i, s) in self.shots.iter().enumerate() {
            if s.p.sub(self.player.p).len() < PLAYER_RADIUS + 3.5
                && self.player.invuln <= 0.0
                && !dead.iter().any(|(di, _)| *di == i)
            {
                dead.push((i, s.p));
                player_hits.push(self.player.p.sub(s.p).norm());
            }
        }
        for (_, at) in &dead {
            self.fx.sparks(*at, 4, 0.6);
        }
        for away in player_hits {
            self.hurt_player(1, away);
        }
        let mut k = 0;
        self.shots.retain(|_| {
            let keep = !dead.iter().any(|(di, _)| *di == k);
            k += 1;
            keep
        });
    }

    fn check_pickups(&mut self) {
        let pxy = self.player.p;
        // Which ones does he touch? (immutable pass)
        let mut grabbed: Vec<Pickup> = Vec::new();
        for pk in self.pickups[self.cur].iter_mut() {
            if !pk.taken && pk.p.sub(pxy).len() <= 11.0 {
                pk.taken = true;
                grabbed.push(pk.kind);
            }
        }
        for kind in grabbed {
            self.fx.sparks(pxy.add(v2(0.0, -4.0)), 6, 0.7);
            match kind {
                Pickup::Heart => {
                    self.inv.hp = (self.inv.hp + 2).min(self.inv.hp_max);
                    self.say("+1 cœur", 1.2, false);
                }
                Pickup::HeartContainer => {
                    self.grow_heart();
                    self.say(
                        "UN CONTENEUR DE CŒUR ! Ta vitalité grandit.",
                        4.0,
                        false,
                    );
                    self.flash = (0.6, crate::gfx::color::rgb(255, 120, 140));
                }
                Pickup::Gem => {
                    self.inv.gems += 1;
                }
                Pickup::Key => {
                    self.inv.keys += 1;
                    self.say("Une petite clé !", 1.6, false);
                }
                Pickup::SilverMirror => {
                    self.inv.mirror = true;
                    self.say(
                        "LE MIROIR D'ARGENT ! Les dalles d'argent résonnent…",
                        5.0,
                        false,
                    );
                    self.flash = (0.7, crate::gfx::color::rgb(200, 220, 255));
                }
                Pickup::SilverSword => {
                    self.inv.silver_sword = true;
                    self.say(
                        "L'ÉPÉE D'ARGENT ! Ta lame tranche deux fois plus fort.",
                        4.0,
                        false,
                    );
                    self.flash = (0.6, crate::gfx::color::rgb(220, 230, 250));
                }
            }
        }
    }

    /// Portals, mirror slabs, doors, NPC chatter.
    fn check_tiles(&mut self, _input: &crate::input::Input) {
        let p = self.player.p;
        let tx = world::tx_of(p.x);
        let ty = world::ty_of(p.y);
        let tile = self.world().tile(tx, ty);

        if self.warp_grace <= 0.0 {
            match tile {
                Tile::Portal | Tile::Cave => {
                    let dest: Option<Portal> = self.world().portal_at(tx, ty).copied();
                    if let Some(portal) = dest {
                        let label = portal.label;
                        let (dw, dtx, dty) =
                            (portal.dest_world, portal.dest_tx, portal.dest_ty);
                        self.enter_world(dw, Some(v2(world::cx(dtx), world::cy(dty))), true);
                        self.say(label, 2.6, false);
                        self.flash = (0.4, crate::gfx::color::rgb(20, 16, 30));
                        return;
                    }
                }
                Tile::MirrorSlab => {
                    if self.inv.mirror {
                        let dest = if self.cur == world::LIGHT_REALM {
                            world::DARK_REALM
                        } else {
                            world::LIGHT_REALM
                        };
                        let label = if dest == world::DARK_REALM {
                            "Le miroir te hurls dans l'OMBRE…"
                        } else {
                            "Le miroir te rend à la LUMIÈRE."
                        };
                        self.enter_world(dest, Some(v2(p.x, p.y)), true);
                        self.say(label, 3.0, false);
                        self.flash = (0.8, crate::gfx::color::rgb(190, 210, 255));
                        return;
                    } else if self.hint_cd <= 0.0 {
                        self.hint_cd = 3.0;
                        self.say("Une dalle d'argent froid… quelque chose manque.", 2.4, false);
                    }
                }
                _ => {}
            }
        }

        // Doors open when you press against them.
        if matches!(tile, Tile::DoorLocked | Tile::DoorSealed) {
            let facing_tile = self
                .world()
                .tile(tx + self.player.facing.vec().x as i32, ty + self.player.facing.vec().y as i32);
            let pressing = facing_tile == tile
                || self.player.p.sub(v2(world::cx(tx), world::cy(ty))).len() < TILE * 0.75;
            if pressing {
                match tile {
                    Tile::DoorLocked => {
                        if self.inv.keys > 0 {
                            self.inv.keys -= 1;
                            self.worlds_mut().set_tile(tx, ty, Tile::Floor);
                            self.fx.sparks(v2(world::cx(tx), world::cy(ty)), 10, 0.9);
                            self.say("La clé tourne… clic.", 2.0, false);
                        } else if self.hint_cd <= 0.0 {
                            self.hint_cd = 2.5;
                            self.say("Porte verrouillée — il faut une clé.", 2.0, true);
                        }
                    }
                    Tile::DoorSealed => {
                        if self.inv.seals >= 4 {
                            self.worlds_mut().set_tile(tx, ty, Tile::Floor);
                            self.fx.sparks(v2(world::cx(tx), world::cy(ty)), 18, 1.3);
                            self.cam_shake = 0.8;
                            self.say("Les quatre fragments vibrent… LE SCEAU SE BRISE !", 4.0, false);
                            self.flash = (0.8, crate::gfx::color::rgb(255, 215, 110));
                        } else if self.seal_msg_cd <= 0.0 {
                            self.seal_msg_cd = 3.0;
                            self.say(
                                &format!(
                                    "Le sceau résiste — fragments : {}/4",
                                    self.inv.seals
                                ),
                                2.6,
                                true,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn worlds_mut(&mut self) -> &mut World {
        &mut self.worlds[self.cur]
    }

    /// NPC interaction. Returns `true` once the victory condition fires.
    fn check_npc(&mut self, dt: f32) -> Option<bool> {
        let npc = self.world().npc?;
        let pos = v2(npc.1, npc.2);
        let d = self.player.p.sub(pos).len();
        if d < 22.0 && self.hint_cd <= 0.0 {
            match npc.0 {
                'V' => {
                    let hints = [
                        "Erfan : Ganar a enlevé la princesse Zahra !",
                        "Erfan : Le puits du village descend vers les Geôles.",
                        "Erfan : Derrière la cascade dort un passage sombre…",
                        "Erfan : Les boss des donjons gardent les 4 fragments du sceau.",
                        "Erfan : L'Épée d'argent brille au fond des Mines de Cuivre.",
                        "Erfan : Le Miroir d'argent sommeille au Palais de Sable.",
                    ];
                    self.say(hints[self.hint_ix % hints.len()], 4.5, false);
                    self.hint_ix += 1;
                    self.hint_cd = 6.0;
                }
                'F' => {
                    if self.inv.hp < self.inv.hp_max {
                        self.inv.hp = self.inv.hp_max;
                        self.fx.sparks(self.player.p.add(v2(0.0, -6.0)), 14, 1.1);
                        self.flash = (0.5, crate::gfx::color::rgb(160, 240, 220));
                        self.say("La Fée du lac murmure… tes blessures se referment.", 4.0, false);
                        self.hint_cd = 4.0;
                    } else if self.hint_cd <= 0.0 {
                        let hints = [
                            "La Fée : le Palmier Bleu veille sur les voyageurs…",
                            "La Fée : la Tour du Vizir perce les nuages au nord de l'Ombre.",
                            "La Fée : repose-toi, prince ; le lac guérit qui s'approche.",
                        ];
                        self.say(hints[self.hint_ix % hints.len()], 4.5, false);
                        self.hint_ix += 1;
                        self.hint_cd = 6.0;
                    }
                }
                'Y' => {
                    if self.ganar_dead {
                        self.say("Zahra : Mon héros ! …", 3.0, false);
                        return Some(true);
                    } else {
                        self.say(
                            "Zahra : Le sceau de Ganar me retient ici… brise-le !",
                            3.5,
                            false,
                        );
                        self.hint_cd = 5.0;
                    }
                }
                _ => {}
            }
        }
        let _ = dt;
        None
    }

    // ------------------------------------------------------------ camera

    pub fn snap_camera(&mut self) {
        self.update_camera(1000.0);
    }

    fn update_camera(&mut self, dt: f32) {
        let w = self.world();
        let target = v2(
            clampf(
                self.player.p.x - self.view_w * 0.5,
                0.0,
                (w.tw as f32 * TILE - self.view_w).max(0.0),
            ),
            clampf(
                self.player.p.y - self.view_h * 0.5,
                0.0,
                (w.th as f32 * TILE - self.view_h).max(0.0),
            ),
        );
        let k = 1.0 - (-dt * 7.0).exp();
        self.cam = self.cam.lerp(target, k);
        let s = self.cam_shake;
        let _ = s;
    }

    /// Camera offset with shake applied — used by the renderer.
    pub fn cam_draw(&self) -> V2 {
        if self.cam_shake <= 0.001 {
            return self.cam;
        }
        let n = noise1(self.time * 30.0, 3) * self.cam_shake * 3.0;
        let m = noise1(self.time * 30.0 + 91.0, 5) * self.cam_shake * 3.0;
        self.cam.add(v2(n, m))
    }

    pub fn set_view_size(&mut self, w: f32, h: f32) {
        self.view_w = w.max(TILE * 8.0);
        self.view_h = h.max(TILE * 6.0);
        self.snap_camera();
    }

    /// Ambient light colour triplet per theme, for the renderer.
    pub fn ambient(&self) -> [f32; 3] {
        match self.theme() {
            ThemeName::Valley => [1.04, 1.02, 0.97],
            ThemeName::Oasis => [0.98, 1.04, 1.00],
            ThemeName::Shadow => [0.62, 0.56, 0.74],
            ThemeName::Dungeon => [0.66, 0.60, 0.55],
            ThemeName::Palace => [0.86, 0.80, 0.70],
            ThemeName::Forest => [0.78, 0.90, 0.72],
            ThemeName::Crystal => [0.66, 0.80, 0.94],
            ThemeName::Desert => [1.08, 0.98, 0.82],
            ThemeName::Necro => [0.62, 0.62, 0.70],
            ThemeName::Swamp => [0.64, 0.74, 0.58],
            ThemeName::Pass => [0.92, 0.88, 0.84],
            ThemeName::Mines => [0.70, 0.62, 0.52],
            ThemeName::Sanctuary => [0.76, 0.82, 0.96],
            ThemeName::Tower => [0.58, 0.54, 0.62],
            ThemeName::Throne => [0.56, 0.44, 0.56],
        }
    }
}
