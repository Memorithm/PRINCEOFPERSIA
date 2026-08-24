//! Prince of Persia — la légende du miroir d'argent (aventure vue du dessus).
//!
//! Run with no arguments to play. See `--help` for the other modes.

use std::io::Write;

use prince_of_persia_rs::adventure::{self, world, Game};
use prince_of_persia_rs::gfx;

const USAGE: &str = "\
Prince of Persia 2D — l'aventure vue du dessus

USAGE
    pop2d [OPTIONS]

MODES
    (défaut)            fenêtre graphique native 960x640, rendu lisse
        --tty           l'aventure dans le terminal (demi-blocs)
        --validate      vérifier tous les mondes (parsing + portails + connexité)
        --map <W>       afficher la carte ASCII du monde W
        --shot <FIC>    écrire une capture PNG 800x600 (SVGA natif)

OPTIONS
    -s, --seed <N>      graine du générateur aléatoire
        --world <W>     (avec --shot) monde à cadrer
        --at <TX,TY>    (avec --shot) placer le prince sur cette tuile
        --frames <N>    (avec --shot) simuler N images avant la capture
        --size <LxH>    taille de la capture en pixels (défaut 800x600)
        --zoom <N>      agrandissement entier du PNG (1 = résolution native)
    -h, --help          afficher cette aide

COMMANDES EN JEU (fenêtre)
    ← ↑ ↓ → / ZQSD   marcher       Espace / X   frapper (maintenir !)
    + / -            vue           P pause      R recommencer le monde
    Q / Échap        quitter

COMMANDES EN JEU (terminal)
    ← ↑ ↓ →          marcher       Espace / X   frapper
    +/-              vue           P pause      F1 aide     Q quitter
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed = 0x5EEDu64;
    let mut mode = Mode::Window;
    let mut shot = String::new();
    let mut world_ix = world::LIGHT_REALM;
    let mut at: Option<(i32, i32)> = None;
    let mut frames = 20i32;
    let mut size: Option<(i32, i32)> = None;
    let mut zoom = 1i32;

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
            "-s" | "--seed" => seed = next(&mut i).parse().unwrap_or(0x5EED),
            "--validate" => mode = Mode::Validate,
            "--tty" => mode = Mode::Tty,
            "--map" => {
                mode = Mode::Map;
                world_ix = next(&mut i).parse::<usize>().unwrap_or(0).min(world::WORLD_COUNT - 1);
            }
            "--shot" => {
                mode = Mode::Shot;
                shot = next(&mut i);
            }
            "--world" => {
                world_ix = next(&mut i)
                    .parse::<usize>()
                    .unwrap_or(0)
                    .min(world::WORLD_COUNT - 1)
            }
            "--at" => at = parse_pair(&next(&mut i)),
            "--frames" => frames = next(&mut i).parse().unwrap_or(20),
            "--size" => {
                size = parse_size(&next(&mut i))
                    .map(|(w, h)| (w.clamp(8, 4096), h.clamp(8, 4096)))
            }
            "--zoom" => zoom = next(&mut i).parse::<i32>().unwrap_or(1).clamp(1, 16),
            other => {
                eprintln!("option inconnue : {other}\n");
                print!("{USAGE}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    match mode {
        Mode::Window => {
            if let Err(e) = adventure::window::play_window(seed) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
        Mode::Tty => {
            if let Err(e) = adventure::app::play(seed) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
        Mode::Validate => {
            if !validate() {
                std::process::exit(1);
            }
        }
        Mode::Map => match Game::new(seed) {
            Ok(g) => print_map(&g, world_ix),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        Mode::Shot => {
            if let Err(e) = screenshot(seed, world_ix, &shot, at, frames, size, zoom) {
                eprintln!("erreur : {e}");
                std::process::exit(1);
            }
        }
    }
}

enum Mode {
    Window,
    Tty,
    Validate,
    Map,
    Shot,
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
        "{:<3} {:<26} {:>7} {:>8} {:>8} {:>7} {:>7}",
        "#", "monde", "tuiles", "ennemis", "objets", "portails", "départ"
    );
    for ix in 0..world::WORLD_COUNT {
        match world::World::parse(ix) {
            Ok(w) => {
                // Portals point to walkable ground in their destination world.
                let mut portals_ok = true;
                for p in world::PORTALS[ix] {
                    let dw = &world::WORLD_DEFS[p.dest_world];
                    let dest_tile = world::Tile::from_char(
                        dw.rows
                            .get(p.dest_ty as usize)
                            .and_then(|r| r.chars().nth(p.dest_tx as usize))
                            .unwrap_or('#'),
                    );
                    if p.dest_world >= world::WORLD_COUNT
                        || dest_tile.map(|t| t.solid()).unwrap_or(true)
                    {
                        portals_ok = false;
                        ok = false;
                        eprintln!(
                            "  ! {} : portail ({},{}) mène sur une case solide de {}",
                            w.name, p.tx, p.ty, dw.name
                        );
                    }
                }
                let starts = world::STARTS[ix];
                let start_ok = w.tile(starts.0, starts.1).walkable();
                if !start_ok {
                    ok = false;
                }
                println!(
                    "{:<3} {:<26} {:>4}x{:<3} {:>7} {:>8} {:>7} {:>7}",
                    ix,
                    w.name,
                    w.tw,
                    w.th,
                    w.spawns.len(),
                    w.pickups.len(),
                    world::PORTALS[ix].len(),
                    if start_ok { "ok" } else { "BLOQUÉ !" }
                );
                let _ = portals_ok;
            }
            Err(e) => {
                ok = false;
                println!("{:<3} ERREUR : {}", ix, e);
            }
        }
    }
    // The two realms must share mirror slabs at identical coordinates.
    let light = world::World::parse(world::LIGHT_REALM).ok();
    let dark = world::World::parse(world::DARK_REALM).ok();
    if let (Some(l), Some(d)) = (&light, &dark) {
        let ms: Vec<usize> = (0..l.tiles.len())
            .filter(|&i| l.tiles[i] == world::Tile::MirrorSlab)
            .collect();
        let aligned = ms.iter().all(|&i| {
            d.tiles[i] == world::Tile::MirrorSlab && l.tiles[i].walkable() && d.tiles[i].walkable()
        });
        println!(
            "\nmiroir : {} dalles, alignées lumière/ombre : {}",
            ms.len(),
            if aligned { "oui" } else { "NON" }
        );
        if !aligned || ms.is_empty() {
            ok = false;
        }
        if l.tw != d.tw || l.th != d.th {
            ok = false;
            println!("! les deux royaumes n'ont pas la même taille");
        }
    }
    if ok {
        println!("\ntous les mondes sont valides.");
    }
    ok
}

fn print_map(g: &Game, ix: usize) {
    let w = &g.worlds[ix];
    println!(
        "{} — {}x{} tuiles, {} ennemis\n",
        w.name, w.tw, w.th, w.spawns.len()
    );
    for ty in 0..w.th {
        let mut line = String::with_capacity(w.tw as usize);
        for tx in 0..w.tw {
            line.push(w.tile(tx, ty).glyph());
        }
        println!("{line}");
    }
}

fn screenshot(
    seed: u64,
    world_ix: usize,
    path: &str,
    at: Option<(i32, i32)>,
    frames: i32,
    size: Option<(i32, i32)>,
    zoom: i32,
) -> std::io::Result<()> {
    use prince_of_persia_rs::gfx::canvas::{Canvas, LightField};
    use prince_of_persia_rs::gfx::layer::Layer;

    let mut g = Game::new(seed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    // Jump to the requested spot.
    let (tx, ty) = at.unwrap_or({
        let s = world::STARTS[world_ix];
        (s.0, s.1)
    });
    g.cur = world_ix;
    g.enter_world(world_ix, None, true);
    g.player.p = prince_of_persia_rs::util::v2(
        (tx as f32 + 0.5) * world::TILE,
        (ty as f32 + 0.5) * world::TILE,
    );

    // SVGA minimum : la capture rend en natif 800x600 (aucun upscale flou).
    let (pw, ph) = size.unwrap_or((800, 600));
    let (pw, ph) = (pw.max(320), ph.max(240));
    let view_h = world::TILE * 13.0;
    let view_w = view_h * pw as f32 / ph as f32;
    g.set_view_size(view_w, view_h);

    let input = prince_of_persia_rs::input::Input::default();
    for _ in 0..frames.max(0) {
        g.update(1.0 / 30.0, &input);
    }

    let ss = 3.0f32.min(8.0 / (pw as f32 / view_w));
    let ss = ss.max(2.0);
    let mut cv = Canvas::new((view_w * ss) as i32, (view_h * ss) as i32);
    let mut layer = Layer::new();
    let mut light = LightField::new();
    adventure::render::draw(&g, &mut cv, &mut layer, &mut light, ss);

    let mut px = vec![gfx::color::Rgb::BLACK; (pw * ph) as usize];
    cv.resample_into(&mut px, pw, ph);
    let png = gfx::png::encode(&px, pw, ph, zoom);
    std::fs::File::create(path)?.write_all(&png)?;
    println!("écrit {} ({}x{})", path, pw * zoom, ph * zoom);
    Ok(())
}
