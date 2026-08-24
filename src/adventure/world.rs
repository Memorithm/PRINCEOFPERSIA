//! Worlds for the top-down adventure.
//!
//! A world is a rectangular ASCII map, one character per tile (see
//! [`MAP_LEGEND`]). The fifteen maps live in
//! [`crate::adventure::world_data`], generated and validated by
//! `tools/genmaps.py`; this module parses them into tile grids and wires the
//! portal network that links the worlds together — including the secret
//! passages.
//!
//! ```text
//!   #  mur / rocher      .  sol sable      ,  herbe        T  arbre
//!   B  buisson           R  rocher         ~  eau          %  pont
//!   :  gouffre           S  statue         t  brasero      "  fleur
//!   P  départ            D  porte verrouillée (clé)   L  porte scellée
//!   X  portail           C  grotte         M  dalle-miroir
//!   h  cœur              g  gemme          k  clé          H  conteneur de cœur
//!   i  miroir d'argent   e  épée d'argent  F  la Fée du lac
//!   V  vieil Erfan       Y  princesse Zahra
//!   o  Craboroc          m  Sbire          b  Chauve-Kese  z  Djinn des flots
//!   n  Garde noir        1  Brute (boss)   2  Garde royal (boss)
//!   3  Golem (boss)      4  Pharaon (boss) 9  Ganar
//! ```

use crate::util::Rect;

/// Side of a tile, art pixels.
pub const TILE: f32 = 24.0;

// ---------------------------------------------------------------- tiles

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Floor,
    Grass,
    Wall,
    Tree,
    Bush,
    Rock,
    Water,
    Bridge,
    Pit,
    Statue,
    Flower,
    /// Bronze brazier — solid, sheds light.
    Brazier,
    /// Locked door: consumes one key when walked through.
    DoorLocked,
    /// Sealed door: opens once the two seal fragments are held.
    DoorSealed,
    /// Portal — destination comes from the portal table.
    Portal,
    /// Cave mouth — also a portal, usually a secret one.
    Cave,
    /// Mirror slab: toggles realms when the silver mirror is held.
    MirrorSlab,
}

impl Tile {
    pub fn from_char(c: char) -> Option<Tile> {
        Some(match c {
            '.' | 'P' => Tile::Floor,
            ',' => Tile::Grass,
            '"' => Tile::Flower,
            '#' | 'R' => Tile::Wall,
            'T' => Tile::Tree,
            'B' => Tile::Bush,
            '~' => Tile::Water,
            '%' => Tile::Bridge,
            ':' => Tile::Pit,
            'S' => Tile::Statue,
            't' => Tile::Brazier,
            'D' => Tile::DoorLocked,
            'L' => Tile::DoorSealed,
            'X' => Tile::Portal,
            'C' => Tile::Cave,
            'M' => Tile::MirrorSlab,
            _ => return None,
        })
    }

    /// Blocks movement outright.
    pub fn solid(self) -> bool {
        matches!(
            self,
            Tile::Wall
                | Tile::Tree
                | Tile::Rock
                | Tile::Water
                | Tile::Pit
                | Tile::Statue
                | Tile::Bush
                | Tile::Brazier
                | Tile::DoorLocked
                | Tile::DoorSealed
        )
    }

    /// Ground something can spawn on / stand on.
    pub fn walkable(self) -> bool {
        !self.solid()
    }

    pub fn glyph(self) -> char {
        match self {
            Tile::Floor => '.',
            Tile::Grass => ',',
            Tile::Flower => '"',
            Tile::Wall => '#',
            Tile::Tree => 'T',
            Tile::Bush => 'B',
            Tile::Rock => 'R',
            Tile::Water => '~',
            Tile::Bridge => '%',
            Tile::Pit => ':',
            Tile::Statue => 'S',
            Tile::Brazier => 't',
            Tile::DoorLocked => 'D',
            Tile::DoorSealed => 'L',
            Tile::Portal => 'X',
            Tile::Cave => 'C',
            Tile::MirrorSlab => 'M',
        }
    }
}

// ---------------------------------------------------------------- things

/// Items lying in the world waiting to be picked up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pickup {
    Heart,
    Gem,
    Key,
    SilverMirror,
    SilverSword,
    /// Rare vessel: raises the prince's maximum hearts by one.
    HeartContainer,
}

impl Pickup {
    pub fn from_char(c: char) -> Option<Pickup> {
        Some(match c {
            'h' => Pickup::Heart,
            'g' => Pickup::Gem,
            'k' => Pickup::Key,
            'i' => Pickup::SilverMirror,
            'e' => Pickup::SilverSword,
            'H' => Pickup::HeartContainer,
            _ => return None,
        })
    }
}

/// Enemy species — the Zelda bestiary wearing Persian robes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FoeKind {
    /// Craboroc — inspired by the Octorok: wanders, spits pebbles.
    Crab,
    /// Sbire — inspired by the Moblin: charges when it sees you.
    Grunt,
    /// Chauve-Kese — inspired by the Keese: erratic flight.
    Bat,
    /// Djinn des flots — inspired by the Zora: spits fire from the water.
    Djinn,
    /// Garde noir — inspired by the Darknut: blocks frontal blows.
    Knight,
    /// Brute des geôles — boss of the dungeon.
    Ogre,
    /// Garde noir royal — boss of the palace (and tower elite).
    BossKnight,
    /// Golem de cristal — boss of the crystal caves.
    CrystalGolem,
    /// Pharaon momifié — boss of the necropolis.
    Pharaoh,
    /// Le Vizir Ganar — final boss, Ganon and Jaffar fused.
    Ganar,
}

impl FoeKind {
    pub fn from_char(c: char) -> Option<FoeKind> {
        Some(match c {
            'o' => FoeKind::Crab,
            'm' => FoeKind::Grunt,
            'b' => FoeKind::Bat,
            'z' => FoeKind::Djinn,
            'n' => FoeKind::Knight,
            '1' => FoeKind::Ogre,
            '2' => FoeKind::BossKnight,
            '3' => FoeKind::CrystalGolem,
            '4' => FoeKind::Pharaoh,
            '9' => FoeKind::Ganar,
            _ => return None,
        })
    }

    pub fn is_boss(self) -> bool {
        matches!(
            self,
            FoeKind::Ogre
                | FoeKind::BossKnight
                | FoeKind::CrystalGolem
                | FoeKind::Pharaoh
                | FoeKind::Ganar
        )
    }

    /// Contact damage, in half-hearts.
    pub fn touch_damage(self) -> i32 {
        match self {
            FoeKind::Crab | FoeKind::Bat | FoeKind::Djinn | FoeKind::Grunt | FoeKind::Knight => 1,
            FoeKind::Ogre
            | FoeKind::BossKnight
            | FoeKind::CrystalGolem
            | FoeKind::Pharaoh
            | FoeKind::Ganar => 2,
        }
    }

    pub fn max_hp(self) -> i32 {
        match self {
            FoeKind::Crab => 1,
            FoeKind::Bat => 1,
            FoeKind::Grunt => 2,
            FoeKind::Djinn => 2,
            FoeKind::Knight => 2,
            FoeKind::Ogre => 8,
            FoeKind::BossKnight => 12,
            FoeKind::CrystalGolem => 14,
            FoeKind::Pharaoh => 14,
            FoeKind::Ganar => 16,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FoeKind::Crab => "un Craboroc",
            FoeKind::Grunt => "un Sbire",
            FoeKind::Bat => "une Chauve-Kese",
            FoeKind::Djinn => "un Djinn des flots",
            FoeKind::Knight => "un Garde noir",
            FoeKind::Ogre => "la Brute des geôles",
            FoeKind::BossKnight => "le Garde noir royal",
            FoeKind::CrystalGolem => "le Golem de cristal",
            FoeKind::Pharaoh => "le Pharaon momifié",
            FoeKind::Ganar => "le Vizir Ganar",
        }
    }
}

// ---------------------------------------------------------------- worlds

pub const LIGHT_REALM: usize = 0;
pub const DUNGEON_WELLS: usize = 1;
pub const PALACE: usize = 2;
pub const DARK_REALM: usize = 3;
pub const FOREST: usize = 4;
pub const CRYSTAL_CAVES: usize = 5;
pub const DESERT: usize = 6;
pub const NECROPOLIS: usize = 7;
pub const SWAMP: usize = 8;
pub const VULTURE_PASS: usize = 9;
pub const COPPER_MINES: usize = 10;
pub const OASIS: usize = 11;
pub const MIRROR_SANCTUARY: usize = 12;
pub const TOWER: usize = 13;
pub const THRONE: usize = 14;
pub const WORLD_COUNT: usize = 15;

/// Static description of one world.
pub struct WorldDef {
    pub name: &'static str,
    pub theme: ThemeName,
    pub rows: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeName {
    Valley,
    Dungeon,
    Palace,
    Shadow,
    Forest,
    Crystal,
    Desert,
    Necro,
    Swamp,
    Pass,
    Mines,
    Oasis,
    Sanctuary,
    Tower,
    Throne,
}

pub const WORLD_DEFS: [WorldDef; WORLD_COUNT] = [
    WorldDef {
        name: "La Vallée d'Ispahan",
        theme: ThemeName::Valley,
        rows: crate::adventure::world_data::MAP_VALLEY,
    },
    WorldDef {
        name: "Les Geôles Oubliées",
        theme: ThemeName::Dungeon,
        rows: crate::adventure::world_data::MAP_DUNGEON,
    },
    WorldDef {
        name: "Le Palais de Sable",
        theme: ThemeName::Palace,
        rows: crate::adventure::world_data::MAP_PALACE,
    },
    WorldDef {
        name: "L'Ombre de la Vallée",
        theme: ThemeName::Shadow,
        rows: crate::adventure::world_data::MAP_SHADOW,
    },
    WorldDef {
        name: "La Forêt aux Mille Susurrations",
        theme: ThemeName::Forest,
        rows: crate::adventure::world_data::MAP_FOREST,
    },
    WorldDef {
        name: "Les Cavernes de Cristal",
        theme: ThemeName::Crystal,
        rows: crate::adventure::world_data::MAP_CRYSTAL,
    },
    WorldDef {
        name: "Le Désert des Sables Mouvants",
        theme: ThemeName::Desert,
        rows: crate::adventure::world_data::MAP_DESERT,
    },
    WorldDef {
        name: "La Nécropole des Rois",
        theme: ThemeName::Necro,
        rows: crate::adventure::world_data::MAP_NECRO,
    },
    WorldDef {
        name: "Le Marais des Sangsues",
        theme: ThemeName::Swamp,
        rows: crate::adventure::world_data::MAP_SWAMP,
    },
    WorldDef {
        name: "Le Col des Vautours",
        theme: ThemeName::Pass,
        rows: crate::adventure::world_data::MAP_PASS,
    },
    WorldDef {
        name: "Les Mines de Cuivre",
        theme: ThemeName::Mines,
        rows: crate::adventure::world_data::MAP_MINES,
    },
    WorldDef {
        name: "L'Oasis du Palmier Bleu",
        theme: ThemeName::Oasis,
        rows: crate::adventure::world_data::MAP_OASIS,
    },
    WorldDef {
        name: "Le Sanctuaire du Miroir",
        theme: ThemeName::Sanctuary,
        rows: crate::adventure::world_data::MAP_SANCTUARY,
    },
    WorldDef {
        name: "La Tour du Vizir",
        theme: ThemeName::Tower,
        rows: crate::adventure::world_data::MAP_TOWER,
    },
    WorldDef {
        name: "Le Trône des Ténèbres",
        theme: ThemeName::Throne,
        rows: crate::adventure::world_data::MAP_THRONE,
    },
];

/// Where a portal tile leads.
#[derive(Clone, Copy)]
pub struct Portal {
    pub tx: i32,
    pub ty: i32,
    pub dest_world: usize,
    pub dest_tx: i32,
    pub dest_ty: i32,
    pub label: &'static str,
}

use Portal as P;

/// The portal network. Several of these are genuine secrets tucked between the
/// worlds: the village well, the cave behind the waterfall, the ferns of the
/// south-west, and the mirror slabs (handled separately since they need the
/// silver mirror).
pub const PORTALS: [&[P]; WORLD_COUNT] = [
    // Vallée d'Ispahan
    &[
        P { tx: 18, ty: 2, dest_world: FOREST, dest_tx: 22, dest_ty: 28, label: "La Forêt aux Mille Susurrations" },
        P { tx: 41, ty: 3, dest_world: PALACE, dest_tx: 19, dest_ty: 24, label: "Le Palais de Sable" },
        P { tx: 26, ty: 18, dest_world: DUNGEON_WELLS, dest_tx: 4, dest_ty: 5, label: "Le puits du village" },
        P { tx: 44, ty: 13, dest_world: DESERT, dest_tx: 23, dest_ty: 16, label: "Le Désert des Sables Mouvants" },
        P { tx: 43, ty: 11, dest_world: VULTURE_PASS, dest_tx: 4, dest_ty: 14, label: "Le Col des Vautours" },
        P { tx: 8, ty: 8, dest_world: DARK_REALM, dest_tx: 8, dest_ty: 11, label: "Une grotte secrète…" },
        P { tx: 3, ty: 33, dest_world: OASIS, dest_tx: 7, dest_ty: 17, label: "Des fougères frémissent…" },
    ],
    // Geôles Oubliées
    &[P { tx: 3, ty: 3, dest_world: LIGHT_REALM, dest_tx: 26, dest_ty: 19, label: "Sortie des geôles" }],
    // Palais de Sable
    &[P { tx: 20, ty: 26, dest_world: LIGHT_REALM, dest_tx: 41, dest_ty: 6, label: "Sortie du palais" }],
    // Ombre de la Vallée
    &[
        P { tx: 26, ty: 18, dest_world: DUNGEON_WELLS, dest_tx: 4, dest_ty: 5, label: "Le puits de l'ombre" },
        P { tx: 41, ty: 3, dest_world: MIRROR_SANCTUARY, dest_tx: 13, dest_ty: 19, label: "Le Sanctuaire du Miroir" },
        P { tx: 44, ty: 30, dest_world: SWAMP, dest_tx: 4, dest_ty: 15, label: "Le Marais des Sangsues" },
        P { tx: 8, ty: 8, dest_world: LIGHT_REALM, dest_tx: 8, dest_ty: 11, label: "Retour à la cascade" },
        P { tx: 3, ty: 33, dest_world: OASIS, dest_tx: 7, dest_ty: 17, label: "Des fougères frémissent…" },
    ],
    // Forêt aux Mille Susurrations
    &[
        P { tx: 22, ty: 29, dest_world: LIGHT_REALM, dest_tx: 18, dest_ty: 3, label: "Retour à la vallée" },
        P { tx: 6, ty: 6, dest_world: CRYSTAL_CAVES, dest_tx: 3, dest_ty: 26, label: "Une grotte de cristal…" },
    ],
    // Cavernes de Cristal
    &[P { tx: 4, ty: 26, dest_world: FOREST, dest_tx: 6, dest_ty: 7, label: "Sortie des cavernes" }],
    // Désert des Sables Mouvants
    &[P { tx: 42, ty: 5, dest_world: NECROPOLIS, dest_tx: 19, dest_ty: 26, label: "La Nécropole des Rois" }],
    // Nécropole des Rois
    &[P { tx: 20, ty: 28, dest_world: DESERT, dest_tx: 42, dest_ty: 7, label: "Sortie de la pyramide" }],
    // Marais des Sangsues
    &[P { tx: 3, ty: 15, dest_world: DARK_REALM, dest_tx: 45, dest_ty: 29, label: "Retour à l'ombre" }],
    // Col des Vautours
    &[
        P { tx: 3, ty: 14, dest_world: LIGHT_REALM, dest_tx: 43, dest_ty: 12, label: "Retour à la vallée" },
        P { tx: 35, ty: 14, dest_world: COPPER_MINES, dest_tx: 4, dest_ty: 23, label: "Les Mines de Cuivre" },
    ],
    // Mines de Cuivre
    &[P { tx: 4, ty: 24, dest_world: VULTURE_PASS, dest_tx: 34, dest_ty: 14, label: "Sortie des mines" }],
    // Oasis du Palmier Bleu
    &[P { tx: 4, ty: 17, dest_world: LIGHT_REALM, dest_tx: 4, dest_ty: 32, label: "Retour par les fougères" }],
    // Sanctuaire du Miroir
    &[
        P { tx: 14, ty: 21, dest_world: DARK_REALM, dest_tx: 41, dest_ty: 4, label: "Retour à l'ombre" },
        P { tx: 14, ty: 5, dest_world: TOWER, dest_tx: 13, dest_ty: 28, label: "La Tour du Vizir" },
    ],
    // Tour du Vizir
    &[
        P { tx: 14, ty: 30, dest_world: MIRROR_SANCTUARY, dest_tx: 14, dest_ty: 19, label: "Descente du sanctuaire" },
        P { tx: 14, ty: 3, dest_world: THRONE, dest_tx: 11, dest_ty: 16, label: "Le Trône des Ténèbres" },
    ],
    // Trône des Ténèbres
    &[P { tx: 11, ty: 18, dest_world: TOWER, dest_tx: 14, dest_ty: 4, label: "Reculer n'est pas fuir…" }],
];

/// Where the prince (re)appears in each world — fresh entry and respawn point.
pub const STARTS: [(i32, i32); WORLD_COUNT] = [
    (26, 20), // vallée : la place du village
    (4, 4),   // geôles : salle d'entrée
    (19, 25), // palais : grand hall
    (26, 20), // ombre : la place dévastée
    (21, 28), // forêt : la grande clairière
    (3, 25),  // cavernes : galerie d'entrée
    (24, 16), // désert : l'oasis central
    (19, 27), // nécropole : salle du sarcophage vide
    (4, 14),  // marais : le ponton
    (4, 13),  // col : l'entrée du défilé
    (3, 23),  // mines : la cage d'extraction
    (6, 17),  // oasis : la rive secrète
    (13, 20), // sanctuaire : le narthex
    (13, 29), // tour : hall d'entrée
    (10, 17), // trône : l'antichambre
];

// ---------------------------------------------------------------- parsed world

/// A world parsed into a tile grid plus its denizens' spawn points.
pub struct World {
    pub ix: usize,
    pub name: &'static str,
    pub theme: ThemeName,
    pub tw: i32,
    pub th: i32,
    pub tiles: Vec<Tile>,
    pub start: (f32, f32),
    pub spawns: Vec<(FoeKind, f32, f32)>,
    pub pickups: Vec<(Pickup, f32, f32)>,
    pub npc: Option<(char, f32, f32)>, // 'V' Erfan, 'F' la Fée ou 'Y' Zahra
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[inline]
pub fn cx(tx: i32) -> f32 {
    (tx as f32 + 0.5) * TILE
}

#[inline]
pub fn cy(ty: i32) -> f32 {
    (ty as f32 + 0.5) * TILE
}

#[inline]
pub fn tx_of(x: f32) -> i32 {
    (x / TILE).floor() as i32
}

#[inline]
pub fn ty_of(y: f32) -> i32 {
    (y / TILE).floor() as i32
}

impl World {
    pub fn parse(ix: usize) -> Result<World, ParseError> {
        let def = &WORLD_DEFS[ix];
        let th = def.rows.len() as i32;
        if th == 0 {
            return Err(ParseError(format!("{} : carte vide", def.name)));
        }
        let tw = def.rows[0].chars().count() as i32;
        let mut tiles = Vec::with_capacity((tw * th) as usize);
        let mut spawns = Vec::new();
        let mut pickups = Vec::new();
        let mut npc = None;
        let mut start = None;
        let mut starts_found = 0;

        for (y, row) in def.rows.iter().enumerate() {
            let n = row.chars().count() as i32;
            if n != tw {
                return Err(ParseError(format!(
                    "{} : ligne {} fait {} caractères, attendu {}",
                    def.name, y, n, tw
                )));
            }
            for (x, c) in row.chars().enumerate() {
                let (x, y) = (x as i32, y as i32);
                if let Some(t) = Tile::from_char(c) {
                    tiles.push(t);
                    continue;
                }
                tiles.push(Tile::Floor);
                let (wx, wy) = (cx(x), cy(y));
                if let Some(k) = FoeKind::from_char(c) {
                    spawns.push((k, wx, wy));
                } else if let Some(k) = Pickup::from_char(c) {
                    pickups.push((k, wx, wy));
                } else {
                    match c {
                        'P' => {
                            start = Some((wx, wy));
                            starts_found += 1;
                        }
                        'V' | 'Y' | 'F' => npc = Some((c, wx, wy)),
                        other => {
                            return Err(ParseError(format!(
                                "{} : caractère inconnu {:?} en ({}, {})",
                                def.name, other, x, y
                            )))
                        }
                    }
                }
            }
        }

        // Fresh-entry position: explicit start, else the designated STARTS.
        let start = start.unwrap_or((cx(STARTS[ix].0), cy(STARTS[ix].1)));
        if starts_found > 1 {
            return Err(ParseError(format!("{} : plusieurs départs", def.name)));
        }

        Ok(World {
            ix,
            name: def.name,
            theme: def.theme,
            tw,
            th,
            tiles,
            start,
            spawns,
            pickups,
            npc,
        })
    }

    #[inline]
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            Tile::Wall
        } else {
            self.tiles[(ty * self.tw + tx) as usize]
        }
    }

    #[inline]
    pub fn set_tile(&mut self, tx: i32, ty: i32, t: Tile) {
        if tx >= 0 && ty >= 0 && tx < self.tw && ty < self.th {
            let i = (ty * self.tw + tx) as usize;
            self.tiles[i] = t;
        }
    }

    #[inline]
    pub fn in_bounds(&self, tx: i32, ty: i32) -> bool {
        tx >= 0 && ty >= 0 && tx < self.tw && ty < self.th
    }

    pub fn rect(&self) -> Rect {
        Rect::from_size(0.0, 0.0, self.tw as f32 * TILE, self.th as f32 * TILE)
    }

    /// Does this world contain a portal tile at (tx, ty)?
    pub fn portal_at(&self, tx: i32, ty: i32) -> Option<&'static Portal> {
        PORTALS[self.ix]
            .iter()
            .find(|p| p.tx == tx && p.ty == ty)
    }
}
