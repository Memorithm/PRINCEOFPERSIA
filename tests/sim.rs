//! Headless simulation tests: drive the engine with scripted input over
//! purpose-built maps and check the prince actually does what the moveset
//! promises.

use prince_of_persia_rs::game::{Carry, Game, PState, Phase};
use prince_of_persia_rs::input::Input;
use prince_of_persia_rs::util::Rng;
use prince_of_persia_rs::world::level::{Level, LevelDef, F, S, W};
use prince_of_persia_rs::world::levels::CAMPAIGN;
use prince_of_persia_rs::world::tile::{Tile, TILE_H, TILE_W};


const DT: f32 = 1.0 / 60.0;

/// Ceiling rows for the test maps. Each ends with an unreachable exit tile,
/// because every level is required to have one.
const TOP2: &[&str] = &[W, "#########X"];
const TOP3: &[&str] = &[W, W, "#########X"];
/// A link layer row that never matches anything.
const NO2: &[&str] = &[S, S];
const NO3: &[&str] = &[S, S, S];

fn game_from(rows: &'static [&'static [&'static str]], links: &'static [&'static [&'static str]]) -> Game {
    static NAME: &str = "test";
    let def = LevelDef {
        name: NAME,
        theme: "dungeon",
        hint: "",
        rows,
        links,
        time: 600,
    };
    let lv = Level::parse(&def).unwrap_or_else(|e| panic!("{}", e));
    Game::from_level(lv, 0, Carry::default(), 12345)
}

fn hold(g: &mut Game, inp: Input, frames: usize) {
    for _ in 0..frames {
        g.update(DT, &inp);
    }
}

fn right() -> Input {
    Input {
        right: true,
        right_edge: true,
        ..Default::default()
    }
}

// ---------------------------------------------------------------- basics

/// A flat corridor: three rooms of floor with the start near the left.
const FLAT: &[&[&str]] = &[TOP3, &[S, S, S], &["=@========", F, F]];
const FLAT_L: &[&[&str]] = &[NO3, NO3, NO3];

#[test]
fn the_prince_runs_to_the_right() {
    let mut g = game_from(FLAT, FLAT_L);
    let x0 = g.player.p.x;
    hold(&mut g, right(), 60);
    let moved = g.player.p.x - x0;
    assert!(
        moved > TILE_W * 1.5,
        "expected to cover more than 1.5 tiles in a second, covered {moved}"
    );
    assert!(matches!(g.player.st, PState::Run | PState::RunStart));
}

#[test]
fn releasing_the_key_brings_him_to_a_halt() {
    let mut g = game_from(FLAT, FLAT_L);
    hold(&mut g, right(), 45);
    hold(&mut g, Input::default(), 60);
    assert_eq!(g.player.st, PState::Stand);
}

#[test]
fn a_careful_step_moves_less_than_a_tile() {
    let mut g = game_from(FLAT, FLAT_L);
    let x0 = g.player.p.x;
    let inp = Input {
        right: true,
        right_edge: true,
        careful: true,
        ..Default::default()
    };
    // One step, then let it finish.
    hold(&mut g, inp, 30);
    hold(&mut g, Input::default(), 10);
    let moved = g.player.p.x - x0;
    assert!(moved > 2.0, "the careful step did not move him: {moved}");
    assert!(moved < TILE_W, "the careful step covered a whole tile: {moved}");
}

// ---------------------------------------------------------------- ledges

/// A step up: floor on the right is one row higher than on the left.
const STEP_UP: &[&[&str]] = &[TOP2, &[S, "======####"], &["=@====####", W]];

#[test]
fn he_climbs_onto_a_ledge_one_level_up() {
    let mut g = game_from(STEP_UP, &[NO2, NO2, NO2]);
    let row0 = Level::ty_of_feet(g.player.p.y);
    // Walk up to the wall, then press up.
    hold(&mut g, right(), 90);
    let inp = Input {
        up: true,
        up_edge: true,
        ..Default::default()
    };
    hold(&mut g, inp, 90);
    hold(&mut g, Input::default(), 30);
    let row1 = Level::ty_of_feet(g.player.p.y);
    assert_eq!(
        row1,
        row0 - 1,
        "expected to end one row higher (from {row0} to {}), ended on {row1}",
        row0 - 1
    );
    assert!(g.player.st == PState::Stand || g.player.st == PState::Ready);
}

/// A gap of three tiles, which a running jump should clear.
const GAP: &[&[&str]] = &[TOP2, &[S, S], &["=@=====...", F]];

#[test]
fn a_running_jump_clears_a_three_tile_gap() {
    let mut g = game_from(GAP, &[NO2, NO2, NO2]);
    // Run up to the brink, then take off — the same timing a player has to find.
    let brink = 7.0 * TILE_W;
    let mut took_off = false;
    for _ in 0..300 {
        let close = g.player.p.x > brink - TILE_W * 0.7;
        if close {
            took_off = true;
        }
        let inp = Input {
            right: true,
            right_edge: true,
            up: close,
            up_edge: close,
            ..Default::default()
        };
        g.update(DT, &inp);
        if g.player.p.x > 11.0 * TILE_W {
            break;
        }
    }
    // Let him come down on the far side.
    hold(&mut g, Input::default(), 60);
    assert!(took_off, "he never reached the edge of the gap");
    assert_ne!(g.phase, Phase::Dead, "he fell into the gap");
    assert!(
        g.player.p.x > 10.0 * TILE_W,
        "he did not make it across: x = {}",
        g.player.p.x
    );
    assert!(!g.player.st.airborne(), "he is still falling");
}

#[test]
fn walking_off_a_ledge_makes_him_fall_and_land() {
    // Floor, then nothing, then floor two rows down.
    const CLIFF: &[&[&str]] = &[TOP2, &["=@===.....", S], &["#####=====", F]];
    let mut g = game_from(CLIFF, &[NO2, NO2, NO2]);
    let y0 = g.player.p.y;
    hold(&mut g, right(), 120);
    assert!(
        g.player.p.y > y0,
        "he never fell: y went from {y0} to {}",
        g.player.p.y
    );
    assert!(
        !g.player.st.airborne(),
        "he is still in the air after two seconds: {:?}",
        g.player.st
    );
    assert_ne!(g.phase, Phase::Dead, "a one-storey drop should not kill him");
}

// ---------------------------------------------------------------- hazards

#[test]
fn a_loose_board_gives_way() {
    const LOOSE: &[&[&str]] = &[TOP2, &[S, S], &["=@b=======", F]];
    let mut g = game_from(LOOSE, &[NO2, NO2, NO2]);
    assert_eq!(g.lv.tile(2, 2), Tile::Loose);
    hold(&mut g, right(), 200);
    assert_eq!(
        g.lv.tile(2, 2),
        Tile::Space,
        "standing on the board should have broken it"
    );
}

#[test]
fn spikes_are_lethal() {
    const SPIKES: &[&[&str]] = &[TOP2, &[S, S], &["=@=^======", F]];
    let mut g = game_from(SPIKES, &[NO2, NO2, NO2]);
    hold(&mut g, right(), 240);
    assert!(
        matches!(g.phase, Phase::Dying(_) | Phase::Dead),
        "he walked over armed spikes unharmed"
    );
}

#[test]
fn a_plate_raises_its_gate() {
    const GATED: &[&[&str]] = &[TOP2, &[S, S], &["=@p====G==", F]];
    const GATED_L: &[&[&str]] = &[NO2, NO2, &["..A....A..", S]];
    let mut g = game_from(GATED, GATED_L);
    assert!(g.dy.a(7, 2) < 0.1, "the gate starts closed");
    // Walk onto the plate and wait.
    hold(&mut g, right(), 40);
    hold(&mut g, Input::default(), 90);
    assert!(
        g.dy.a(7, 2) > 0.55,
        "the gate did not open: travel = {}",
        g.dy.a(7, 2)
    );
    assert!(g.gate_passable(7, 2));
}

#[test]
fn a_closed_gate_blocks_the_way() {
    const GATED: &[&[&str]] = &[TOP2, &[S, S], &["=@===G====", F]];
    const GATED_L: &[&[&str]] = &[NO2, NO2, &[".....A....", S]];
    let mut g = game_from(GATED, GATED_L);
    hold(&mut g, right(), 180);
    assert!(
        g.player.p.x < 5.0 * TILE_W,
        "he walked through a closed portcullis: x = {}",
        g.player.p.x
    );
}

// ---------------------------------------------------------------- items & combat

#[test]
fn picking_up_the_sword_arms_him() {
    const SWORD: &[&[&str]] = &[TOP2, &[S, S], &["=@==s=====", F]];
    let mut g = game_from(SWORD, &[NO2, NO2, NO2]);
    assert!(!g.player.sword);
    hold(&mut g, right(), 90);
    assert!(g.player.sword, "the sword was not picked up");
    assert!(g.player.armed);
}

#[test]
fn a_healing_potion_restores_a_heart() {
    const POTION: &[&[&str]] = &[TOP2, &[S, S], &["=@==h=====", F]];
    let mut g = game_from(POTION, &[NO2, NO2, NO2]);
    g.player.hp = 1;
    hold(&mut g, right(), 90);
    assert_eq!(g.player.hp, 2, "the potion did not heal him");
}

#[test]
fn the_prince_can_kill_a_guard() {
    const FIGHT: &[&[&str]] = &[TOP2, &[S, S], &["=@=s===g==", F]];
    const FIGHT_L: &[&[&str]] = &[NO2, NO2, &[".......0..", S]];
    let mut g = game_from(FIGHT, FIGHT_L);
    // Collect the sword.
    hold(&mut g, right(), 80);
    assert!(g.player.sword, "he never reached the sword");
    // Step into striking distance and keep swinging. The prince is given plenty
    // of health so the test measures whether strikes land, not who wins a duel.
    let gx = g.guards[0].p.x;
    g.player.hp_max = 40;
    g.player.hp = 40;
    let hp0 = g.guards[0].hp;
    for round in 0..900 {
        if (g.player.p.x - (gx - 18.0)).abs() > 3.0 && !g.player.st.locked() {
            g.player.p.x = gx - 18.0;
            g.player.facing = 1.0;
        }
        let inp = Input {
            attack: round % 6 == 0,
            ..Default::default()
        };
        g.update(DT, &inp);
        if g.kills > 0 {
            break;
        }
    }
    assert!(
        g.kills > 0,
        "an unskilled guard survived 15 seconds of thrusts (hp {} -> {})",
        hp0,
        g.guards[0].hp
    );
}

#[test]
fn a_thrown_dagger_flies_and_lands() {
    const DAGGERS: &[&[&str]] = &[TOP2, &[S, S], &["=@=D======", F]];
    let mut g = game_from(DAGGERS, &[NO2, NO2, NO2]);
    hold(&mut g, right(), 70);
    assert!(g.player.daggers > 0, "the daggers were not picked up");
    let before = g.player.daggers;
    let inp = Input {
        throw: true,
        ..Default::default()
    };
    g.update(DT, &inp);
    assert_eq!(g.player.daggers, before - 1);
    assert_eq!(g.shots.len(), 1);
    hold(&mut g, Input::default(), 200);
    assert!(g.shots.is_empty(), "the dagger never came to rest");
}

// ---------------------------------------------------------------- robustness

#[test]
fn the_campaign_survives_random_input() {
    // Not a gameplay assertion: this is a crash and invariant sweep across every
    // level, with the prince flailing at the controls.
    for (i, _) in CAMPAIGN.iter().enumerate() {
        let mut g = Game::new(i, Carry::default(), 0xABCDEF ^ i as u64).unwrap();
        let mut rng = Rng::new(7 + i as u64);
        for frame in 0..4000 {
            let inp = Input {
                left: rng.chance(0.22),
                right: rng.chance(0.30),
                up: rng.chance(0.14),
                down: rng.chance(0.10),
                careful: rng.chance(0.12),
                attack: rng.chance(0.06),
                parry: rng.chance(0.08),
                throw: rng.chance(0.03),
                cast: rng.chance(0.03),
                sheathe: rng.chance(0.01),
                up_edge: rng.chance(0.10),
                down_edge: rng.chance(0.08),
                left_edge: rng.chance(0.08),
                right_edge: rng.chance(0.10),
            };
            g.update(DT, &inp);
            if matches!(g.phase, Phase::Dead) {
                g.restart();
            }
            assert!(
                g.player.p.x.is_finite() && g.player.p.y.is_finite(),
                "level {}: position went non-finite on frame {frame}",
                i + 1
            );
            assert!(
                g.player.p.y < (g.lv.th + 6) as f32 * TILE_H,
                "level {}: the prince left the world on frame {frame}",
                i + 1
            );
            assert!(
                g.player.hp <= g.player.hp_max,
                "level {}: health above maximum",
                i + 1
            );
        }
    }
}

#[test]
fn the_prince_never_stands_inside_masonry() {
    for (i, _) in CAMPAIGN.iter().enumerate() {
        let mut g = Game::new(i, Carry::default(), 99 + i as u64).unwrap();
        let mut rng = Rng::new(31 + i as u64);
        for frame in 0..2500 {
            let inp = Input {
                left: rng.chance(0.25),
                right: rng.chance(0.35),
                up: rng.chance(0.18),
                down: rng.chance(0.08),
                up_edge: rng.chance(0.12),
                ..Default::default()
            };
            g.update(DT, &inp);
            if matches!(g.phase, Phase::Dead) {
                g.restart();
                continue;
            }
            if g.player.st == PState::Dead {
                continue;
            }
            let (tx, ty) = g.player.foot_tile();
            let t = g.lv.tile(tx, ty);
            assert!(
                !t.solid(),
                "level {}: standing inside {:?} at ({tx}, {ty}) on frame {frame}",
                i + 1,
                t
            );
        }
    }
}

#[test]
fn rendering_a_frame_does_not_panic() {
    use prince_of_persia_rs::gfx::canvas::{Canvas, LightField};
    use prince_of_persia_rs::gfx::layer::Layer;

    for (i, _) in CAMPAIGN.iter().enumerate() {
        let mut g = Game::new(i, Carry::default(), 5 + i as u64).unwrap();
        hold(&mut g, right(), 40);
        // A deliberately awkward aspect ratio and a small super-sample factor.
        g.set_view_size(TILE_W * 10.0, TILE_H * 3.0 * 1.4);
        let mut cv = Canvas::new(211, 97);
        let mut layer = Layer::new();
        let mut light = LightField::new();
        g.draw(&mut cv, &mut layer, &mut light, 1.7);
        assert!(cv.px.iter().any(|p| p.luma() > 4.0), "the frame is black");
    }
}

// ---------------------------------------------------------------- feel & rigging

#[test]
fn the_hang_reach_matches_the_hang_drop() {
    // HANG_DROP is how far below a ledge the prince's feet sit while hanging; it
    // has to equal how far his hands reach above his feet, or they float off the
    // ledge they are gripping. Nothing else ties these two numbers together, so
    // this test does.
    use prince_of_persia_rs::art::skel::Prop;
    use prince_of_persia_rs::world::tile::HANG_DROP;
    let reach = Prop::PRINCE.reach_up();
    assert!(
        (reach - HANG_DROP).abs() < 1.0,
        "the arms reach {reach:.2} art px but HANG_DROP is {HANG_DROP:.2}"
    );
}

#[test]
fn hanging_puts_the_hands_on_the_ledge() {
    // Walk off a ledge holding the grab key and check he ends up hanging at the
    // height the art expects. The underside of the ledge has to be open, or there
    // would be nowhere for the body to hang.
    const LEDGE: &[&[&str]] = &[
        TOP2,
        &[S, S],
        &["=@===.....", S],
        &[S, S],
        &[S, S],
        &[F, F],
    ];
    let mut g = game_from(LEDGE, &[NO2, NO2, NO2, NO2, NO2, NO2]);
    // Shift is the careful step while standing — which deliberately refuses to
    // walk off a brink — and the grab while airborne. So: run off, then reach.
    let mut hung = false;
    for _ in 0..300 {
        let falling = g.player.st.airborne();
        let inp = Input {
            right: !falling,
            right_edge: !falling,
            careful: falling,
            ..Default::default()
        };
        g.update(DT, &inp);
        if g.player.st == PState::Hang {
            hung = true;
            break;
        }
    }
    assert!(hung, "he never caught the ledge: {:?}", g.player.st);
    let (_, ly) = g.player.ledge;
    let want = Level::surf(ly) + prince_of_persia_rs::world::tile::HANG_DROP;
    assert!(
        (g.player.p.y - want).abs() < 0.5,
        "hanging at y {} but the ledge wants {want}",
        g.player.p.y
    );
}

#[test]
fn a_state_change_never_snaps_the_pose() {
    // Every transition cross-fades, so the pose one frame after a state change is
    // still close to the pose one frame before it.
    let mut g = game_from(FLAT, FLAT_L);
    hold(&mut g, right(), 90);
    let before = g.player.pose();
    // Stop running: RunStop begins on this step.
    g.update(DT, &Input::default());
    let after = g.player.pose();
    let d = (before.hip - after.hip).abs()
        + (before.torso - after.torso).abs() * 0.1
        + (before.leg[0][0] - after.leg[0][0]).abs() * 0.1;
    assert!(
        d < 1.5,
        "the pose jumped across a state change (delta {d:.2})"
    );
}

#[test]
fn the_run_cycle_is_driven_by_distance() {
    // Two runs over the same ground must produce the same gait phase whatever
    // the step size, which is what stops the feet skating.
    let mut a = game_from(FLAT, FLAT_L);
    let mut b = game_from(FLAT, FLAT_L);
    for _ in 0..180 {
        a.update(1.0 / 120.0, &right());
    }
    for _ in 0..90 {
        b.update(1.0 / 60.0, &right());
    }
    let dx_a = a.player.p.x;
    let dx_b = b.player.p.x;
    // Same elapsed time, so they should be at nearly the same place...
    assert!((dx_a - dx_b).abs() < TILE_W, "{dx_a} vs {dx_b}");
    // ...and the gait tracks distance, not the number of steps taken.
    let per_px_a = a.player.gait / dx_a;
    let per_px_b = b.player.gait / dx_b;
    assert!(
        (per_px_a - per_px_b).abs() < 0.01,
        "gait per pixel differs with the step size: {per_px_a} vs {per_px_b}"
    );
}

#[test]
fn a_jump_pressed_during_a_skid_is_not_swallowed() {
    let mut g = game_from(FLAT, FLAT_L);
    hold(&mut g, right(), 90);
    // Release the direction so he starts skidding, then press jump once.
    g.update(DT, &Input::default());
    assert_eq!(g.player.st, PState::RunStop);
    let inp = Input {
        up: false,
        up_edge: true,
        ..Default::default()
    };
    g.update(DT, &inp);
    // The press is buffered and must fire within the buffer window.
    let mut jumped = false;
    for _ in 0..30 {
        g.update(DT, &Input::default());
        if g.player.st.airborne() || g.player.st == PState::Climb {
            jumped = true;
            break;
        }
    }
    assert!(jumped, "the buffered jump was dropped: {:?}", g.player.st);
}

#[test]
fn the_magnified_camera_stays_inside_the_level() {
    for (i, _) in CAMPAIGN.iter().enumerate() {
        let mut g = Game::new(i, Carry::default(), 3 + i as u64).unwrap();
        g.zoom = 2.5;
        g.set_view_size(TILE_W * 5.0, TILE_H * 1.9);
        g.centre_camera();
        let mut rng = Rng::new(11 + i as u64);
        for _ in 0..1500 {
            let inp = Input {
                left: rng.chance(0.3),
                right: rng.chance(0.4),
                up: rng.chance(0.15),
                up_edge: rng.chance(0.1),
                ..Default::default()
            };
            g.update(DT, &inp);
            if matches!(g.phase, Phase::Dead) {
                g.restart();
                g.zoom = 2.5;
                g.set_view_size(TILE_W * 5.0, TILE_H * 1.9);
                continue;
            }
            let r = g.view_rect();
            assert!(
                r.x0 >= -1.0 && r.y0 >= -1.0,
                "level {}: camera left the level at {:?}",
                i + 1,
                (r.x0, r.y0)
            );
            assert!(
                r.x1 <= g.lv.tw as f32 * TILE_W + 1.0,
                "level {}: camera ran off the right edge",
                i + 1
            );
        }
    }
}
