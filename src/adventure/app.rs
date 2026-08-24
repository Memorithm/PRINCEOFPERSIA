//! Terminal application for the adventure: setup, frame loop, HUD and menus.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::{cursor, execute, terminal};

use crate::adventure::render;
use crate::adventure::world::{TILE, WORLD_DEFS};
use crate::adventure::{Game, Phase};
use crate::gfx::canvas::{Canvas, LightField};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::layer::Layer;
use crate::gfx::term::{Screen, A_BOLD, A_DIM};
use crate::input::{Cmd, Input, Reader};

const FRAME_60: Duration = Duration::from_micros(16_667);
/// Fixed simulation step, as in the classic engine.
const SIM_DT: f32 = 1.0 / 120.0;
const MAX_CATCHUP: f32 = 0.25;
/// View heights in tiles, per zoom step. The default (8 tiles) keeps the
/// prince at a readable size even on a 30-row terminal; `+` widens the view.
const VIEWS_TALL: [f32; 4] = [8.0, 10.0, 13.0, 16.0];
const MIN_COLS: i32 = 56;
const MIN_ROWS: i32 = 14;

pub const HUD_TOP: i32 = 1;
pub const HUD_BOTTOM: i32 = 1;

const PANEL_BG: Rgb = rgb(16, 13, 22);
const PANEL_EDGE: Rgb = rgb(150, 122, 62);
const GOLD: Rgb = rgb(232, 194, 100);
const TEXT: Rgb = rgb(214, 206, 190);
const DIMTEXT: Rgb = rgb(126, 118, 128);

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Title,
    Play,
    Paused,
    Help,
    Dead,
    Victory,
    Quit,
}

pub struct App {
    screen: Screen,
    canvas: Canvas,
    layer: Layer,
    light: LightField,
    reader: Reader,
    game: Game,
    state: State,
    ss: f32,
    menu_sel: usize,
    total_time: f32,
    seed: u64,
    zoom_ix: usize,
    acc: f32,
    frame: Duration,
    /// Death fade-out timer.
    dead_t: f32,
}

impl App {
    pub fn new(seed: u64) -> io::Result<App> {
        let (c, r) = terminal::size().unwrap_or((100, 30));
        let game = Game::new(seed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let enhanced = event::poll(Duration::ZERO).is_ok()
            && terminal::supports_keyboard_enhancement().unwrap_or(false);
        Ok(App {
            screen: Screen::new(c as i32, r as i32),
            canvas: Canvas::new(320, 160),
            layer: Layer::new(),
            light: LightField::new(),
            reader: Reader::new(enhanced),
            game,
            state: State::Title,
            ss: 2.0,
            menu_sel: 0,
            total_time: 0.0,
            seed,
            zoom_ix: 0,
            acc: 0.0,
            frame: FRAME_60,
            dead_t: 0.0,
        })
    }

    fn viewport_rows(&self) -> i32 {
        (self.screen.rows - HUD_TOP - HUD_BOTTOM).max(2)
    }

    fn layout(&mut self) {
        let pw = self.screen.cols.max(1);
        let ph = (self.viewport_rows() * 2).max(2);
        let aspect = pw as f32 / ph as f32;
        let vh = TILE * VIEWS_TALL[self.zoom_ix.min(VIEWS_TALL.len() - 1)];
        let vw = vh * aspect;
        self.game.set_view_size(vw, vh);
        let mut ss = (pw as f32 / vw * 1.4).clamp(2.0, 4.0);
        while vw * ss * vh * ss > 620_000.0 && ss > 1.0 {
            ss -= 0.25;
        }
        self.ss = ss;
        self.frame = self.frame_budget();
        self.canvas
            .resize((vw * ss).round() as i32, (vh * ss).round() as i32);
    }

    fn frame_budget(&self) -> Duration {
        let cells = self.screen.cols as i64 * self.screen.rows as i64;
        let fps = if cells <= 5_000 {
            60
        } else if cells <= 12_000 {
            45
        } else {
            30
        };
        Duration::from_micros(1_000_000 / fps)
    }

    // ---------------------------------------------------------------- loop

    pub fn run(&mut self) -> io::Result<()> {
        let mut out = io::stdout();
        let mut last = Instant::now();
        self.layout();

        while self.state != State::Quit {
            // ---- input ----------------------------------------------------
            let mut guard = 0;
            while event::poll(Duration::ZERO)? && guard < 64 {
                guard += 1;
                match event::read()? {
                    Event::Key(k) => self.reader.feed(k),
                    Event::Resize(c, r) => {
                        self.screen.resize(c as i32, r as i32);
                        self.layout();
                    }
                    Event::FocusLost => self.reader.release_all(),
                    _ => {}
                }
            }
            let cmds = self.reader.take_cmds();
            let inp = self.reader.poll();
            self.handle_cmds(&cmds);

            // ---- update ---------------------------------------------------
            let now = Instant::now();
            let dt = (now - last).as_secs_f32().min(0.1);
            last = now;
            self.acc = (self.acc + dt).min(MAX_CATCHUP);
            let live = self.state == State::Play;
            let mut first = if live { inp } else { Input::default() };
            let mut rest = first;
            rest.attack = false;
            if live {
                self.total_time += dt;
                if matches!(self.game.phase, Phase::Dying(_)) {
                    self.dead_t += dt;
                } else {
                    self.dead_t = 0.0;
                }
            }
            while self.acc >= SIM_DT {
                self.game.update(SIM_DT, &first);
                first = rest;
                self.acc -= SIM_DT;
            }
            if live {
                self.check_phase();
            }

            // ---- draw -----------------------------------------------------
            self.draw();
            self.screen.flush(&mut out)?;

            let spent = Instant::now() - now;
            if spent < self.frame {
                std::thread::sleep(self.frame - spent);
            }
        }
        Ok(())
    }

    fn handle_cmds(&mut self, cmds: &[Cmd]) {
        for c in cmds {
            match (self.state, c) {
                (_, Cmd::Quit) => self.state = State::Quit,
                (State::Title, Cmd::Up) | (State::Title, Cmd::Down) => {
                    self.menu_sel = (self.menu_sel + 1) % 3;
                }
                (State::Title, Cmd::Confirm) => match self.menu_sel {
                    0 => {
                        let seed = self.seed;
                        if let Ok(g) = Game::new(seed) {
                            self.game = g;
                            self.layout();
                            self.state = State::Play;
                            self.total_time = 0.0;
                        }
                    }
                    1 => self.state = State::Help,
                    _ => self.state = State::Quit,
                },
                (State::Help, _) => {
                    self.state = if self.total_time > 0.5 {
                        State::Paused
                    } else {
                        State::Title
                    };
                }
                (State::Play, Cmd::Pause) | (State::Play, Cmd::Back) => {
                    self.reader.release_all();
                    self.state = State::Paused;
                }
                (_, Cmd::ZoomIn) => self.set_zoom(self.zoom_ix + 1),
                (_, Cmd::ZoomOut) => self.set_zoom(self.zoom_ix.saturating_sub(1)),
                (State::Paused, Cmd::Pause) | (State::Paused, Cmd::Confirm) => {
                    self.state = State::Play
                }
                (State::Paused, Cmd::Restart) => {
                    self.game.respawn();
                    self.state = State::Play;
                }
                (State::Paused, Cmd::Back) => self.state = State::Title,
                (State::Dead, Cmd::Restart) | (State::Dead, Cmd::Confirm) => {
                    self.game.respawn();
                    self.state = State::Play;
                }
                (State::Victory, _) => self.state = State::Title,
                _ => {}
            }
        }
    }

    fn set_zoom(&mut self, ix: usize) {
        let ix = ix.min(VIEWS_TALL.len() - 1);
        if ix == self.zoom_ix {
            return;
        }
        self.zoom_ix = ix;
        self.layout();
    }

    fn check_phase(&mut self) {
        match self.game.phase {
            Phase::Dead => {
                self.reader.release_all();
                self.state = State::Dead;
            }
            Phase::Victory => {
                self.reader.release_all();
                self.state = State::Victory;
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------- drawing

    fn draw(&mut self) {
        if self.screen.cols < MIN_COLS || self.screen.rows < MIN_ROWS {
            self.screen.clear(rgb(12, 10, 16));
            let msg = format!(
                "Fenêtre trop petite : {}x{}. Il faut au moins {}x{}.",
                self.screen.cols, self.screen.rows, MIN_COLS, MIN_ROWS
            );
            self.screen.text_centred(
                self.screen.rows / 2,
                &msg,
                rgb(230, 120, 100),
                rgb(12, 10, 16),
                A_BOLD,
            );
            return;
        }

        render::draw(
            &self.game,
            &mut self.canvas,
            &mut self.layer,
            &mut self.light,
            self.ss,
        );
        let vrows = self.viewport_rows();

        // Death fade: darken the world view progressively.
        if matches!(self.game.phase, Phase::Dying(_)) {
            let k = (self.dead_t / 1.2).clamp(0.0, 1.0);
            for px in self.canvas.px.iter_mut() {
                *px = px.lerp(rgb(30, 8, 8), k * 0.9);
            }
        }

        self.screen
            .blit(&self.canvas, 0, HUD_TOP, self.screen.cols, vrows);
        self.draw_hud();

        match self.state {
            State::Title => self.draw_title(),
            State::Paused => self.draw_paused(),
            State::Help => self.draw_help(),
            State::Dead => self.draw_dead(),
            State::Victory => self.draw_victory(),
            _ => {}
        }
    }

    fn draw_hud(&mut self) {
        let g = &self.game;
        self.screen.fill_row(0, rgb(20, 15, 24));
        self.screen.fill_row(self.screen.rows - 1, rgb(20, 15, 24));

        // Hearts.
        let full = g.inv.hp / 2;
        let half = g.inv.hp % 2;
        let hearts = g.inv.hp_max / 2;
        let mut x = 1;
        for i in 0..hearts {
            let (ch, col) = if i < full {
                ('♥', rgb(226, 60, 66))
            } else if i == full && half == 1 {
                ('♥', rgb(140, 44, 52))
            } else {
                ('♡', rgb(96, 70, 76))
            };
            x += self.screen.text(x, 0, &ch.to_string(), col, rgb(20, 15, 24)) + 0;
        }
        x += 1;
        // Gems.
        let s = format!("◆{}", g.inv.gems);
        x += self.screen.text(x, 0, &s, rgb(90, 210, 220), rgb(20, 15, 24));
        // Keys.
        let s = format!("  ⚿{}", g.inv.keys);
        x += self.screen.text(x, 0, &s, GOLD, rgb(20, 15, 24));
        // Seals.
        let s = format!("  ◉{}/4", g.inv.seals);
        x += self.screen.text(x, 0, &s, rgb(240, 200, 90), rgb(20, 15, 24));
        if g.inv.mirror {
            let s = "  ✦miroir";
            x += self.screen.text(x, 0, &s, rgb(190, 215, 255), rgb(20, 15, 24));
        }
        if g.inv.silver_sword {
            let s = "  ✦argent";
            x += self.screen.text(x, 0, &s, SILVER_TXT, rgb(20, 15, 24));
        }
        // World name, right-aligned.
        let name = g.world().name;
        let w = name.chars().count() as i32;
        self.screen.text(
            (self.screen.cols - w - 1).max(x + 1),
            0,
            name,
            TEXT,
            rgb(20, 15, 24),
        );

        // Bottom: message or controls hint.
        let bottom = self.screen.rows - 1;
        if let Some((msg, _, warn)) = &g.msg {
            let col = if *warn {
                rgb(236, 130, 90)
            } else {
                rgb(240, 230, 200)
            };
            self.screen.text_over(1, bottom, msg, col, A_BOLD);
        } else if self.state == State::Play {
            let hint = "←↑↓→ marcher   Espace frapper   P pause   +/- vue";
            self.screen
                .text_over(1, bottom, hint, rgb(110, 100, 116), A_DIM);
        }
    }

    fn panel(&mut self, w: i32, h: i32) -> (i32, i32) {
        let x = (self.screen.cols - w) / 2;
        let y = (self.screen.rows - h) / 2;
        for row in 0..h {
            for col in 0..w {
                let edge = row == 0 || row == h - 1 || col == 0 || col == w - 1;
                let ch = if edge {
                    match (row, col) {
                        (0, 0) => '╔',
                        (0, c) if c == w - 1 => '╗',
                        (r, 0) if r == h - 1 => '╚',
                        (r, c) if r == h - 1 && c == w - 1 => '╝',
                        _ if row == 0 || row == h - 1 => '═',
                        _ => '║',
                    }
                } else {
                    ' '
                };
                self.screen.set(
                    x + col,
                    y + row,
                    crate::gfx::term::Cell {
                        ch,
                        fg: PANEL_EDGE,
                        bg: PANEL_BG,
                        attr: 0,
                    },
                );
            }
        }
        (x, y)
    }

    fn draw_title(&mut self) {
        let w = 56.min(self.screen.cols - 2);
        let h = 12.min(self.screen.rows - 2);
        let (x, y) = self.panel(w, h);
        self.screen
            .text_attr(x + 4, y + 1, "PRINCE OF PERSIA", GOLD, PANEL_BG, A_BOLD);
        self.screen.text_attr(
            x + 4,
            y + 2,
            "la légende du miroir d'argent",
            rgb(180, 160, 200),
            PANEL_BG,
            A_DIM,
        );
        let items = [
            "Nouvelle partie",
            "Commandes",
            "Quitter",
        ];
        for (i, item) in items.iter().enumerate() {
            let sel = i == self.menu_sel;
            let line = format!(
                "{} {}",
                if sel { "▶" } else { " " },
                item
            );
            self.screen.text_attr(
                x + 6,
                y + 4 + i as i32,
                &line,
                if sel { GOLD } else { TEXT },
                PANEL_BG,
                if sel { A_BOLD } else { 0 },
            );
        }
        self.screen.text_attr(
            x + 4,
            y + 8,
            "↑↓ choisir   Entrée valider   Q quitter",
            DIMTEXT,
            PANEL_BG,
            A_DIM,
        );
        self.screen.text_attr(
            x + 4,
            y + 10,
            "quinze mondes t'attendent, prince…",
            rgb(120, 150, 130),
            PANEL_BG,
            A_DIM,
        );
    }

    fn draw_paused(&mut self) {
        let w = 40.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 7);
        self.screen
            .text_attr(x + 2, y + 1, "PAUSE", GOLD, PANEL_BG, A_BOLD);
        self.screen
            .text_attr(x + 2, y + 3, "P / Entrée  reprendre", TEXT, PANEL_BG, 0);
        self.screen
            .text_attr(x + 2, y + 4, "R  reprendre depuis le départ", TEXT, PANEL_BG, 0);
        self.screen
            .text_attr(x + 2, y + 5, "Échap  menu      Q  quitter", DIMTEXT, PANEL_BG, A_DIM);
    }

    fn draw_help(&mut self) {
        let lines: [(&str, &str); 12] = [
            ("← ↑ ↓ →", "marcher (diagonales comprises)"),
            ("Espace / X", "frapper à l'épée"),
            ("Portes D", "consomment une petite clé"),
            ("Porte L", "le sceau brisé (4 fragments) l'ouvre"),
            ("Dalles M", "avec le Miroir d'argent : changer de monde"),
            ("Puits, grottes,", "portails vers d'autres mondes"),
            ("Gardes noirs", "leur bouclier pare les coups de face"),
            ("♥ ♡ ◆ ⚿ ◉", "vie, gemmes, clés, sceaux"),
            ("+ / -", "rapprocher / éloigner la vue"),
            ("P / Échap", "pause"),
            ("R", "recommencer le monde courant"),
            ("Q", "quitter"),
        ];
        let w = 58.min(self.screen.cols - 2);
        let h = (lines.len() as i32 + 6).min(self.screen.rows - 2);
        let (x, y) = self.panel(w, h);
        self.screen
            .text_attr(x + 2, y + 1, "COMMANDES", GOLD, PANEL_BG, A_BOLD);
        let mut r = y + 3;
        for (k, d) in lines.iter() {
            if r >= y + h - 2 {
                break;
            }
            self.screen
                .text_attr(x + 2, r, k, rgb(180, 200, 230), PANEL_BG, A_BOLD);
            self.screen.text_attr(x + 16, r, d, TEXT, PANEL_BG, 0);
            r += 1;
        }
        self.screen.text_attr(
            x + 2,
            y + h - 2,
            "une touche pour revenir",
            DIMTEXT,
            PANEL_BG,
            A_DIM,
        );
    }

    fn draw_dead(&mut self) {
        let w = 46.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 8);
        self.screen
            .text_attr(x + 2, y + 1, "TU ES MORT", rgb(226, 84, 78), PANEL_BG, A_BOLD);
        let s = format!(
            "Game over… {} mort(s), {} gemmes conservées.",
            self.game.deaths, self.game.inv.gems
        );
        self.screen.text_attr(x + 2, y + 3, &s, TEXT, PANEL_BG, 0);
        self.screen.text_attr(
            x + 2,
            y + 5,
            "Entrée / R  réessayer      Échap  menu",
            GOLD,
            PANEL_BG,
            0,
        );
    }

    fn draw_victory(&mut self) {
        let w = 56.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 11);
        self.screen
            .text_attr(x + 2, y + 1, "LE SCEAU EST BRISÉ", GOLD, PANEL_BG, A_BOLD);
        self.screen.text_attr(
            x + 2,
            y + 3,
            "Ganar n'est plus. Zahra est libre, la vallée fleurit à nouveau.",
            TEXT,
            PANEL_BG,
            0,
        );
        let s = format!(
            "{} mondes traversés — {} ennemis — {} morts",
            WORLD_DEFS.len(),
            self.game.kills,
            self.game.deaths
        );
        self.screen.text_attr(x + 2, y + 5, &s, DIMTEXT, PANEL_BG, A_DIM);
        self.screen
            .text_attr(x + 2, y + 8, "une touche pour revenir au menu", GOLD, PANEL_BG, 0);
    }
}

const SILVER_TXT: Rgb = rgb(220, 232, 250);

// ---------------------------------------------------------------- entry point

pub fn play(seed: u64) -> io::Result<()> {
    let mut out = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        let _ = execute!(
            out,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        );
    }

    // Make sure a panic cannot leave the terminal in raw mode.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut o = io::stdout();
        let _ = execute!(o, terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
        let _ = writeln!(o);
        prev(info);
    }));

    let result = (|| -> io::Result<()> {
        let mut app = App::new(seed)?;
        app.reader.enhanced = enhanced;
        app.run()
    })();

    if enhanced {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    execute!(out, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    result
}
