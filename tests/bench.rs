use prince_of_persia_rs::game::{Carry, Game};
use prince_of_persia_rs::gfx::canvas::{Canvas, LightField};
use prince_of_persia_rs::gfx::layer::Layer;
use prince_of_persia_rs::gfx::term::Screen;
use prince_of_persia_rs::input::Input;
use std::time::Instant;

/// Frame budget and escape-code bandwidth for a range of terminal sizes. Run
/// with `cargo test --release --test bench -- --nocapture` to see the numbers.
#[test]
fn frame_cost() {
    for (cols, rows) in [(100i32, 30i32), (120, 32), (200, 50), (280, 70)] {
        let mut g = Game::new(0, Carry::default(), 1).unwrap();
        let vrows = rows - 3;
        let pw = cols;
        let ph = vrows * 2;
        let aspect = pw as f32 / ph as f32;
        let (vw, vh) = if aspect > 320.0 / 120.0 {
            (120.0 * aspect, 120.0)
        } else {
            (320.0, 320.0 / aspect)
        };
        g.set_view_size(vw, vh);
        let mut ss = (pw as f32 / vw * 1.4).clamp(2.0, 3.5);
        while vw * ss * vh * ss > 430_000.0 && ss > 1.0 {
            ss -= 0.25;
        }
        let mut cv = Canvas::new((vw * ss) as i32, (vh * ss) as i32);
        let mut layer = Layer::new();
        let mut light = LightField::new();
        let mut scr = Screen::new(cols, rows);
        let mut sink: Vec<u8> = Vec::with_capacity(1 << 22);
        // warm up
        for _ in 0..20 { g.update(1.0/120.0, &Input::default()); }
        let n = 60;
        let t0 = Instant::now();
        for _ in 0..n {
            // One rendered frame at 60 fps is two simulation steps at 120 Hz.
            g.update(1.0 / 120.0, &Input::default());
            g.update(1.0 / 120.0, &Input::default());
            g.draw(&mut cv, &mut layer, &mut light, ss);
            scr.blit(&cv, 0, 1, cols, vrows);
            g.draw_hud(&mut scr);
            scr.flush(&mut sink).unwrap();
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        let kbs = (sink.len() as f64 / 1024.0) / (n as f64 / 60.0);
        println!(
            "{cols}x{rows}: canvas {}x{} ss={ss:.2}  {ms:.2} ms/frame  ({:.0} fps max)  {kbs:.0} KB/s escape output",
            cv.w, cv.h, 1000.0 / ms
        );
        // The game targets 30 fps; a debug build is slower, so leave headroom.
        assert!(
            ms < 200.0,
            "{cols}x{rows}: {ms:.1} ms/frame is far too slow to hold 30 fps"
        );
    }
}
