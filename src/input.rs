//! Keyboard handling.
//!
//! Terminals traditionally report only key *presses*, never releases, which is a
//! problem for a game that needs "is the run key still held?". Where the
//! terminal supports the kitty keyboard protocol (kitty, ghostty, WezTerm, foot,
//! recent Alacritty and Windows Terminal) crossterm gives us real release events
//! and control is exact. Everywhere else we fall back to a hold window: a press
//! keeps the action alive for [`HOLD_FALLBACK`] and each auto-repeat extends it.
//! In that mode `Shift`+direction — a single short, precise step — is the safe
//! way to walk up to a ledge or a spike pit.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub const HOLD_FALLBACK: Duration = Duration::from_millis(420);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Left,
    Right,
    Up,
    Down,
    Careful,
    Attack,
    Parry,
    Throw,
    Cast,
    Sheathe,
}
const NACT: usize = 10;

fn slot(a: Act) -> usize {
    match a {
        Act::Left => 0,
        Act::Right => 1,
        Act::Up => 2,
        Act::Down => 3,
        Act::Careful => 4,
        Act::Attack => 5,
        Act::Parry => 6,
        Act::Throw => 7,
        Act::Cast => 8,
        Act::Sheathe => 9,
    }
}

/// What the simulation sees: continuous holds plus one-shot edges.
#[derive(Clone, Copy, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub careful: bool,
    pub attack: bool,
    pub parry: bool,
    pub throw: bool,
    pub cast: bool,
    pub sheathe: bool,
    /// Rising edges, consumed once.
    pub up_edge: bool,
    pub down_edge: bool,
    pub left_edge: bool,
    pub right_edge: bool,
}

impl Input {
    pub fn dir(&self) -> f32 {
        match (self.left, self.right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        }
    }
    pub fn any_dir(&self) -> bool {
        self.left || self.right
    }
}

/// Commands that are not part of the character's moveset.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Quit,
    Pause,
    Restart,
    Help,
    NextWeapon,
    Confirm,
    Back,
    Up,
    Down,
    Skip,
}

pub struct Reader {
    /// True when the terminal gives us real key-release events.
    pub enhanced: bool,
    last: [Option<Instant>; NACT],
    held: [bool; NACT],
    edge: [bool; NACT],
    pub cmds: Vec<Cmd>,
}

impl Reader {
    pub fn new(enhanced: bool) -> Self {
        Reader {
            enhanced,
            last: [None; NACT],
            held: [false; NACT],
            edge: [false; NACT],
            cmds: Vec::new(),
        }
    }

    fn map(code: KeyCode, m: KeyModifiers) -> Option<Act> {
        let a = match code {
            KeyCode::Left => Act::Left,
            KeyCode::Right => Act::Right,
            KeyCode::Up => Act::Up,
            KeyCode::Down => Act::Down,
            KeyCode::Char('a') | KeyCode::Char('A') => Act::Left,
            KeyCode::Char('d') | KeyCode::Char('D') => Act::Right,
            KeyCode::Char('w') | KeyCode::Char('W') => Act::Up,
            KeyCode::Char('s') | KeyCode::Char('S') => Act::Down,
            KeyCode::Char(' ') => Act::Attack,
            KeyCode::Char('x') | KeyCode::Char('X') => Act::Attack,
            KeyCode::Char('z') | KeyCode::Char('Z') => Act::Parry,
            KeyCode::Char('t') | KeyCode::Char('T') => Act::Throw,
            KeyCode::Char('f') | KeyCode::Char('F') => Act::Cast,
            KeyCode::Char('c') | KeyCode::Char('C') => Act::Sheathe,
            KeyCode::Char('e') | KeyCode::Char('E') => Act::Careful,
            KeyCode::Char('.') => Act::Careful,
            _ => return None,
        };
        // Shift + a direction is the careful step; report both so the movement
        // code can tell "walk carefully left" from "run left".
        let _ = m;
        Some(a)
    }

    pub fn feed(&mut self, ev: KeyEvent) {
        let press = matches!(ev.kind, KeyEventKind::Press | KeyEventKind::Repeat);
        let release = matches!(ev.kind, KeyEventKind::Release);

        // Commands first — they never participate in the hold logic.
        if press {
            let cmd = match ev.code {
                KeyCode::Esc => Some(Cmd::Back),
                KeyCode::Char('q') | KeyCode::Char('Q') => Some(Cmd::Quit),
                KeyCode::Char('p') | KeyCode::Char('P') => Some(Cmd::Pause),
                KeyCode::Char('r') | KeyCode::Char('R') => Some(Cmd::Restart),
                KeyCode::F(1) | KeyCode::Char('?') => Some(Cmd::Help),
                KeyCode::Tab => Some(Cmd::NextWeapon),
                KeyCode::Enter => Some(Cmd::Confirm),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Cmd::Skip),
                _ => None,
            };
            if let Some(c) = cmd {
                self.cmds.push(c);
            }
            match ev.code {
                KeyCode::Up => self.cmds.push(Cmd::Up),
                KeyCode::Down => self.cmds.push(Cmd::Down),
                _ => {}
            }
        }

        if ev.modifiers.contains(KeyModifiers::SHIFT) && press {
            let s = slot(Act::Careful);
            self.held[s] = true;
            self.last[s] = Some(Instant::now());
        }
        if ev.modifiers.contains(KeyModifiers::SHIFT) && release && self.enhanced {
            self.held[slot(Act::Careful)] = false;
        }

        if let Some(a) = Self::map(ev.code, ev.modifiers) {
            let s = slot(a);
            if press {
                if !self.held[s] {
                    self.edge[s] = true;
                }
                self.held[s] = true;
                self.last[s] = Some(Instant::now());
            } else if release && self.enhanced {
                self.held[s] = false;
            }
        }
    }

    /// Collapse the raw key state into this frame's [`Input`].
    pub fn poll(&mut self) -> Input {
        if !self.enhanced {
            let now = Instant::now();
            for s in 0..NACT {
                if self.held[s] {
                    if let Some(t) = self.last[s] {
                        if now.duration_since(t) > HOLD_FALLBACK {
                            self.held[s] = false;
                        }
                    }
                }
            }
        }
        let h = |a: Act| self.held[slot(a)];
        let e = |a: Act| self.edge[slot(a)];
        let inp = Input {
            left: h(Act::Left),
            right: h(Act::Right),
            up: h(Act::Up),
            down: h(Act::Down),
            careful: h(Act::Careful),
            attack: e(Act::Attack),
            parry: h(Act::Parry),
            throw: e(Act::Throw),
            cast: e(Act::Cast),
            sheathe: e(Act::Sheathe),
            up_edge: e(Act::Up),
            down_edge: e(Act::Down),
            left_edge: e(Act::Left),
            right_edge: e(Act::Right),
        };
        self.edge = [false; NACT];
        inp
    }

    pub fn take_cmds(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut self.cmds)
    }

    /// Forget every held key — used when the game is paused or a menu opens.
    pub fn release_all(&mut self) {
        self.held = [false; NACT];
        self.edge = [false; NACT];
        self.last = [None; NACT];
    }
}
