//! The heads-up display. Drawn as real terminal text rather than pixels so it
//! stays crisp at any font size.

use crate::game::*;
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::term::{Screen, A_BOLD, A_DIM};
use crate::world::level::Level;

pub const BAR_BG: Rgb = rgb(20, 17, 26);
pub const BAR_FG: Rgb = rgb(206, 198, 184);
const GOLD: Rgb = rgb(226, 186, 92);
const RED: Rgb = rgb(214, 66, 62);
const DIMTEXT: Rgb = rgb(124, 118, 130);

pub fn fmt_time(secs: f32) -> String {
    let s = secs.max(0.0) as i32;
    format!("{}:{:02}", s / 60, s % 60)
}

impl Game {
    /// Rows reserved at the top and bottom of the terminal.
    pub const TOP_ROWS: i32 = 1;
    pub const BOTTOM_ROWS: i32 = 2;

    pub fn draw_hud(&self, scr: &mut Screen) {
        let cols = scr.cols;
        let rows = scr.rows;

        // ---- title bar ----------------------------------------------------
        scr.fill_row(0, BAR_BG);
        let n = crate::world::levels::count();
        let title = format!(" {}/{}  {} ", self.idx + 1, n, self.lv.name);
        scr.text_attr(0, 0, &title, GOLD, BAR_BG, A_BOLD);

        let (tx, ty) = self.player.foot_tile();
        let room = Level::room_of(tx, ty);
        let mid = format!(
            "salle {},{} sur {}x{}",
            room.0 + 1,
            room.1 + 1,
            self.lv.rw,
            self.lv.rh
        );
        let midx = (cols - mid.chars().count() as i32) / 2;
        scr.text_attr(midx, 0, &mid, DIMTEXT, BAR_BG, A_DIM);

        let low = self.clock < 60.0;
        let right = format!(
            "☠{}  ⚔{}  ⏱ {} ",
            self.deaths,
            self.kills,
            fmt_time(self.clock)
        );
        let rx = cols - right.chars().count() as i32;
        scr.text_attr(
            rx,
            0,
            &right,
            if low { RED } else { BAR_FG },
            BAR_BG,
            if low { A_BOLD } else { 0 },
        );

        // ---- status line --------------------------------------------------
        let sy = rows - 2;
        scr.fill_row(sy, BAR_BG);
        let mut x = 1;
        // Health.
        for i in 0..self.player.hp_max {
            let full = i < self.player.hp;
            x = scr.text_attr(
                x,
                sy,
                if full { "♥" } else { "♡" },
                if full { RED } else { rgb(80, 48, 52) },
                BAR_BG,
                if full { A_BOLD } else { A_DIM },
            );
        }
        x += 1;

        // Melee weapon.
        let (wsym, wname, wcol) = match self.player.melee {
            Melee::None => ("·", "mains nues", DIMTEXT),
            Melee::Sword => ("†", "épée", rgb(198, 206, 218)),
            Melee::Scimitar => ("≯", "cimeterre", rgb(226, 214, 168)),
        };
        let drawn = if self.player.armed { "" } else { " (rengainée)" };
        x = scr.text_attr(
            x,
            sy,
            &format!("{} {}{}", wsym, wname, drawn),
            wcol,
            BAR_BG,
            if self.player.armed { A_BOLD } else { A_DIM },
        );
        x += 2;

        if self.player.daggers > 0 {
            x = scr.text_attr(
                x,
                sy,
                &format!("✦ dagues {}", self.player.daggers),
                rgb(180, 200, 220),
                BAR_BG,
                0,
            );
            x += 2;
        }
        if self.player.wand {
            x = scr.text_attr(
                x,
                sy,
                &format!("✹ flamme {}", self.player.charges),
                rgb(255, 170, 70),
                BAR_BG,
                0,
            );
            x += 2;
        }
        if self.player.buckler {
            x = scr.text_attr(x, sy, "◉ bouclier", rgb(196, 150, 96), BAR_BG, 0);
            x += 2;
        }
        if self.player.float_t > 0.0 {
            x = scr.text_attr(
                x,
                sy,
                &format!("↟ plume {:.0}s", self.player.float_t),
                rgb(120, 200, 235),
                BAR_BG,
                0,
            );
            x += 2;
        }
        if self.player.swift_t > 0.0 {
            x = scr.text_attr(
                x,
                sy,
                &format!("» célérité {:.0}s", self.player.swift_t),
                rgb(240, 214, 90),
                BAR_BG,
                0,
            );
            x += 2;
        }
        let _ = x;

        // Boss health, right-aligned.
        if let Some((name, frac)) = self.boss() {
            let w = 14;
            let filled = (frac * w as f32).round() as i32;
            let mut bar = String::new();
            for i in 0..w {
                bar.push(if i < filled { '█' } else { '░' });
            }
            let s = format!("{} {} ", name, bar);
            let bx = cols - s.chars().count() as i32;
            scr.text_attr(bx, sy, &s, rgb(226, 90, 96), BAR_BG, A_BOLD);
        }

        // ---- message line -------------------------------------------------
        let my = rows - 1;
        scr.fill_row(my, rgb(12, 10, 16));
        match &self.msg {
            Some(m) => {
                let a = m.t.clamp(0.25, 1.0);
                let base = if m.warn {
                    rgb(240, 120, 100)
                } else {
                    rgb(212, 202, 176)
                };
                scr.text_attr(
                    1,
                    my,
                    &m.text,
                    base.scale(0.55 + 0.45 * a),
                    rgb(12, 10, 16),
                    if m.warn { A_BOLD } else { 0 },
                );
            }
            None => {
                let keys = "←→ courir   ↑ sauter/grimper   ↓ s'accroupir   Maj+dir pas prudent   Espace frapper   Z parer   T dague   F flamme   C rengainer   P pause   R recommencer";
                scr.text_attr(1, my, keys, rgb(72, 68, 80), rgb(12, 10, 16), A_DIM);
            }
        }
    }
}
