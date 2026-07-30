//! Game state and the main simulation step.

pub mod combat;
pub mod guard;
pub mod hud;
pub mod player;
pub mod render;

use crate::gfx::particles::Particles;
use crate::util::{clampf, v2, Rng, V2};
use crate::world::dynamics::*;
use crate::world::level::{Level, ParseError};
use crate::world::levels::CAMPAIGN;
use crate::world::tile::*;

pub use combat::{Melee, Shot, ShotKind};
pub use guard::{GState, Guard};
pub use player::{PState, Player};

// ---------------------------------------------------------------- constants

pub const GRAVITY: f32 = 430.0;
pub const RUN_SPEED: f32 = 80.0;
pub const JUMP_UP_VY: f32 = -178.0;
/// A running jump must clear a three-tile gap as in the original, and it must do
/// so from anywhere in the last tile before the brink — a terminal gives the
/// player far less timing precision than a joystick did, so the arc is long and
/// flat rather than pixel-perfect. It uses its own, gentler gravity to stay flat
/// without arcing up into the ceiling.
pub const JUMP_RUN_VY: f32 = -160.0;
pub const JUMP_RUN_VX: f32 = 126.0;
pub const GRAVITY_JUMP: f32 = 300.0;
/// Half-width of a character's body for wall collisions.
pub const BODY_HW: f32 = 6.5;
/// Drops shorter than this are free.
pub const FALL_SAFE: f32 = 58.0;
/// Drops at least this long are fatal.
pub const FALL_LETHAL: f32 = 112.0;

/// How long a timed gate stays up after its plate is released.
pub const GATE_HOLD: f32 = 7.0;
pub const GATE_RISE: f32 = 5.0;
pub const GATE_FALL: f32 = 0.9;
/// Chomper period, seconds.
pub const CHOMP_PERIOD: f32 = 2.1;
/// Fuse on a loose board once something stands on it.
pub const LOOSE_FUSE: f32 = 0.62;

// ---------------------------------------------------------------- phases

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Play,
    /// Dying animation, then restart.
    Dying(f32),
    Dead,
    /// Walking into the exit, then the next level.
    Leaving(f32),
    LevelDone,
    Victory,
    TimeUp,
}

#[derive(Clone)]
pub struct Msg {
    pub text: String,
    pub t: f32,
    pub warn: bool,
}

/// Things the player carries between levels.
#[derive(Clone, Copy)]
pub struct Carry {
    pub hp_max: i32,
    pub sword: bool,
    pub scimitar: bool,
    pub buckler: bool,
    pub wand: bool,
    pub daggers: i32,
}

impl Default for Carry {
    fn default() -> Self {
        Carry {
            hp_max: 3,
            sword: false,
            scimitar: false,
            buckler: false,
            wand: false,
            daggers: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Item {
    pub kind: ItemKind,
    pub tx: i32,
    pub ty: i32,
    pub taken: bool,
}

/// Screen-space camera state: which room we are looking at, and the eased
/// transition between rooms.
pub struct CamState {
    pub room: (i32, i32),
    /// Current world-space top-left of the view.
    pub at: V2,
    pub target: V2,
    pub shake: f32,
}

pub struct Game {
    pub lv: Level,
    pub dy: Dynamics,
    pub idx: usize,
    pub player: Player,
    pub guards: Vec<Guard>,
    pub items: Vec<Item>,
    pub shots: Vec<Shot>,
    pub fx: Particles,
    pub cam: CamState,
    pub phase: Phase,
    pub msg: Option<Msg>,
    pub clock: f32,
    pub elapsed: f32,
    pub rng: Rng,
    pub carry: Carry,
    pub deaths: u32,
    pub kills: u32,
    /// Flash overlay, e.g. when drinking a potion.
    pub flash: (f32, crate::gfx::color::Rgb),
    /// Set when a level has just been completed and the app should advance.
    pub advance: bool,
    /// Size of the world rectangle on screen; the renderer keeps this in step
    /// with the terminal's aspect ratio.
    pub view_w: f32,
    pub view_h: f32,
    /// Sword-stroke flourishes waiting to be drawn: (position, facing, age).
    pub slashes: Vec<(V2, f32, f32)>,
    /// Magnification. 1.0 frames exactly one room, the way the original did;
    /// above that the camera follows the prince and he is drawn correspondingly
    /// larger, which is the only way a character reads on a narrow terminal.
    pub zoom: f32,
}

impl Game {
    pub fn new(idx: usize, carry: Carry, seed: u64) -> Result<Game, ParseError> {
        let def = &CAMPAIGN[idx.min(CAMPAIGN.len() - 1)];
        Ok(Game::from_level(Level::parse(def)?, idx, carry, seed))
    }

    /// Build a game around an already-parsed level. Tests use this to run the
    /// simulation over purpose-built maps.
    pub fn from_level(lv: Level, idx: usize, carry: Carry, seed: u64) -> Game {
        let dy = Dynamics::new(lv.tw, lv.th);
        let mut g = Game {
            player: Player::new(
                v2(Level::cx(lv.start.0), Level::surf(lv.start.1)),
                lv.start_face,
                &carry,
            ),
            guards: Vec::new(),
            items: lv
                .items
                .iter()
                .map(|i| Item {
                    kind: i.kind,
                    tx: i.tx,
                    ty: i.ty,
                    taken: false,
                })
                .collect(),
            shots: Vec::new(),
            fx: Particles::new(seed ^ 0xC0FFEE),
            cam: CamState {
                room: Level::room_of(lv.start.0, lv.start.1),
                at: V2::ZERO,
                target: V2::ZERO,
                shake: 0.0,
            },
            phase: Phase::Play,
            msg: None,
            clock: lv.time as f32,
            elapsed: 0.0,
            rng: Rng::new(seed),
            carry,
            deaths: 0,
            kills: 0,
            flash: (0.0, crate::gfx::color::rgb(0, 0, 0)),
            advance: false,
            view_w: ROOM_W,
            view_h: ROOM_H,
            slashes: Vec::new(),
            zoom: 1.0,
            dy,
            idx,
            lv,
        };
        for m in g.lv.mobs.clone() {
            g.guards.push(Guard::new(&m));
        }
        g.init_dynamics();
        let r = g.room_rect(g.cam.room);
        g.cam.at = v2(r.0, r.1);
        g.cam.target = g.cam.at;
        let hint = g.lv.hint.to_string();
        g.say(&hint, 6.0, false);
        g
    }

    /// Reload the current level, keeping carried equipment.
    pub fn restart(&mut self) {
        let deaths = self.deaths;
        let carry = self.carry;
        let seed = self.rng.next_u64();
        if let Ok(mut g) = Game::new(self.idx, carry, seed) {
            g.deaths = deaths;
            g.kills = self.kills;
            std::mem::swap(self, &mut g);
        }
    }

    fn init_dynamics(&mut self) {
        for ty in 0..self.lv.th {
            for tx in 0..self.lv.tw {
                let t = self.lv.tile(tx, ty);
                let c = self.dy.at(tx, ty);
                match t {
                    Tile::Chomper => {
                        // Stagger the phase so a row of chompers ripples.
                        c.b = ((tx * 7 + ty * 13) % 21) as f32 / 21.0 * CHOMP_PERIOD;
                    }
                    Tile::Spikes => {
                        c.a = 0.0;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn say(&mut self, text: &str, t: f32, warn: bool) {
        self.msg = Some(Msg {
            text: text.to_string(),
            t,
            warn,
        });
    }

    // ------------------------------------------------------------ queries

    #[inline]
    pub fn gate_passable(&self, tx: i32, ty: i32) -> bool {
        self.dy.a(tx, ty) >= 0.55
    }

    /// Can a body occupy this cell?
    pub fn open(&self, tx: i32, ty: i32) -> bool {
        let t = self.lv.tile(tx, ty);
        if t.solid() {
            return false;
        }
        if t == Tile::Gate {
            return self.gate_passable(tx, ty);
        }
        true
    }

    /// Does this cell give something to stand on?
    pub fn supported(&self, tx: i32, ty: i32) -> bool {
        if !self.lv.in_bounds(tx, ty) {
            return false;
        }
        let here = self.lv.tile(tx, ty);
        if here.solid() {
            return false;
        }
        here.walkable() || self.lv.tile(tx, ty + 1).solid()
    }

    /// Is standing in this cell lethal right now?
    pub fn lethal(&self, tx: i32, ty: i32) -> bool {
        match self.lv.tile(tx, ty) {
            Tile::Spikes => self.dy.a(tx, ty) > 0.42,
            Tile::Chomper => self.dy.a(tx, ty) > 0.72,
            _ => false,
        }
    }

    /// World rectangle of a room: (x, y, w, h).
    pub fn room_rect(&self, room: (i32, i32)) -> (f32, f32, f32, f32) {
        (
            room.0 as f32 * ROOM_W,
            room.1 as f32 * ROOM_H,
            ROOM_W,
            ROOM_H,
        )
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, dt: f32, input: &crate::input::Input) {
        let dt = dt.min(1.0 / 15.0);
        self.elapsed += dt;
        if self.phase == Phase::Play {
            self.clock -= dt;
            if self.clock <= 0.0 {
                self.clock = 0.0;
                self.phase = Phase::TimeUp;
            }
        }

        self.update_tiles(dt);
        match self.phase {
            Phase::Play => {
                self.update_player(dt, input);
                self.update_guards(dt);
                self.update_shots(dt);
                self.check_items();
            }
            Phase::Dying(ref mut t) => {
                let nt = *t - dt;
                self.phase = if nt <= 0.0 {
                    Phase::Dead
                } else {
                    Phase::Dying(nt)
                };
                self.update_player_dead(dt);
                self.update_guards(dt);
            }
            Phase::Leaving(ref mut t) => {
                let nt = *t - dt;
                self.phase = if nt <= 0.0 {
                    Phase::LevelDone
                } else {
                    Phase::Leaving(nt)
                };
            }
            _ => {}
        }

        self.spawn_ambient(dt);
        self.update_slashes(dt);
        self.fx.update(dt);
        self.update_camera(dt);

        if let Some(m) = &mut self.msg {
            m.t -= dt;
            if m.t <= 0.0 {
                self.msg = None;
            }
        }
        self.flash.0 = (self.flash.0 - dt * 2.6).max(0.0);
        self.cam.shake = (self.cam.shake - dt * 4.0).max(0.0);
    }

    /// Gates, plates, spikes, chompers, loose boards.
    fn update_tiles(&mut self, dt: f32) {
        // Which cells have weight on them this frame?
        let mut pressed: Vec<(i32, i32)> = Vec::new();
        if self.phase == Phase::Play || matches!(self.phase, Phase::Dying(_)) {
            let (tx, ty) = self.player.foot_tile();
            pressed.push((tx, ty));
        }
        for g in &self.guards {
            if g.st != GState::Dead {
                pressed.push(g.foot_tile());
            }
        }

        // Plates: latch or start a hold timer on their group.
        let mut raise: Vec<u8> = Vec::new();
        let mut drop: Vec<u8> = Vec::new();
        for &(tx, ty) in &pressed {
            let c = self.lv.cell(tx, ty);
            if c.tile == Tile::PlateRaise && c.group != 0 {
                raise.push(c.group);
            } else if c.tile == Tile::PlateDrop && c.group != 0 {
                drop.push(c.group);
            }
        }

        for ty in 0..self.lv.th {
            for tx in 0..self.lv.tw {
                let cell = self.lv.cell(tx, ty);
                match cell.tile {
                    Tile::PlateRaise | Tile::PlateDrop => {
                        let on = pressed.contains(&(tx, ty));
                        let d = self.dy.at(tx, ty);
                        d.a = crate::util::approach(d.a, if on { 1.0 } else { 0.0 }, dt * 7.0);
                        if on {
                            d.flag |= F_PRESSED;
                        } else {
                            d.flag &= !F_PRESSED;
                        }
                    }
                    Tile::Gate | Tile::Exit => {
                        let g = cell.group;
                        let latching = g >= 40;
                        let want_up = g != 0 && raise.contains(&g);
                        let want_down = g != 0 && drop.contains(&g);
                        let d = self.dy.at(tx, ty);
                        if want_up {
                            if latching {
                                d.flag |= F_LATCHED;
                            }
                            d.b = GATE_HOLD;
                        }
                        if want_down {
                            d.flag &= !F_LATCHED;
                            d.b = 0.0;
                        }
                        let held = d.flag & F_LATCHED != 0 || d.b > 0.0;
                        if d.b > 0.0 {
                            d.b -= dt;
                        }
                        let rate = if held { GATE_RISE } else { GATE_FALL };
                        let goal = if held { 1.0 } else { 0.0 };
                        d.a = crate::util::approach(d.a, goal, dt * rate);
                    }
                    Tile::Spikes => {
                        // Arm when something is on the tile or the one beside it,
                        // then snap out and stay out for a while.
                        let near = pressed.iter().any(|&(px, py)| py == ty && (px - tx).abs() <= 1);
                        let d = self.dy.at(tx, ty);
                        if near {
                            d.flag |= F_ARMED;
                            d.b = 1.4;
                        }
                        if d.b > 0.0 {
                            d.b -= dt;
                        } else {
                            d.flag &= !F_ARMED;
                        }
                        let goal = if d.flag & F_ARMED != 0 { 1.0 } else { 0.0 };
                        let rate = if goal > 0.5 { 9.0 } else { 2.2 };
                        d.a = crate::util::approach(d.a, goal, dt * rate);
                    }
                    Tile::Chomper => {
                        let d = self.dy.at(tx, ty);
                        d.b += dt;
                        if d.b >= CHOMP_PERIOD {
                            d.b -= CHOMP_PERIOD;
                        }
                        // Long open phase, fast snap — readable and fair.
                        let f = d.b / CHOMP_PERIOD;
                        d.a = if f < 0.62 {
                            0.0
                        } else if f < 0.72 {
                            (f - 0.62) / 0.10
                        } else if f < 0.86 {
                            1.0
                        } else {
                            1.0 - (f - 0.86) / 0.14
                        };
                    }
                    Tile::Loose => {
                        let on = pressed.contains(&(tx, ty));
                        let d = self.dy.at(tx, ty);
                        if on && d.flag & F_TRIGGERED == 0 {
                            d.flag |= F_TRIGGERED;
                            d.b = LOOSE_FUSE;
                        }
                        if d.flag & F_TRIGGERED != 0 {
                            d.b -= dt;
                            d.a = (LOOSE_FUSE - d.b) / LOOSE_FUSE;
                            if d.b <= 0.0 {
                                self.break_board(tx, ty);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn break_board(&mut self, tx: i32, ty: i32) {
        self.lv.set_tile(tx, ty, Tile::Space);
        let d = self.dy.at(tx, ty);
        d.flag = 0;
        d.a = 0.0;
        d.b = 0.0;
        let col = self.lv.theme.slab_face;
        let x = Level::cx(tx);
        let y = Level::surf(ty);
        // Where the rubble comes to rest.
        let mut land = y;
        for k in 1..12 {
            if self.supported(tx, ty + k) {
                land = Level::surf(ty + k);
                break;
            }
        }
        self.fx.debris(v2(x, y + 2.0), 16, col, land);
        self.fx.dust(v2(x, y + 4.0), 12, 1.2, col.scale(0.7));
        self.cam.shake = 0.55;
        // Leave debris on the floor it fell onto.
        for k in 1..12 {
            if self.supported(tx, ty + k) {
                if self.lv.tile(tx, ty + k) == Tile::Floor {
                    self.lv.set_tile(tx, ty + k, Tile::Rubble);
                }
                break;
            }
        }
    }

    fn spawn_ambient(&mut self, dt: f32) {
        // Torch flames only for the visible part of the world.
        let view = self.view_rect();
        let tx0 = (view.x0 / TILE_W).floor() as i32 - 1;
        let tx1 = (view.x1 / TILE_W).ceil() as i32 + 1;
        let ty0 = (view.y0 / TILE_H).floor() as i32 - 1;
        let ty1 = (view.y1 / TILE_H).ceil() as i32 + 1;
        let hue = self.lv.theme.torch;
        let rate = (dt * 46.0).min(3.0);
        for ty in ty0..ty1 {
            for tx in tx0..tx1 {
                if self.lv.tile(tx, ty) == Tile::Torch {
                    let (fx, fy) = crate::art::tiles::torch_flame_pos(tx, ty);
                    let n = if self.rng.unit() < rate.fract() {
                        rate as i32 + 1
                    } else {
                        rate as i32
                    };
                    for _ in 0..n {
                        self.fx.flame(v2(fx, fy), 26.0, 1.0, hue);
                    }
                    if self.rng.chance(dt * 3.0) {
                        self.fx.smoke(v2(fx, fy - 6.0), 0.7);
                    }
                }
            }
        }
    }

    /// The world rectangle currently on screen. Set by the renderer each frame
    /// (it depends on the terminal's aspect ratio); defaults to one room.
    pub fn view_rect(&self) -> crate::util::Rect {
        crate::util::Rect::from_size(self.cam.at.x, self.cam.at.y, self.view_w, self.view_h)
    }

    fn update_camera(&mut self, dt: f32) {
        let (tx, ty) = self.player.foot_tile();
        self.cam.room = Level::room_of(tx, ty);
        let tgt = if self.zoom <= 1.001 {
            // Authentic: frame the room the prince is standing in.
            let r = self.room_rect(self.cam.room);
            v2(
                r.0 + (ROOM_W - self.view_w) * 0.5,
                r.1 + (ROOM_H - self.view_h) * 0.5,
            )
        } else {
            // Magnified: follow him, but never look outside the level.
            let mut c = v2(
                self.player.p.x - self.view_w * 0.5,
                self.player.p.y - 13.0 - self.view_h * 0.5,
            );
            let maxx = (self.lv.tw as f32 * TILE_W - self.view_w).max(0.0);
            let maxy = (self.lv.th as f32 * TILE_H - self.view_h).max(0.0);
            c.x = clampf(c.x, 0.0, maxx);
            c.y = clampf(c.y, 0.0, maxy);
            c
        };
        self.cam.target = tgt;
        // Exponential smoothing, written so the result does not depend on the
        // simulation step.
        let k = 1.0 - (-dt * 13.0).exp();
        self.cam.at = self.cam.at.lerp(self.cam.target, k);
    }
}

/// Extra fields the renderer needs to publish back to the game. Kept separate
/// from `Game::new` so the simulation does not depend on terminal geometry.
impl Game {
    pub fn set_view_size(&mut self, w: f32, h: f32) {
        self.view_w = w.max(TILE_W * 3.0);
        self.view_h = h.max(TILE_H * 1.2);
    }

    /// Snap the camera to wherever it should be right now, with no easing —
    /// used when the level starts or the magnification changes.
    pub fn centre_camera(&mut self) {
        self.update_camera(1000.0);
        self.cam.at = self.cam.target;
    }
}
