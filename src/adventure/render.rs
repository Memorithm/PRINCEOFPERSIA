//! The SNES-style renderer.
//!
//! Everything is drawn as flat, cel-shaded vector shapes into the
//! super-sampled canvas — tiles get a top face and a lit front face, water
//! shimmers, trees cast blob shadows and every character is a small directional
//! sprite with a walk cycle. The look is 16-bit: strong palettes, hard
//! highlights, soft ambient occlusion where walls meet the ground.

use crate::adventure::world::{FoeKind, Pickup, ThemeName, Tile, TILE};
use crate::adventure::Game;
use crate::gfx::canvas::{Cam, Canvas, LightField};
use crate::gfx::color::rgb;
use crate::gfx::color::Rgb;
use crate::gfx::layer::Layer;
use crate::gfx::target::Target;
use crate::util::{hash2, hashf, noise1, v2, V2};

// ---------------------------------------------------------------- palettes

pub struct Palette {
    /// Ground base + variation.
    pub ground_a: Rgb,
    pub ground_b: Rgb,
    pub ground_speck: Rgb,
    /// Wall faces.
    pub wall_top: Rgb,
    pub wall_face: Rgb,
    pub wall_dark: Rgb,
    pub wall_mortar: Rgb,
    /// Water.
    pub water_deep: Rgb,
    pub water_hi: Rgb,
    /// Foliage.
    pub leaf_dark: Rgb,
    pub leaf_mid: Rgb,
    pub leaf_hi: Rgb,
    pub trunk: Rgb,
    /// Pit / chasm.
    pub pit: Rgb,
    /// Accents.
    pub roof: Rgb,
    pub torch_glow: Rgb,
}

const P_VALLEY: Palette = Palette {
    ground_a: rgb(196, 168, 106),
    ground_b: rgb(206, 180, 118),
    ground_speck: rgb(172, 144, 88),
    wall_top: rgb(214, 190, 140),
    wall_face: rgb(156, 122, 82),
    wall_dark: rgb(112, 86, 58),
    wall_mortar: rgb(128, 100, 66),
    water_deep: rgb(52, 108, 168),
    water_hi: rgb(120, 178, 226),
    leaf_dark: rgb(44, 104, 56),
    leaf_mid: rgb(66, 138, 72),
    leaf_hi: rgb(110, 176, 96),
    trunk: rgb(112, 78, 48),
    pit: rgb(24, 18, 26),
    roof: rgb(178, 74, 60),
    torch_glow: rgb(255, 170, 70),
};

const P_SHADOW: Palette = Palette {
    ground_a: rgb(94, 84, 116),
    ground_b: rgb(106, 96, 130),
    ground_speck: rgb(76, 66, 98),
    wall_top: rgb(120, 110, 146),
    wall_face: rgb(72, 62, 94),
    wall_dark: rgb(46, 38, 64),
    wall_mortar: rgb(54, 46, 72),
    water_deep: rgb(30, 22, 48),
    water_hi: rgb(96, 80, 140),
    leaf_dark: rgb(40, 34, 58),
    leaf_mid: rgb(58, 50, 82),
    leaf_hi: rgb(88, 78, 114),
    trunk: rgb(52, 40, 40),
    pit: rgb(10, 6, 16),
    roof: rgb(96, 44, 66),
    torch_glow: rgb(160, 110, 255),
};

const P_DUNGEON: Palette = Palette {
    ground_a: rgb(126, 112, 96),
    ground_b: rgb(136, 122, 104),
    ground_speck: rgb(104, 92, 78),
    wall_top: rgb(148, 134, 116),
    wall_face: rgb(96, 84, 72),
    wall_dark: rgb(62, 54, 46),
    wall_mortar: rgb(76, 66, 56),
    water_deep: rgb(40, 70, 96),
    water_hi: rgb(90, 130, 160),
    leaf_dark: rgb(60, 70, 50),
    leaf_mid: rgb(80, 92, 64),
    leaf_hi: rgb(110, 124, 88),
    trunk: rgb(80, 62, 44),
    pit: rgb(14, 12, 12),
    roof: rgb(120, 70, 50),
    torch_glow: rgb(255, 150, 60),
};

const P_PALACE: Palette = Palette {
    ground_a: rgb(216, 192, 142),
    ground_b: rgb(228, 206, 158),
    ground_speck: rgb(190, 164, 118),
    wall_top: rgb(238, 220, 172),
    wall_face: rgb(186, 152, 100),
    wall_dark: rgb(136, 106, 68),
    wall_mortar: rgb(154, 124, 82),
    water_deep: rgb(64, 140, 190),
    water_hi: rgb(140, 200, 236),
    leaf_dark: rgb(52, 116, 62),
    leaf_mid: rgb(76, 148, 78),
    leaf_hi: rgb(122, 188, 102),
    trunk: rgb(120, 84, 52),
    pit: rgb(26, 20, 28),
    roof: rgb(64, 130, 150),
    torch_glow: rgb(255, 180, 80),
};

const P_TOWER: Palette = Palette {
    ground_a: rgb(104, 96, 118),
    ground_b: rgb(116, 108, 132),
    ground_speck: rgb(84, 76, 98),
    wall_top: rgb(136, 126, 152),
    wall_face: rgb(84, 74, 102),
    wall_dark: rgb(54, 46, 70),
    wall_mortar: rgb(64, 56, 80),
    water_deep: rgb(36, 40, 72),
    water_hi: rgb(90, 96, 150),
    leaf_dark: rgb(46, 42, 66),
    leaf_mid: rgb(64, 58, 88),
    leaf_hi: rgb(96, 88, 120),
    trunk: rgb(56, 44, 44),
    pit: rgb(10, 8, 14),
    roof: rgb(130, 50, 70),
    torch_glow: rgb(190, 120, 255),
};

const P_FOREST: Palette = Palette {
    ground_a: rgb(96, 138, 82),
    ground_b: rgb(108, 152, 92),
    ground_speck: rgb(78, 114, 68),
    wall_top: rgb(120, 160, 100),
    wall_face: rgb(70, 102, 62),
    wall_dark: rgb(44, 66, 40),
    wall_mortar: rgb(54, 82, 48),
    water_deep: rgb(38, 96, 110),
    water_hi: rgb(110, 180, 186),
    leaf_dark: rgb(30, 78, 44),
    leaf_mid: rgb(48, 112, 60),
    leaf_hi: rgb(88, 158, 82),
    trunk: rgb(88, 62, 40),
    pit: rgb(16, 20, 18),
    roof: rgb(140, 84, 52),
    torch_glow: rgb(255, 176, 80),
};

const P_CRYSTAL: Palette = Palette {
    ground_a: rgb(96, 104, 132),
    ground_b: rgb(108, 118, 148),
    ground_speck: rgb(76, 82, 110),
    wall_top: rgb(150, 170, 208),
    wall_face: rgb(86, 94, 128),
    wall_dark: rgb(52, 58, 86),
    wall_mortar: rgb(64, 70, 100),
    water_deep: rgb(48, 120, 172),
    water_hi: rgb(140, 210, 240),
    leaf_dark: rgb(56, 90, 120),
    leaf_mid: rgb(80, 120, 150),
    leaf_hi: rgb(120, 170, 200),
    trunk: rgb(70, 76, 106),
    pit: rgb(8, 10, 20),
    roof: rgb(110, 160, 220),
    torch_glow: rgb(140, 220, 255),
};

const P_DESERT: Palette = Palette {
    ground_a: rgb(216, 182, 116),
    ground_b: rgb(228, 196, 130),
    ground_speck: rgb(190, 156, 96),
    wall_top: rgb(236, 206, 146),
    wall_face: rgb(178, 142, 90),
    wall_dark: rgb(128, 98, 60),
    wall_mortar: rgb(148, 116, 72),
    water_deep: rgb(46, 150, 168),
    water_hi: rgb(130, 214, 224),
    leaf_dark: rgb(58, 110, 60),
    leaf_mid: rgb(84, 144, 74),
    leaf_hi: rgb(128, 184, 96),
    trunk: rgb(122, 88, 54),
    pit: rgb(30, 22, 22),
    roof: rgb(180, 90, 60),
    torch_glow: rgb(255, 180, 90),
};

const P_NECRO: Palette = Palette {
    ground_a: rgb(168, 156, 134),
    ground_b: rgb(180, 168, 146),
    ground_speck: rgb(140, 128, 108),
    wall_top: rgb(198, 188, 162),
    wall_face: rgb(134, 124, 102),
    wall_dark: rgb(92, 84, 68),
    wall_mortar: rgb(110, 100, 82),
    water_deep: rgb(42, 92, 96),
    water_hi: rgb(110, 176, 176),
    leaf_dark: rgb(70, 84, 62),
    leaf_mid: rgb(94, 110, 80),
    leaf_hi: rgb(126, 142, 104),
    trunk: rgb(96, 82, 60),
    pit: rgb(12, 12, 14),
    roof: rgb(150, 120, 60),
    torch_glow: rgb(180, 230, 200),
};

const P_SWAMP: Palette = Palette {
    ground_a: rgb(94, 108, 74),
    ground_b: rgb(104, 120, 82),
    ground_speck: rgb(76, 88, 60),
    wall_top: rgb(118, 134, 94),
    wall_face: rgb(72, 84, 56),
    wall_dark: rgb(46, 56, 38),
    wall_mortar: rgb(56, 68, 46),
    water_deep: rgb(44, 84, 66),
    water_hi: rgb(104, 164, 120),
    leaf_dark: rgb(36, 66, 44),
    leaf_mid: rgb(56, 92, 56),
    leaf_hi: rgb(92, 130, 72),
    trunk: rgb(70, 58, 40),
    pit: rgb(10, 14, 10),
    roof: rgb(110, 90, 50),
    torch_glow: rgb(160, 255, 150),
};

const P_PASS: Palette = Palette {
    ground_a: rgb(150, 134, 112),
    ground_b: rgb(162, 146, 122),
    ground_speck: rgb(122, 108, 88),
    wall_top: rgb(178, 162, 138),
    wall_face: rgb(116, 102, 84),
    wall_dark: rgb(78, 68, 56),
    wall_mortar: rgb(94, 82, 68),
    water_deep: rgb(50, 100, 140),
    water_hi: rgb(120, 176, 210),
    leaf_dark: rgb(52, 88, 52),
    leaf_mid: rgb(74, 116, 64),
    leaf_hi: rgb(110, 152, 84),
    trunk: rgb(90, 68, 46),
    pit: rgb(14, 12, 14),
    roof: rgb(140, 80, 60),
    torch_glow: rgb(255, 170, 80),
};

const P_MINES: Palette = Palette {
    ground_a: rgb(122, 96, 72),
    ground_b: rgb(134, 106, 80),
    ground_speck: rgb(96, 74, 56),
    wall_top: rgb(150, 120, 90),
    wall_face: rgb(92, 70, 52),
    wall_dark: rgb(58, 44, 34),
    wall_mortar: rgb(72, 54, 42),
    water_deep: rgb(52, 90, 110),
    water_hi: rgb(120, 170, 190),
    leaf_dark: rgb(64, 72, 48),
    leaf_mid: rgb(86, 96, 62),
    leaf_hi: rgb(116, 128, 84),
    trunk: rgb(84, 62, 40),
    pit: rgb(12, 9, 8),
    roof: rgb(196, 120, 60),
    torch_glow: rgb(255, 160, 70),
};

const P_OASIS: Palette = Palette {
    ground_a: rgb(120, 168, 92),
    ground_b: rgb(132, 182, 102),
    ground_speck: rgb(98, 142, 76),
    wall_top: rgb(150, 194, 116),
    wall_face: rgb(88, 128, 70),
    wall_dark: rgb(56, 88, 48),
    wall_mortar: rgb(68, 106, 58),
    water_deep: rgb(52, 160, 190),
    water_hi: rgb(150, 224, 238),
    leaf_dark: rgb(40, 110, 60),
    leaf_mid: rgb(64, 144, 78),
    leaf_hi: rgb(112, 192, 104),
    trunk: rgb(110, 84, 58),
    pit: rgb(18, 22, 20),
    roof: rgb(80, 150, 160),
    torch_glow: rgb(170, 255, 200),
};

const P_SANCTUARY: Palette = Palette {
    ground_a: rgb(178, 182, 202),
    ground_b: rgb(190, 194, 214),
    ground_speck: rgb(148, 152, 176),
    wall_top: rgb(212, 218, 238),
    wall_face: rgb(142, 148, 178),
    wall_dark: rgb(96, 102, 134),
    wall_mortar: rgb(116, 122, 154),
    water_deep: rgb(70, 120, 190),
    water_hi: rgb(150, 200, 245),
    leaf_dark: rgb(70, 110, 110),
    leaf_mid: rgb(96, 140, 136),
    leaf_hi: rgb(136, 176, 168),
    trunk: rgb(104, 100, 120),
    pit: rgb(12, 12, 20),
    roof: rgb(150, 160, 220),
    torch_glow: rgb(190, 220, 255),
};

const P_THRONE: Palette = Palette {
    ground_a: rgb(66, 52, 78),
    ground_b: rgb(76, 60, 90),
    ground_speck: rgb(50, 38, 62),
    wall_top: rgb(104, 84, 122),
    wall_face: rgb(58, 44, 72),
    wall_dark: rgb(34, 24, 46),
    wall_mortar: rgb(42, 32, 56),
    water_deep: rgb(60, 26, 66),
    water_hi: rgb(140, 70, 150),
    leaf_dark: rgb(44, 32, 56),
    leaf_mid: rgb(62, 46, 78),
    leaf_hi: rgb(92, 70, 110),
    trunk: rgb(44, 34, 44),
    pit: rgb(6, 3, 10),
    roof: rgb(170, 40, 60),
    torch_glow: rgb(255, 110, 90),
};

fn pal(theme: ThemeName) -> &'static Palette {
    match theme {
        ThemeName::Valley => &P_VALLEY,
        ThemeName::Shadow => &P_SHADOW,
        ThemeName::Dungeon => &P_DUNGEON,
        ThemeName::Palace => &P_PALACE,
        ThemeName::Forest => &P_FOREST,
        ThemeName::Crystal => &P_CRYSTAL,
        ThemeName::Desert => &P_DESERT,
        ThemeName::Necro => &P_NECRO,
        ThemeName::Swamp => &P_SWAMP,
        ThemeName::Pass => &P_PASS,
        ThemeName::Mines => &P_MINES,
        ThemeName::Oasis => &P_OASIS,
        ThemeName::Sanctuary => &P_SANCTUARY,
        ThemeName::Tower => &P_TOWER,
        ThemeName::Throne => &P_THRONE,
    }
}

// ---------------------------------------------------------------- helpers

/// Flat filled polygon.
fn poly(t: &mut impl Target, pts: &[V2], col: Rgb) {
    crate::art::shape::flat(t, pts, col, 1.0);
}

/// Axis-aligned rect in world coords.
fn wrect(t: &mut impl Target, cam: &Cam, x: f32, y: f32, w: f32, h: f32, col: Rgb) {
    let a = cam.p(v2(x, y));
    let b = cam.p(v2(x + w, y + h));
    poly(
        t,
        &[v2(a.x, a.y), v2(b.x, a.y), v2(b.x, b.y), v2(a.x, b.y)],
        col,
    );
}

/// Filled circle (world centre, world radius).
fn wcircle(t: &mut impl Target, cam: &Cam, c: V2, r: f32, col: Rgb, n: usize) {
    let cc = cam.p(c);
    let rr = cam.l(r);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        pts.push(v2(cc.x + a.cos() * rr, cc.y + a.sin() * rr));
    }
    poly(t, &pts, col);
}

fn wline(t: &mut impl Target, cam: &Cam, a: V2, b: V2, wdt: f32, col: Rgb) {
    crate::art::shape::contour(t, cam.p(a), cam.p(b), cam.l(wdt), col, 1.0);
}

/// Soft round shadow under an entity — two nested ellipses fake a blur.
fn shadow(t: &mut impl Target, cam: &Cam, at: V2, rx: f32, ry: f32, alpha: f32) {
    let c = cam.p(at);
    let n = 14;
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        pts.push(v2(a.cos(), a.sin()));
    }
    for (scale, k) in [(1.25f32, 0.45f32), (0.92, 0.75)] {
        let (rx, ry) = (cam.l(rx * scale), cam.l(ry * scale));
        let ring: Vec<V2> = pts.iter().map(|d| v2(c.x + d.x * rx, c.y + d.y * ry)).collect();
        crate::art::shape::flat(t, &ring, Rgb::BLACK, alpha * 0.35 * k);
    }
}

// ---------------------------------------------------------------- terrain

/// Draw one tile of terrain. `time` drives the animated tiles.
fn draw_tile(g: &Game, t: &mut Canvas, cam: &Cam, tx: i32, ty: i32, time: f32) {
    let p = pal(g.theme());
    let tile = g.world().tile(tx, ty);
    let x = tx as f32 * TILE;
    let y = ty as f32 * TILE;
    let h = hash2(tx, ty);
    let hf = |salt: i32| -> f32 { hashf(tx, ty, salt) };

    match tile {
        Tile::Grass | Tile::Flower => {
            // Slow, organic drift of tone across the meadow — no checkerboard.
            let drift = (noise1(x * 0.021 + y * 0.029, 11) * 0.5 + 0.5).clamp(0.0, 1.0);
            let base = p
                .ground_a
                .lerp(p.ground_b, drift)
                .lerp(p.ground_speck, (drift - 0.5).abs() * 0.5);
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, base);
            // Tufts of grass.
            let tufts = 3 + (h % 3) as usize;
            for k in 0..tufts {
                let fx = x + hf(10 + k as i32) * TILE;
                let fy = y + hf(20 + k as i32) * TILE;
                let lean = (hf(30 + k as i32) - 0.5) * 3.0;
                let col = if k % 2 == 0 { p.ground_speck } else { p.leaf_mid };
                wline(t, cam, v2(fx, fy), v2(fx + lean, fy - 4.0), 1.1, col);
            }
            if tile == Tile::Flower || hf(7) > 0.93 {
                // A little flower.
                let fx = x + 6.0 + hf(11) * 12.0;
                let fy = y + 6.0 + hf(12) * 12.0;
                let petal = [rgb(240, 240, 230), rgb(240, 190, 90), rgb(222, 110, 130)]
                    [(h % 3) as usize];
                for d in 0..4 {
                    let a = d as f32 * std::f32::consts::FRAC_PI_2;
                    wcircle(t, cam, v2(fx + a.cos() * 2.0, fy + a.sin() * 2.0), 1.4, petal, 6);
                }
                wcircle(t, cam, v2(fx, fy), 1.2, rgb(250, 214, 96), 6);
            }
        }
        Tile::Floor | Tile::Portal | Tile::Cave | Tile::MirrorSlab => {
            let drift = (noise1(x * 0.025 + y * 0.033, 13) * 0.5 + 0.5).clamp(0.0, 1.0);
            let base = p
                .ground_a
                .lerp(p.ground_b, drift)
                .lerp(p.ground_speck, (drift - 0.5).abs() * 0.4);
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, base);
            // Speckles + faint slab seams.
            for k in 0..4 {
                let sx = x + hf(40 + k as i32) * TILE;
                let sy = y + hf(50 + k as i32) * TILE;
                wrect(t, cam, sx, sy, 1.6, 1.0, p.ground_speck);
            }
            if hf(60) > 0.75 {
                wline(t, cam, v2(x + 3.0, y + TILE * 0.55), v2(x + TILE - 3.0, y + TILE * 0.55), 0.9, p.ground_speck);
            }

            match tile {
                Tile::Portal => draw_portal(t, cam, x, y, time, p),
                Tile::Cave => draw_cave(t, cam, x, y, p),
                Tile::MirrorSlab => draw_mirror_slab(t, cam, x, y, time, g),
                _ => {}
            }
        }
        Tile::Bridge => {
            // Planks over water or pit.
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, rgb(146, 104, 62));
            for k in 0..4 {
                let px = x + 3.0 + k as f32 * 6.0;
                wrect(t, cam, px, y, 2.2, TILE, rgb(120, 84, 50));
            }
            wrect(t, cam, x, y, TILE + 0.5, 2.5, rgb(96, 66, 40));
            wrect(t, cam, x, y + TILE - 2.5, TILE + 0.5, 2.5, rgb(96, 66, 40));
        }
        Tile::Water => {
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.water_deep);
            // Two shimmering wave lines per tile.
            for k in 0..2 {
                let phase = time * 1.7 + tx as f32 * 0.9 + ty as f32 * 1.7 + k as f32 * 2.1;
                let wy = y + 6.0 + k as f32 * 10.0 + phase.sin() * 2.0;
                let wx = x + 4.0 + ((phase * 0.7).sin() * 0.5 + 0.5) * 6.0;
                wline(t, cam, v2(wx, wy), v2(wx + 9.0, wy), 1.6, p.water_hi);
            }
            // Occasional sun glints.
            let tw = (time * 1.6 + hf(8) * std::f32::consts::TAU).sin();
            if tw > 0.86 {
                let gx = x + 5.0 + hf(9) * 14.0;
                let gy = y + 5.0 + hf(10) * 14.0;
                let g = (tw - 0.86) / 0.14;
                wcircle(t, cam, v2(gx, gy), 1.6 * g, rgb(240, 250, 255), 6);
            }
            // Foam on shores facing land.
            let up = !g.world().tile(tx, ty - 1).solid();
            let left = !g.world().tile(tx - 1, ty).solid();
            let down = !g.world().tile(tx, ty + 1).solid();
            let right = !g.world().tile(tx + 1, ty).solid();
            let foam = p.water_hi.lerp(rgb(235, 245, 252), 0.5);
            if up {
                wrect(t, cam, x, y, TILE + 0.5, 2.0, foam);
            }
            if down {
                wrect(t, cam, x, y + TILE - 2.0, TILE + 0.5, 2.0, foam);
            }
            if left {
                wrect(t, cam, x, y, 2.0, TILE + 0.5, foam);
            }
            if right {
                wrect(t, cam, x + TILE - 2.0, y, 2.0, TILE + 0.5, foam);
            }
        }
        Tile::Pit => {
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.pit);
            // Crumbling rim on the open sides.
            let rim = p.wall_dark;
            if g.world().tile(tx, ty - 1).walkable() && !g.world().tile(tx, ty - 1).solid() {
                wrect(t, cam, x, y, TILE + 0.5, 3.0, rim);
            }
            // Faint depth streaks.
            if hf(3) > 0.6 {
                wrect(t, cam, x + 8.0, y + 8.0, 2.0, 10.0, p.pit.lerp(Rgb::BLACK, 0.5));
            }
        }
        Tile::Wall => draw_wall(g, t, cam, tx, ty, p),
        Tile::Tree => {
            // Ground beneath.
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.ground_b);
            shadow(t, cam, v2(x + TILE * 0.5, y + TILE * 0.78), 8.5, 3.2, 1.0);
            let cxm = x + TILE * 0.5;
            let cym = y + TILE * 0.42;
            wrect(t, cam, cxm - 2.5, y + TILE * 0.45, 5.0, TILE * 0.5, p.trunk);
            // Canopy: dark base + mid blobs + highlights.
            wcircle(t, cam, v2(cxm, cym), 11.0, p.leaf_dark, 12);
            wcircle(t, cam, v2(cxm - 4.0, cym - 3.0), 6.5, p.leaf_mid, 10);
            wcircle(t, cam, v2(cxm + 4.5, cym - 1.0), 5.5, p.leaf_mid, 10);
            wcircle(t, cam, v2(cxm - 1.0 + (hf(2) - 0.5) * 4.0, cym - 6.0), 4.0, p.leaf_hi, 8);
            wcircle(t, cam, v2(cxm + 3.0, cym - 5.5 + (hf(3) - 0.5) * 3.0), 2.4, p.leaf_hi, 6);
        }
        Tile::Bush => {
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.ground_b);
            shadow(t, cam, v2(x + TILE * 0.5, y + TILE * 0.72), 7.5, 2.6, 1.0);
            let cxm = x + TILE * 0.5;
            let cym = y + TILE * 0.52;
            wcircle(t, cam, v2(cxm, cym), 8.0, p.leaf_dark, 10);
            wcircle(t, cam, v2(cxm - 2.5, cym - 2.0), 5.0, p.leaf_mid, 8);
            wcircle(t, cam, v2(cxm + 2.5, cym - 1.5), 4.2, p.leaf_mid, 8);
            wcircle(t, cam, v2(cxm - 1.0, cym - 3.5), 2.2, p.leaf_hi, 6);
            if hf(9) > 0.8 {
                wcircle(t, cam, v2(cxm + 3.0, cym + 2.0), 1.4, rgb(210, 60, 80), 6);
            }
        }
        Tile::Statue => {
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.ground_b);
            shadow(t, cam, v2(x + TILE * 0.5, y + TILE * 0.8), 8.0, 2.8, 1.0);
            let cxm = x + TILE * 0.5;
            // Pedestal.
            wrect(t, cam, cxm - 7.0, y + TILE - 7.0, 14.0, 5.0, p.wall_dark);
            wrect(t, cam, cxm - 6.0, y + TILE - 11.0, 12.0, 5.0, p.wall_face);
            // Robed figure.
            poly(
                t,
                &[
                    cam.p(v2(cxm - 5.0, y + TILE - 11.0)),
                    cam.p(v2(cxm + 5.0, y + TILE - 11.0)),
                    cam.p(v2(cxm + 3.0, y + 5.0)),
                    cam.p(v2(cxm - 3.0, y + 5.0)),
                ],
                p.wall_top,
            );
            wcircle(t, cam, v2(cxm, y + 5.5), 3.4, rgb(198, 190, 176), 8);
            wrect(t, cam, cxm - 2.0, y + 5.0, 4.0, 1.4, p.wall_dark); // blind eyes
        }
        Tile::Brazier => {
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.ground_b);
            shadow(t, cam, v2(x + TILE * 0.5, y + TILE * 0.75), 6.0, 2.2, 1.0);
            let cxm = x + TILE * 0.5;
            wrect(t, cam, cxm - 1.6, y + 10.0, 3.2, 8.0, rgb(96, 66, 40));
            wcircle(t, cam, v2(cxm, y + 10.0), 5.2, rgb(120, 84, 50), 8);
            wcircle(t, cam, v2(cxm, y + 9.4), 4.2, rgb(70, 50, 34), 8);
            // Flame.
            let flick = 1.0 + noise1(time * 9.0, tx * 7 + ty * 13) * 0.35;
            let fh = 8.0 * flick;
            poly(
                t,
                &[
                    cam.p(v2(cxm - 3.2, y + 8.0)),
                    cam.p(v2(cxm + 3.2, y + 8.0)),
                    cam.p(v2(cxm + 1.0, y + 8.0 - fh)),
                    cam.p(v2(cxm, y + 8.0 - fh - 2.5)),
                    cam.p(v2(cxm - 1.0, y + 8.0 - fh)),
                ],
                rgb(255, 168, 66),
            );
            poly(
                t,
                &[
                    cam.p(v2(cxm - 1.6, y + 8.0)),
                    cam.p(v2(cxm + 1.6, y + 8.0)),
                    cam.p(v2(cxm, y + 8.0 - fh * 0.62)),
                ],
                rgb(255, 232, 150),
            );
        }
        Tile::Rock => {
            // A boulder with a lit facet and a crack.
            wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.ground_b);
            shadow(t, cam, v2(x + TILE * 0.5, y + TILE * 0.75), 8.5, 3.0, 1.0);
            let cxm = x + TILE * 0.5;
            let cym = y + TILE * 0.52;
            wcircle(t, cam, v2(cxm, cym), 8.6, p.wall_face, 10);
            wcircle(t, cam, v2(cxm - 2.4, cym - 2.6), 4.6, p.wall_top, 8);
            wcircle(t, cam, v2(cxm + 3.2, cym + 2.4), 4.2, p.wall_dark, 8);
            wline(
                t,
                cam,
                v2(cxm - 1.0 + hf(4) * 3.0, cym - 4.0),
                v2(cxm + 2.0 + hf(5) * 3.0, cym + 4.0),
                1.0,
                p.wall_dark,
            );
        }
        Tile::DoorLocked => draw_door(t, cam, x, y, p, false),
        Tile::DoorSealed => draw_door(t, cam, x, y, p, true),
    }
}

/// Masonry with a lit front face and mortar lines — the classic dungeon wall.
fn draw_wall(g: &Game, t: &mut Canvas, cam: &Cam, tx: i32, ty: i32, p: &Palette) {
    let x = tx as f32 * TILE;
    let y = ty as f32 * TILE;
    let h = hash2(tx, ty);

    let face_below = matches!(
        g.world().tile(tx, ty + 1),
        Tile::Floor | Tile::Grass | Tile::Flower | Tile::Bridge | Tile::Water | Tile::Pit
            | Tile::MirrorSlab | Tile::Portal | Tile::Cave
    );

    // Body.
    wrect(t, cam, x, y, TILE + 0.5, TILE + 0.5, p.wall_face);
    // Brick pattern: three courses with offset joints.
    for row in 0..3 {
        let by = y + row as f32 * 8.0;
        wrect(t, cam, x, by, TILE + 0.5, 0.9, p.wall_mortar);
        let off = if (row + h) % 2 == 0 { 0.0 } else { 12.0 };
        for k in 0..2 {
            let bx = x + off + k as f32 * 24.0;
            if bx > x + 1.0 && bx < x + TILE {
                wrect(t, cam, bx, by, 0.9, 8.0, p.wall_mortar);
            }
        }
        // Per-brick highlight/shade for relief.
        if (h + row) % 3 == 0 {
            wrect(t, cam, x + 3.0 + off % 12.0, by + 1.6, 8.0, 1.6, p.wall_top.lerp(p.wall_face, 0.4));
        }
    }
    // Lit cap when open ground above shows the "top" of the wall.
    let above_open = !g.world().tile(tx, ty - 1).solid();
    if above_open {
        wrect(t, cam, x, y, TILE + 0.5, 4.5, p.wall_top);
        wrect(t, cam, x, y + 4.5, TILE + 0.5, 1.4, p.wall_dark);
    }
    // Cast a contact shadow onto open floor below.
    if face_below {
        wrect(t, cam, x, y + TILE, TILE + 0.5, 3.2, Rgb::BLACK.blend_a(0.16));
    }
    // Side shading against openness.
    if !g.world().tile(tx - 1, ty).solid() {
        wrect(t, cam, x, y + 4.5, 1.8, TILE - 4.0, p.wall_dark);
    }
    if !g.world().tile(tx + 1, ty).solid() {
        wrect(t, cam, x + TILE - 1.8, y + 4.5, 1.8, TILE - 4.0, p.wall_dark);
    }
}

trait BlendA {
    fn blend_a(self, a: f32) -> Rgb;
}
impl BlendA for Rgb {
    fn blend_a(self, a: f32) -> Rgb {
        // Pre-blended colour that reads as alpha over typical ground tones.
        self.lerp(Rgb::BLACK, a)
    }
}

fn draw_portal(t: &mut Canvas, cam: &Cam, x: f32, y: f32, time: f32, p: &Palette) {
    let cxm = x + TILE * 0.5;
    let cym = y + TILE * 0.5;
    // Stone kerb.
    wcircle(t, cam, v2(cxm, cym), 10.5, p.wall_face, 14);
    wcircle(t, cam, v2(cxm, cym), 8.5, p.pit, 12);
    // Swirling arcs.
    for k in 0..3 {
        let r = 7.5 - k as f32 * 2.2;
        let spin = time * (1.4 + k as f32 * 0.5) + k as f32 * 2.1;
        let segs = 10;
        let mut pts = Vec::new();
        for i in 0..=segs {
            let a0 = spin + i as f32 / segs as f32 * std::f32::consts::TAU;
            let wob = 1.0 + (a0 * 2.0 + time * 3.0).sin() * 0.12;
            pts.push(cam.p(v2(cxm + a0.cos() * r * wob, cym + a0.sin() * r * wob)));
        }
        let col = [
            rgb(150, 120, 220),
            rgb(96, 160, 220),
            rgb(220, 200, 255),
        ][k];
        crate::art::shape::contour(t, pts[0], pts[0], 0.0, col, 0.0);
        for i in 1..pts.len() {
            crate::art::shape::contour(t, pts[i - 1], pts[i], cam.l(1.8), col, 0.85);
        }
    }
    // Bright core.
    wcircle(t, cam, v2(cxm, cym), 2.2 + (time * 3.0).sin().abs() * 0.8, rgb(235, 225, 255), 8);
}

fn draw_cave(t: &mut Canvas, cam: &Cam, x: f32, y: f32, p: &Palette) {
    // Dark mouth framed by rock.
    let cxm = x + TILE * 0.5;
    let base = y + TILE - 2.0;
    wcircle(t, cam, v2(cxm, base), 11.0, p.wall_dark, 12);
    // Clip the bottom half-circle visually with a floor strip.
    wrect(t, cam, x, base, TILE + 0.5, 3.0, p.ground_a);
    wcircle(t, cam, v2(cxm, base + 0.5), 8.6, rgb(8, 6, 10), 12);
    // Depth gradient.
    wcircle(t, cam, v2(cxm, base + 2.5), 5.0, rgb(3, 2, 5), 10);
    // Lintel highlight.
    crate::art::shape::contour(
        t,
        cam.p(v2(cxm - 8.0, base - 3.0)),
        cam.p(v2(cxm + 8.0, base - 3.0)),
        cam.l(2.0),
        p.wall_top,
        0.9,
    );
}

fn draw_mirror_slab(t: &mut Canvas, cam: &Cam, x: f32, y: f32, time: f32, g: &Game) {
    let pad = 2.5;
    let active = g.inv.mirror;
    let glow = if active {
        0.5 + 0.5 * (time * 2.2).sin()
    } else {
        0.0
    };
    let frame = rgb(148, 152, 168);
    let face = rgb(196, 208, 224).lerp(rgb(150, 190, 240), glow * 0.5);
    // Slab body with bevelled edge.
    wrect(t, cam, x + pad, y + pad, TILE - pad * 2.0, TILE - pad * 2.0, frame);
    wrect(t, cam, x + pad + 1.6, y + pad + 1.6, TILE - pad * 2.0 - 3.2, TILE - pad * 2.0 - 3.2, face);
    // Diagonal sheen.
    poly(
        t,
        &[
            cam.p(v2(x + pad + 2.0, y + TILE - pad - 4.0)),
            cam.p(v2(x + TILE - pad - 6.0, y + pad + 2.0)),
            cam.p(v2(x + TILE - pad - 2.0, y + pad + 2.0)),
            cam.p(v2(x + pad + 6.0, y + TILE - pad - 4.0)),
        ],
        rgb(235, 242, 250),
    );
    // Sparkle.
    let sp = (time * 1.3 + x * 0.13 + y * 0.17).sin();
    if sp > 0.55 {
        let sxx = x + TILE * (0.3 + 0.4 * sp.fract());
        let syy = y + TILE * 0.4;
        wline(t, cam, v2(sxx - 2.5, syy), v2(sxx + 2.5, syy), 1.0, rgb(255, 255, 255));
        wline(t, cam, v2(sxx, syy - 2.5), v2(sxx, syy + 2.5), 1.0, rgb(255, 255, 255));
    }
}

fn draw_door(t: &mut Canvas, cam: &Cam, x: f32, y: f32, p: &Palette, sealed: bool) {
    let pad = 1.5;
    if sealed {
        // Golden sealed gate with the seal emblem.
        wrect(t, cam, x + pad, y + pad, TILE - pad * 2.0, TILE - pad * 2.0, rgb(158, 120, 44));
        wrect(t, cam, x + pad + 2.0, y + pad + 2.0, TILE - pad * 2.0 - 4.0, TILE - pad * 2.0 - 4.0, rgb(196, 152, 60));
        let cxm = x + TILE * 0.5;
        let cym = y + TILE * 0.5;
        wcircle(t, cam, v2(cxm, cym), 6.0, rgb(120, 88, 30), 10);
        wcircle(t, cam, v2(cxm, cym), 4.6, rgb(232, 196, 96), 10);
        // Triangle emblem.
        poly(
            t,
            &[
                cam.p(v2(cxm, cym - 3.2)),
                cam.p(v2(cxm + 3.0, cym + 2.2)),
                cam.p(v2(cxm - 3.0, cym + 2.2)),
            ],
            rgb(120, 88, 30),
        );
    } else {
        // Wooden double door with iron bands.
        wrect(t, cam, x + pad, y + pad, TILE - pad * 2.0, TILE - pad * 2.0, rgb(104, 70, 40));
        wrect(t, cam, x + pad + 1.4, y + pad + 1.4, TILE * 0.5 - pad - 2.2, TILE - pad * 2.0 - 2.8, rgb(134, 94, 56));
        wrect(t, cam, x + TILE * 0.5 + 0.8, y + pad + 1.4, TILE * 0.5 - pad - 2.2, TILE - pad * 2.0 - 2.8, rgb(134, 94, 56));
        for band_y in [y + 6.0, y + TILE - 8.0] {
            wrect(t, cam, x + pad, band_y, TILE - pad * 2.0, 2.2, rgb(72, 74, 84));
        }
        // Keyhole plate.
        wrect(t, cam, x + TILE * 0.5 - 1.6, y + TILE * 0.5 - 2.4, 3.2, 5.0, rgb(72, 74, 84));
        wrect(t, cam, x + TILE * 0.5 - 0.7, y + TILE * 0.5 - 1.2, 1.4, 2.6, rgb(24, 24, 30));
    }
    let _ = p;
}

// ---------------------------------------------------------------- actors

const SKIN: Rgb = rgb(226, 178, 132);
const TUNIC: Rgb = rgb(88, 140, 92);
const TUNIC_DARK: Rgb = rgb(64, 108, 72);
const HEADBAND: Rgb = rgb(198, 60, 54);
const BOOT: Rgb = rgb(84, 58, 40);
const STEEL: Rgb = rgb(212, 218, 228);
const SILVER: Rgb = rgb(240, 246, 255);

fn prince(g: &Game, t: &mut Canvas, cam: &Cam, time: f32) {
    let pl = &g.player;
    let p = pl.p;
    let bob = if pl.moving { (pl.anim * 2.0).sin() * 0.8 } else { (time * 2.2).sin() * 0.4 };
    let step = if pl.moving { pl.anim.sin() } else { 0.0 };
    let blink = pl.invuln > 0.0 && (pl.invuln * 14.0) as i32 % 2 == 0;

    shadow(t, cam, v2(p.x, p.y + 7.0), 7.0, 2.6, 1.0);
    if blink {
        return; // invulnerability flicker
    }

    let facing = pl.facing;
    let fv = facing.vec();
    // Feet.
    let foot_off = v2(fv.x, fv.y * 0.4);
    let f1 = v2(p.x - fv.y * 3.0 + foot_off.x * step * 3.0, p.y + 5.0 + foot_off.y.abs() * step * 2.0);
    let f2 = v2(p.x + fv.y * 3.0 - foot_off.x * step * 3.0, p.y + 5.0 - foot_off.y.abs() * step * 2.0);
    wcircle(t, cam, f1, 2.2, BOOT, 6);
    wcircle(t, cam, f2, 2.2, BOOT, 6);

    // Body: tunic cone with belt.
    let body_c = v2(p.x, p.y + bob * 0.5);
    wcircle(t, cam, v2(body_c.x, body_c.y + 1.5), 6.2, TUNIC_DARK, 10);
    wcircle(t, cam, v2(body_c.x, body_c.y), 6.0, TUNIC, 10);
    wrect(t, cam, body_c.x - 5.6, body_c.y + 1.2, 11.2, 2.0, rgb(120, 84, 48)); // belt
    wcircle(t, cam, v2(body_c.x, body_c.y + 2.2), 1.4, rgb(226, 186, 90), 6);   // buckle

    // Arms hinting at shoulders.
    let arm_side = v2(-fv.y, fv.x);
    let shoulder_l = body_c.add(arm_side.mul(-5.4)).add(v2(0.0, -1.0));
    let shoulder_r = body_c.add(arm_side.mul(5.4)).add(v2(0.0, -1.0));
    wcircle(t, cam, shoulder_l, 2.4, TUNIC_DARK, 6);
    wcircle(t, cam, shoulder_r, 2.4, TUNIC_DARK, 6);

    // Head with headband and hair.
    let head = v2(p.x, p.y - 6.0 + bob);
    wcircle(t, cam, head, 4.6, rgb(60, 44, 32), 10); // hair mass
    wcircle(t, cam, head.add(fv.mul(-0.6)), 4.0, SKIN, 10);
    wrect(t, cam, head.x - 4.4, head.y - 1.6, 8.8, 2.4, HEADBAND); // band
    if facing != Dir::Up {
        // Eyes as two dots toward facing.
        let ex = head.x + fv.x * 1.6;
        let ey = head.y + 1.2 + fv.y * 1.0;
        let dx = if fv.x == 0.0 { 1.8 } else { 0.0 };
        wrect(t, cam, ex - dx - 0.7, ey, 1.4, 1.6, rgb(40, 30, 26));
        wrect(t, cam, ex + dx - 0.7, ey, 1.4, 1.6, rgb(40, 30, 26));
    } else {
        wcircle(t, cam, v2(head.x, head.y - 1.5), 2.6, rgb(74, 54, 40), 8); // back of head
    }

    // Sword: idle at side, sweeping arc during attack.
    let blade_col = if g.inv.silver_sword { SILVER } else { STEEL };
    let atk = pl.attack_t / crate::adventure::ATTACK_TIME; // 1 -> 0
    if atk > 0.0 {
        // Sweep from -60° to +60° around facing.
        let sweep = (atk - 0.5) * 2.0; // +1 .. -1
        let ang = sweep * 1.15;
        let ca = ang.cos();
        let sa = ang.sin();
        let dir = v2(
            fv.x * ca - fv.y * sa,
            fv.x * sa + fv.y * ca,
        );
        let hilt = body_c.add(dir.mul(3.0)).add(v2(0.0, -2.0));
        let tip = body_c.add(dir.mul(15.0)).add(v2(0.0, -2.0));
        wline(t, cam, hilt, tip, 2.6, blade_col);
        wline(t, cam, body_c.add(dir.mul(2.0)), body_c.add(dir.mul(4.4)), 3.4, rgb(150, 110, 60));
        // Trail.
        let trail_ang = ang + 0.55;
        let tc = trail_ang.cos();
        let ts = trail_ang.sin();
        let td = v2(fv.x * tc - fv.y * ts, fv.x * ts + fv.y * tc);
        crate::art::shape::contour(
            t,
            cam.p(body_c.add(td.mul(13.0)).add(v2(0.0, -2.0))),
            cam.p(body_c.add(dir.mul(13.0)).add(v2(0.0, -2.0))),
            cam.l(3.0),
            rgb(255, 255, 255),
            atk * 0.5,
        );
    } else {
        // Resting blade behind the shoulder.
        let rest_dir = v2(fv.x * 0.4 - fv.y * 0.9, fv.y * 0.4 + fv.x * 0.9);
        let hilt = body_c.add(rest_dir.mul(-2.0)).add(v2(0.0, -4.0));
        let tip = body_c.add(rest_dir.mul(9.0)).add(v2(0.0, -10.0));
        wline(t, cam, hilt, tip, 2.2, blade_col);
    }

    // Small shield on the opposite arm.
    let sh_dir = v2(fv.y, -fv.x);
    let shield_at = body_c.add(sh_dir.mul(6.5));
    wcircle(t, cam, shield_at, 3.4, rgb(120, 84, 48), 8);
    wcircle(t, cam, shield_at, 2.2, rgb(190, 150, 90), 8);
}

use crate::adventure::Dir;

fn foes(g: &Game, t: &mut Canvas, cam: &Cam, time: f32) {
    for foe in &g.foes {
        if !foe.alive {
            continue;
        }
        let p = foe.p;
        let flash = foe.hurt_t > 0.0;
        let whiteout = |c: Rgb| if flash { c.lerp(rgb(255, 255, 255), 0.65) } else { c };
        let face = foe.dir.norm();
        let face_d = if face.len() < 0.1 { v2(0.0, 1.0) } else { face };

        match foe.kind {
            FoeKind::Crab => {
                shadow(t, cam, v2(p.x, p.y + 5.0), 6.5, 2.2, 1.0);
                let wig = (foe.anim * 2.0).sin();
                // Legs.
                for s in [-1.0f32, 1.0] {
                    for k in 0..2 {
                        let lx = p.x + s * (5.0 + k as f32 * 2.5);
                        let ly = p.y + 2.0 + k as f32 * 3.0 + wig * s * 1.2;
                        wline(t, cam, v2(p.x + s * 3.0, p.y), v2(lx, ly + 2.0), 1.6, whiteout(rgb(140, 60, 40)));
                    }
                }
                let shell = whiteout(rgb(204, 84, 58));
                let shell_hi = whiteout(rgb(232, 128, 88));
                wcircle(t, cam, v2(p.x, p.y), 6.4, shell, 10);
                wcircle(t, cam, v2(p.x - 2.0, p.y - 2.0), 3.4, shell_hi, 8);
                // Snout toward player-ish facing.
                let sn = p.add(face_d.mul(5.5));
                wcircle(t, cam, sn, 2.6, shell_hi, 8);
                wcircle(t, cam, sn.add(face_d.mul(1.2)), 1.2, rgb(40, 24, 20), 6);
                // Eye stalks.
                let e1 = p.add(face_d.mul(2.0)).add(face_d.perp().mul(2.6)).add(v2(0.0, -4.5));
                let e2 = p.add(face_d.mul(2.0)).sub(face_d.perp().mul(2.6)).add(v2(0.0, -4.5));
                wline(t, cam, p.add(v2(0.0, -3.0)), e1, 1.2, shell);
                wline(t, cam, p.add(v2(0.0, -3.0)), e2, 1.2, shell);
                wcircle(t, cam, e1, 1.1, rgb(250, 240, 220), 5);
                wcircle(t, cam, e2, 1.1, rgb(250, 240, 220), 5);
            }
            FoeKind::Grunt | FoeKind::Ogre => {
                let big = foe.kind == FoeKind::Ogre;
                let sc = if big { 1.5 } else { 1.0 };
                shadow(t, cam, v2(p.x, p.y + 6.0 * sc), 7.0 * sc, 2.6 * sc, 1.0);
                let hide = whiteout(if big { rgb(122, 84, 130) } else { rgb(104, 118, 74) });
                let hide_hi = whiteout(if big { rgb(158, 116, 166) } else { rgb(134, 148, 98) });
                let charge_lean = if vel_forward(foe, face_d) { 1.5 } else { 0.0 };
                let bc = v2(p.x, p.y - 1.0 * sc - charge_lean * 0.3);
                // Feet.
                wcircle(t, cam, v2(p.x - 3.5 * sc, p.y + 4.5), 2.2 * sc, whiteout(rgb(70, 52, 40)), 6);
                wcircle(t, cam, v2(p.x + 3.5 * sc, p.y + 4.5), 2.2 * sc, whiteout(rgb(70, 52, 40)), 6);
                // Body.
                wcircle(t, cam, bc, 6.6 * sc, hide, 10);
                wcircle(t, cam, v2(bc.x - 2.0 * sc, bc.y - 2.0 * sc), 3.4 * sc, hide_hi, 8);
                // Snout + tusks toward facing.
                let sn = bc.add(face_d.mul(5.0 * sc));
                wcircle(t, cam, sn, 3.2 * sc, hide_hi, 8);
                wcircle(t, cam, sn.add(face_d.mul(1.6 * sc)), 1.4 * sc, rgb(50, 34, 30), 6);
                let tusk1 = sn.add(face_d.mul(2.6 * sc)).add(face_d.perp().mul(2.0 * sc));
                let tusk2 = sn.add(face_d.mul(2.6 * sc)).sub(face_d.perp().mul(2.0 * sc));
                wcircle(t, cam, tusk1, 0.9 * sc, rgb(240, 236, 220), 5);
                wcircle(t, cam, tusk2, 0.9 * sc, rgb(240, 236, 220), 5);
                // Angry eyes.
                let ebase = bc.add(face_d.mul(2.4 * sc)).add(v2(0.0, -3.0 * sc));
                let eo = face_d.perp().mul(2.2 * sc);
                wcircle(t, cam, ebase.add(eo), 1.1 * sc, rgb(220, 50, 40), 5);
                wcircle(t, cam, ebase.sub(eo), 1.1 * sc, rgb(220, 50, 40), 5);
                // Horns.
                let h1 = bc.add(face_d.perp().mul(5.4 * sc)).add(v2(0.0, -4.0 * sc));
                let h2 = bc.sub(face_d.perp().mul(5.4 * sc)).add(v2(0.0, -4.0 * sc));
                wline(t, cam, h1, h1.add(v2(0.0, -3.0 * sc)).add(face_d.perp().mul(1.5 * sc)), 1.6 * sc, rgb(226, 220, 200));
                wline(t, cam, h2, h2.add(v2(0.0, -3.0 * sc)).sub(face_d.perp().mul(1.5 * sc)), 1.6 * sc, rgb(226, 220, 200));
                // Club.
                let club_h = bc.add(face_d.mul(7.5 * sc)).add(v2(0.0, 2.0));
                wline(t, cam, bc.add(face_d.mul(3.0 * sc)), club_h, 2.4 * sc, rgb(96, 68, 44));
                wcircle(t, cam, club_h, 2.6 * sc, rgb(116, 84, 54), 7);
                if big {
                    // Chains.
                    wline(t, cam, v2(bc.x - 6.0 * sc, bc.y + 3.0), v2(bc.x - 9.0 * sc, bc.y + 6.0), 1.4, rgb(150, 150, 160));
                    wline(t, cam, v2(bc.x + 6.0 * sc, bc.y + 3.0), v2(bc.x + 9.0 * sc, bc.y + 6.0), 1.4, rgb(150, 150, 160));
                }
            }
            FoeKind::Bat => {
                let flap = (foe.anim).sin();
                let hover = (foe.anim * 0.7).sin() * 2.0;
                let bp = v2(p.x, p.y - 6.0 + hover);
                shadow(t, cam, v2(p.x, p.y + 6.0), 4.5, 1.8, 0.7);
                let wing = whiteout(rgb(70, 58, 88));
                let wing_hi = whiteout(rgb(104, 88, 128));
                // Wings as triangles.
                for s in [-1.0f32, 1.0] {
                    let root = bp.add(v2(s * 2.5, 0.0));
                    let tip = root.add(v2(s * 7.0, -3.0 - flap * 4.0));
                    let mid = root.add(v2(s * 4.5, 1.5 - flap * 2.0));
                    poly(
                        t,
                        &[cam.p(root), cam.p(tip), cam.p(mid)],
                        if s > 0.0 { wing_hi } else { wing },
                    );
                }
                wcircle(t, cam, bp, 3.4, whiteout(rgb(56, 46, 72)), 8);
                wcircle(t, cam, bp.add(v2(0.0, -2.6)), 1.4, wing, 5); // ears hint
                // Glowing eyes.
                let eo = face_d.perp().mul(1.4);
                let ec = whiteout(rgb(255, 90, 70));
                wcircle(t, cam, bp.add(face_d.mul(1.6)).add(eo), 0.9, ec, 5);
                wcircle(t, cam, bp.add(face_d.mul(1.6)).sub(eo), 0.9, ec, 5);
            }
            FoeKind::Djinn => {
                // Crested serpent rising from the water.
                let rise = (time * 2.0 + p.x * 0.05).sin() * 1.5;
                let dp = v2(p.x, p.y - 4.0 + rise);
                // Ripple ring.
                let rr = 7.0 + (time * 2.4).rem_euclid(1.0) * 5.0;
                let ra = 0.5 - (time * 2.4).rem_euclid(1.0) * 0.4;
                ring(t, cam, v2(p.x, p.y + 2.0), rr, rgb(220, 240, 250), ra);
                let fin = whiteout(rgb(58, 150, 160));
                let fin_hi = whiteout(rgb(96, 196, 200));
                // Fan crest.
                for k in -2..=2i32 {
                    let a = k as f32 * 0.5;
                    let dvec = v2(a.sin(), -a.cos() * 0.6 - 0.55).norm();
                    wline(t, cam, dp, dp.add(dvec.mul(8.5)), 2.0, if k % 2 == 0 { fin } else { fin_hi });
                }
                wcircle(t, cam, dp, 5.0, fin, 9);
                wcircle(t, cam, dp.add(v2(-1.5, -1.5)), 2.6, fin_hi, 7);
                // Mouth + eyes aimed at the prince.
                let mo = dp.add(face_d.mul(4.0)).add(v2(0.0, 1.0));
                wcircle(t, cam, mo, 1.8, rgb(20, 40, 46), 6);
                let eo = face_d.perp().mul(2.2);
                wcircle(t, cam, dp.add(face_d.mul(1.5)).add(eo).add(v2(0.0, -2.0)), 1.0, rgb(250, 220, 90), 5);
                wcircle(t, cam, dp.add(face_d.mul(1.5)).sub(eo).add(v2(0.0, -2.0)), 1.0, rgb(250, 220, 90), 5);
            }
            FoeKind::Knight | FoeKind::BossKnight => {
                let boss = foe.kind == FoeKind::BossKnight;
                let sc = if boss { 1.35 } else { 1.0 };
                shadow(t, cam, v2(p.x, p.y + 6.5 * sc), 7.2 * sc, 2.6 * sc, 1.0);
                let armour = whiteout(if boss { rgb(96, 84, 110) } else { rgb(84, 88, 104) });
                let armour_hi = whiteout(if boss { rgb(140, 126, 160) } else { rgb(126, 132, 152) });
                let trim = whiteout(if boss { rgb(212, 168, 70) } else { rgb(120, 128, 142) });
                // Feet.
                wcircle(t, cam, v2(p.x - 3.6 * sc, p.y + 5.0), 2.3 * sc, armour, 6);
                wcircle(t, cam, v2(p.x + 3.6 * sc, p.y + 5.0), 2.3 * sc, armour, 6);
                let bc = v2(p.x, p.y - 1.0);
                wcircle(t, cam, bc, 6.4 * sc, armour, 10);
                wcircle(t, cam, v2(bc.x - 1.8 * sc, bc.y - 2.0 * sc), 3.0 * sc, armour_hi, 8);
                // Helmet with visor slit.
                let hd = bc.add(v2(0.0, -5.5 * sc));
                wcircle(t, cam, hd, 4.4 * sc, armour_hi, 9);
                wrect(t, cam, hd.x - 3.6 * sc, hd.y - 0.9 * sc, 7.2 * sc, 1.8 * sc, rgb(16, 14, 20));
                // Glowing eyes inside the slit, toward facing.
                let eo = face_d.perp().mul(1.9 * sc);
                let eye = hd.add(face_d.mul(1.2 * sc));
                let glow_eye = whiteout(if boss { rgb(255, 170, 60) } else { rgb(255, 70, 60) });
                wcircle(t, cam, eye.add(eo), 0.8 * sc, glow_eye, 5);
                wcircle(t, cam, eye.sub(eo), 0.8 * sc, glow_eye, 5);
                if boss {
                    // Crimson plume.
                    wline(t, cam, hd.add(v2(0.0, -4.2 * sc)), hd.add(v2(-face_d.x * 3.0, -8.5 * sc)), 2.2, whiteout(rgb(200, 50, 60)));
                }
                // Tower shield on the facing side — this is what blocks.
                let sh = bc.add(face_d.mul(6.8 * sc)).add(v2(0.0, 0.5));
                wrect(t, cam, sh.x - 3.0 * sc, sh.y - 5.0 * sc, 6.0 * sc, 10.0 * sc, armour);
                wrect(t, cam, sh.x - 2.2 * sc, sh.y - 4.2 * sc, 4.4 * sc, 8.4 * sc, armour_hi);
                wrect(t, cam, sh.x - 0.8 * sc, sh.y - 2.0 * sc, 1.6 * sc, 4.0 * sc, trim);
            }
            FoeKind::CrystalGolem => {
                // A hulking figure of living crystal, facets catching light.
                let sc = 1.45;
                shadow(t, cam, v2(p.x, p.y + 6.5 * sc), 8.0 * sc, 2.8 * sc, 1.0);
                let core = whiteout(rgb(120, 190, 230));
                let core_hi = whiteout(rgb(180, 230, 255));
                let core_dark = whiteout(rgb(70, 120, 170));
                // Feet.
                wcircle(t, cam, v2(p.x - 4.5 * sc, p.y + 5.0), 2.6 * sc, core_dark, 6);
                wcircle(t, cam, v2(p.x + 4.5 * sc, p.y + 5.0), 2.6 * sc, core_dark, 6);
                let bc = v2(p.x, p.y - 1.5 * sc);
                // Body: faceted slab.
                poly(
                    t,
                    &[
                        cam.p(bc.add(v2(-7.0 * sc, -4.0 * sc))),
                        cam.p(bc.add(v2(0.0, -8.0 * sc))),
                        cam.p(bc.add(v2(7.0 * sc, -4.0 * sc))),
                        cam.p(bc.add(v2(6.0 * sc, 5.0 * sc))),
                        cam.p(bc.add(v2(0.0, 7.0 * sc))),
                        cam.p(bc.add(v2(-6.0 * sc, 5.0 * sc))),
                    ],
                    core,
                );
                poly(
                    t,
                    &[
                        cam.p(bc.add(v2(-7.0 * sc, -4.0 * sc))),
                        cam.p(bc.add(v2(0.0, -8.0 * sc))),
                        cam.p(bc.add(v2(0.0, 0.0))),
                    ],
                    core_hi,
                );
                poly(
                    t,
                    &[
                        cam.p(bc.add(v2(0.0, 0.0))),
                        cam.p(bc.add(v2(6.0 * sc, 5.0 * sc))),
                        cam.p(bc.add(v2(0.0, 7.0 * sc))),
                    ],
                    core_dark,
                );
                // Crystal head.
                let hd = bc.add(v2(0.0, -9.0 * sc + (foe.anim * 1.4).sin() * 0.5));
                poly(
                    t,
                    &[
                        cam.p(hd.add(v2(-3.4 * sc, 1.0))),
                        cam.p(hd.add(v2(0.0, -4.6 * sc))),
                        cam.p(hd.add(v2(3.4 * sc, 1.0))),
                        cam.p(hd.add(v2(0.0, 2.6 * sc))),
                    ],
                    core,
                );
                // Burning eyes inside the visor line.
                let pulse = 0.65 + 0.35 * (time * 4.0).sin();
                let eo = face_d.perp().mul(1.7 * sc);
                let eg = rgb(255, 250, 200).lerp(rgb(160, 240, 255), pulse);
                wcircle(t, cam, hd.add(face_d.mul(1.0)).add(eo), 0.9 * sc, eg, 5);
                wcircle(t, cam, hd.add(face_d.mul(1.0)).sub(eo), 0.9 * sc, eg, 5);
                // Shard clusters on the shoulders.
                for s in [-1.0f32, 1.0] {
                    let sh = bc.add(v2(s * 7.5 * sc, -3.0 * sc));
                    wline(t, cam, sh, sh.add(v2(s * 1.6, -5.0)), 2.0, core_hi);
                    wline(t, cam, sh.add(v2(s * 2.0, 1.0)), sh.add(v2(s * 3.6, -2.6)), 1.6, core);
                }
            }
            FoeKind::Pharaoh => {
                // A mummified king trailing bandages, gold and dust.
                let sc = 1.4;
                shadow(t, cam, v2(p.x, p.y + 6.5 * sc), 7.4 * sc, 2.6 * sc, 0.9);
                let wrap = whiteout(rgb(214, 202, 172));
                let wrap_sh = whiteout(rgb(174, 162, 134));
                let gold = whiteout(rgb(226, 186, 90));
                // Shuffling feet.
                let step = (foe.anim).sin();
                wcircle(t, cam, v2(p.x - 3.4 * sc, p.y + 5.0 + step * 1.4), 2.2 * sc, wrap_sh, 6);
                wcircle(t, cam, v2(p.x + 3.4 * sc, p.y + 5.0 - step * 1.4), 2.2 * sc, wrap_sh, 6);
                // Body cone of bandages.
                let bc = v2(p.x, p.y - 1.0 * sc);
                poly(
                    t,
                    &[
                        cam.p(bc.add(v2(-5.6 * sc, -4.0 * sc))),
                        cam.p(bc.add(v2(5.6 * sc, -4.0 * sc))),
                        cam.p(bc.add(v2(4.0 * sc, 6.0 * sc))),
                        cam.p(bc.add(v2(-4.0 * sc, 6.0 * sc))),
                    ],
                    wrap,
                );
                // Bandage lines.
                for k in 0..3 {
                    let ly = bc.y - 2.0 * sc + k as f32 * 3.4;
                    wline(
                        t,
                        cam,
                        v2(bc.x - 4.6 * sc, ly),
                        v2(bc.x + (if k % 2 == 0 { 4.6 } else { 3.4 }) * sc, ly + 1.6),
                        1.2,
                        wrap_sh,
                    );
                }
                // Gold collar.
                wrect(t, cam, bc.x - 4.6 * sc, bc.y - 4.6 * sc, 9.2 * sc, 2.0, gold);
                // Mummy head with pharaoh stripes.
                let hd = bc.add(v2(face_d.x * 0.6, -8.0 * sc));
                wcircle(t, cam, hd, 4.2 * sc, wrap, 9);
                wcircle(t, cam, hd.add(v2(0.0, -2.6 * sc)), 3.4 * sc, gold, 8); // headdress crown
                wline(t, cam, hd.add(v2(-4.0 * sc, -1.0)), hd.add(v2(-5.2 * sc, 3.4 * sc)), 1.8, gold);
                wline(t, cam, hd.add(v2(4.0 * sc, -1.0)), hd.add(v2(5.2 * sc, 3.4 * sc)), 1.8, gold);
                // Glowing eyes in the dark sockets.
                let eo = face_d.perp().mul(1.7 * sc);
                let ec = whiteout(rgb(140, 255, 170));
                wcircle(t, cam, hd.add(face_d.mul(1.2 * sc)).add(eo), 0.95 * sc, ec, 5);
                wcircle(t, cam, hd.add(face_d.mul(1.2 * sc)).sub(eo), 0.95 * sc, ec, 5);
                // Crook & flail crossed on the chest.
                wline(t, cam, bc.add(v2(-4.0 * sc, 4.0 * sc)), bc.add(v2(3.2 * sc, -1.6 * sc)), 1.5, gold);
                wline(t, cam, bc.add(v2(4.0 * sc, 4.0 * sc)), bc.add(v2(-3.2 * sc, -1.6 * sc)), 1.5, gold);
            }
            FoeKind::Ganar => {
                let float = (time * 2.0).sin() * 1.6;
                let gp = v2(p.x, p.y - 5.0 + float);
                shadow(t, cam, v2(p.x, p.y + 7.0), 7.5, 2.6, 0.9);
                // Cape flaring behind.
                let cape = whiteout(rgb(58, 26, 66));
                let sway = (time * 3.1).sin() * 2.0;
                poly(
                    t,
                    &[
                        cam.p(gp.add(v2(-5.0, -4.0))),
                        cam.p(gp.add(v2(5.0, -4.0))),
                        cam.p(gp.add(v2(8.0 + sway, 8.0))),
                        cam.p(gp.add(v2(-8.0 - sway, 8.0))),
                    ],
                    cape,
                );
                // Robe.
                poly(
                    t,
                    &[
                        cam.p(gp.add(v2(-5.5, -4.0))),
                        cam.p(gp.add(v2(5.5, -4.0))),
                        cam.p(gp.add(v2(3.5, 8.5))),
                        cam.p(gp.add(v2(-3.5, 8.5))),
                    ],
                    whiteout(rgb(40, 20, 48)),
                );
                // Gold trim lines.
                wline(t, cam, gp.add(v2(-4.6, -2.0)), gp.add(v2(-3.0, 7.5)), 1.2, whiteout(rgb(212, 168, 70)));
                wline(t, cam, gp.add(v2(4.6, -2.0)), gp.add(v2(3.0, 7.5)), 1.2, whiteout(rgb(212, 168, 70)));
                // Head: green skin, glowing eyes, twin-curved hat.
                let hd = gp.add(v2(0.0, -7.5));
                wcircle(t, cam, hd, 4.2, whiteout(rgb(96, 140, 84)), 9);
                // Hat: two curved horns of office.
                wline(t, cam, hd.add(v2(-3.4, -2.4)), hd.add(v2(-6.5, -10.0)), 2.0, whiteout(rgb(30, 18, 36)));
                wline(t, cam, hd.add(v2(3.4, -2.4)), hd.add(v2(6.5, -10.0)), 2.0, whiteout(rgb(30, 18, 36)));
                wcircle(t, cam, hd.add(v2(-6.5, -10.0)), 1.4, whiteout(rgb(212, 168, 70)), 5);
                wcircle(t, cam, hd.add(v2(6.5, -10.0)), 1.4, whiteout(rgb(212, 168, 70)), 5);
                // Eyes burn.
                let pulse = 0.7 + 0.3 * (time * 5.0).sin();
                let eo = face_d.perp().mul(1.8);
                let eg = rgb(255, 200, 60).lerp(rgb(255, 240, 140), pulse);
                wcircle(t, cam, hd.add(face_d.mul(1.4)).add(eo), 1.1, eg, 5);
                wcircle(t, cam, hd.add(face_d.mul(1.4)).sub(eo), 1.1, eg, 5);
                // Staff with orb.
                let st = gp.add(face_d.perp().mul(7.0));
                wline(t, cam, st.add(v2(0.0, -8.0)), st.add(v2(0.0, 8.0)), 1.8, rgb(70, 44, 30));
                let orb_c = st.add(v2(0.0, -10.0));
                let orb_r = 2.6 + (time * 4.0).sin().abs() * 0.7;
                wcircle(t, cam, orb_c, orb_r, rgb(150, 60, 220), 8);
                wcircle(t, cam, orb_c, orb_r * 0.5, rgb(230, 180, 255), 6);
            }
        }
    }
}

fn vel_forward(foe: &crate::adventure::Foe, face: V2) -> bool {
    foe.t > 0.0 && face.len() > 0.1
}

fn ring(t: &mut Canvas, cam: &Cam, c: V2, r: f32, col: Rgb, alpha: f32) {
    let cc = cam.p(c);
    let rr = cam.l(r);
    let n = 14;
    let mut prev = v2(cc.x + rr, cc.y);
    for i in 1..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let pt = v2(cc.x + a.cos() * rr, cc.y + a.sin() * rr);
        crate::art::shape::contour(t, prev, pt, cam.l(1.2), col, alpha);
        prev = pt;
    }
}

fn npcs(g: &Game, t: &mut Canvas, cam: &Cam, time: f32) {
    if let Some((ch, nx, ny)) = g.world().npc {
        let p = v2(nx, ny);
        let bob = (time * 2.4).sin() * 0.6;
        shadow(t, cam, v2(p.x, p.y + 7.0), 6.5, 2.4, 1.0);
        match ch {
            'V' => {
                // Vieil Erfan : brown robe, long white beard, staff.
                let robe = rgb(122, 92, 62);
                let robe_hi = rgb(148, 114, 80);
                poly(
                    t,
                    &[
                        cam.p(v2(p.x - 5.5, p.y - 4.0 + bob)),
                        cam.p(v2(p.x + 5.5, p.y - 4.0 + bob)),
                        cam.p(v2(p.x + 4.0, p.y + 7.0)),
                        cam.p(v2(p.x - 4.0, p.y + 7.0)),
                    ],
                    robe,
                );
                poly(
                    t,
                    &[
                        cam.p(v2(p.x - 5.5, p.y - 4.0 + bob)),
                        cam.p(v2(p.x - 1.0, p.y - 4.0 + bob)),
                        cam.p(v2(p.x - 1.0, p.y + 6.0)),
                        cam.p(v2(p.x - 4.0, p.y + 6.5)),
                    ],
                    robe_hi,
                );
                let hd = v2(p.x, p.y - 6.5 + bob);
                wcircle(t, cam, hd, 3.6, SKIN, 8);
                // Beard.
                poly(
                    t,
                    &[
                        cam.p(hd.add(v2(-2.8, 1.0))),
                        cam.p(hd.add(v2(2.8, 1.0))),
                        cam.p(hd.add(v2(0.0, 8.0))),
                    ],
                    rgb(232, 228, 218),
                );
                // Hood hint.
                wcircle(t, cam, hd.add(v2(0.0, -2.2)), 3.2, robe_hi, 8);
                // Staff.
                wline(t, cam, v2(p.x + 6.5, p.y - 9.0), v2(p.x + 6.5, p.y + 7.0), 1.6, rgb(96, 68, 44));
                wcircle(t, cam, v2(p.x + 6.5, p.y - 9.5), 1.8, rgb(226, 186, 90), 6);
            }
            'Y' => {
                // Princesse Zahra.
                let freed = g.ganar_dead;
                let dress = if freed { rgb(238, 214, 226) } else { rgb(206, 182, 208) };
                poly(
                    t,
                    &[
                        cam.p(v2(p.x - 4.5, p.y - 4.0 + bob)),
                        cam.p(v2(p.x + 4.5, p.y - 4.0 + bob)),
                        cam.p(v2(p.x + 6.5, p.y + 7.0)),
                        cam.p(v2(p.x - 6.5, p.y + 7.0)),
                    ],
                    dress,
                );
                wrect(t, cam, p.x - 4.6, p.y + 0.5, 9.2, 1.6, rgb(198, 148, 170)); // ceinture
                let hd = v2(p.x, p.y - 6.5 + bob);
                wcircle(t, cam, hd, 3.8, SKIN, 8);
                // Long dark hair.
                wcircle(t, cam, hd.add(v2(0.0, -1.4)), 3.9, rgb(58, 40, 34), 8);
                wcircle(t, cam, hd.add(v2(0.0, 1.2)), 3.2, SKIN, 8);
                wrect(t, cam, hd.x - 3.4, hd.y - 3.4, 6.8, 1.6, rgb(226, 186, 90)); // diadème
                if freed {
                    // Sparkles of freedom.
                    for k in 0..3 {
                        let a = time * 2.0 + k as f32 * 2.1;
                        let sp = p.add(v2(a.cos() * 9.0, a.sin() * 5.0 - 6.0));
                        wcircle(t, cam, sp, 1.0, rgb(255, 240, 180), 5);
                    }
                }
            }
            'F' => {
                // La Fée du lac : a tiny glowing fairy over the water.
                let hover = (time * 2.6).sin() * 3.0;
                let fp = v2(p.x, p.y - 8.0 + hover);
                // Wings.
                let flap = (time * 14.0).sin();
                for s in [-1.0f32, 1.0] {
                    let root = fp.add(v2(s * 2.0, 0.0));
                    let tip = root.add(v2(s * 6.5, -2.5 - flap * 2.5));
                    let mid = root.add(v2(s * 4.0, 2.0 + flap * 1.5));
                    poly(
                        t,
                        &[cam.p(root), cam.p(tip), cam.p(mid)],
                        rgb(200, 240, 250),
                    );
                }
                // Body + head.
                wcircle(t, cam, fp.add(v2(0.0, 2.4)), 2.0, rgb(120, 200, 210), 7);
                wcircle(t, cam, fp, 2.8, SKIN, 8);
                wcircle(t, cam, fp.add(v2(0.0, -2.6)), 2.4, rgb(220, 160, 90), 7); // hair
                // Aura sparkles.
                for k in 0..4 {
                    let a = time * 1.8 + k as f32 * 1.57;
                    let sp = fp.add(v2(a.cos() * 8.0, a.sin() * 5.0));
                    wcircle(t, cam, sp, 0.9 + (a.sin() * 0.5).abs(), rgb(190, 255, 230), 5);
                }
                ring(t, cam, fp, 9.0 + (time * 2.0).sin() * 2.0, rgb(170, 245, 220), 0.35);
            }
            _ => {}
        }
    }
}

fn pickups(g: &Game, t: &mut Canvas, cam: &Cam, time: f32) {
    for pk in &g.pickups[g.cur] {
        if pk.taken {
            continue;
        }
        let bob = (time * 3.0 + pk.bob).sin() * 1.5;
        let p = v2(pk.p.x, pk.p.y + bob);
        shadow(t, cam, v2(pk.p.x, pk.p.y + 6.0), 4.0, 1.5, 0.8);
        match pk.kind {
            Pickup::Heart => {
                let c = rgb(220, 52, 62);
                let hi = rgb(244, 120, 120);
                wcircle(t, cam, p.add(v2(-1.8, -1.0)), 2.4, c, 7);
                wcircle(t, cam, p.add(v2(1.8, -1.0)), 2.4, c, 7);
                poly(
                    t,
                    &[
                        cam.p(v2(p.x - 4.0, p.y - 0.2)),
                        cam.p(v2(p.x + 4.0, p.y - 0.2)),
                        cam.p(v2(p.x, p.y + 4.6)),
                    ],
                    c,
                );
                wcircle(t, cam, p.add(v2(-1.2, -1.6)), 0.9, hi, 5);
            }
            Pickup::HeartContainer => {
                // A big heart in a golden vessel, pulsing.
                let pulse = 1.0 + 0.12 * (time * 5.0).sin();
                let c = rgb(220, 52, 62);
                let hi = rgb(250, 130, 130);
                ring(t, cam, p, 8.0 * pulse, rgb(255, 190, 120), 0.4);
                wcircle(t, cam, p.add(v2(-2.6 * pulse, -1.4)), 3.4 * pulse, c, 8);
                wcircle(t, cam, p.add(v2(2.6 * pulse, -1.4)), 3.4 * pulse, c, 8);
                poly(
                    t,
                    &[
                        cam.p(v2(p.x - 5.6 * pulse, p.y - 0.2)),
                        cam.p(v2(p.x + 5.6 * pulse, p.y - 0.2)),
                        cam.p(v2(p.x, p.y + 6.4 * pulse)),
                    ],
                    c,
                );
                wcircle(t, cam, p.add(v2(-1.6, -2.2)), 1.3, hi, 5);
                wcircle(t, cam, p.add(v2(0.0, 0.0)), 1.0, rgb(255, 240, 220), 5);
            }
            Pickup::Gem => {
                let c = rgb(70, 190, 200);
                let hi = rgb(150, 236, 240);
                poly(
                    t,
                    &[
                        cam.p(v2(p.x, p.y - 4.2)),
                        cam.p(v2(p.x + 3.6, p.y - 1.0)),
                        cam.p(v2(p.x, p.y + 4.2)),
                        cam.p(v2(p.x - 3.6, p.y - 1.0)),
                    ],
                    c,
                );
                poly(
                    t,
                    &[
                        cam.p(v2(p.x, p.y - 4.2)),
                        cam.p(v2(p.x + 3.6, p.y - 1.0)),
                        cam.p(v2(p.x, p.y - 0.5)),
                    ],
                    hi,
                );
            }
            Pickup::Key => {
                let c = rgb(226, 186, 90);
                let hi = rgb(248, 222, 140);
                wcircle(t, cam, p.add(v2(-2.5, 0.0)), 2.6, c, 8);
                wcircle(t, cam, p.add(v2(-2.5, 0.0)), 1.1, rgb(40, 32, 24), 5);
                wline(t, cam, p.add(v2(-0.4, 0.0)), p.add(v2(5.5, 0.0)), 1.6, c);
                wrect(t, cam, p.x + 3.4, p.y + 0.8, 1.4, 2.4, c);
                wrect(t, cam, p.x + 5.2, p.y + 0.8, 1.4, 2.4, c);
                wcircle(t, cam, p.add(v2(-3.2, -1.0)), 0.7, hi, 4);
            }
            Pickup::SilverMirror => {
                // A proud hand mirror.
                let c = rgb(196, 208, 224);
                wcircle(t, cam, p.add(v2(0.0, -1.0)), 4.2, rgb(148, 152, 168), 10);
                wcircle(t, cam, p.add(v2(0.0, -1.0)), 3.2, c, 9);
                wline(t, cam, p.add(v2(0.0, 2.6)), p.add(v2(0.0, 6.0)), 1.8, rgb(148, 152, 168));
                wline(t, cam, p.add(v2(-2.0, 6.0)), p.add(v2(2.0, 6.0)), 1.4, rgb(148, 152, 168));
                // Shine.
                let sp = (time * 2.4).sin();
                if sp > 0.0 {
                    wline(t, cam, p.add(v2(-1.5, -2.5)), p.add(v2(0.5, -0.5)), 1.0, rgb(255, 255, 255));
                }
                ring(t, cam, p, 7.0 + (time * 2.0).sin() * 1.5, rgb(190, 215, 255), 0.35);
            }
            Pickup::SilverSword => {
                let c = rgb(240, 246, 255);
                wline(t, cam, p.add(v2(-5.0, 4.0)), p.add(v2(4.0, -5.0)), 2.4, c);
                wline(t, cam, p.add(v2(-4.0, 5.6)), p.add(v2(-1.6, 3.2)), 1.6, rgb(180, 150, 90));
                wline(t, cam, p.add(v2(-6.4, 2.6)), p.add(v2(-3.6, 5.4)), 1.4, rgb(150, 120, 70));
                ring(t, cam, p, 8.0 + (time * 2.4).sin() * 1.2, rgb(220, 235, 255), 0.4);
            }
        }
    }
}

fn shots(g: &Game, t: &mut Canvas, cam: &Cam, time: f32) {
    for s in &g.shots {
        if s.fire {
            let r = 3.2 + (time * 9.0).sin().abs() * 0.8;
            wcircle(t, cam, s.p, r, rgb(255, 140, 50), 8);
            wcircle(t, cam, s.p, r * 0.55, rgb(255, 220, 130), 6);
        } else {
            // Spinning pebble.
            let a = time * 12.0;
            wcircle(t, cam, s.p, 2.4, rgb(140, 130, 120), 7);
            wline(
                t,
                cam,
                s.p.add(v2(a.cos() * 2.0, a.sin() * 2.0)),
                s.p.add(v2(-a.cos() * 2.0, -a.sin() * 2.0)),
                0.9,
                rgb(96, 88, 80),
            );
        }
    }
}

// ---------------------------------------------------------------- entry

/// Draw the whole scene into the canvas.
pub fn draw(g: &Game, cv: &mut Canvas, layer: &mut Layer, light: &mut LightField, ss: f32) {
    use crate::gfx::canvas::Blend;
    let pal_ = pal(g.theme());
    let cam_pos = g.cam_draw();
    let cam = Cam {
        ox: cam_pos.x,
        oy: cam_pos.y,
        s: ss,
    };

    // Sky/ground base.
    cv.clear(match g.theme() {
        ThemeName::Valley => rgb(150, 128, 84),
        ThemeName::Shadow => rgb(56, 48, 74),
        ThemeName::Dungeon => rgb(58, 50, 44),
        ThemeName::Palace => rgb(160, 138, 100),
        ThemeName::Forest => rgb(56, 82, 52),
        ThemeName::Crystal => rgb(48, 54, 78),
        ThemeName::Desert => rgb(172, 142, 90),
        ThemeName::Necro => rgb(104, 96, 80),
        ThemeName::Swamp => rgb(58, 72, 48),
        ThemeName::Pass => rgb(96, 86, 72),
        ThemeName::Mines => rgb(62, 48, 38),
        ThemeName::Oasis => rgb(88, 130, 74),
        ThemeName::Sanctuary => rgb(110, 116, 146),
        ThemeName::Tower => rgb(52, 46, 64),
        ThemeName::Throne => rgb(30, 20, 36),
    });

    // Visible tile range.
    let tx0 = (cam_pos.x / TILE).floor() as i32 - 1;
    let ty0 = (cam_pos.y / TILE).floor() as i32 - 1;
    let tx1 = ((cam_pos.x + g.view_w) / TILE).ceil() as i32 + 1;
    let ty1 = ((cam_pos.y + g.view_h) / TILE).ceil() as i32 + 1;

    // Pass 1: terrain.
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            if tx < 0 || ty < 0 || tx >= g.world().tw || ty >= g.world().th {
                // Outside: solid rock face so the map edge reads as a cliff.
                wrect(cv, &cam, tx as f32 * TILE, ty as f32 * TILE, TILE, TILE, pal_.wall_dark);
                continue;
            }
            draw_tile(g, cv, &cam, tx, ty, g.time);
        }
    }

    // Pass 2: things, painter's order by feet position.
    cv.blend = Blend::Alpha;
    let mut order: Vec<(f32, u8)> = Vec::new();
    order.push((g.player.p.y, 0)); // prince last among movers? no—sort below
    for foe in &g.foes {
        if foe.alive {
            order.push((foe.p.y, 1));
        }
    }
    if g.world().npc.is_some() {
        order.push((g.world().npc.unwrap().2, 2));
    }
    order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Pickups first (flat on the ground).
    pickups(g, cv, &cam, g.time);

    let mut npc_drawn = false;
    for (_, kind) in order.iter() {
        match *kind {
            0 => prince(g, cv, &cam, g.time),
            1 => {} // drawn together below
            _ => {
                npcs(g, cv, &cam, g.time);
                npc_drawn = true;
            }
        }
    }
    // Foes are drawn per-entity to respect sorting loosely: simple approach —
    // redraw all foes after the prince unless he stands further south.
    let prince_y = g.player.p.y;
    let any_foe_north = g.foes.iter().any(|f| f.alive && f.p.y <= prince_y);
    let any_foe_south = g.foes.iter().any(|f| f.alive && f.p.y > prince_y);
    if any_foe_north {
        foes(g, cv, &cam, g.time);
        prince(g, cv, &cam, g.time);
    } else {
        prince(g, cv, &cam, g.time);
        if any_foe_south {
            foes(g, cv, &cam, g.time);
        }
    }
    if !npc_drawn && g.world().npc.is_some() {
        npcs(g, cv, &cam, g.time);
    }

    shots(g, cv, &cam, g.time);

    // Particles above everything.
    g.fx.draw_matter(cv, &cam);

    // Lighting pass.
    let amb = g.ambient();
    light.begin(cv.w, cv.h, amb);
    // Torches.
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            if g.world().tile(tx, ty) == Tile::Brazier {
                let c = cam.p(v2(
                    (tx as f32 + 0.5) * TILE,
                    (ty as f32 + 0.35) * TILE,
                ));
                let flick = 0.85 + 0.15 * noise1(g.time * 8.0, tx * 31 + ty * 17);
                light.add(c, 62.0 * ss, pal_.torch_glow, 0.85 * flick);
            }
            if g.world().tile(tx, ty) == Tile::Portal {
                let c = cam.p(v2((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE));
                light.add(c, 46.0 * ss, rgb(150, 120, 230), 0.5 + 0.2 * (g.time * 2.0).sin());
            }
        }
    }
    // Fireball glow.
    for s in &g.shots {
        if s.fire {
            let c = cam.p(s.p);
            light.add(c, 30.0 * ss, rgb(255, 150, 60), 0.8);
        }
    }
    // Ganar's orb.
    for f in &g.foes {
        if f.kind == FoeKind::Ganar && f.alive {
            let c = cam.p(f.p.add(v2(7.0, -15.0)));
            light.add(c, 40.0 * ss, rgb(170, 90, 255), 0.8);
        }
    }
    // The princess glows softly once freed.
    if let Some(('Y', nx, ny)) = g.world().npc {
        if g.ganar_dead {
            let c = cam.p(v2(nx, ny));
            light.add(c, 50.0 * ss, rgb(255, 220, 170), 0.5);
        }
    }
    // The fairy sheds her own gentle light.
    if let Some(('F', nx, ny)) = g.world().npc {
        let c = cam.p(v2(nx, ny - 8.0));
        light.add(
            c,
            44.0 * ss,
            rgb(150, 240, 210),
            0.55 + 0.2 * (g.time * 3.0).sin(),
        );
    }
    light.apply(cv);

    // Post: a whisper of vignette — no dithering, the look stays smooth and
    // fine-grained rather than retro-pixelated.
    let vig_tint = match g.theme() {
        ThemeName::Valley => rgb(40, 30, 20),
        ThemeName::Palace => rgb(40, 30, 24),
        _ => rgb(14, 8, 22),
    };
    cv.vignette(0.16, vig_tint);

    // Emissive overlay (flames etc.) drawn additively on top.
    cv.blend = Blend::Add;
    g.fx.draw_light(cv, &cam);
    cv.blend = Blend::Alpha;

    let _ = layer;
}
