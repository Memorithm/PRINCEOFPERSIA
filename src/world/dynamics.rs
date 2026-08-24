//! Per-cell animated state: gate travel, spike extension, chomper phase,
//! pressure-plate depression, loose-board wobble.
//!
//! Kept as a flat array parallel to the tile map so the renderer can ask about
//! any cell in constant time without a hash lookup per tile per frame.

#[derive(Clone, Copy, Default)]
pub struct CellDyn {
    /// Primary animated value, meaning depends on the tile:
    /// gate/exit openness, spike extension, chomper closure, plate depression.
    pub a: f32,
    /// Secondary: countdown timers (gate hold, loose-board fuse, chomper phase).
    pub b: f32,
    /// Bit flags — see the `F_*` constants.
    pub flag: u8,
}

/// A loose board has been triggered and is counting down.
pub const F_TRIGGERED: u8 = 1;
/// Spikes have been armed by something stepping near them.
pub const F_ARMED: u8 = 2;
/// This gate/door is latched open permanently.
pub const F_LATCHED: u8 = 4;
/// Something is standing on this plate right now.
pub const F_PRESSED: u8 = 8;
/// Spikes that have already tasted blood — drawn with red tips.
pub const F_BLOODY: u8 = 16;

pub struct Dynamics {
    pub tw: i32,
    pub th: i32,
    pub v: Vec<CellDyn>,
}

impl Dynamics {
    pub fn new(tw: i32, th: i32) -> Self {
        Dynamics {
            tw,
            th,
            v: vec![CellDyn::default(); (tw * th).max(1) as usize],
        }
    }

    #[inline]
    fn idx(&self, tx: i32, ty: i32) -> Option<usize> {
        if tx < 0 || ty < 0 || tx >= self.tw || ty >= self.th {
            None
        } else {
            Some((ty * self.tw + tx) as usize)
        }
    }

    #[inline]
    pub fn get(&self, tx: i32, ty: i32) -> CellDyn {
        match self.idx(tx, ty) {
            Some(i) => self.v[i],
            None => CellDyn::default(),
        }
    }

    /// Mutable cell state. Out-of-bounds coordinates return `None` instead of
    /// silently aliasing cell (0, 0).
    #[inline]
    pub fn at(&mut self, tx: i32, ty: i32) -> Option<&mut CellDyn> {
        self.idx(tx, ty).map(|i| &mut self.v[i])
    }

    /// Checked variant for the hot loops that already know the tile exists.
    #[inline]
    pub fn set_a(&mut self, tx: i32, ty: i32, a: f32) {
        if let Some(c) = self.at(tx, ty) {
            c.a = a;
        }
    }

    #[inline]
    pub fn a(&self, tx: i32, ty: i32) -> f32 {
        self.get(tx, ty).a
    }

    #[inline]
    pub fn has(&self, tx: i32, ty: i32, f: u8) -> bool {
        self.get(tx, ty).flag & f != 0
    }

    pub fn set_flag(&mut self, tx: i32, ty: i32, f: u8, on: bool) {
        if let Some(i) = self.idx(tx, ty) {
            if on {
                self.v[i].flag |= f;
            } else {
                self.v[i].flag &= !f;
            }
        }
    }
}
