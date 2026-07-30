//! Environment art: brickwork, floor slabs, columns, portcullises, spike traps,
//! chompers, pressure plates, torches, mirrors, windows and the exit door.
//!
//! Everything is procedural and seeded from the tile coordinate, so a given wall
//! always has the same bricks, cracks and stains, while no two walls look alike.
//!
//! Drawing is split into ordered passes rather than a per-cell loop, because a
//! floor slab belongs visually to the top of the cell *below* it — exactly as in
//! the original art, where one row's floor is the next row's ceiling trim.

use crate::art::theme::Theme;
use crate::gfx::canvas::{Blend, Cam, Canvas};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::target::{
    fill_capsule, fill_circle, fill_ellipse, fill_poly, fill_poly_shaded, fill_rect,
    fill_rect_grad, radial_glow,
};
use crate::util::{clampf, hashf, lerp, noise1, v2, Rect, V2};
use crate::world::dynamics::{Dynamics, F_BLOODY, F_LATCHED};
use crate::world::level::Level;
use crate::world::tile::*;

/// Convert a world-space rectangle and fill it.
fn wrect(cv: &mut Canvas, cam: &Cam, r: Rect, c: Rgb, a: f32) {
    fill_rect(cv, cam.x(r.x0), cam.y(r.y0), cam.x(r.x1), cam.y(r.y1), c, a);
}

fn wrect_grad(cv: &mut Canvas, cam: &Cam, r: Rect, top: Rgb, bot: Rgb, a: f32) {
    fill_rect_grad(
        cv,
        cam.x(r.x0),
        cam.y(r.y0),
        cam.x(r.x1),
        cam.y(r.y1),
        top,
        bot,
        a,
    );
}

fn cell_rect(tx: i32, ty: i32) -> Rect {
    Rect::from_size(tx as f32 * TILE_W, ty as f32 * TILE_H, TILE_W, TILE_H)
}

// ---------------------------------------------------------------- brickwork

#[allow(clippy::too_many_arguments)]
fn brickwork(
    cv: &mut Canvas,
    cam: &Cam,
    r: Rect,
    base: Rgb,
    dark: Rgb,
    mortar: Rgb,
    seed: i32,
    rows: i32,
    bw: f32,
    detail: f32,
) {
    wrect(cv, cam, r, mortar, 1.0);
    let bh = r.h() / rows as f32;
    let gap = 0.55;
    for row in 0..rows {
        let y0 = r.y0 + row as f32 * bh;
        let stagger = if (row + seed) % 2 == 0 { 0.0 } else { bw * 0.5 };
        let mut x = r.x0 - stagger;
        let mut k = 0;
        while x < r.x1 {
            let bx0 = (x + gap).max(r.x0);
            let bx1 = (x + bw - gap).min(r.x1);
            x += bw;
            k += 1;
            if bx1 <= bx0 {
                continue;
            }
            let h = hashf(seed * 131 + k * 17, row * 29 + seed, 3);
            let h2 = hashf(seed * 71 + k * 5, row * 13 + 7, 11);
            let col = dark.lerp(base, 0.35 + h * 0.75).scale(0.94 + h2 * 0.13);
            let brick = Rect::new(bx0, y0 + gap, bx1, y0 + bh - gap);
            wrect_grad(cv, cam, brick, col.scale(1.07), col.scale(0.88), 1.0);
            // Bevel: a lit top edge and a shaded bottom edge give the stone
            // some relief once the light pass multiplies over it.
            wrect(
                cv,
                cam,
                Rect::new(brick.x0, brick.y0, brick.x1, brick.y0 + 0.7),
                col.scale(1.3),
                0.8 * detail,
            );
            wrect(
                cv,
                cam,
                Rect::new(brick.x0, brick.y1 - 0.7, brick.x1, brick.y1),
                col.scale(0.6),
                0.8 * detail,
            );
            // Occasional crack or stain.
            if detail > 0.5 && h2 > 0.86 {
                let cxp = lerp(brick.x0 + 2.0, brick.x1 - 2.0, h);
                fill_capsule(
                    cv,
                    v2(cam.x(cxp), cam.y(brick.y0 + 0.8)),
                    v2(cam.x(cxp + (h - 0.5) * 3.0), cam.y(brick.y1 - 0.8)),
                    cam.l(0.28),
                    cam.l(0.18),
                    mortar,
                    0.75,
                );
            }
            if detail > 0.5 && h > 0.93 {
                fill_ellipse(
                    cv,
                    v2(cam.x(lerp(brick.x0, brick.x1, h2)), cam.y(brick.y1 - 1.0)),
                    cam.l(2.6),
                    cam.l(1.1),
                    col.scale(0.72),
                    0.5,
                );
            }
        }
    }
}

// ---------------------------------------------------------------- passes

/// Draw the whole visible environment. `view` is the world rectangle on screen.
pub fn draw_environment(
    cv: &mut Canvas,
    cam: &Cam,
    lv: &Level,
    dy: &Dynamics,
    view: Rect,
    time: f32,
) {
    let th = &lv.theme;
    let tx0 = (view.x0 / TILE_W).floor() as i32 - 1;
    let tx1 = (view.x1 / TILE_W).ceil() as i32 + 1;
    let ty0 = (view.y0 / TILE_H).floor() as i32 - 1;
    let ty1 = (view.y1 / TILE_H).ceil() as i32 + 1;

    cv.blend = Blend::Alpha;

    // ---- pass 1: the distant back wall behind every open cell -----------
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let t = lv.tile(tx, ty);
            if t == Tile::Wall || t == Tile::Pillar {
                continue;
            }
            draw_back(cv, cam, tx, ty, th);
        }
    }

    // ---- pass 2: masonry ------------------------------------------------
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            match lv.tile(tx, ty) {
                Tile::Wall => draw_wall(cv, cam, lv, tx, ty, th),
                Tile::Pillar => draw_pillar(cv, cam, tx, ty, th),
                _ => {}
            }
        }
    }

    // ---- pass 3: floor slabs -------------------------------------------
    // A cell gets a slab if you can stand in it, whether that is because it is
    // a floor tile or because there is masonry directly beneath it. Drawing both
    // cases the same way is what keeps "walk along a floor" and "walk along the
    // top of a wall" at exactly the same height.
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let t = lv.tile(tx, ty);
            if t.solid() {
                continue;
            }
            if !(t.walkable() || lv.tile(tx, ty + 1).solid()) {
                continue;
            }
            let wobble = if t == Tile::Loose {
                dy.get(tx, ty).a
            } else {
                0.0
            };
            draw_slab(cv, cam, lv, tx, ty, th, wobble, t);
        }
    }

    // ---- pass 4: props --------------------------------------------------
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let c = lv.cell(tx, ty);
            match c.tile {
                Tile::Torch => draw_torch(cv, cam, tx, ty, th),
                Tile::Gate => draw_gate(cv, cam, tx, ty, th, dy.a(tx, ty)),
                Tile::Spikes => draw_spikes(cv, cam, tx, ty, th, dy.a(tx, ty), dy.get(tx, ty).flag),
                Tile::Chomper => draw_chomper(cv, cam, tx, ty, th, dy.a(tx, ty)),
                Tile::PlateRaise | Tile::PlateDrop => {
                    draw_plate(cv, cam, tx, ty, th, dy.a(tx, ty), c.tile == Tile::PlateRaise)
                }
                Tile::Mirror => draw_mirror(cv, cam, tx, ty, th, time),
                Tile::Window => draw_window(cv, cam, tx, ty, th),
                Tile::Arch => draw_arch(cv, cam, tx, ty, th),
                Tile::Bones => draw_bones_pile(cv, cam, tx, ty, th),
                Tile::Exit => draw_exit(cv, cam, tx, ty, th, dy.a(tx, ty)),
                _ => {}
            }
        }
    }
}

/// The distant wall seen through open space.
///
/// Fine brickwork, plus the recessed arched niches and the cornice band that ran
/// along the top of every row in the original background art. Without them a
/// room of open space reads as a flat sheet of texture.
fn draw_back(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme) {
    let r = cell_rect(tx, ty);
    brickwork(
        cv,
        cam,
        r,
        th.back,
        th.back_dk,
        th.back_dk.scale(0.5),
        (tx * 7 + ty * 3) | 1,
        4,
        15.0,
        0.6,
    );

    // Recessed niche, on roughly a third of the cells, aligned in pairs so the
    // wall has rhythm rather than noise.
    if hashf(tx.div_euclid(2), ty, 41) > 0.62 {
        let n = Rect::new(r.x0 + 7.0, r.y0 + 11.0, r.x1 - 7.0, r.y1 - 9.0);
        // Body of the recess.
        wrect_grad(
            cv,
            cam,
            Rect::new(n.x0, n.y0, n.x1, n.y1),
            th.back_dk.scale(0.82),
            th.back.scale(0.72),
            1.0,
        );
        // Arched head.
        fill_ellipse(
            cv,
            v2(cam.x(n.cx()), cam.y(n.y0)),
            cam.l(n.w() * 0.5),
            cam.l(4.5),
            th.back_dk.scale(0.82),
            1.0,
        );
        // Bevel: lit on the upper-left, shaded on the lower-right.
        wrect(
            cv,
            cam,
            Rect::new(n.x0 - 1.1, n.y0 - 1.0, n.x0, n.y1),
            th.back.scale(1.35),
            0.55,
        );
        wrect(
            cv,
            cam,
            Rect::new(n.x1, n.y0 - 1.0, n.x1 + 1.1, n.y1),
            th.back_dk.scale(0.5),
            0.6,
        );
        wrect(
            cv,
            cam,
            Rect::new(n.x0 - 1.0, n.y1, n.x1 + 1.0, n.y1 + 1.1),
            th.back.scale(1.2),
            0.4,
        );
    }

    // Cornice along the top of each room.
    if ty.rem_euclid(ROOM_TH) == 0 {
        wrect_grad(
            cv,
            cam,
            Rect::new(r.x0, r.y0, r.x1, r.y0 + 3.2),
            th.back.scale(1.5),
            th.back_dk,
            1.0,
        );
        wrect(
            cv,
            cam,
            Rect::new(r.x0, r.y0 + 3.2, r.x1, r.y0 + 4.0),
            th.back_dk.scale(0.5),
            0.7,
        );
        // Dentils.
        for i in 0..4 {
            let dx = r.x0 + (i as f32 + 0.5) * (TILE_W / 4.0);
            wrect(
                cv,
                cam,
                Rect::new(dx - 1.6, r.y0 + 4.0, dx + 1.6, r.y0 + 5.6),
                th.back.scale(1.15),
                0.7,
            );
        }
    }

    // Deepen the upper part of the cell so rooms recede into gloom.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0, r.y0, r.x1, r.y0 + TILE_H * 0.6),
        th.back_dk.scale(0.55),
        th.back_dk.scale(1.05),
        0.26,
    );
}

fn draw_wall(cv: &mut Canvas, cam: &Cam, lv: &Level, tx: i32, ty: i32, th: &Theme) {
    let r = cell_rect(tx, ty);
    brickwork(
        cv,
        cam,
        r,
        th.brick,
        th.brick_dk,
        th.mortar,
        (tx * 13 + ty * 5) | 1,
        3,
        18.0,
        1.0,
    );
    // Ambient occlusion where the wall meets its neighbours.
    if !lv.tile(tx - 1, ty).solid() {
        wrect_grad(
            cv,
            cam,
            Rect::new(r.x0, r.y0, r.x0 + 3.0, r.y1),
            th.mortar,
            th.brick_dk,
            0.5,
        );
    }
    if !lv.tile(tx + 1, ty).solid() {
        wrect_grad(
            cv,
            cam,
            Rect::new(r.x1 - 3.0, r.y0, r.x1, r.y1),
            th.brick_dk,
            th.mortar,
            0.5,
        );
    }
}

fn draw_pillar(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme) {
    let r = cell_rect(tx, ty);
    let cx = r.cx();
    let hw = TILE_W * 0.30;
    // Shaft with flutes.
    wrect_grad(
        cv,
        cam,
        Rect::new(cx - hw, r.y0 + 3.0, cx + hw, r.y1 - 2.0),
        th.brick,
        th.brick_dk,
        1.0,
    );
    for i in 0..3 {
        let fx = cx - hw + hw * 0.5 * (i as f32 + 0.5);
        wrect(
            cv,
            cam,
            Rect::new(fx, r.y0 + 4.0, fx + 1.2, r.y1 - 3.0),
            th.brick.scale(1.2),
            0.45,
        );
    }
    wrect(
        cv,
        cam,
        Rect::new(cx + hw * 0.45, r.y0 + 4.0, cx + hw, r.y1 - 3.0),
        th.brick_dk.scale(0.8),
        0.55,
    );
    // Capital and base.
    for (yy, hh) in [(r.y0, 3.4f32), (r.y1 - 3.4, 3.4)] {
        wrect_grad(
            cv,
            cam,
            Rect::new(cx - hw - 2.6, yy, cx + hw + 2.6, yy + hh),
            th.slab_top,
            th.slab_face,
            1.0,
        );
    }
    wrect(
        cv,
        cam,
        Rect::new(cx - hw - 2.6, r.y0, cx + hw + 2.6, r.y0 + 0.8),
        th.slab_top.scale(1.2),
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_slab(
    cv: &mut Canvas,
    cam: &Cam,
    lv: &Level,
    tx: i32,
    ty: i32,
    th: &Theme,
    wobble: f32,
    t: Tile,
) {
    let surf = Level::surf(ty);
    let r = cell_rect(tx, ty);
    // A loose board tips and rattles before it goes.
    let tilt = if wobble > 0.0 {
        noise1(wobble * 46.0, tx * 7 + ty) * 1.5 * clampf(wobble * 2.0, 0.0, 1.0)
    } else {
        0.0
    };
    let y0 = surf + tilt;
    let bottom = y0 + FLOOR_H;

    // Shadow cast onto whatever is below — an explicit alpha ramp, so it
    // darkens the masonry underneath instead of tinting it.
    for i in 0..6 {
        let a = 0.30 * (1.0 - i as f32 / 6.0);
        wrect(
            cv,
            cam,
            Rect::new(r.x0, bottom + i as f32, r.x1, bottom + i as f32 + 1.0),
            rgb(0, 0, 0),
            a,
        );
    }

    // Top face — the surface the prince actually walks on.
    let top_col = match t {
        Tile::Rubble => th.slab_top.scale(0.85),
        Tile::Loose => th.slab_top.scale(0.95),
        _ => th.slab_top,
    };
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0, y0, r.x1, y0 + 2.6),
        top_col.scale(1.1),
        top_col.scale(0.9),
        1.0,
    );
    wrect(
        cv,
        cam,
        Rect::new(r.x0, y0, r.x1, y0 + 0.8),
        top_col.scale(1.25),
        1.0,
    );
    // Front face with a carved band.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0, y0 + 2.6, r.x1, bottom),
        th.slab_face,
        th.slab_dk,
        1.0,
    );
    let n = 4;
    for i in 0..n {
        let bx = r.x0 + (i as f32 + 0.5) * (TILE_W / n as f32);
        wrect(
            cv,
            cam,
            Rect::new(bx - 1.6, y0 + 4.0, bx + 1.6, bottom - 1.4),
            th.slab_dk,
            0.55,
        );
        wrect(
            cv,
            cam,
            Rect::new(bx - 1.6, y0 + 4.0, bx + 1.6, y0 + 4.7),
            th.slab_face.scale(1.25),
            0.5,
        );
    }
    // Vertical seams at the tile joints when the neighbour is also floor.
    if lv.tile(tx - 1, ty).walkable() {
        wrect(
            cv,
            cam,
            Rect::new(r.x0 - 0.4, y0, r.x0 + 0.4, bottom),
            th.slab_dk,
            0.6,
        );
    }
    if t == Tile::Loose {
        // A visible seam all round: the board is not part of the masonry.
        wrect(
            cv,
            cam,
            Rect::new(r.x0 + 0.6, y0, r.x0 + 1.4, bottom),
            th.slab_dk,
            0.85,
        );
        wrect(
            cv,
            cam,
            Rect::new(r.x1 - 1.4, y0, r.x1 - 0.6, bottom),
            th.slab_dk,
            0.85,
        );
    }
    if t == Tile::Rubble {
        // Broken edge where the board used to be.
        for i in 0..5 {
            let h = hashf(tx * 11 + i, ty * 5, 21);
            let bx = r.x0 + 3.0 + i as f32 * 6.0;
            fill_poly(
                cv,
                &[
                    v2(cam.x(bx), cam.y(y0)),
                    v2(cam.x(bx + 4.5), cam.y(y0)),
                    v2(cam.x(bx + 2.2), cam.y(y0 - 1.4 - h * 2.4)),
                ],
                th.slab_face.scale(0.9),
                1.0,
            );
        }
    }
}

fn draw_torch(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme) {
    let (bx, by) = torch_flame_pos(tx, ty);
    let base = v2(bx, by + 8.0);
    // Soot on the wall above.
    for i in 0..4 {
        let f = i as f32 / 4.0;
        fill_ellipse(
            cv,
            v2(cam.x(bx), cam.y(by - 6.0 - f * 9.0)),
            cam.l(5.0 - f * 2.0),
            cam.l(4.5),
            rgb(16, 12, 12),
            0.22 * (1.0 - f),
        );
    }
    // Iron bracket.
    fill_capsule(
        cv,
        v2(cam.x(bx - 3.0), cam.y(by + 13.0)),
        v2(cam.x(bx + 1.0), cam.y(by + 5.0)),
        cam.l(1.3),
        cam.l(1.0),
        th.metal.scale(0.42),
        1.0,
    );
    fill_capsule(
        cv,
        v2(cam.x(bx - 5.0), cam.y(by + 14.0)),
        v2(cam.x(bx + 4.0), cam.y(by + 14.0)),
        cam.l(1.1),
        cam.l(1.1),
        th.metal.scale(0.34),
        1.0,
    );
    // Cup and fuel.
    fill_poly_shaded(
        cv,
        &[
            v2(cam.x(base.x - 3.4), cam.y(base.y - 4.6)),
            v2(cam.x(base.x + 3.4), cam.y(base.y - 4.6)),
            v2(cam.x(base.x + 2.2), cam.y(base.y)),
            v2(cam.x(base.x - 2.2), cam.y(base.y)),
        ],
        th.metal.scale(0.62),
        th.metal.scale(0.3),
        1.0,
    );
    fill_ellipse(
        cv,
        v2(cam.x(base.x), cam.y(base.y - 4.4)),
        cam.l(3.3),
        cam.l(1.1),
        rgb(60, 30, 16),
        1.0,
    );
}

/// Where a torch's flame sits in world coordinates.
pub fn torch_flame_pos(tx: i32, ty: i32) -> (f32, f32) {
    (
        Level::cx(tx),
        ty as f32 * TILE_H + TILE_H * 0.42,
    )
}

fn draw_gate(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, open: f32) {
    let mut r = cell_rect(tx, ty);
    r.y1 = Level::surf(ty);
    let open = clampf(open, 0.0, 1.0);
    // Recess and the slot the grille retracts into.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0 + 1.0, r.y0, r.x1 - 1.0, r.y1),
        th.back_dk.scale(0.55),
        th.back_dk.scale(0.8),
        1.0,
    );
    // Lintel.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0 - 1.0, r.y0, r.x1 + 1.0, r.y0 + 4.2),
        th.slab_top.scale(0.9),
        th.slab_dk,
        1.0,
    );
    // Side jambs.
    for sx in [r.x0, r.x1 - 2.4] {
        wrect_grad(
            cv,
            cam,
            Rect::new(sx, r.y0, sx + 2.4, r.y1),
            th.brick,
            th.brick_dk,
            1.0,
        );
    }

    let travel = (r.y1 - r.y0 - 4.0).max(1.0);
    let top = r.y0 + 4.0;
    let bottom = top + travel * (1.0 - open);
    if bottom <= top + 0.4 {
        return;
    }
    let metal = th.metal;
    // Vertical bars with spear tips.
    let bars = 4;
    for i in 0..bars {
        let bxc = r.x0 + 3.6 + (i as f32 + 0.5) * ((TILE_W - 7.2) / bars as f32);
        wrect_grad(
            cv,
            cam,
            Rect::new(bxc - 1.25, top, bxc + 1.25, bottom - 2.6),
            metal.scale(1.05),
            metal.scale(0.55),
            1.0,
        );
        wrect(
            cv,
            cam,
            Rect::new(bxc - 1.25, top, bxc - 0.4, bottom - 2.6),
            metal.scale(1.35),
            0.8,
        );
        fill_poly_shaded(
            cv,
            &[
                v2(cam.x(bxc - 1.6), cam.y(bottom - 3.0)),
                v2(cam.x(bxc + 1.6), cam.y(bottom - 3.0)),
                v2(cam.x(bxc), cam.y(bottom + 0.6)),
            ],
            metal.scale(1.1),
            metal.scale(0.5),
            1.0,
        );
    }
    // Horizontal bands.
    for f in [0.22f32, 0.68] {
        let by = lerp(top, bottom - 3.0, f);
        if by > top && by < bottom {
            wrect_grad(
                cv,
                cam,
                Rect::new(r.x0 + 2.6, by, r.x1 - 2.6, by + 2.2),
                metal.scale(0.9),
                metal.scale(0.45),
                1.0,
            );
        }
    }
}

fn draw_spikes(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, ext: f32, flag: u8) {
    let surf = Level::surf(ty);
    let r = cell_rect(tx, ty);
    let ext = clampf(ext, 0.0, 1.0);
    // Slots in the slab.
    let n = 5;
    for i in 0..n {
        let sx = r.x0 + 3.0 + (i as f32 + 0.5) * ((TILE_W - 6.0) / n as f32);
        wrect(
            cv,
            cam,
            Rect::new(sx - 1.7, surf - 0.4, sx + 1.7, surf + 1.6),
            rgb(10, 9, 12),
            0.9,
        );
    }
    if ext <= 0.01 {
        return;
    }
    let hmax = 15.0;
    for i in 0..n {
        let sx = r.x0 + 3.0 + (i as f32 + 0.5) * ((TILE_W - 6.0) / n as f32);
        let jitter = hashf(tx * 3 + i, ty, 5) * 0.22 + 0.88;
        let h = hmax * ext * jitter;
        let tip = surf - h;
        let w = 2.0;
        fill_poly_shaded(
            cv,
            &[
                v2(cam.x(sx - w), cam.y(surf + 1.0)),
                v2(cam.x(sx + w), cam.y(surf + 1.0)),
                v2(cam.x(sx + w * 0.35), cam.y(tip + 1.5)),
                v2(cam.x(sx), cam.y(tip)),
                v2(cam.x(sx - w * 0.35), cam.y(tip + 1.5)),
            ],
            th.metal.scale(1.15),
            th.metal.scale(0.45),
            1.0,
        );
        // Specular edge.
        fill_capsule(
            cv,
            v2(cam.x(sx - 0.5), cam.y(surf)),
            v2(cam.x(sx - 0.1), cam.y(tip + 1.0)),
            cam.l(0.45),
            cam.l(0.15),
            Rgb::WHITE,
            0.55,
        );
        if flag & F_BLOODY != 0 {
            fill_capsule(
                cv,
                v2(cam.x(sx), cam.y(tip + 0.5)),
                v2(cam.x(sx), cam.y(tip + 5.5)),
                cam.l(0.9),
                cam.l(0.3),
                rgb(150, 22, 26),
                0.85,
            );
        }
    }
}

fn draw_chomper(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, closure: f32) {
    let mut r = cell_rect(tx, ty);
    r.y1 = Level::surf(ty);
    let c = clampf(closure, 0.0, 1.0);
    // Frame posts.
    for sx in [r.x0 + 0.5, r.x1 - 3.5] {
        wrect_grad(
            cv,
            cam,
            Rect::new(sx, r.y0, sx + 3.0, r.y1),
            th.brick,
            th.brick_dk,
            1.0,
        );
    }
    let inner0 = r.x0 + 3.5;
    let inner1 = r.x1 - 3.5;
    let mid = (inner0 + inner1) * 0.5;
    let reach = (inner1 - inner0) * 0.5 - 1.0;
    let blades = 4;
    let metal = rgb(198, 200, 206);
    for i in 0..blades {
        let by = r.y0 + 3.0 + (i as f32 + 0.5) * ((r.y1 - r.y0 - 7.0) / blades as f32);
        let bh = 3.4;
        // Left blade sweeps right, right blade sweeps left.
        let l_tip = inner0 + reach * c;
        let r_tip = inner1 - reach * c;
        fill_poly_shaded(
            cv,
            &[
                v2(cam.x(inner0), cam.y(by - bh)),
                v2(cam.x(l_tip), cam.y(by)),
                v2(cam.x(inner0), cam.y(by + bh)),
            ],
            metal,
            metal.scale(0.45),
            1.0,
        );
        fill_poly_shaded(
            cv,
            &[
                v2(cam.x(inner1), cam.y(by - bh)),
                v2(cam.x(r_tip), cam.y(by)),
                v2(cam.x(inner1), cam.y(by + bh)),
            ],
            metal,
            metal.scale(0.45),
            1.0,
        );
        if c > 0.9 {
            // Blades meeting — a bright line and a hint of old blood.
            fill_capsule(
                cv,
                v2(cam.x(mid), cam.y(by - bh * 0.7)),
                v2(cam.x(mid), cam.y(by + bh * 0.7)),
                cam.l(0.5),
                cam.l(0.5),
                rgb(255, 240, 220),
                0.7,
            );
        }
    }
    fill_capsule(
        cv,
        v2(cam.x(mid), cam.y(r.y0 + 2.0)),
        v2(cam.x(mid), cam.y(r.y1 - 2.0)),
        cam.l(0.6),
        cam.l(0.6),
        rgb(96, 18, 22),
        0.35,
    );
}

fn draw_plate(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, press: f32, raise: bool) {
    let surf = Level::surf(ty);
    let r = cell_rect(tx, ty);
    let d = clampf(press, 0.0, 1.0) * 1.6;
    let w = TILE_W * 0.34;
    let cxp = r.cx();
    let tint = if raise {
        th.accent
    } else {
        rgb(150, 90, 80)
    };
    // Recess.
    wrect(
        cv,
        cam,
        Rect::new(cxp - w - 1.0, surf - 0.6, cxp + w + 1.0, surf + 3.0),
        th.slab_dk,
        0.9,
    );
    // The stone itself.
    wrect_grad(
        cv,
        cam,
        Rect::new(cxp - w, surf - 1.8 + d, cxp + w, surf + 2.2),
        th.slab_top.lerp(tint, 0.35).scale(1.05),
        th.slab_face.lerp(tint, 0.2),
        1.0,
    );
    wrect(
        cv,
        cam,
        Rect::new(cxp - w, surf - 1.8 + d, cxp + w, surf - 1.0 + d),
        th.slab_top.lerp(tint, 0.5).scale(1.2),
        1.0,
    );
    // A carved mark so raise and drop plates are told apart at a glance.
    if raise {
        fill_poly(
            cv,
            &[
                v2(cam.x(cxp), cam.y(surf - 1.4 + d)),
                v2(cam.x(cxp + 2.4), cam.y(surf + 0.6 + d)),
                v2(cam.x(cxp - 2.4), cam.y(surf + 0.6 + d)),
            ],
            tint.scale(1.3),
            0.85,
        );
    } else {
        fill_poly(
            cv,
            &[
                v2(cam.x(cxp), cam.y(surf + 0.8 + d)),
                v2(cam.x(cxp + 2.4), cam.y(surf - 1.2 + d)),
                v2(cam.x(cxp - 2.4), cam.y(surf - 1.2 + d)),
            ],
            tint.scale(1.3),
            0.85,
        );
    }
}

fn draw_mirror(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, time: f32) {
    let mut r = cell_rect(tx, ty);
    r.y1 = Level::surf(ty);
    let inner = Rect::new(r.x0 + 4.0, r.y0 + 4.0, r.x1 - 4.0, r.y1 - 2.0);
    // Frame.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0 + 1.5, r.y0 + 1.5, r.x1 - 1.5, r.y1),
        th.accent.scale(1.1),
        th.accent.scale(0.5),
        1.0,
    );
    // Glass.
    wrect_grad(
        cv,
        cam,
        inner,
        rgb(150, 168, 196),
        rgb(48, 58, 84),
        1.0,
    );
    // Sweeping highlight.
    let s = (time * 0.5).sin() * 0.5 + 0.5;
    let hx = lerp(inner.x0, inner.x1 - 5.0, s);
    fill_poly(
        cv,
        &[
            v2(cam.x(hx), cam.y(inner.y0)),
            v2(cam.x(hx + 4.0), cam.y(inner.y0)),
            v2(cam.x(hx - 3.0), cam.y(inner.y1)),
            v2(cam.x(hx - 7.0), cam.y(inner.y1)),
        ],
        Rgb::WHITE,
        0.16,
    );
    wrect(
        cv,
        cam,
        Rect::new(inner.x0, inner.y0, inner.x1, inner.y0 + 0.9),
        Rgb::WHITE,
        0.35,
    );
}

fn draw_window(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme) {
    let r = cell_rect(tx, ty);
    let inner = Rect::new(r.x0 + 8.0, r.y0 + 5.0, r.x1 - 8.0, r.y0 + TILE_H * 0.66);
    // Arched opening.
    wrect(cv, cam, inner, rgb(196, 216, 244), 1.0);
    fill_ellipse(
        cv,
        v2(cam.x(inner.cx()), cam.y(inner.y0)),
        cam.l(inner.w() * 0.5),
        cam.l(5.0),
        rgb(196, 216, 244),
        1.0,
    );
    // Bars.
    for i in 0..3 {
        let bx = lerp(inner.x0, inner.x1, (i as f32 + 0.5) / 3.0);
        wrect(
            cv,
            cam,
            Rect::new(bx - 0.7, inner.y0 - 4.0, bx + 0.7, inner.y1),
            th.metal.scale(0.3),
            1.0,
        );
    }
    // Reveal.
    wrect(
        cv,
        cam,
        Rect::new(inner.x0 - 2.0, inner.y0 - 5.0, inner.x0, inner.y1 + 1.0),
        th.brick_dk,
        0.9,
    );
    wrect(
        cv,
        cam,
        Rect::new(inner.x1, inner.y0 - 5.0, inner.x1 + 2.0, inner.y1 + 1.0),
        th.brick_dk,
        0.9,
    );
    wrect(
        cv,
        cam,
        Rect::new(inner.x0 - 2.5, inner.y1, inner.x1 + 2.5, inner.y1 + 2.6),
        th.slab_face,
        1.0,
    );
}

fn draw_arch(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme) {
    let r = cell_rect(tx, ty);
    // Two corbels curving in from the sides, meeting in a point.
    for s in [-1.0f32, 1.0] {
        let x_out = if s < 0.0 { r.x0 } else { r.x1 };
        let pts = [
            v2(cam.x(x_out), cam.y(r.y0)),
            v2(cam.x(x_out), cam.y(r.y0 + 11.0)),
            v2(cam.x(x_out - s * 6.0), cam.y(r.y0 + 7.0)),
            v2(cam.x(r.cx()), cam.y(r.y0 + 5.5)),
            v2(cam.x(r.cx()), cam.y(r.y0)),
        ];
        fill_poly_shaded(cv, &pts, th.brick, th.brick_dk, 1.0);
    }
    wrect(
        cv,
        cam,
        Rect::new(r.x0, r.y0, r.x1, r.y0 + 1.4),
        th.accent,
        0.45,
    );
    fill_circle(
        cv,
        v2(cam.x(r.cx()), cam.y(r.y0 + 5.0)),
        cam.l(2.2),
        th.accent,
        0.8,
    );
}

fn draw_bones_pile(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, _th: &Theme) {
    let surf = Level::surf(ty);
    let cxp = Level::cx(tx);
    let bone = rgb(214, 208, 186);
    for i in 0..4 {
        let h = hashf(tx * 5 + i, ty, 33);
        let x0 = cxp - 10.0 + h * 20.0;
        fill_capsule(
            cv,
            v2(cam.x(x0), cam.y(surf - 1.0 - h * 1.5)),
            v2(cam.x(x0 + 7.0 - h * 3.0), cam.y(surf - 0.6)),
            cam.l(0.9),
            cam.l(0.7),
            bone.scale(0.8 + h * 0.3),
            1.0,
        );
    }
    fill_circle(
        cv,
        v2(cam.x(cxp - 4.0), cam.y(surf - 2.6)),
        cam.l(2.8),
        bone,
        1.0,
    );
    fill_circle(
        cv,
        v2(cam.x(cxp - 2.6), cam.y(surf - 2.8)),
        cam.l(0.85),
        rgb(28, 22, 24),
        1.0,
    );
    fill_capsule(
        cv,
        v2(cam.x(cxp + 3.0), cam.y(surf - 1.2)),
        v2(cam.x(cxp + 9.0), cam.y(surf - 1.2)),
        cam.l(0.8),
        cam.l(0.8),
        bone.scale(0.9),
        1.0,
    );
}

fn draw_exit(cv: &mut Canvas, cam: &Cam, tx: i32, ty: i32, th: &Theme, open: f32) {
    let r = cell_rect(tx, ty);
    let open = clampf(open, 0.0, 1.0);
    let floor = Level::surf(ty);
    let inner = Rect::new(r.x0 + 3.0, r.y0 + 6.0, r.x1 - 3.0, floor);
    // Lit stairway behind the doors.
    wrect_grad(
        cv,
        cam,
        inner,
        rgb(240, 228, 190),
        rgb(120, 96, 62),
        1.0,
    );
    for i in 0..5 {
        let sy = lerp(inner.y0 + 6.0, inner.y1, i as f32 / 5.0);
        wrect(
            cv,
            cam,
            Rect::new(inner.x0, sy, inner.x1, sy + 1.4),
            rgb(90, 70, 44),
            0.6,
        );
    }
    // Arch over the door.
    for s in [-1.0f32, 1.0] {
        let x_out = if s < 0.0 { r.x0 } else { r.x1 };
        fill_poly_shaded(
            cv,
            &[
                v2(cam.x(x_out), cam.y(r.y0)),
                v2(cam.x(x_out), cam.y(r.y0 + 12.0)),
                v2(cam.x(x_out - s * 5.0), cam.y(r.y0 + 8.0)),
                v2(cam.x(r.cx()), cam.y(r.y0 + 5.0)),
                v2(cam.x(r.cx()), cam.y(r.y0)),
            ],
            th.accent.scale(0.95),
            th.accent.scale(0.4),
            1.0,
        );
    }
    // Two leaves sliding apart.
    let leaf_w = inner.w() * 0.5;
    let slide = leaf_w * open;
    for s in [-1.0f32, 1.0] {
        let x0 = if s < 0.0 {
            inner.x0 - slide
        } else {
            inner.cx() + slide
        };
        let x1 = x0 + leaf_w;
        if x1 <= inner.x0 || x0 >= inner.x1 {
            continue;
        }
        let cx0 = x0.max(inner.x0 - leaf_w);
        wrect_grad(
            cv,
            cam,
            Rect::new(cx0, inner.y0, x1.min(inner.x1 + leaf_w), inner.y1),
            rgb(104, 66, 38),
            rgb(58, 36, 22),
            1.0,
        );
        // Planks and studs.
        for i in 0..3 {
            let px = lerp(cx0, x1, (i as f32 + 0.5) / 3.0);
            wrect(
                cv,
                cam,
                Rect::new(px - 0.5, inner.y0, px + 0.5, inner.y1),
                rgb(40, 24, 14),
                0.65,
            );
        }
        for i in 0..3 {
            let py = lerp(inner.y0 + 4.0, inner.y1 - 4.0, i as f32 / 2.0);
            fill_circle(
                cv,
                v2(cam.x(lerp(cx0, x1, 0.3)), cam.y(py)),
                cam.l(1.1),
                th.metal.scale(0.7),
                1.0,
            );
            fill_circle(
                cv,
                v2(cam.x(lerp(cx0, x1, 0.7)), cam.y(py)),
                cam.l(1.1),
                th.metal.scale(0.7),
                1.0,
            );
        }
    }
    // Threshold.
    wrect_grad(
        cv,
        cam,
        Rect::new(r.x0, floor - 2.0, r.x1, floor),
        th.slab_top,
        th.slab_face,
        1.0,
    );
}

// ---------------------------------------------------------------- light & emissive

/// Report every light source in view. Positions are world-space.
pub fn collect_lights(
    lv: &Level,
    dy: &Dynamics,
    view: Rect,
    time: f32,
    out: &mut Vec<(V2, f32, Rgb, f32)>,
) {
    let tx0 = (view.x0 / TILE_W).floor() as i32 - 1;
    let tx1 = (view.x1 / TILE_W).ceil() as i32 + 1;
    let ty0 = (view.y0 / TILE_H).floor() as i32 - 1;
    let ty1 = (view.y1 / TILE_H).ceil() as i32 + 1;
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            match lv.tile(tx, ty) {
                Tile::Torch => {
                    let (fx, fy) = torch_flame_pos(tx, ty);
                    // Flicker: two noise octaves keep it from looking periodic.
                    let f = 1.0
                        + 0.16 * noise1(time * 6.0 + tx as f32 * 3.7, ty * 11)
                        + 0.07 * noise1(time * 17.0 + ty as f32 * 1.3, tx * 5);
                    out.push((
                        v2(fx, fy - 3.0),
                        TILE_W * 3.4 * f,
                        lv.theme.torch,
                        1.30 * f,
                    ));
                }
                Tile::Window => {
                    let r = cell_rect(tx, ty);
                    out.push((
                        v2(r.cx(), r.y0 + 10.0),
                        TILE_W * 2.4,
                        rgb(180, 210, 255),
                        0.85,
                    ));
                }
                Tile::Exit => {
                    let o = dy.a(tx, ty);
                    if o > 0.02 {
                        let r = cell_rect(tx, ty);
                        out.push((
                            v2(r.cx(), r.cy()),
                            TILE_W * 2.2 * o,
                            rgb(255, 226, 168),
                            1.1 * o,
                        ));
                    }
                }
                Tile::PlateRaise => {
                    if dy.has(tx, ty, F_LATCHED) {
                        let r = cell_rect(tx, ty);
                        out.push((
                            v2(r.cx(), r.y1 - 2.0),
                            TILE_W * 0.8,
                            lv.theme.accent,
                            0.35,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Additive pass: light shafts from windows, the glow spilling from an open
/// exit, and the sheen on polished metal.
pub fn draw_emissive(cv: &mut Canvas, cam: &Cam, lv: &Level, dy: &Dynamics, view: Rect, time: f32) {
    let tx0 = (view.x0 / TILE_W).floor() as i32 - 1;
    let tx1 = (view.x1 / TILE_W).ceil() as i32 + 1;
    let ty0 = (view.y0 / TILE_H).floor() as i32 - 1;
    let ty1 = (view.y1 / TILE_H).ceil() as i32 + 1;
    cv.blend = Blend::Add;
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            match lv.tile(tx, ty) {
                Tile::Window => {
                    let r = cell_rect(tx, ty);
                    // A shaft of daylight slanting down and to the right.
                    let top = v2(r.cx() - 7.0, r.y0 + 8.0);
                    let len = TILE_H * 2.3;
                    let pts = [
                        v2(cam.x(top.x), cam.y(top.y)),
                        v2(cam.x(top.x + 14.0), cam.y(top.y)),
                        v2(cam.x(top.x + 14.0 + len * 0.55), cam.y(top.y + len)),
                        v2(cam.x(top.x + len * 0.55 - 5.0), cam.y(top.y + len)),
                    ];
                    fill_poly_shaded(cv, &pts, rgb(120, 146, 190), rgb(10, 14, 24), 0.5);
                    radial_glow(
                        cv,
                        v2(cam.x(r.cx()), cam.y(r.y0 + 9.0)),
                        cam.l(TILE_W * 0.9),
                        rgb(200, 224, 255),
                        0.5,
                    );
                }
                Tile::Exit => {
                    let o = dy.a(tx, ty);
                    if o > 0.02 {
                        let r = cell_rect(tx, ty);
                        radial_glow(
                            cv,
                            v2(cam.x(r.cx()), cam.y(r.cy())),
                            cam.l(TILE_W * 1.5 * o),
                            rgb(255, 226, 172),
                            0.42 * o,
                        );
                    }
                }
                Tile::Mirror => {
                    let r = cell_rect(tx, ty);
                    let s = (time * 0.5).sin() * 0.5 + 0.5;
                    radial_glow(
                        cv,
                        v2(cam.x(lerp(r.x0 + 6.0, r.x1 - 6.0, s)), cam.y(r.cy())),
                        cam.l(TILE_W * 0.6),
                        rgb(180, 210, 255),
                        0.3,
                    );
                }
                _ => {}
            }
        }
    }
    cv.blend = Blend::Alpha;
}
