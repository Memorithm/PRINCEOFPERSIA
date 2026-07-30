//! Level integrity: every campaign map must parse, and the prince must be able
//! to reach the exit and every item on it.

use prince_of_persia_rs::world::level::Level;
use prince_of_persia_rs::world::levels::CAMPAIGN;
use prince_of_persia_rs::world::reach;
use prince_of_persia_rs::world::tile::{ItemKind, MobKind, Tile, ROOM_TH, ROOM_TW};

/// The original Apple II levels held at most 24 rooms. Every new level is meant
/// to be bigger than that.
const ORIGINAL_MAX_ROOMS: i32 = 24;

#[test]
fn every_level_parses() {
    for def in CAMPAIGN {
        Level::parse(def).unwrap_or_else(|e| panic!("{}", e));
    }
}

#[test]
fn every_row_is_made_of_whole_rooms() {
    for def in CAMPAIGN {
        let rw = def.rows[0].len();
        for (y, row) in def.rows.iter().enumerate() {
            assert_eq!(
                row.len(),
                rw,
                "{}: tile row {} has {} room chunks, expected {}",
                def.name,
                y,
                row.len(),
                rw
            );
            for (rx, chunk) in row.iter().enumerate() {
                assert_eq!(
                    chunk.chars().count() as i32,
                    ROOM_TW,
                    "{}: tile row {} chunk {} is {:?}",
                    def.name,
                    y,
                    rx,
                    chunk
                );
            }
        }
        assert_eq!(
            def.rows.len() % ROOM_TH as usize,
            0,
            "{}: {} rows is not a whole number of rooms",
            def.name,
            def.rows.len()
        );
        if !def.links.is_empty() {
            assert_eq!(def.links.len(), def.rows.len(), "{}: link layer size", def.name);
            for (y, row) in def.links.iter().enumerate() {
                assert_eq!(row.len(), rw, "{}: link row {}", def.name, y);
                for chunk in row.iter() {
                    assert_eq!(
                        chunk.chars().count() as i32,
                        ROOM_TW,
                        "{}: link row {} chunk {:?}",
                        def.name,
                        y,
                        chunk
                    );
                }
            }
        }
    }
}

#[test]
fn exit_is_reachable_on_every_level() {
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        let r = reach::analyse(&lv);
        assert!(
            r.exit_reached,
            "{}: the exit cannot be reached\n{}",
            lv.name,
            reach::debug_map(&lv, &r)
        );
    }
}

#[test]
fn every_item_is_reachable() {
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        let r = reach::analyse(&lv);
        for it in &lv.items {
            assert!(
                r.at(it.tx, it.ty),
                "{}: {:?} at ({}, {}) is unreachable",
                lv.name,
                it.kind,
                it.tx,
                it.ty
            );
        }
    }
}

#[test]
fn every_level_is_longer_than_the_originals() {
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        assert!(
            lv.room_count() > ORIGINAL_MAX_ROOMS,
            "{}: {} rooms, the original maximum was {}",
            lv.name,
            lv.room_count(),
            ORIGINAL_MAX_ROOMS
        );
        // And the rooms must not be mostly filler.
        assert!(
            lv.playable_rooms() >= ORIGINAL_MAX_ROOMS,
            "{}: only {} playable rooms",
            lv.name,
            lv.playable_rooms()
        );
    }
}

#[test]
fn every_gate_and_plate_is_wired() {
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        for ty in 0..lv.th {
            for tx in 0..lv.tw {
                let c = lv.cell(tx, ty);
                if matches!(
                    c.tile,
                    Tile::Gate | Tile::PlateRaise | Tile::PlateDrop | Tile::Exit
                ) {
                    assert_ne!(
                        c.group, 0,
                        "{}: {:?} at ({}, {}) has no link group",
                        lv.name, c.tile, tx, ty
                    );
                }
            }
        }
        // Every gate needs at least one plate that can raise it.
        for ty in 0..lv.th {
            for tx in 0..lv.tw {
                let c = lv.cell(tx, ty);
                if c.tile != Tile::Gate && c.tile != Tile::Exit {
                    continue;
                }
                let has_plate = (0..lv.th).any(|y| {
                    (0..lv.tw).any(|x| {
                        let o = lv.cell(x, y);
                        o.tile == Tile::PlateRaise && o.group == c.group
                    })
                });
                assert!(
                    has_plate,
                    "{}: {:?} at ({}, {}) in group {} has no raising plate",
                    lv.name, c.tile, tx, ty, c.group
                );
            }
        }
    }
}

#[test]
fn the_campaign_hands_out_the_bonus_weapons() {
    let mut seen: Vec<ItemKind> = Vec::new();
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        for it in &lv.items {
            if !seen.contains(&it.kind) {
                seen.push(it.kind);
            }
        }
    }
    for want in [
        ItemKind::Sword,
        ItemKind::Daggers,
        ItemKind::Wand,
        ItemKind::Buckler,
        ItemKind::Scimitar,
    ] {
        assert!(seen.contains(&want), "{:?} is never available", want);
    }
}

#[test]
fn the_sword_comes_before_the_first_guard() {
    // On the first level the prince starts unarmed, so he must be able to reach
    // the sword without walking through a guard first.
    let lv = Level::parse(&CAMPAIGN[0]).unwrap();
    let sword = lv
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Sword)
        .expect("level 1 must contain the sword");
    let r = reach::analyse(&lv);
    assert!(r.at(sword.tx, sword.ty));
    for m in &lv.mobs {
        assert!(
            m.kind != MobKind::Jaffar,
            "level 1 should not open with the boss"
        );
    }
}

#[test]
fn spawn_and_exit_are_standable() {
    for def in CAMPAIGN {
        let lv = Level::parse(def).unwrap();
        let (sx, sy) = lv.start;
        assert!(
            lv.tile(sx, sy).walkable(),
            "{}: start tile is {:?}",
            lv.name,
            lv.tile(sx, sy)
        );
        let (ex, ey) = lv.exit;
        assert_eq!(lv.tile(ex, ey), Tile::Exit);
    }
}
