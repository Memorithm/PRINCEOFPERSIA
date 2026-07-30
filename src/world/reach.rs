//! Level validation: can the prince actually get from the start to the exit?
//!
//! Hand-authored maps are easy to get wrong — one missing floor tile and a whole
//! wing becomes unreachable. This module walks the map with a deliberately
//! *conservative* model of the prince's moveset (no move here is easier than
//! what the physics actually allows), so if it finds a route the route exists.
//! Gates are resolved by fixpoint: a gate opens once a plate on its group has
//! been proven reachable, which may unlock further plates, and so on.

use std::collections::VecDeque;

use crate::world::level::Level;
use crate::world::tile::*;

/// Longest horizontal gap a running jump may cross, in tiles.
const MAX_GAP: i32 = 3;

pub struct Reach {
    pub tw: i32,
    pub th: i32,
    pub seen: Vec<bool>,
    pub groups: Vec<bool>,
    pub exit_reached: bool,
    /// Items the route passes over — used to check a level hands out its sword.
    pub items_seen: Vec<ItemKind>,
}

impl Reach {
    #[inline]
    pub fn at(&self, tx: i32, ty: i32) -> bool {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            false
        } else {
            self.seen[(ty * self.tw + tx) as usize]
        }
    }
}

/// Does the cell provide a place to stand?
fn supported(lv: &Level, tx: i32, ty: i32) -> bool {
    if !lv.in_bounds(tx, ty) {
        return false;
    }
    let here = lv.tile(tx, ty);
    if here.solid() {
        return false;
    }
    here.walkable() || lv.tile(tx, ty + 1).solid()
}

/// Can a body occupy this cell (ignoring support)?
fn passable(lv: &Level, tx: i32, ty: i32, groups: &[bool]) -> bool {
    if !lv.in_bounds(tx, ty) {
        return false;
    }
    let c = lv.cell(tx, ty);
    if c.tile.solid() {
        return false;
    }
    if c.tile == Tile::Gate {
        // A gate blocks until one of its plates has been reached.
        return c.group != 0 && groups.get(c.group as usize).copied().unwrap_or(false);
    }
    true
}

/// Where does a body dropped into `(tx, ty)` come to rest?
fn fall_to(lv: &Level, tx: i32, ty: i32, groups: &[bool]) -> Option<i32> {
    let mut y = ty;
    while y < lv.th {
        if !passable(lv, tx, y, groups) {
            return None;
        }
        if supported(lv, tx, y) {
            return Some(y);
        }
        y += 1;
    }
    None
}

pub fn analyse(lv: &Level) -> Reach {
    let tw = lv.tw;
    let th = lv.th;
    let mut groups = vec![false; 96];
    let mut seen = vec![false; (tw * th) as usize];
    let mut items_seen = Vec::new();

    loop {
        // A fresh flood fill each round; gates opened last round now let us
        // through, which may put new plates within reach.
        for s in seen.iter_mut() {
            *s = false;
        }
        let mut q = VecDeque::new();
        let start = lv.start;
        if supported(lv, start.0, start.1) {
            seen[(start.1 * tw + start.0) as usize] = true;
            q.push_back(start);
        }
        let mut new_group = false;

        while let Some((tx, ty)) = q.pop_front() {
            let c = lv.cell(tx, ty);
            if c.tile.plate() && c.group != 0 {
                let g = c.group as usize;
                if g < groups.len() && !groups[g] && c.tile == Tile::PlateRaise {
                    groups[g] = true;
                    new_group = true;
                }
            }

            let push = |x: i32, y: i32, q: &mut VecDeque<(i32, i32)>, seen: &mut Vec<bool>| {
                if x < 0 || y < 0 || x >= tw || y >= th {
                    return;
                }
                let i = (y * tw + x) as usize;
                if !seen[i] {
                    seen[i] = true;
                    q.push_back((x, y));
                }
            };

            for dx in [-1i32, 1] {
                let nx = tx + dx;
                // --- walk one tile ---------------------------------------
                if passable(lv, nx, ty, &groups) {
                    if supported(lv, nx, ty) {
                        push(nx, ty, &mut q, &mut seen);
                    } else if let Some(fy) = fall_to(lv, nx, ty, &groups) {
                        // Step off the edge and fall.
                        push(nx, fy, &mut q, &mut seen);
                    }
                }

                // --- climb up one level onto an adjacent ledge -----------
                if passable(lv, tx, ty - 1, &groups)
                    && passable(lv, nx, ty - 1, &groups)
                    && supported(lv, nx, ty - 1)
                {
                    push(nx, ty - 1, &mut q, &mut seen);
                }

                // --- hang and drop to the level below --------------------
                if passable(lv, nx, ty, &groups) && supported(lv, nx, ty + 1) {
                    push(nx, ty + 1, &mut q, &mut seen);
                }

                // --- running jumps across a gap --------------------------
                // Needs a tile of run-up: the cell behind must be standable.
                let has_runup = supported(lv, tx - dx, ty) || supported(lv, tx - 2 * dx, ty);
                for gap in 1..=MAX_GAP {
                    let land = tx + dx * (gap + 1);
                    let mut clear = true;
                    for k in 1..=gap {
                        let mx = tx + dx * k;
                        if !passable(lv, mx, ty, &groups) || !passable(lv, mx, ty - 1, &groups) {
                            clear = false;
                            break;
                        }
                    }
                    if !clear {
                        continue;
                    }
                    // A standing jump clears one tile; anything longer needs a run.
                    if gap > 1 && !has_runup {
                        continue;
                    }
                    if passable(lv, land, ty, &groups) && supported(lv, land, ty) {
                        push(land, ty, &mut q, &mut seen);
                    }
                    // Jump up onto a ledge one level higher across the gap.
                    if gap <= 2
                        && passable(lv, land, ty - 1, &groups)
                        && supported(lv, land, ty - 1)
                        && passable(lv, tx, ty - 1, &groups)
                    {
                        push(land, ty - 1, &mut q, &mut seen);
                    }
                    // Or fall into the gap on purpose.
                    if let Some(fy) = fall_to(lv, land, ty, &groups) {
                        if passable(lv, land, ty, &groups) {
                            push(land, fy, &mut q, &mut seen);
                        }
                    }
                }
            }
        }

        if !new_group {
            break;
        }
    }

    // What did we pass over?
    for it in &lv.items {
        if seen[(it.ty * tw + it.tx) as usize] {
            items_seen.push(it.kind);
        }
    }

    let (ex, ey) = lv.exit;
    let eg = lv.cell(ex, ey).group as usize;
    let exit_open = eg == 0 || groups.get(eg).copied().unwrap_or(false);
    let exit_reached = seen[(ey * tw + ex) as usize] && exit_open;

    Reach {
        tw,
        th,
        seen,
        groups,
        exit_reached,
        items_seen,
    }
}

/// Render the map with reachable cells highlighted — used by `pop --validate`.
pub fn debug_map(lv: &Level, r: &Reach) -> String {
    let mut s = String::new();
    for ty in 0..lv.th {
        for tx in 0..lv.tw {
            let t = lv.tile(tx, ty);
            let g = t.glyph();
            if (tx, ty) == lv.start {
                s.push('@');
            } else if r.at(tx, ty) {
                s.push(if g == ' ' { '·' } else { g });
            } else if t == Tile::Wall {
                s.push('#');
            } else if g == ' ' {
                s.push(' ');
            } else {
                // Reachable-looking terrain that the prince cannot get to.
                s.push(g.to_ascii_lowercase());
            }
        }
        s.push('\n');
        if (ty + 1) % ROOM_TH == 0 {
            s.push('\n');
        }
    }
    s
}
