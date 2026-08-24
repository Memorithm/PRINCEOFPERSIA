//! Le prince doit marcher : événements clavier synthétiques -> Reader -> Game.

use crossterm::event::{KeyCode, KeyEvent, KeyEventState, KeyModifiers};
use prince_of_persia_rs::adventure::Game;
use prince_of_persia_rs::input::{Reader, Act};

fn key(code: KeyCode, kind: crossterm::event::KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind,
        state: KeyEventState::empty(),
    }
}

#[test]
fn arrow_press_holds_direction_in_fallback_mode() {
    let mut r = Reader::new(false);
    r.feed(key(KeyCode::Right, crossterm::event::KeyEventKind::Press));
    let inp = r.poll();
    assert!(inp.right, "une flèche pressée doit tenir la direction");
    assert!(!inp.left);
}

#[test]
fn wasd_moves_too() {
    let mut r = Reader::new(false);
    r.feed(key(KeyCode::Char('d'), crossterm::event::KeyEventKind::Press));
    assert!(r.poll().right);
}

#[test]
fn release_clears_hold_in_enhanced_mode() {
    let mut r = Reader::new(true);
    r.feed(key(KeyCode::Right, crossterm::event::KeyEventKind::Press));
    assert!(r.poll().right);
    r.feed(key(KeyCode::Right, crossterm::event::KeyEventKind::Release));
    assert!(!r.poll().right);
}

#[test]
fn the_prince_walks_right_when_right_is_held() {
    let mut g = Game::new(7).expect("mondes valides");
    let start = g.player.p;
    let mut inp = prince_of_persia_rs::input::Input::default();
    inp.right = true;
    for _ in 0..120 {
        g.update(1.0 / 120.0, &inp);
    }
    let dx = g.player.p.x - start.x;
    assert!(
        dx > 40.0,
        "le prince doit avancer vers la droite, dx = {dx}"
    );
}

#[test]
fn the_prince_walks_all_eight_directions() {
    let dirs: [(&str, bool, bool, bool, bool); 8] = [
        ("droite", false, true, false, false),
        ("gauche", true, false, false, false),
        ("haut", false, false, false, true),
        ("bas", false, false, true, false),
        ("diag HD", false, true, false, true),
        ("diag HG", true, false, false, true),
        ("diag BD", false, true, true, false),
        ("diag BG", true, false, true, false),
    ];
    for (name, l, r, d, u) in dirs {
        let mut g = Game::new(7).expect("mondes valides");
        let start = g.player.p;
        let mut inp = prince_of_persia_rs::input::Input::default();
        inp.left = l;
        inp.right = r;
        inp.down = d;
        inp.up = u;
        for _ in 0..60 {
            g.update(1.0 / 120.0, &inp);
        }
        let moved = (g.player.p.x - start.x).abs() + (g.player.p.y - start.y).abs();
        assert!(moved > 5.0, "direction {name} : déplacement de {moved}px seulement");
    }
}

#[test]
fn reader_into_game_end_to_end() {
    // Le chemin complet : touches -> Reader -> poll() -> Game::update.
    let mut r = Reader::new(false);
    r.feed(key(KeyCode::Right, crossterm::event::KeyEventKind::Press));
    r.feed(key(KeyCode::Down, crossterm::event::KeyEventKind::Press));
    let mut g = Game::new(7).expect("mondes valides");
    let start = g.player.p;
    let inp = r.poll();
    assert!(inp.right && inp.down);
    for _ in 0..60 {
        g.update(1.0 / 120.0, &inp);
    }
    let moved = (g.player.p.x - start.x) + (g.player.p.y - start.y);
    assert!(moved > 5.0, "le prince ne bouge pas : {moved}px");
    let _ = Act::Left;
}

#[test]
fn walls_stop_the_prince() {
    // Depuis la place du village, longer le mur nord de la carte : il doit
    // rester dans le monde et ne jamais traverser la maçonnerie.
    let mut g = Game::new(3).expect("mondes valides");
    let mut inp = prince_of_persia_rs::input::Input::default();
    inp.up = true;
    for _ in 0..600 {
        g.update(1.0 / 120.0, &inp);
    }
    let w = g.world();
    let r = 7.0;
    let (tx0, tx1) = (
        ((g.player.p.x - r) / 24.0).floor() as i32,
        ((g.player.p.x + r) / 24.0).floor() as i32,
    );
    let (ty0, ty1) = (
        ((g.player.p.y - r) / 24.0).floor() as i32,
        ((g.player.p.y + r) / 24.0).floor() as i32,
    );
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            assert!(
                !w.tile(tx, ty).solid(),
                "le prince chevauche une tuile solide ({tx},{ty})"
            );
        }
    }
}

#[test]
fn walking_into_a_portal_changes_world() {
    // Le portail de la forêt (18,2) est juste au nord : après expiration de
    // la grâce de téléportation, marcher vers le haut doit changer de monde.
    let mut g = Game::new(5).expect("mondes valides");
    g.player.p = prince_of_persia_rs::util::v2(18.5 * 24.0, 4.5 * 24.0);
    let idle = prince_of_persia_rs::input::Input::default();
    for _ in 0..120 {
        g.update(1.0 / 120.0, &idle); // la grâce de warp expire
    }
    let mut inp = prince_of_persia_rs::input::Input::default();
    inp.up = true;
    for _ in 0..240 {
        g.update(1.0 / 120.0, &inp);
        if g.cur == prince_of_persia_rs::adventure::world::FOREST {
            return; // téléporté
        }
    }
    panic!("le prince n'est jamais entré dans le portail, monde courant : {}", g.cur);
}

// ------------------------------------------------------------------ combat

use prince_of_persia_rs::adventure::Foe;
use prince_of_persia_rs::adventure::world::FoeKind;
use prince_of_persia_rs::util::v2;

fn lone_foe(g: &mut Game, kind: FoeKind, offset: (f32, f32)) {
    let fp = g.player.p.add(v2(offset.0, offset.1));
    g.foes.clear();
    g.foes.push(Foe {
        kind,
        home: fp,
        p: fp,
        hp: kind.max_hp(),
        dir: v2(-1.0, 0.0),
        t: 1.0,
        cd: 9.0,
        hurt_t: 0.0,
        knock: V2_ZERO,
        alive: true,
        anim: 0.0,
    });
}

use prince_of_persia_rs::util::V2;
const V2_ZERO: prince_of_persia_rs::util::V2 = prince_of_persia_rs::util::V2 { x: 0.0, y: 0.0 };

fn face_and_swing(g: &mut Game, steps: u32) {
    // Regarde à droite et maintient l'attaque : l'auto-répétition frappe
    // dès que le cooldown le permet.
    let mut inp = prince_of_persia_rs::input::Input::default();
    inp.right = true;
    inp.attack_held = true;
    for _ in 0..steps {
        g.update(1.0 / 120.0, &inp);
    }
}

#[test]
fn one_blow_fells_a_crab() {
    let mut g = Game::new(9).expect("mondes valides");
    lone_foe(&mut g, FoeKind::Crab, (14.0, 0.0));
    face_and_swing(&mut g, 30); // un coup (0,25 s)
    assert!(!g.foes[0].alive, "un Craboroc tombe d'un coup");
}

#[test]
fn holding_attack_repeats_and_fells_a_grunt() {
    let mut g = Game::new(9).expect("mondes valides");
    lone_foe(&mut g, FoeKind::Grunt, (14.0, 0.0));
    face_and_swing(&mut g, 90); // ~3 coups en 0,75 s
    assert!(
        !g.foes[0].alive,
        "garder Espace enfoncé doit enchaîner les coups (hp restant : {})",
        g.foes[0].hp
    );
}

#[test]
fn a_blow_knocks_the_foe_back() {
    let mut g = Game::new(9).expect("mondes valides");
    lone_foe(&mut g, FoeKind::Grunt, (14.0, 0.0));
    let before = g.foes[0].p;
    // Un seul coup, sans laisser l'ennemi se rétablir.
    let mut inp = prince_of_persia_rs::input::Input::default();
    inp.right = true;
    inp.attack = true;
    g.update(1.0 / 120.0, &inp);
    for _ in 0..12 {
        g.update(1.0 / 120.0, &inp2_idle());
    }
    let pushed = g.foes[0].p.x - before.x;
    assert!(pushed > 6.0, "l'ennemi doit reculer sous le coup (+{pushed}px)");
    assert!(g.foes[0].hp < FoeKind::Grunt.max_hp(), "l'ennemi doit être blessé");
}

fn inp2_idle() -> prince_of_persia_rs::input::Input {
    prince_of_persia_rs::input::Input::default()
}
