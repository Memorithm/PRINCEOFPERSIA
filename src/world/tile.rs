//! Tile vocabulary and the world's metric constants.
//!
//! The tile set follows the original Apple II piece list (space, floor, spikes,
//! posts, gate, pressure plates, loose board, mirror, rubble, exit, chomper,
//! torch, block, bones…) so level layouts read the same way they did in 1989.
//!
//! Geometry, all in "art pixels":
//!
//! ```text
//!            tx*32                      (tx+1)*32
//!   ty*40    +-------------------------------+
//!            |                               |   free space inside the cell
//!            |        a character standing   |
//!            |        in this cell           |
//! (ty+1)*40  +===============================+  <- standing surface = cell bottom
//!            |///// floor slab (9 px) ///////|     drawn into the cell below
//! ```
//!
//! The slab of one row doubles as the ceiling trim of the row beneath it, which
//! is exactly how the original art is laid out.

use crate::gfx::color::{rgb, Rgb};

pub const TILE_W: f32 = 32.0;
pub const TILE_H: f32 = 40.0;
/// Thickness of the floor slab drawn below a cell's standing surface.
pub const FLOOR_H: f32 = 9.0;

pub const ROOM_TW: i32 = 10;
pub const ROOM_TH: i32 = 3;
pub const ROOM_W: f32 = TILE_W * ROOM_TW as f32;
pub const ROOM_H: f32 = TILE_H * ROOM_TH as f32;

/// How far below a ledge a hanging character's base sits.
///
/// This has to equal how far the prince's hands reach above his feet with the
/// arms straight up ([`crate::art::skel::Prop::reach_up`], 31.2 art pixels for
/// [`crate::art::skel::Prop::PRINCE`]) — otherwise the hands float off the ledge
/// they are supposed to be gripping. A test pins the two together.
pub const HANG_DROP: f32 = 31.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// Nothing — you fall through it.
    Space,
    /// Walkable floor.
    Floor,
    /// Solid masonry.
    Wall,
    /// Decorative column; solid, but you can stand on top of it.
    Pillar,
    /// Loose board: gives way under weight.
    Loose,
    /// What a loose board leaves behind.
    Rubble,
    /// Spike trap in the floor.
    Spikes,
    /// Blades in a doorway that snap shut on a cycle.
    Chomper,
    /// Portcullis.
    Gate,
    /// Pressure plate that raises linked gates.
    PlateRaise,
    /// Pressure plate that drops linked gates.
    PlateDrop,
    /// Wall torch (the cell itself stays walkable space).
    Torch,
    /// Framed mirror.
    Mirror,
    /// Barred window; casts a shaft of light.
    Window,
    /// Decorative arch over an opening.
    Arch,
    /// Remains of a previous visitor.
    Bones,
    /// The level exit.
    Exit,
}

impl Tile {
    /// Blocks horizontal movement outright.
    #[inline]
    pub fn solid(self) -> bool {
        matches!(self, Tile::Wall | Tile::Pillar)
    }

    /// Provides a standing surface at the bottom of its own cell.
    #[inline]
    pub fn walkable(self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::Loose
                | Tile::Rubble
                | Tile::Spikes
                | Tile::PlateRaise
                | Tile::PlateDrop
                | Tile::Bones
                | Tile::Exit
                | Tile::Gate
                | Tile::Chomper
        )
    }

    /// A pressure plate of either polarity.
    #[inline]
    pub fn plate(self) -> bool {
        matches!(self, Tile::PlateRaise | Tile::PlateDrop)
    }

    /// Free space a character's body can occupy (ignoring gate state).
    #[inline]
    pub fn open(self) -> bool {
        !self.solid()
    }

    pub fn glyph(self) -> char {
        match self {
            Tile::Space => ' ',
            Tile::Floor => '=',
            Tile::Wall => '#',
            Tile::Pillar => '|',
            Tile::Loose => 'b',
            Tile::Rubble => ':',
            Tile::Spikes => '^',
            Tile::Chomper => 'V',
            Tile::Gate => 'G',
            Tile::PlateRaise => 'p',
            Tile::PlateDrop => 'o',
            Tile::Torch => 't',
            Tile::Mirror => 'm',
            Tile::Window => 'w',
            Tile::Arch => 'A',
            Tile::Bones => 'n',
            Tile::Exit => 'X',
        }
    }
}

/// One map cell: a tile plus the link group used by plates, gates and doors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub tile: Tile,
    /// 0 = unlinked. Plates, gates and exits sharing a group are wired together.
    pub group: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            tile: Tile::Space,
            group: 0,
        }
    }
}

// ---------------------------------------------------------------- items

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    /// Restores one unit of health.
    PotionHeal,
    /// Permanently raises maximum health.
    PotionLife,
    /// Feather fall: no damage from long drops for a while.
    PotionFloat,
    /// Poison: costs health.
    PotionPoison,
    /// Quickens the prince for a while.
    PotionSwift,
    /// The prince's sword.
    Sword,
    /// Bonus: throwing daggers.
    Daggers,
    /// Bonus: the alchemist's flame wand.
    Wand,
    /// Bonus: a buckler that parries reliably and stops projectiles.
    Buckler,
    /// Bonus: the vizier's heavy scimitar.
    Scimitar,
}

impl ItemKind {
    pub fn is_potion(self) -> bool {
        matches!(
            self,
            ItemKind::PotionHeal
                | ItemKind::PotionLife
                | ItemKind::PotionFloat
                | ItemKind::PotionPoison
                | ItemKind::PotionSwift
        )
    }

    /// Bottle colour for potions.
    pub fn colour(self) -> Rgb {
        match self {
            ItemKind::PotionHeal => rgb(214, 40, 62),
            ItemKind::PotionLife => rgb(232, 74, 148),
            ItemKind::PotionFloat => rgb(72, 186, 226),
            ItemKind::PotionPoison => rgb(120, 216, 92),
            ItemKind::PotionSwift => rgb(238, 206, 74),
            _ => rgb(200, 206, 214),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ItemKind::PotionHeal => "Potion de vie",
            ItemKind::PotionLife => "Élixir de vigueur",
            ItemKind::PotionFloat => "Potion de plume",
            ItemKind::PotionPoison => "Poison !",
            ItemKind::PotionSwift => "Potion de célérité",
            ItemKind::Sword => "Épée",
            ItemKind::Daggers => "Dagues de jet",
            ItemKind::Wand => "Bâton de flamme",
            ItemKind::Buckler => "Bouclier",
            ItemKind::Scimitar => "Cimeterre du Vizir",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemSpec {
    pub kind: ItemKind,
    pub tx: i32,
    pub ty: i32,
}

// ---------------------------------------------------------------- mobs

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobKind {
    Guard,
    Fat,
    Skeleton,
    Shadow,
    Jaffar,
}

impl MobKind {
    pub fn name(self) -> &'static str {
        match self {
            MobKind::Guard => "Garde",
            MobKind::Fat => "Geôlier",
            MobKind::Skeleton => "Squelette",
            MobKind::Shadow => "L'Ombre",
            MobKind::Jaffar => "Jaffar",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MobSpec {
    pub kind: MobKind,
    pub tx: i32,
    pub ty: i32,
    /// 0..9 — drives strike/parry probability and reaction time.
    pub skill: u8,
    pub facing: f32,
}
