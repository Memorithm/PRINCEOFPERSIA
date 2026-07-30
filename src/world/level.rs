//! Level representation and the ASCII map parser.
//!
//! A level is a grid of rooms; each room is 10 x 3 tiles exactly as in the
//! original, but a level may be any number of rooms wide and tall. Maps are
//! authored as two aligned text layers:
//!
//! * the **tile layer** — terrain, items, the prince's start and the exit;
//! * the **link layer** — group ids wiring pressure plates to gates and exits,
//!   and skill digits for guards.
//!
//! Both layers must be the same size; the link layer may be omitted entirely.

use crate::art::theme::{self, Theme};
use crate::world::tile::*;

/// Static, hand-authored map data.
///
/// Each row is given as a list of **room chunks** of exactly [`ROOM_TW`]
/// characters. Writing maps this way makes the room grid visible in the source
/// and makes a miscounted row a compile-time-visible mistake rather than a
/// subtly broken level.
pub struct LevelDef {
    pub name: &'static str,
    pub theme: &'static str,
    pub hint: &'static str,
    pub rows: &'static [&'static [&'static str]],
    pub links: &'static [&'static [&'static str]],
    /// Seconds on the clock for this level.
    pub time: i32,
}

/// Solid rock.
pub const W: &str = "##########";
/// Open air.
pub const S: &str = "..........";
/// Floor all the way across.
pub const F: &str = "==========";
/// Staircase climbing to the left (used between two corridors).
pub const UPL_A: &str = "..====....";
pub const UPL_B: &str = "......====";
/// Staircase climbing to the right.
pub const UPR_A: &str = "....====..";
pub const UPR_B: &str = "====......";

pub struct Level {
    pub name: &'static str,
    pub hint: &'static str,
    pub theme: Theme,
    pub tw: i32,
    pub th: i32,
    pub rw: i32,
    pub rh: i32,
    pub cells: Vec<Cell>,
    pub items: Vec<ItemSpec>,
    pub mobs: Vec<MobSpec>,
    pub start: (i32, i32),
    pub start_face: f32,
    pub exit: (i32, i32),
    pub time: i32,
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn group_of(c: char) -> u8 {
    match c {
        '1'..='9' => c as u8 - b'0',
        'a'..='z' => 10 + (c as u8 - b'a'),
        'A'..='Z' => 40 + (c as u8 - b'A'),
        _ => 0,
    }
}

impl Level {
    /// Join a layer of per-room chunks into flat rows, checking every chunk.
    fn flatten(
        name: &str,
        layer: &'static [&'static [&'static str]],
        what: &str,
        expect_rw: Option<i32>,
    ) -> Result<Vec<String>, ParseError> {
        let mut rows = Vec::with_capacity(layer.len());
        let rw = expect_rw.unwrap_or(layer.first().map(|r| r.len() as i32).unwrap_or(0));
        for (y, chunks) in layer.iter().enumerate() {
            if chunks.len() as i32 != rw {
                return Err(ParseError(format!(
                    "{}: {} row {} has {} room chunks, expected {}",
                    name,
                    what,
                    y,
                    chunks.len(),
                    rw
                )));
            }
            let mut row = String::with_capacity((rw * ROOM_TW) as usize);
            for (rx, c) in chunks.iter().enumerate() {
                let n = c.chars().count() as i32;
                if n != ROOM_TW {
                    return Err(ParseError(format!(
                        "{}: {} row {} room {} is {} chars, must be {}: {:?}",
                        name, what, y, rx, n, ROOM_TW, c
                    )));
                }
                row.push_str(c);
            }
            rows.push(row);
        }
        Ok(rows)
    }

    pub fn parse(def: &LevelDef) -> Result<Level, ParseError> {
        let th = def.rows.len() as i32;
        if th == 0 {
            return Err(ParseError(format!("{}: empty map", def.name)));
        }
        let rows = Self::flatten(def.name, def.rows, "tile", None)?;
        let rw = def.rows[0].len() as i32;
        let tw = rw * ROOM_TW;
        if th % ROOM_TH != 0 {
            return Err(ParseError(format!(
                "{}: map is {} rows tall; must be a multiple of {}",
                def.name, th, ROOM_TH
            )));
        }
        let links = if def.links.is_empty() {
            Vec::new()
        } else {
            if def.links.len() as i32 != th {
                return Err(ParseError(format!(
                    "{}: link layer has {} rows, tile layer has {}",
                    def.name,
                    def.links.len(),
                    th
                )));
            }
            Self::flatten(def.name, def.links, "link", Some(rw))?
        };

        let mut cells = vec![Cell::default(); (tw * th) as usize];
        let mut items = Vec::new();
        let mut mobs = Vec::new();
        let mut start = None;
        let mut start_face = 1.0f32;
        let mut exit = None;

        let link_char = |x: i32, y: i32| -> char {
            if links.is_empty() {
                return '.';
            }
            links[y as usize].chars().nth(x as usize).unwrap_or('.')
        };

        for y in 0..th {
            for (x, ch) in rows[y as usize].chars().enumerate() {
                let x = x as i32;
                let lc = link_char(x, y);
                let tile;
                let mut item: Option<ItemKind> = None;
                let mut mob: Option<MobKind> = None;

                match ch {
                    ' ' | '.' => tile = Tile::Space,
                    '=' => tile = Tile::Floor,
                    '#' => tile = Tile::Wall,
                    '|' => tile = Tile::Pillar,
                    'b' => tile = Tile::Loose,
                    ':' => tile = Tile::Rubble,
                    '^' => tile = Tile::Spikes,
                    'V' => tile = Tile::Chomper,
                    'G' => tile = Tile::Gate,
                    'p' => tile = Tile::PlateRaise,
                    'o' => tile = Tile::PlateDrop,
                    't' => tile = Tile::Torch,
                    'm' => tile = Tile::Mirror,
                    'w' => tile = Tile::Window,
                    'A' => tile = Tile::Arch,
                    'n' => tile = Tile::Bones,
                    'X' => {
                        tile = Tile::Exit;
                        exit = Some((x, y));
                    }
                    '@' => {
                        tile = Tile::Floor;
                        start = Some((x, y));
                        if lc == '<' {
                            start_face = -1.0;
                        }
                    }
                    'h' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::PotionHeal);
                    }
                    'H' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::PotionLife);
                    }
                    'f' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::PotionFloat);
                    }
                    'x' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::PotionPoison);
                    }
                    'q' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::PotionSwift);
                    }
                    's' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::Sword);
                    }
                    'D' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::Daggers);
                    }
                    'F' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::Wand);
                    }
                    'C' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::Buckler);
                    }
                    'M' => {
                        tile = Tile::Floor;
                        item = Some(ItemKind::Scimitar);
                    }
                    'g' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Guard);
                    }
                    'z' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Fat);
                    }
                    'k' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Skeleton);
                    }
                    'S' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Shadow);
                    }
                    'J' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Jaffar);
                    }
                    'Y' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Vizier);
                    }
                    'P' => {
                        tile = Tile::Floor;
                        mob = Some(MobKind::Princess);
                    }
                    other => {
                        return Err(ParseError(format!(
                            "{}: unknown map char {:?} at ({}, {})",
                            def.name, other, x, y
                        )))
                    }
                }

                let group = if matches!(
                    tile,
                    Tile::Gate | Tile::PlateRaise | Tile::PlateDrop | Tile::Exit
                ) {
                    group_of(lc)
                } else {
                    0
                };
                cells[(y * tw + x) as usize] = Cell { tile, group };
                if let Some(k) = item {
                    items.push(ItemSpec { kind: k, tx: x, ty: y });
                }
                if let Some(k) = mob {
                    let skill = match lc {
                        '0'..='9' => lc as u8 - b'0',
                        _ => 3,
                    };
                    mobs.push(MobSpec {
                        kind: k,
                        tx: x,
                        ty: y,
                        skill,
                        facing: if lc == '>' { 1.0 } else { -1.0 },
                    });
                }
            }
        }

        let start = start.ok_or_else(|| ParseError(format!("{}: no '@' start tile", def.name)))?;
        let exit = exit.ok_or_else(|| ParseError(format!("{}: no 'X' exit tile", def.name)))?;

        Ok(Level {
            name: def.name,
            hint: def.hint,
            theme: theme::by_name(def.theme),
            tw,
            th,
            rw: tw / ROOM_TW,
            rh: th / ROOM_TH,
            cells,
            items,
            mobs,
            start,
            start_face,
            exit,
            time: def.time,
        })
    }

    #[inline]
    pub fn cell(&self, tx: i32, ty: i32) -> Cell {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            // Outside the map is solid rock, so nothing can wander off the edge.
            Cell {
                tile: Tile::Wall,
                group: 0,
            }
        } else {
            self.cells[(ty * self.tw + tx) as usize]
        }
    }

    #[inline]
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        self.cell(tx, ty).tile
    }

    #[inline]
    pub fn in_bounds(&self, tx: i32, ty: i32) -> bool {
        tx >= 0 && ty >= 0 && tx < self.tw && ty < self.th
    }

    pub fn set_tile(&mut self, tx: i32, ty: i32, t: Tile) {
        if self.in_bounds(tx, ty) {
            self.cells[(ty * self.tw + tx) as usize].tile = t;
        }
    }

    /// Number of rooms in the level — the headline "how long is this?" figure.
    pub fn room_count(&self) -> i32 {
        self.rw * self.rh
    }

    /// Rooms that contain at least one non-solid cell, i.e. rooms the player can
    /// actually be inside.
    pub fn playable_rooms(&self) -> i32 {
        let mut n = 0;
        for ry in 0..self.rh {
            for rx in 0..self.rw {
                let mut open = false;
                'scan: for y in 0..ROOM_TH {
                    for x in 0..ROOM_TW {
                        let t = self.tile(rx * ROOM_TW + x, ry * ROOM_TH + y);
                        if t != Tile::Wall && t != Tile::Space {
                            open = true;
                            break 'scan;
                        }
                    }
                }
                if open {
                    n += 1;
                }
            }
        }
        n
    }

    // ------------------------------------------------------------ geometry

    /// Standing surface (world y) for a character in tile row `ty`.
    ///
    /// The floor slab lives in the bottom [`FLOOR_H`] pixels of its own cell, so
    /// the surface sits that far above the cell's lower edge. That keeps every
    /// surface of a room inside the room's own rectangle — important, because the
    /// camera frames exactly one room at a time.
    #[inline]
    pub fn surf(ty: i32) -> f32 {
        (ty + 1) as f32 * TILE_H - FLOOR_H
    }

    /// Centre x of tile column `tx`.
    #[inline]
    pub fn cx(tx: i32) -> f32 {
        (tx as f32 + 0.5) * TILE_W
    }

    #[inline]
    pub fn tx_of(x: f32) -> i32 {
        (x / TILE_W).floor() as i32
    }

    /// Tile row whose *interior* contains world y.
    #[inline]
    pub fn ty_of(y: f32) -> i32 {
        (y / TILE_H).floor() as i32
    }

    /// The tile row a character whose feet are at world `y` occupies.
    #[inline]
    pub fn ty_of_feet(y: f32) -> i32 {
        (y / TILE_H).floor() as i32
    }

    #[inline]
    pub fn room_of(tx: i32, ty: i32) -> (i32, i32) {
        (tx.div_euclid(ROOM_TW), ty.div_euclid(ROOM_TH))
    }
}
