//! Prince of Persia — a terminal reimplementation in Rust.
//!
//! Run with no arguments to play. See `--help` for the other modes.

use std::io::Write;

use prince_of_persia_rs::game::{self, Carry, Game, PState};
use prince_of_persia_rs::gfx::canvas::{Canvas, LightField};
use prince_of_persia_rs::gfx::layer::Layer;
use prince_of_persia_rs::world::level::Level;
use prince_of_persia_rs::world::levels::CAMPAIGN;
use prince_of_persia_rs::{app, gfx, input, util, world};

const USAGE: &str = "\
Prince of Persia — réécriture pour le terminal, en Rust

USAGE
    pop [OPTIONS]

OPTIONS
    -l, --level <N>        commencer au niveau N (1..6)
    -s, --seed <N>         graine du générateur aléatoire
        --validate         vérifier tous les niveaux (parsing + accessibilité)
        --map <N>          afficher la carte du niveau N en ASCII
        --shot <FICHIER>   écrire une capture PNG au lieu de jouer
        --tty-shot <FIC>   capture PNG telle que l'affiche un terminal
        --cells <CxL>      (avec --tty-shot) taille du terminal simulé
        --room <X,Y>       (avec --shot) salle à cadrer
        --at <TX,TY>       (avec --shot) placer le prince sur cette tuile
        --pose <NOM>       (avec --shot) pose du prince : stand run jump fall
                           hang climb crouch sword strike parry dead drink
        --frames <N>       (avec --shot) simuler N images avant la capture
        --size <LxH>       (avec --shot) taille de la capture en pixels
        --zoom <N>         (avec --shot) agrandissement entier du PNG
    -h, --help             afficher cette aide

COMMANDES EN JEU
    ← →         courir            Maj + ← →   pas prudent
    ↑           sauter / grimper  ↓           s'accroupir
    Espace / X  frapper           Z           parer
    T           lancer une dague  F           bâton de flamme
    C           rengainer         Tab         changer d'arme
    P / Échap   pause             R           recommencer
    F1          commandes         Q           quitter
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut level = 1usize;
    let mut seed = 0x5EEDu64;
    let mut mode = Mode::Play;
    let mut shot = String::new();
    let mut room: Option<(i32, i32)> = None;
    let mut at: Option<(i32, i32)> = None;
    let mut pose = String::new();
    let mut frames = 12i32;
    let mut size: Option<(i32, i32)> = None;
    let mut zoom = 2i32;
    let mut cells: Option<(i32, i32)> = None;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).cloned().unwrap_or_default()
        };
        match a {
            "-h" | "--help" => {
                print!("{USAGE}");
                return;
            }
            "-l" | "--level" => level = next(&mut i).parse().unwrap_or(1),
            "-s" | "--seed" => seed = next(&mut i).parse().unwrap_or(0x5EED),
            "--validate" => mode = Mode::Validate,
            "--map" => {
                mode = Mode::Map;
                level = next(&mut i).parse().unwrap_or(1);
            }
            "--shot" => {
                mode = Mode::Shot;
                shot = next(&mut i);
            }
            "--tty-shot" => {
                mode = Mode::TtyShot;
                shot = next(&mut i);
            }
            "--cells" => cells = parse_size(&next(&mut i)),
            "--room" => room = parse_pair(&next(&mut i)),
            "--at" => at = parse_pair(&next(&mut i)),
            "--pose" => pose = next(&mut i),
            "--frames" => frames = next(&mut i).parse().unwrap_or(12),
            "--size" => size = parse_size(&next(&mut i)),
            "--zoom" => zoom = next(&mut i).parse().unwrap_or(2),
            other => {
                eprintln!("option inconnue : {other}\n");
                print!("{USAGE}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let idx = level.clamp(1, CAMPAIGN.len()) - 1;

    match mode {
        Mode::Play => {
            if let Err(e) = app::play(idx, seed) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
        Mode::Validate => {
            if !validate() {
                std::process::exit(1);
            }
        }
        Mode::Map => match Level::parse(&CAMPAIGN[idx]) {
            Ok(lv) => {
                let r = world::reach::analyse(&lv);
                println!(
                    "{}  {}x{} salles, {} jouables, sortie {}\n",
                    lv.name,
                    lv.rw,
                    lv.rh,
                    lv.playable_rooms(),
                    if r.exit_reached {
                        "accessible"
                    } else {
                        "INACCESSIBLE"
                    }
                );
                print!("{}", world::reach::debug_map(&lv, &r));
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        Mode::Shot => {
            if let Err(e) = screenshot(idx, seed, &shot, room, at, &pose, frames, size, zoom) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
        Mode::TtyShot => {
            let (c, r) = cells.unwrap_or((120, 32));
            if let Err(e) = tty_shot(idx, seed, &shot, at, c, r, frames, zoom) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
    }
}

enum Mode {
    Play,
    Validate,
    Map,
    Shot,
    TtyShot,
}

/// Render one frame through the *terminal* path — resample to a cell grid, then
/// expand each cell back into its two half-block pixels — so a PNG shows exactly
/// what a terminal of the given size displays. Text rows are drawn as flat bars,
/// since glyphs cannot be reproduced at one pixel per cell.
#[allow(clippy::too_many_arguments)]
fn tty_shot(
    idx: usize,
    seed: u64,
    path: &str,
    at: Option<(i32, i32)>,
    cols: i32,
    rows: i32,
    frames: i32,
    zoom: i32,
) -> std::io::Result<()> {
    use gfx::term::{Screen, HALF};
    use world::tile::{ROOM_H, ROOM_W};

    let mut g = Game::new(idx, Carry::default(), seed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    if let Some((tx, ty)) = at {
        g.player.p = util::v2(Level::cx(tx), Level::surf(ty));
        g.cam.room = Level::room_of(tx, ty);
    }

    let vrows = (rows - Game::TOP_ROWS - Game::BOTTOM_ROWS).max(2);
    let pw = cols;
    let ph = vrows * 2;
    let aspect = pw as f32 / ph as f32;
    let (vw, vh) = if aspect > ROOM_W / ROOM_H {
        (ROOM_H * aspect, ROOM_H)
    } else {
        (ROOM_W, ROOM_W / aspect)
    };
    g.set_view_size(vw, vh);
    let rr = g.room_rect(g.cam.room);
    g.cam.at = util::v2(rr.0 + (ROOM_W - vw) * 0.5, rr.1 + (ROOM_H - vh) * 0.5);
    g.cam.target = g.cam.at;
    let input = input::Input::default();
    for _ in 0..frames.max(0) {
        g.update(1.0 / 30.0, &input);
    }
    if let Some((tx, ty)) = at {
        g.player.p = util::v2(Level::cx(tx), Level::surf(ty));
    }

    let ss = (pw as f32 / vw * 1.4).clamp(2.0, 3.5);
    let mut cv = Canvas::new((vw * ss) as i32, (vh * ss) as i32);
    let mut layer = Layer::new();
    let mut light = LightField::new();
    g.draw(&mut cv, &mut layer, &mut light, ss);

    let mut scr = Screen::new(cols, rows);
    scr.clear(gfx::color::rgb(10, 9, 14));
    scr.blit(&cv, 0, Game::TOP_ROWS, cols, vrows);
    g.draw_hud(&mut scr);

    let mut px = vec![gfx::color::Rgb::BLACK; (cols * rows * 2) as usize];
    for y in 0..rows {
        for x in 0..cols {
            let c = scr.get(x, y);
            let (top, bot) = if c.ch == HALF {
                (c.fg, c.bg)
            } else if c.ch == ' ' {
                (c.bg, c.bg)
            } else {
                // Any other glyph: hint at it by mixing the ink into the cell.
                (c.bg.lerp(c.fg, 0.55), c.bg.lerp(c.fg, 0.2))
            };
            px[((y * 2) * cols + x) as usize] = top;
            px[((y * 2 + 1) * cols + x) as usize] = bot;
        }
    }
    let png = gfx::png::encode(&px, cols, rows * 2, zoom);
    std::fs::File::create(path)?.write_all(&png)?;
    println!("écrit {path} — aperçu d'un terminal {cols}x{rows}");
    Ok(())
}

fn parse_pair(s: &str) -> Option<(i32, i32)> {
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn parse_size(s: &str) -> Option<(i32, i32)> {
    let (a, b) = s.split_once(['x', 'X'])?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn validate() -> bool {
    let mut ok = true;
    println!(
        "{:<3} {:<26} {:>7} {:>9} {:>9} {:>8}  sortie",
        "#", "niveau", "salles", "jouables", "tuiles", "objets"
    );
    for (i, def) in CAMPAIGN.iter().enumerate() {
        match Level::parse(def) {
            Ok(lv) => {
                let r = world::reach::analyse(&lv);
                let reachable_items = r.items_seen.len();
                let all_items = lv.items.len();
                if !r.exit_reached || reachable_items != all_items {
                    ok = false;
                }
                println!(
                    "{:<3} {:<26} {:>7} {:>9} {:>9} {:>4}/{:<3}  {}",
                    i + 1,
                    lv.name,
                    format!("{}x{}", lv.rw, lv.rh),
                    lv.playable_rooms(),
                    lv.tw * lv.th,
                    reachable_items,
                    all_items,
                    if r.exit_reached { "ok" } else { "INACCESSIBLE" }
                );
                if !r.exit_reached {
                    println!("{}", world::reach::debug_map(&lv, &r));
                }
            }
            Err(e) => {
                ok = false;
                println!("{:<3} ERREUR: {}", i + 1, e);
            }
        }
    }
    let total: i32 = CAMPAIGN
        .iter()
        .filter_map(|d| Level::parse(d).ok())
        .map(|l| l.playable_rooms())
        .sum();
    println!(
        "\n{} niveaux, {} salles jouables au total (l'original plafonnait à 24 par niveau)",
        CAMPAIGN.len(),
        total
    );
    ok
}

#[allow(clippy::too_many_arguments)]
fn screenshot(
    idx: usize,
    seed: u64,
    path: &str,
    room: Option<(i32, i32)>,
    at: Option<(i32, i32)>,
    pose: &str,
    frames: i32,
    size: Option<(i32, i32)>,
    zoom: i32,
) -> std::io::Result<()> {
    let mut g = Game::new(idx, Carry::default(), seed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    if let Some((tx, ty)) = at {
        g.player.p = util::v2(Level::cx(tx), Level::surf(ty));
        g.cam.room = Level::room_of(tx, ty);
    }
    if let Some(r) = room {
        g.cam.room = (r.0, r.1);
    }
    let (pw, ph) = size.unwrap_or((640, 240));
    g.set_view_size(
        world::tile::ROOM_H * pw as f32 / ph as f32,
        world::tile::ROOM_H,
    );
    let rr = g.room_rect(g.cam.room);
    g.cam.at = util::v2(
        rr.0 + (world::tile::ROOM_W - g.view_w) * 0.5,
        rr.1 + (world::tile::ROOM_H - g.view_h) * 0.5,
    );
    g.cam.target = g.cam.at;

    // Let the world breathe so torches have flames and gates have settled.
    let input = input::Input::default();
    for _ in 0..frames.max(0) {
        g.update(1.0 / 30.0, &input);
    }
    if let Some((tx, ty)) = at {
        g.player.p = util::v2(Level::cx(tx), Level::surf(ty));
    }
    if !pose.is_empty() {
        g.player.st = match pose {
            "run" => PState::Run,
            "jump" => PState::JumpRun,
            "jumpup" => PState::JumpUp,
            "fall" => PState::Fall,
            "hang" => PState::Hang,
            "climb" => PState::Climb,
            "crouch" => PState::Crouch,
            "sword" | "ready" => PState::Ready,
            "strike" => PState::Strike,
            "parry" => PState::Parry,
            "dead" => PState::Dead,
            "drink" => PState::Drink,
            "turn" => PState::Turn,
            _ => PState::Stand,
        };
        if matches!(g.player.st, PState::Ready | PState::Strike | PState::Parry) {
            g.player.sword = true;
            g.player.melee = game::Melee::Sword;
            g.player.armed = true;
        }
        // Park the animation at a readable moment.
        let (clip, rate) = g.player.clip();
        g.player.t = clip.total() / rate.abs().max(0.01) * 0.45;
    }

    let ss = (pw as f32 / g.view_w).clamp(1.0, 4.0).max(2.0);
    let mut cv = Canvas::new((g.view_w * ss) as i32, (g.view_h * ss) as i32);
    let mut layer = Layer::new();
    let mut light = LightField::new();
    g.draw(&mut cv, &mut layer, &mut light, ss);

    // Resample to the requested pixel size, then blow it up for viewing.
    let mut px = vec![gfx::color::Rgb::BLACK; (pw * ph) as usize];
    cv.resample_into(&mut px, pw, ph);
    let png = gfx::png::encode(&px, pw, ph, zoom);
    let mut f = std::fs::File::create(path)?;
    f.write_all(&png)?;
    println!("écrit {} ({}x{})", path, pw * zoom, ph * zoom);
    Ok(())
}
