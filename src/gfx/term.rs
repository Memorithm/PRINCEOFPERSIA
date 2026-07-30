//! Terminal front-end: a cell grid that can hold either text or a pair of
//! pixels, plus a diffing writer that only emits the cells that changed.
//!
//! Pixels are drawn with the upper-half-block glyph `▀`: the foreground colour
//! paints the top half of the cell and the background colour the bottom half.
//! With 24-bit colour that gives two independently coloured, square-ish pixels
//! per character cell, which is the highest fidelity a plain terminal can offer
//! without giving up per-pixel colour.

use std::fmt::Write as _;
use std::io::Write;

use crate::gfx::canvas::Canvas;
use crate::gfx::color::{rgb, Rgb};

pub const HALF: char = '\u{2580}'; // ▀

pub const A_BOLD: u8 = 1;
pub const A_DIM: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub attr: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: rgb(200, 200, 200),
            bg: Rgb::BLACK,
            attr: 0,
        }
    }
}

/// Largest per-channel difference treated as "no visible change".
///
/// Flickering torchlight nudges almost every cell by a unit or two every frame.
/// Re-sending all of them would cost megabytes per second of escape codes for no
/// visible benefit, so near-identical cells are left alone; a rotating band of
/// rows is repainted unconditionally each frame so nothing can drift out of date.
const TOL: i16 = 3;

#[inline]
fn near(a: Rgb, b: Rgb) -> bool {
    (a.r as i16 - b.r as i16).abs() <= TOL
        && (a.g as i16 - b.g as i16).abs() <= TOL
        && (a.b as i16 - b.b as i16).abs() <= TOL
}

impl Cell {
    /// Would repainting this cell make a visible difference?
    #[inline]
    fn differs(&self, o: &Cell) -> bool {
        if self.ch != o.ch || self.attr != o.attr {
            return true;
        }
        // A space shows only its background.
        if self.ch == ' ' {
            return !near(self.bg, o.bg);
        }
        !near(self.fg, o.fg) || !near(self.bg, o.bg)
    }
}

pub struct Screen {
    pub cols: i32,
    pub rows: i32,
    cur: Vec<Cell>,
    prev: Vec<Cell>,
    /// Scratch pixel grid for resampling.
    pixbuf: Vec<Rgb>,
    out: String,
    force: bool,
    /// Row repainted unconditionally this frame, to stop slow colour drift.
    sweep: i32,
}

impl Screen {
    pub fn new(cols: i32, rows: i32) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let n = (cols * rows) as usize;
        Screen {
            cols,
            rows,
            cur: vec![Cell::default(); n],
            prev: vec![Cell::default(); n],
            pixbuf: Vec::new(),
            out: String::with_capacity(1 << 16),
            force: true,
            sweep: 0,
        }
    }

    pub fn resize(&mut self, cols: i32, rows: i32) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let n = (cols * rows) as usize;
        self.cur = vec![Cell::default(); n];
        self.prev = vec![Cell::default(); n];
        self.force = true;
    }

    /// Next flush repaints everything (after a resize or leaving an alt screen).
    pub fn invalidate(&mut self) {
        self.force = true;
    }

    pub fn clear(&mut self, bg: Rgb) {
        for c in self.cur.iter_mut() {
            *c = Cell {
                ch: ' ',
                fg: rgb(200, 200, 200),
                bg,
                attr: 0,
            };
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: Cell) {
        if x >= 0 && y >= 0 && x < self.cols && y < self.rows {
            self.cur[(y * self.cols + x) as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Cell {
        if x >= 0 && y >= 0 && x < self.cols && y < self.rows {
            self.cur[(y * self.cols + x) as usize]
        } else {
            Cell::default()
        }
    }

    /// Draw a string, returning the column just past the end. Wide characters
    /// are not supported on purpose: every glyph used by the HUD is 1 cell.
    pub fn text(&mut self, x: i32, y: i32, s: &str, fg: Rgb, bg: Rgb) -> i32 {
        self.text_attr(x, y, s, fg, bg, 0)
    }

    pub fn text_attr(&mut self, x: i32, y: i32, s: &str, fg: Rgb, bg: Rgb, attr: u8) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.cols {
                break;
            }
            if cx >= 0 {
                self.set(cx, y, Cell { ch, fg, bg, attr });
            }
            cx += 1;
        }
        cx
    }

    /// Draw a string keeping whatever background is already in those cells.
    pub fn text_over(&mut self, x: i32, y: i32, s: &str, fg: Rgb, attr: u8) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.cols {
                break;
            }
            if cx >= 0 {
                let bg = self.get(cx, y).bg;
                self.set(cx, y, Cell { ch, fg, bg, attr });
            }
            cx += 1;
        }
        cx
    }

    pub fn text_centred(&mut self, y: i32, s: &str, fg: Rgb, bg: Rgb, attr: u8) {
        let n = s.chars().count() as i32;
        self.text_attr((self.cols - n) / 2, y, s, fg, bg, attr);
    }

    pub fn fill_row(&mut self, y: i32, bg: Rgb) {
        for x in 0..self.cols {
            self.set(
                x,
                y,
                Cell {
                    ch: ' ',
                    fg: rgb(200, 200, 200),
                    bg,
                    attr: 0,
                },
            );
        }
    }

    /// Resample `cv` into the cell rectangle `(cx, cy, cw, ch)`, two pixels per
    /// cell row.
    pub fn blit(&mut self, cv: &Canvas, cx: i32, cy: i32, cw: i32, ch: i32) {
        if cw <= 0 || ch <= 0 {
            return;
        }
        let pw = cw;
        let ph = ch * 2;
        let n = (pw * ph) as usize;
        if self.pixbuf.len() != n {
            self.pixbuf = vec![Rgb::BLACK; n];
        }
        cv.resample_into(&mut self.pixbuf, pw, ph);
        for row in 0..ch {
            for col in 0..cw {
                let top = self.pixbuf[((row * 2) * pw + col) as usize];
                let bot = self.pixbuf[((row * 2 + 1) * pw + col) as usize];
                self.set(
                    cx + col,
                    cy + row,
                    Cell {
                        ch: HALF,
                        fg: top,
                        bg: bot,
                        attr: 0,
                    },
                );
            }
        }
    }

    /// Emit the minimal escape sequence run needed to bring the terminal in
    /// line with the current buffer.
    pub fn flush(&mut self, w: &mut impl Write) -> std::io::Result<()> {
        self.out.clear();
        let mut cur_fg: Option<Rgb> = None;
        let mut cur_bg: Option<Rgb> = None;
        let mut cur_attr: u8 = 0xFF;
        let mut cursor: Option<(i32, i32)> = None;

        // Repaint a moving band of rows in full each frame: at 30 fps a
        // 30-row terminal is completely refreshed once a second.
        let band = (self.rows / 12).max(1);
        let sweep_lo = self.sweep;
        let sweep_hi = self.sweep + band;
        self.sweep = (self.sweep + band) % self.rows.max(1);

        for y in 0..self.rows {
            let swept = self.force || (y >= sweep_lo && y < sweep_hi);
            let mut x = 0;
            while x < self.cols {
                let i = (y * self.cols + x) as usize;
                if !swept && !self.cur[i].differs(&self.prev[i]) {
                    x += 1;
                    continue;
                }
                // This cell is being sent, so it becomes the new reference. Cells
                // that were skipped keep their old value, which is what the
                // terminal is actually showing — that way the tolerance can never
                // accumulate into a visible error.
                self.prev[i] = self.cur[i];
                // Move the cursor unless we are already in the right place.
                if cursor != Some((x, y)) {
                    let _ = write!(self.out, "\x1b[{};{}H", y + 1, x + 1);
                }
                let c = self.cur[i];
                if cur_attr != c.attr {
                    // Reset then re-apply; colours must be re-sent after SGR 0.
                    self.out.push_str("\x1b[0m");
                    if c.attr & A_BOLD != 0 {
                        self.out.push_str("\x1b[1m");
                    }
                    if c.attr & A_DIM != 0 {
                        self.out.push_str("\x1b[2m");
                    }
                    cur_attr = c.attr;
                    cur_fg = None;
                    cur_bg = None;
                }
                if cur_fg != Some(c.fg) {
                    let _ = write!(self.out, "\x1b[38;2;{};{};{}m", c.fg.r, c.fg.g, c.fg.b);
                    cur_fg = Some(c.fg);
                }
                if cur_bg != Some(c.bg) {
                    let _ = write!(self.out, "\x1b[48;2;{};{};{}m", c.bg.r, c.bg.g, c.bg.b);
                    cur_bg = Some(c.bg);
                }
                self.out.push(c.ch);
                cursor = Some((x + 1, y));
                x += 1;
            }
        }
        if !self.out.is_empty() {
            self.out.push_str("\x1b[0m");
            w.write_all(self.out.as_bytes())?;
            w.flush()?;
        }
        self.force = false;
        Ok(())
    }
}
