//! Fenêtre graphique native (minifb) : le moteur vectoriel rend dans un
//! véritable framebuffer haute résolution — plus de limite des 2 pixels par
//! cellule du terminal.

use minifb::{Key, Window, WindowOptions};
use std::time::Instant;

use crate::adventure::render;
use crate::adventure::world::TILE;
use crate::adventure::{Game, Phase};
use crate::gfx::canvas::{Canvas, LightField};
use crate::gfx::color::rgb;
use crate::gfx::color::Rgb;
use crate::gfx::layer::Layer;
use crate::util::v2;

const W: i32 = 960;
const H: i32 = 640;
const SIM_DT: f32 = 1.0 / 120.0;
const VIEWS_TALL: [f32; 4] = [8.0, 10.0, 13.0, 16.0];

// ---------------------------------------------------------------- police 5x7

const FONT: &[(char, [&str; 7])] = &[
    ('A', ["01110", "10001", "10001", "11111", "10001", "10001", "10001"]),
    ('B', ["11110", "10001", "10001", "11110", "10001", "10001", "11110"]),
    ('C', ["01110", "10001", "10000", "10000", "10000", "10001", "01110"]),
    ('D', ["11110", "10001", "10001", "10001", "10001", "10001", "11110"]),
    ('E', ["11111", "10000", "10000", "11110", "10000", "10000", "11111"]),
    ('F', ["11111", "10000", "10000", "11110", "10000", "10000", "10000"]),
    ('G', ["01110", "10001", "10000", "10111", "10001", "10001", "01111"]),
    ('H', ["10001", "10001", "10001", "11111", "10001", "10001", "10001"]),
    ('I', ["01110", "00100", "00100", "00100", "00100", "00100", "01110"]),
    ('J', ["00111", "00010", "00010", "00010", "00010", "10010", "01100"]),
    ('K', ["10001", "10010", "10100", "11000", "10100", "10010", "10001"]),
    ('L', ["10000", "10000", "10000", "10000", "10000", "10000", "11111"]),
    ('M', ["10001", "11011", "10101", "10101", "10001", "10001", "10001"]),
    ('N', ["10001", "11001", "10101", "10011", "10001", "10001", "10001"]),
    ('O', ["01110", "10001", "10001", "10001", "10001", "10001", "01110"]),
    ('P', ["11110", "10001", "10001", "11110", "10000", "10000", "10000"]),
    ('Q', ["01110", "10001", "10001", "10001", "10101", "10010", "01101"]),
    ('R', ["11110", "10001", "10001", "11110", "10100", "10010", "10001"]),
    ('S', ["01111", "10000", "10000", "01110", "00001", "00001", "11110"]),
    ('T', ["11111", "00100", "00100", "00100", "00100", "00100", "00100"]),
    ('U', ["10001", "10001", "10001", "10001", "10001", "10001", "01110"]),
    ('V', ["10001", "10001", "10001", "10001", "10001", "01010", "00100"]),
    ('W', ["10001", "10001", "10001", "10101", "10101", "11011", "10001"]),
    ('X', ["10001", "01010", "00100", "00100", "00100", "01010", "10001"]),
    ('Y', ["10001", "01010", "00100", "00100", "00100", "00100", "00100"]),
    ('Z', ["11111", "00001", "00010", "00100", "01000", "10000", "11111"]),
    ('0', ["01110", "10001", "10011", "10101", "11001", "10001", "01110"]),
    ('1', ["00100", "01100", "00100", "00100", "00100", "00100", "01110"]),
    ('2', ["01110", "10001", "00001", "00010", "00100", "01000", "11111"]),
    ('3', ["11111", "00010", "00100", "00010", "00001", "10001", "01110"]),
    ('4', ["00010", "00110", "01010", "10010", "11111", "00010", "00010"]),
    ('5', ["11111", "10000", "11110", "00001", "00001", "10001", "01110"]),
    ('6', ["00110", "01000", "10000", "11110", "10001", "10001", "01110"]),
    ('7', ["11111", "00001", "00010", "00100", "01000", "01000", "01000"]),
    ('8', ["01110", "10001", "10001", "01110", "10001", "10001", "01110"]),
    ('9', ["01110", "10001", "10001", "01111", "00001", "00010", "01100"]),
    (' ', ["00000", "00000", "00000", "00000", "00000", "00000", "00000"]),
    ('!', ["00100", "00100", "00100", "00100", "00100", "00000", "00100"]),
    ('?', ["01110", "10001", "00001", "00010", "00100", "00000", "00100"]),
    ('\'', ["00100", "00100", "00000", "00000", "00000", "00000", "00000"]),
    ('-', ["00000", "00000", "00000", "01110", "00000", "00000", "00000"]),
    ('.', ["00000", "00000", "00000", "00000", "00000", "00000", "00100"]),
    (',', ["00000", "00000", "00000", "00000", "00000", "00100", "01000"]),
    (':', ["00000", "00100", "00100", "00000", "00100", "00100", "00000"]),
    ('/', ["00001", "00010", "00010", "00100", "01000", "01000", "10000"]),
    ('+', ["00000", "00100", "00100", "11111", "00100", "00100", "00000"]),
    ('(', ["00010", "00100", "01000", "01000", "01000", "00100", "00010"]),
    (')', ["01000", "00100", "00010", "00010", "00010", "00100", "01000"]),
];

/// Uppercase + strip accents — the font only carries the Latin base.
fn normalize(c: char) -> char {
    let u = c.to_ascii_uppercase();
    if u.is_ascii_graphic() || u == ' ' {
        return match u {
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            _ => u,
        };
    }
    match c {
        'é' | 'è' | 'ê' | 'ë' => 'E',
        'à' | 'â' | 'ä' => 'A',
        'ç' => 'C',
        'î' | 'ï' => 'I',
        'ô' | 'ö' => 'O',
        'û' | 'ù' | 'ü' => 'U',
        '’' => '\'',
        '…' => '.',
        '—' | '–' => '-',
        _ => '?',
    }
}

/// Draw a string into the canvas in art-pixel coordinates (1 font px = `s`).
fn draw_text(cv: &mut Canvas, x: i32, y: i32, s: &str, sc: i32, col: Rgb) {
    let mut cx = x;
    for ch in s.chars() {
        let c = normalize(ch);
        if let Some((_, rows)) = FONT.iter().find(|(g, _)| *g == c) {
            for (ry, row) in rows.iter().enumerate() {
                for (rx, cell) in row.chars().enumerate() {
                    if cell == '1' {
                        for dy in 0..sc {
                            for dx in 0..sc {
                                cv.set_pixel(cx + rx as i32 * sc + dx, y + ry as i32 * sc + dy, col);
                            }
                        }
                    }
                }
            }
        }
        cx += 6 * sc;
    }
}

fn text_width(s: &str, sc: i32) -> i32 {
    s.chars().count() as i32 * 6 * sc - sc
}

// ---------------------------------------------------------------- fenêtre

pub fn play_window(seed: u64) -> Result<(), String> {
    let mut window = Window::new(
        "Prince of Persia — la légende du miroir d'argent",
        W as usize,
        H as usize,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| format!("fenêtre impossible : {e}"))?;
    window.set_target_fps(60);

    let mut g = Game::new(seed).map_err(|e| e.to_string())?;
    let mut zoom_ix = 1usize;
    let mut ss = 2.0f32;
    let mut cv = Canvas::new(1, 1);
    let mut layer = Layer::new();
    let mut light = LightField::new();
    let mut buf = vec![0u32; (W * H) as usize];
    let mut acc = 0.0f32;
    let mut last = Instant::now();
    let mut paused = false;
    let mut dead_t = 0.0f32;
    let mut slow_frames = 0u32;

    layout(&mut g, zoom_ix, ss, &mut cv);

    while window.is_open() && !window.is_key_down(Key::Q) && !window.is_key_down(Key::Escape) {
        // ---- entrées -----------------------------------------------------
        let mut inp = crate::input::Input::default();
        for k in window.get_keys() {
            match k {
                Key::Left | Key::A => inp.left = true,
                Key::Right | Key::D => inp.right = true,
                Key::Up | Key::W => inp.up = true,
                Key::Down | Key::S => inp.down = true,
                Key::Space | Key::X => {
                    inp.attack = true;
                    inp.attack_held = true;
                }
                _ => {}
            }
        }
        if window.is_key_pressed(Key::P, minifb::KeyRepeat::No) {
            paused = !paused;
        }
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            g.respawn();
            dead_t = 0.0;
        }
        let zin = window.is_key_pressed(Key::Equal, minifb::KeyRepeat::No);
        let zout = window.is_key_pressed(Key::Minus, minifb::KeyRepeat::No);
        if zin || zout {
            zoom_ix = (zoom_ix as i32 + if zin { 1 } else { -1 }).clamp(0, 3) as usize;
            layout(&mut g, zoom_ix, ss, &mut cv);
        }

        // ---- simulation ----------------------------------------------------
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        if !paused {
            acc = (acc + dt).min(0.25);
            let mut first = inp;
            let mut rest = first;
            rest.attack = false;
            while acc >= SIM_DT {
                g.update(SIM_DT, &first);
                first = rest;
                acc -= SIM_DT;
            }
            // Réapparition automatique après le foncé de mort.
            if matches!(g.phase, Phase::Dying(_)) {
                dead_t += dt;
            } else if matches!(g.phase, Phase::Dead) {
                dead_t += dt;
                if dead_t > 1.4 {
                    g.respawn();
                    dead_t = 0.0;
                }
            } else {
                dead_t = 0.0;
            }
        }

        // ---- rendu ---------------------------------------------------------
        render::draw(&g, &mut cv, &mut layer, &mut light, ss);
        if matches!(g.phase, Phase::Dying(_) | Phase::Dead) {
            let k = (dead_t / 1.2).clamp(0.0, 1.0);
            for px in cv.px.iter_mut() {
                *px = px.lerp(rgb(30, 8, 8), k * 0.9);
            }
        }
        draw_hud(&g, &mut cv, ss);
        if paused {
            overlay(&mut cv, ss, "PAUSE", "P REPRENDRE   R RECOMMENCER   Q QUITTER");
        } else if g.phase == Phase::Victory {
            overlay(&mut cv, ss, "LE SCEAU EST BRISE", "GANAR N'EST PLUS. ZAHRA EST LIBREE.   R RECOMMENCER");
        }

        // Canvas -> framebuffer.
        canvas_to_buffer(&cv, &mut buf, W, H);
        window
            .update_with_buffer(&buf, W as usize, H as usize)
            .map_err(|e| e.to_string())?;

        // Adaptation : si le rendu est trop lent, on baisse le sur-échantillonnage.
        let spent = (Instant::now() - now).as_secs_f32();
        if spent > 0.024 && ss > 1.25 {
            slow_frames += 1;
            if slow_frames > 40 {
                ss = (ss - 0.5).max(1.0);
                slow_frames = 0;
                layout(&mut g, zoom_ix, ss, &mut cv);
            }
        } else {
            slow_frames = 0;
        }
    }
    Ok(())
}

fn layout(g: &mut Game, zoom_ix: usize, ss: f32, cv: &mut Canvas) {
    let vh = TILE * VIEWS_TALL[zoom_ix.min(3)];
    let vw = vh * W as f32 / H as f32;
    g.set_view_size(vw, vh);
    cv.resize((vw * ss).round() as i32, (vh * ss).round() as i32);
}

fn canvas_to_buffer(cv: &Canvas, buf: &mut [u32], w: i32, h: i32) {
    // Le canvas est (vw*ss)x(vh*ss) : il peut déborder du cadre — on
    // rééchantillonne vers w x h via le pixbuf de Canvas.
    let mut tmp = vec![Rgb::BLACK; (w * h) as usize];
    cv.resample_into(&mut tmp, w, h);
    for (d, s) in buf.iter_mut().zip(tmp.iter()) {
        *d = ((s.r as u32) << 16) | ((s.g as u32) << 8) | s.b as u32;
    }
}

// ---------------------------------------------------------------- HUD

/// Caméra "écran" : dessine en coordonnées art-pixels depuis le coin haut gauche.
fn screen_cam(ss: f32) -> crate::gfx::canvas::Cam {
    crate::gfx::canvas::Cam { ox: 0.0, oy: 0.0, s: ss }
}

fn draw_hud(g: &Game, cv: &mut Canvas, ss: f32) {
    let cam = screen_cam(ss);
    // Mise en page en pixels art (la cam multiplie par ss) ; le texte et les
    // aplats écrivent directement dans le canvas -> on convertit à la main.
    let band_h = (22.0 * ss) as i32;
    for y in 0..band_h {
        for x in 0..cv.w {
            let i = (y * cv.w + x) as usize;
            cv.px[i] = cv.px[i].lerp(rgb(14, 10, 18), 0.55);
        }
    }

    let t = g.time;
    let v = |x: f32, y: f32| v2(x, y);
    let sc = ss.round().max(1.0) as i32; // ~19 px écran par glyphe, stable

    // Cœurs.
    let full = g.inv.hp / 2;
    let half = g.inv.hp % 2 == 1;
    let total = g.inv.hp_max / 2 + if g.inv.hp_max % 2 == 1 { 1 } else { 0 };
    let mut hx = 10.0f32;
    for i in 0..total {
        let filled = if i < full {
            1.0
        } else if i == full && half {
            0.45
        } else {
            0.0
        };
        let beat = if i < full { 1.0 + (t * 2.4 + i as f32).sin().abs() * 0.06 } else { 1.0 };
        heart_icon(cv, &cam, v(hx, 11.0), beat, filled);
        hx += 16.0;
    }
    // Gemmes, clés, fragments du sceau, reliques.
    let mut ix = hx + 8.0;
    gem_icon(cv, &cam, v(ix, 11.0));
    let s = format!("{}", g.inv.gems);
    draw_text(cv, (ix + 8.0) as i32 * ss as i32, (4.0 * ss) as i32, &s, sc, rgb(120, 220, 230));
    ix += 8.0 + text_width(&s, sc) as f32 / ss + 12.0;
    key_icon(cv, &cam, v(ix, 11.0));
    let s = format!("{}", g.inv.keys);
    draw_text(cv, (ix + 8.0) as i32 * ss as i32, (4.0 * ss) as i32, &s, sc, rgb(240, 205, 110));
    ix += 8.0 + text_width(&s, sc) as f32 / ss + 12.0;
    for k in 0..4u32 {
        seal_icon(cv, &cam, v(ix, 11.0), g.inv.seals > k, t);
        ix += 14.0;
    }
    if g.inv.mirror {
        mirror_icon(cv, &cam, v(ix + 4.0, 11.0), t);
        ix += 20.0;
    }
    if g.inv.silver_sword {
        sword_icon(cv, &cam, v(ix + 4.0, 11.0));
    }

    // Nom du monde, à droite du bandeau.
    let name = g.world().name;
    let wpx = text_width(name, sc);
    draw_text(cv, cv.w - wpx - (8.0 * ss) as i32, (4.0 * ss) as i32, name, sc, rgb(226, 216, 190));

    // Message courant, bas d'écran.
    if let Some((msg, _, warn)) = &g.msg {
        let col = if *warn { rgb(240, 140, 96) } else { rgb(244, 234, 205) };
        let wpx = text_width(msg, sc);
        let x = ((cv.w - wpx) / 2).max((6.0 * ss) as i32);
        let y = cv.h - (26.0 * ss) as i32;
        let x0 = (x - (6.0 * ss) as i32).max(0);
        let x1 = (x + wpx + (6.0 * ss) as i32).min(cv.w);
        let y0 = (y - (4.0 * ss) as i32).max(0);
        let y1 = (y + (16.0 * ss) as i32).min(cv.h);
        for yy in y0..y1 {
            for xx in x0..x1 {
                let i = (yy * cv.w + xx) as usize;
                cv.px[i] = cv.px[i].lerp(rgb(10, 8, 14), 0.6);
            }
        }
        draw_text(cv, x, y, msg, sc, col);
    }
}

fn heart_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2, beat: f32, filled: f32) {
    let c = rgb(226, 60, 66).lerp(rgb(70, 40, 48), 1.0 - filled);
    let hi = rgb(246, 130, 130).lerp(rgb(96, 58, 66), 1.0 - filled);
    let r = 5.2 * beat;
    crate::adventure::render::wcircle(cv, cam, at.add(v2(-r * 0.5, -r * 0.28)), r * 0.58, c, 10);
    crate::adventure::render::wcircle(cv, cam, at.add(v2(r * 0.5, -r * 0.28)), r * 0.58, c, 10);
    crate::adventure::render::wpoly(
        cv,
        cam,
        &[
            at.add(v2(-r * 1.02, -r * 0.1)),
            at.add(v2(r * 1.02, -r * 0.1)),
            at.add(v2(0.0, r * 1.15)),
        ],
        c,
    );
    if filled > 0.8 {
        crate::adventure::render::wcircle(cv, cam, at.add(v2(-r * 0.38, -r * 0.42)), r * 0.2, hi, 5);
    }
}

fn gem_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2) {
    let c = rgb(80, 200, 210);
    let r = 5.0;
    crate::adventure::render::wpoly(
        cv,
        cam,
        &[
            at.add(v2(0.0, -r)),
            at.add(v2(r * 0.84, -r * 0.2)),
            at.add(v2(0.0, r)),
            at.add(v2(-r * 0.84, -r * 0.2)),
        ],
        c,
    );
    crate::adventure::render::wpoly(
        cv,
        cam,
        &[at.add(v2(0.0, -r)), at.add(v2(r * 0.84, -r * 0.2)), at.add(v2(0.0, -r * 0.1))],
        rgb(160, 240, 244),
    );
}

fn key_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2) {
    let c = rgb(238, 200, 100);
    let r = 3.0;
    crate::adventure::render::wcircle(cv, cam, at.add(v2(-r, 0.0)), r, c, 8);
    crate::adventure::render::wcircle(cv, cam, at.add(v2(-r, 0.0)), r * 0.4, rgb(30, 24, 18), 5);
    crate::adventure::render::wline(cv, cam, at.add(v2(-0.4, 0.0)), at.add(v2(6.0, 0.0)), 1.8, c);
    crate::adventure::render::wline(cv, cam, at.add(v2(3.6, 0.0)), at.add(v2(3.6, 2.6)), 1.6, c);
}

fn seal_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2, filled: bool, t: f32) {
    let c = if filled { rgb(244, 204, 96) } else { rgb(96, 82, 52) };
    let glow = if filled { 1.0 + 0.12 * (t * 3.0).sin() } else { 1.0 };
    let r = 5.5 * glow;
    crate::adventure::render::wpoly(
        cv,
        cam,
        &[
            at.add(v2(0.0, -r)),
            at.add(v2(r * 0.84, 0.0)),
            at.add(v2(0.0, r)),
            at.add(v2(-r * 0.84, 0.0)),
        ],
        c,
    );
    if filled {
        crate::adventure::render::wcircle(cv, cam, at, 1.4, rgb(255, 240, 190), 5);
    }
}

fn mirror_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2, t: f32) {
    let c = rgb(200, 214, 232);
    crate::adventure::render::wcircle(cv, cam, at.add(v2(0.0, -1.5)), 4.4, rgb(150, 156, 172), 10);
    crate::adventure::render::wcircle(cv, cam, at.add(v2(0.0, -1.5)), 3.2, c, 9);
    crate::adventure::render::wline(cv, cam, at.add(v2(0.0, 2.2)), at.add(v2(0.0, 5.6)), 1.8, rgb(150, 156, 172));
    if (t * 2.2).sin() > 0.0 {
        crate::adventure::render::wline(cv, cam, at.add(v2(-1.6, -3.0)), at.add(v2(0.6, -0.6)), 1.0, rgb(255, 255, 255));
    }
}

fn sword_icon(cv: &mut Canvas, cam: &crate::gfx::canvas::Cam, at: crate::util::V2) {
    crate::adventure::render::wline(cv, cam, at.add(v2(-3.5, 4.0)), at.add(v2(3.5, -4.0)), 2.2, rgb(240, 246, 255));
    crate::adventure::render::wline(cv, cam, at.add(v2(-2.8, 5.0)), at.add(v2(-0.8, 3.0)), 1.6, rgb(190, 158, 96));
}

fn overlay(cv: &mut Canvas, ss: f32, title: &str, sub: &str) {
    for p in cv.px.iter_mut() {
        *p = p.lerp(rgb(8, 6, 12), 0.62);
    }
    let tsc = (ss * 2.5).round().max(2.0) as i32;
    let ssc = (ss * 1.5).round().max(1.0) as i32;
    let w = text_width(title, tsc);
    draw_text(cv, (cv.w - w) / 2, (cv.h as f32 * 0.42) as i32, title, tsc, rgb(240, 205, 110));
    let w2 = text_width(sub, ssc);
    draw_text(cv, (cv.w - w2) / 2, (cv.h as f32 * 0.42) as i32 + tsc * 10, sub, ssc, rgb(220, 210, 190));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le HUD, la police et les icônes rendent sans paniquer — et on peut
    /// inspecter le résultat : `cargo test --release hud -- --nocapture`.
    #[test]
    fn hud_frame_renders() {
        let mut g = Game::new(7).unwrap();
        g.inv.seals = 2;
        g.inv.keys = 3;
        g.inv.gems = 27;
        g.inv.mirror = true;
        g.inv.silver_sword = true;
        g.say("Fragment du sceau 2/4 ! La porte du Sanctuaire frémit…", 5.0, false);
        let ss = 2.0;
        let vh = TILE * 10.0;
        let vw = vh * W as f32 / H as f32;
        g.set_view_size(vw, vh);
        let mut cv = Canvas::new((vw * ss) as i32, (vh * ss) as i32);
        let mut layer = Layer::new();
        let mut light = LightField::new();
        render::draw(&g, &mut cv, &mut layer, &mut light, ss);
        draw_hud(&g, &mut cv, ss);
        let mut buf = vec![0u32; (W * H) as usize];
        canvas_to_buffer(&cv, &mut buf, W, H);
        // Reconversion en Rgb pour l'aperçu PNG.
        let mut px = vec![Rgb::BLACK; (W * H) as usize];
        for (d, s) in px.iter_mut().zip(buf.iter()) {
            *d = Rgb {
                r: ((*s >> 16) & 0xFF) as u8,
                g: ((*s >> 8) & 0xFF) as u8,
                b: (*s & 0xFF) as u8,
            };
        }
        let png = crate::gfx::png::encode(&px, W, H, 1);
        std::fs::write("/tmp/opencode/hud.png", png).unwrap();
    }

    #[test]
    fn font_covers_the_alphabet() {
        for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .,!?'-:+/()".chars() {
            assert!(
                FONT.iter().any(|(g, _)| *g == c),
                "glyphe manquant : {c}"
            );
        }
        assert_eq!(normalize('é'), 'E');
        assert_eq!(normalize('’'), '\'');
    }
}
