//! Terminal application: setup, the frame loop, menus and overlays.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::{cursor, execute, terminal};

use crate::game::hud::fmt_time;
use crate::game::{Carry, Game, Phase};
use crate::gfx::canvas::{Canvas, LightField};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::layer::Layer;
use crate::gfx::term::{Screen, A_BOLD, A_DIM};
use crate::input::{Cmd, Input, Reader};
use crate::world::levels::{self, CAMPAIGN};
use crate::world::tile::{ROOM_H, ROOM_W};

const FRAME: Duration = Duration::from_micros(33_333); // ~30 fps
const MIN_COLS: i32 = 56;
const MIN_ROWS: i32 = 14;

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
    LevelDone,
    Victory,
    TimeUp,
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
}

impl App {
    pub fn new(level: usize, seed: u64) -> io::Result<App> {
        let (c, r) = terminal::size().unwrap_or((100, 30));
        let game = Game::new(level, Carry::default(), seed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let enhanced = event::poll(Duration::ZERO).is_ok()
            && terminal::supports_keyboard_enhancement().unwrap_or(false);
        Ok(App {
            screen: Screen::new(c as i32, r as i32),
            canvas: Canvas::new(320, 120),
            layer: Layer::new(),
            light: LightField::new(),
            reader: Reader::new(enhanced),
            game,
            state: State::Title,
            ss: 2.0,
            menu_sel: level,
            total_time: 0.0,
            seed,
        })
    }

    fn viewport_rows(&self) -> i32 {
        (self.screen.rows - Game::TOP_ROWS - Game::BOTTOM_ROWS).max(2)
    }

    /// Recompute the world view and canvas size for the current terminal.
    fn layout(&mut self) {
        let pw = self.screen.cols.max(1);
        let ph = (self.viewport_rows() * 2).max(2);
        let aspect = pw as f32 / ph as f32;
        let room_aspect = ROOM_W / ROOM_H;
        // Always show at least a whole room; use any spare screen for a peek
        // into the neighbouring rooms rather than letterboxing.
        let (vw, vh) = if aspect > room_aspect {
            (ROOM_H * aspect, ROOM_H)
        } else {
            (ROOM_W, ROOM_W / aspect)
        };
        self.game.set_view_size(vw, vh);
        let mut ss = (pw as f32 / vw * 1.4).clamp(2.0, 3.5);
        while vw * ss * vh * ss > 430_000.0 && ss > 1.0 {
            ss -= 0.25;
        }
        self.ss = ss;
        self.canvas
            .resize((vw * ss).round() as i32, (vh * ss).round() as i32);
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
            match self.state {
                State::Play => {
                    self.total_time += dt;
                    self.game.update(dt, &inp);
                    self.check_phase();
                }
                State::Title | State::Help => {
                    // Keep the scene alive behind the menu.
                    self.game.update(dt, &Input::default());
                    if self.game.phase != Phase::Play {
                        self.game.phase = Phase::Play;
                    }
                }
                _ => {
                    self.game.update(dt, &Input::default());
                }
            }

            // ---- draw -----------------------------------------------------
            self.draw();
            self.screen.flush(&mut out)?;

            let spent = Instant::now() - now;
            if spent < FRAME {
                std::thread::sleep(FRAME - spent);
            }
        }
        Ok(())
    }

    fn handle_cmds(&mut self, cmds: &[Cmd]) {
        for c in cmds {
            match (self.state, c) {
                (_, Cmd::Quit) => self.state = State::Quit,
                (State::Title, Cmd::Up) => {
                    self.menu_sel = self.menu_sel.saturating_sub(1);
                    self.preview();
                }
                (State::Title, Cmd::Down) => {
                    self.menu_sel = (self.menu_sel + 1).min(CAMPAIGN.len() - 1);
                    self.preview();
                }
                (State::Title, Cmd::Confirm) => {
                    self.start_level(self.menu_sel);
                }
                (State::Title, Cmd::Help) => self.state = State::Help,
                (State::Title, Cmd::Back) => self.state = State::Quit,
                (State::Help, _) => {
                    self.state = if self.game.elapsed > 0.5 && self.total_time > 0.0 {
                        State::Paused
                    } else {
                        State::Title
                    };
                }
                (State::Play, Cmd::Pause) | (State::Play, Cmd::Back) => {
                    self.reader.release_all();
                    self.state = State::Paused;
                }
                (State::Play, Cmd::Help) => {
                    self.reader.release_all();
                    self.state = State::Help;
                }
                (State::Play, Cmd::Restart) => {
                    self.game.restart();
                }
                (State::Play, Cmd::NextWeapon) => self.cycle_weapon(),
                (State::Paused, Cmd::Pause) | (State::Paused, Cmd::Confirm) => {
                    self.state = State::Play
                }
                (State::Paused, Cmd::Back) => self.state = State::Title,
                (State::Paused, Cmd::Restart) => {
                    self.game.restart();
                    self.state = State::Play;
                }
                (State::Paused, Cmd::Help) => self.state = State::Help,
                (State::Dead, Cmd::Restart) | (State::Dead, Cmd::Confirm) => {
                    self.game.restart();
                    self.state = State::Play;
                }
                (State::Dead, Cmd::Back) | (State::TimeUp, Cmd::Back) => self.state = State::Title,
                (State::TimeUp, Cmd::Restart) | (State::TimeUp, Cmd::Confirm) => {
                    self.game.restart();
                    self.state = State::Play;
                }
                (State::LevelDone, Cmd::Confirm) | (State::LevelDone, Cmd::Skip) => {
                    self.next_level();
                }
                (State::Victory, _) => self.state = State::Title,
                _ => {}
            }
        }
    }

    fn cycle_weapon(&mut self) {
        let pl = &mut self.game.player;
        let has_both = pl.sword && pl.scimitar;
        if has_both {
            pl.melee = if pl.melee == crate::game::Melee::Sword {
                crate::game::Melee::Scimitar
            } else {
                crate::game::Melee::Sword
            };
            let l = pl.melee.label();
            self.game.say(&format!("Arme : {}", l), 1.6, false);
        }
    }

    fn preview(&mut self) {
        let seed = self.seed ^ (self.menu_sel as u64 * 0x9E37);
        if let Ok(g) = Game::new(self.menu_sel, Carry::default(), seed) {
            self.game = g;
            self.layout();
        }
    }

    fn start_level(&mut self, idx: usize) {
        let seed = self.seed ^ (idx as u64 * 0x51ED);
        if let Ok(g) = Game::new(idx, Carry::default(), seed) {
            self.game = g;
            self.layout();
            self.state = State::Play;
            self.total_time = 0.0;
        }
    }

    fn next_level(&mut self) {
        let carry = self.game.player.carry();
        let idx = self.game.idx + 1;
        if idx >= levels::count() {
            self.state = State::Victory;
            return;
        }
        let deaths = self.game.deaths;
        let kills = self.game.kills;
        let seed = self.game.rng.next_u64();
        if let Ok(mut g) = Game::new(idx, carry, seed) {
            g.deaths = deaths;
            g.kills = kills;
            self.game = g;
            self.layout();
            self.menu_sel = idx;
            self.state = State::Play;
        }
    }

    fn check_phase(&mut self) {
        match self.game.phase {
            Phase::Dead => {
                self.reader.release_all();
                self.state = State::Dead;
            }
            Phase::LevelDone => {
                self.reader.release_all();
                self.state = State::LevelDone;
            }
            Phase::TimeUp => {
                self.reader.release_all();
                self.state = State::TimeUp;
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
            self.screen
                .text_centred(self.screen.rows / 2, &msg, rgb(230, 120, 100), rgb(12, 10, 16), A_BOLD);
            return;
        }

        self.game
            .draw(&mut self.canvas, &mut self.layer, &mut self.light, self.ss);
        let vrows = self.viewport_rows();
        self.screen
            .blit(&self.canvas, 0, Game::TOP_ROWS, self.screen.cols, vrows);
        self.game.draw_hud(&mut self.screen);

        match self.state {
            State::Title => self.draw_title(),
            State::Paused => self.draw_paused(),
            State::Help => self.draw_help(),
            State::Dead => self.draw_dead(),
            State::LevelDone => self.draw_level_done(),
            State::Victory => self.draw_victory(),
            State::TimeUp => self.draw_time_up(),
            State::Play | State::Quit => {}
        }
    }

    /// A bordered, filled panel centred on the viewport.
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
                        (0, _) | (_, _) if row == 0 || row == h - 1 => '═',
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
        let w = 60.min(self.screen.cols - 2);
        let h = (12 + CAMPAIGN.len() as i32).min(self.screen.rows - 2);
        let (x, y) = self.panel(w, h);
        let mut r = y + 1;
        self.screen.text_attr(
            x + 2,
            r,
            "PRINCE  OF  PERSIA",
            GOLD,
            PANEL_BG,
            A_BOLD,
        );
        r += 1;
        self.screen.text_attr(
            x + 2,
            r,
            "une réécriture pour le terminal, en Rust",
            DIMTEXT,
            PANEL_BG,
            A_DIM,
        );
        r += 2;
        self.screen
            .text_attr(x + 2, r, "Choisis un niveau :", TEXT, PANEL_BG, 0);
        r += 1;
        for (i, d) in CAMPAIGN.iter().enumerate() {
            let sel = i == self.menu_sel;
            let rooms = crate::world::level::Level::parse(d)
                .map(|l| format!("{}x{}", l.rw, l.rh))
                .unwrap_or_default();
            let line = format!(
                "{} {}. {:<26} {:>5} salles",
                if sel { "▶" } else { " " },
                i + 1,
                d.name,
                rooms
            );
            self.screen.text_attr(
                x + 2,
                r,
                &line,
                if sel { GOLD } else { TEXT },
                PANEL_BG,
                if sel { A_BOLD } else { 0 },
            );
            r += 1;
        }
        r += 1;
        self.screen.text_attr(
            x + 2,
            r,
            "↑↓ choisir    Entrée jouer    F1 commandes    Q quitter",
            DIMTEXT,
            PANEL_BG,
            A_DIM,
        );
        r += 1;
        let mode = if self.reader.enhanced {
            "contrôle précis (protocole clavier détecté)"
        } else {
            "contrôle par maintien — Maj+direction pour un pas prudent"
        };
        self.screen.text_attr(x + 2, r, mode, rgb(120, 150, 130), PANEL_BG, A_DIM);
    }

    fn draw_paused(&mut self) {
        let w = 40.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 7);
        self.screen
            .text_attr(x + 2, y + 1, "PAUSE", GOLD, PANEL_BG, A_BOLD);
        self.screen.text_attr(
            x + 2,
            y + 3,
            "P / Entrée  reprendre",
            TEXT,
            PANEL_BG,
            0,
        );
        self.screen
            .text_attr(x + 2, y + 4, "R  recommencer   F1  aide", TEXT, PANEL_BG, 0);
        self.screen
            .text_attr(x + 2, y + 5, "Échap  menu      Q  quitter", DIMTEXT, PANEL_BG, A_DIM);
    }

    fn draw_help(&mut self) {
        let lines: [(&str, &str); 14] = [
            ("← →", "courir ; deux fois pour repartir"),
            ("Maj + ← →", "pas prudent : s'arrête au bord"),
            ("↑", "sauter ; se hisser sur la corniche devant soi"),
            ("← → puis ↑", "saut en course : franchit 3 dalles"),
            ("↓", "s'accroupir ; Maj+↓ descendre en se suspendant"),
            ("↑ (suspendu)", "se hisser        ↓ lâcher prise"),
            ("Espace / X", "frapper (dégaine l'épée si besoin)"),
            ("Z", "parer — le bouclier pare aussi tout seul"),
            ("T", "lancer une dague"),
            ("F", "bâton de flamme"),
            ("C", "rengainer (indispensable face à l'Ombre)"),
            ("Tab", "changer d'arme de corps à corps"),
            ("P / Échap", "pause      R recommencer le niveau"),
            ("Q", "quitter"),
        ];
        let w = 62.min(self.screen.cols - 2);
        let h = (lines.len() as i32 + 6).min(self.screen.rows - 2);
        let (x, y) = self.panel(w, h);
        self.screen
            .text_attr(x + 2, y + 1, "COMMANDES", GOLD, PANEL_BG, A_BOLD);
        let mut r = y + 3;
        for (k, d) in lines.iter() {
            if r >= y + h - 2 {
                break;
            }
            self.screen.text_attr(x + 2, r, k, rgb(180, 200, 230), PANEL_BG, A_BOLD);
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
        let cause = self.game.player.cause.unwrap_or("Le palais a eu raison de toi.");
        self.screen.text_attr(x + 2, y + 3, cause, TEXT, PANEL_BG, 0);
        let s = format!("morts : {}", self.game.deaths);
        self.screen.text_attr(x + 2, y + 4, &s, DIMTEXT, PANEL_BG, A_DIM);
        self.screen.text_attr(
            x + 2,
            y + 6,
            "Entrée / R  recommencer      Échap  menu",
            GOLD,
            PANEL_BG,
            0,
        );
    }

    fn draw_time_up(&mut self) {
        let w = 44.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 7);
        self.screen
            .text_attr(x + 2, y + 1, "LE TEMPS EST ÉCOULÉ", rgb(226, 150, 78), PANEL_BG, A_BOLD);
        self.screen.text_attr(
            x + 2,
            y + 3,
            "La princesse n'attendra pas davantage.",
            TEXT,
            PANEL_BG,
            0,
        );
        self.screen.text_attr(
            x + 2,
            y + 5,
            "Entrée  recommencer      Échap  menu",
            GOLD,
            PANEL_BG,
            0,
        );
    }

    fn draw_level_done(&mut self) {
        let w = 50.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 9);
        self.screen
            .text_attr(x + 2, y + 1, "NIVEAU FRANCHI", GOLD, PANEL_BG, A_BOLD);
        let s = format!("{}. {}", self.game.idx + 1, self.game.lv.name);
        self.screen.text_attr(x + 2, y + 3, &s, TEXT, PANEL_BG, 0);
        let s = format!(
            "temps restant {}   gardes {}   morts {}",
            fmt_time(self.game.clock),
            self.game.kills,
            self.game.deaths
        );
        self.screen.text_attr(x + 2, y + 4, &s, DIMTEXT, PANEL_BG, A_DIM);
        let next = if self.game.idx + 1 < levels::count() {
            format!("Suivant : {}", CAMPAIGN[self.game.idx + 1].name)
        } else {
            "Dernière porte…".to_string()
        };
        self.screen.text_attr(x + 2, y + 6, &next, TEXT, PANEL_BG, 0);
        self.screen
            .text_attr(x + 2, y + 7, "Entrée  continuer", GOLD, PANEL_BG, A_BOLD);
    }

    fn draw_victory(&mut self) {
        let w = 56.min(self.screen.cols - 2);
        let (x, y) = self.panel(w, 10);
        self.screen
            .text_attr(x + 2, y + 1, "LE SABLIER EST BRISÉ", GOLD, PANEL_BG, A_BOLD);
        self.screen.text_attr(
            x + 2,
            y + 3,
            "Jaffar est tombé. Le palais respire de nouveau.",
            TEXT,
            PANEL_BG,
            0,
        );
        let s = format!(
            "six niveaux — {} gardes abattus — {} morts — {}",
            self.game.kills,
            self.game.deaths,
            fmt_time(self.total_time)
        );
        self.screen.text_attr(x + 2, y + 5, &s, DIMTEXT, PANEL_BG, A_DIM);
        self.screen
            .text_attr(x + 2, y + 8, "une touche pour revenir au menu", GOLD, PANEL_BG, 0);
    }
}

// ---------------------------------------------------------------- entry point

pub fn play(level: usize, seed: u64) -> io::Result<()> {
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
        let mut app = App::new(level, seed)?;
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
