//! Scene composition: environment, characters, effects, lighting.
//!
//! The order of passes is what gives the picture its depth:
//!
//! 1. brickwork, slabs and props at full brightness
//! 2. items and characters
//! 3. matter particles (dust, blood, rubble)
//! 4. **multiply** the whole canvas by the light field — torchlight therefore
//!    falls on floors, walls, the prince and the guards alike
//! 5. **add** everything that emits: flames, sparks, fireballs, window shafts
//! 6. vignette, dither, damage flash

use crate::art::items;
use crate::art::skel::{self, Blade, Figure, Pose, Prop, Style};
use crate::art::tiles;
use crate::game::*;
use crate::gfx::canvas::{dither, Blend, Cam, Canvas, LightField};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::layer::Layer;
use crate::gfx::target::{fill_ellipse, fill_rect};
use crate::util::{clampf, noise1, v2, Rect, V2};
use crate::world::level::Level;
use crate::world::tile::*;

/// A sword-stroke flourish, queued by the combat code and drawn for a few frames.
pub fn mark_slash(g: &mut Game, at: V2, facing: f32) {
    g.slashes.push((at, facing, 0.0));
}

impl Game {
    pub fn update_slashes(&mut self, dt: f32) {
        for s in self.slashes.iter_mut() {
            s.2 += dt * 5.0;
        }
        self.slashes.retain(|s| s.2 < 1.0);
    }

    /// Camera for this frame, including screen shake.
    fn frame_cam(&self, ss: f32) -> Cam {
        let sh = self.cam.shake;
        let ox = if sh > 0.0 {
            noise1(self.elapsed * 42.0, 3) * sh * 3.4
        } else {
            0.0
        };
        let oy = if sh > 0.0 {
            noise1(self.elapsed * 37.0, 9) * sh * 2.6
        } else {
            0.0
        };
        Cam {
            ox: self.cam.at.x + ox,
            oy: self.cam.at.y + oy,
            s: ss,
        }
    }

    pub fn draw(&self, cv: &mut Canvas, layer: &mut Layer, light: &mut LightField, ss: f32) {
        let cam = self.frame_cam(ss);
        let view = Rect::from_size(cam.ox, cam.oy, self.view_w, self.view_h);
        let th = self.lv.theme;

        // ---- 1. environment ------------------------------------------------
        cv.blend = Blend::Alpha;
        cv.fill_gradient(th.back_dk.scale(0.7), th.back_dk.scale(1.1));
        tiles::draw_environment(cv, &cam, &self.lv, &self.dy, view, self.elapsed);

        // ---- 2. items ------------------------------------------------------
        for it in &self.items {
            if it.taken {
                continue;
            }
            let bob = (self.elapsed * 2.2 + it.tx as f32).sin() * 0.7;
            items::draw_item(
                cv,
                &cam,
                it.kind,
                Level::cx(it.tx),
                Level::surf(it.ty),
                bob,
            );
        }

        // ---- 3. characters -------------------------------------------------
        for g in &self.guards {
            if g.p.y < -1000.0 {
                continue;
            }
            let alpha = if g.kind == MobKind::Shadow { 0.82 } else { 1.0 };
            if g.kind == MobKind::Shadow {
                items::draw_shadow_aura(cv, &cam, v2(g.p.x, g.p.y - 16.0), cam.l(22.0));
            }
            self.draw_figure_at(
                cv,
                layer,
                &cam,
                &g.prop(),
                &g.style(),
                &g.pose(),
                v2(g.p.x, g.p.y),
                g.facing,
                g.blade(),
                alpha,
            );
        }

        let pl = &self.player;
        let flicker = if pl.invuln > 0.0 {
            0.45 + 0.55 * ((self.elapsed * 34.0).sin() * 0.5 + 0.5)
        } else {
            1.0
        };
        let mut pstyle = Style::PRINCE;
        if pl.swift_t > 0.0 {
            pstyle.sash = rgb(240, 200, 60);
            pstyle.sash_dk = rgb(160, 120, 20);
        }
        if pl.float_t > 0.0 {
            pstyle.cloth = rgb(226, 238, 250);
            pstyle.cloth_dk = rgb(150, 178, 210);
        }
        self.draw_figure_at(
            cv,
            layer,
            &cam,
            &Prop::PRINCE.scaled(0.88, 1.0),
            &pstyle,
            &pl.pose(),
            v2(pl.p.x, pl.p.y),
            pl.facing,
            pl.blade(),
            flicker,
        );

        // ---- 4. matter particles -------------------------------------------
        self.fx.draw_matter(cv, &cam);

        // ---- 5. lighting ---------------------------------------------------
        let mut amb = th.ambient;
        if pl.float_t > 0.0 {
            amb = [amb[0] * 0.95, amb[1] * 1.02, amb[2] * 1.15];
        }
        light.begin(cv.w, cv.h, amb);
        let mut sources: Vec<(V2, f32, Rgb, f32)> = Vec::new();
        tiles::collect_lights(&self.lv, &self.dy, view, self.elapsed, &mut sources);
        for (p, r, c, i) in sources {
            light.add(cam.p(p), cam.l(r), c, i);
        }
        // Flames light their surroundings.
        {
            let mut add = |p: V2, r: f32, c: Rgb, i: f32| light.add(p, r, c, i);
            self.fx.emitter_lights(&cam, &mut add);
        }
        // Fireballs in flight.
        for s in &self.shots {
            if s.kind == ShotKind::Fireball {
                light.add(cam.p(s.p), cam.l(46.0), rgb(255, 150, 60), 1.1);
            }
        }
        // The prince is very slightly self-lit so he never vanishes into the dark.
        light.add(
            cam.p(v2(pl.p.x, pl.p.y - 16.0)),
            cam.l(30.0),
            rgb(180, 190, 220),
            0.20,
        );
        light.apply(cv);

        // ---- 6. emissive ---------------------------------------------------
        tiles::draw_emissive(cv, &cam, &self.lv, &self.dy, view, self.elapsed);
        for it in &self.items {
            if it.taken {
                continue;
            }
            let bob = (self.elapsed * 2.2 + it.tx as f32).sin() * 0.7;
            let pulse = (self.elapsed * 3.0 + it.ty as f32).sin();
            items::draw_item_glow(
                cv,
                &cam,
                it.kind,
                Level::cx(it.tx),
                Level::surf(it.ty),
                bob,
                pulse,
            );
        }
        self.fx.draw_light(cv, &cam);
        for s in &self.shots {
            match s.kind {
                ShotKind::Dagger => items::draw_dagger_flight(cv, &cam, s.p, s.spin),
                ShotKind::Fireball => items::draw_fireball(
                    cv,
                    &cam,
                    s.p,
                    s.kind.radius(),
                    (self.elapsed * 22.0).sin(),
                ),
            }
        }
        for (at, facing, t) in &self.slashes {
            items::draw_slash(cv, &cam, *at, *facing, *t);
        }

        // ---- 7. grade ------------------------------------------------------
        cv.vignette(0.52, th.vignette);
        if self.flash.0 > 0.0 {
            cv.blend = Blend::Alpha;
            let a = clampf(self.flash.0, 0.0, 1.0) * 0.4;
            fill_rect(cv, 0.0, 0.0, cv.w as f32, cv.h as f32, self.flash.1, a);
        }
        // Fade out while dying or leaving.
        let fade = match self.phase {
            Phase::Dying(t) => 1.0 - clampf(t / 2.1, 0.0, 1.0),
            Phase::Leaving(t) => 1.0 - clampf(t / 1.4, 0.0, 1.0),
            Phase::Dead | Phase::LevelDone | Phase::Victory | Phase::TimeUp => 1.0,
            _ => 0.0,
        };
        if fade > 0.001 {
            cv.blend = Blend::Alpha;
            fill_rect(
                cv,
                0.0,
                0.0,
                cv.w as f32,
                cv.h as f32,
                Rgb::BLACK,
                fade * 0.92,
            );
        }
        dither(cv, 3.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_figure_at(
        &self,
        cv: &mut Canvas,
        layer: &mut Layer,
        cam: &Cam,
        prop: &Prop,
        style: &Style,
        pose: &Pose,
        feet: V2,
        facing: f32,
        blade: Blade,
        alpha: f32,
    ) {
        let fig: Figure = skel::solve(pose, prop, feet, facing);
        // Contact shadow on the floor.
        let sw = 9.0 * prop.scale * prop.girth.max(0.8);
        fill_ellipse(
            cv,
            v2(cam.x(feet.x), cam.y(feet.y - 0.5)),
            cam.l(sw),
            cam.l(2.4),
            Rgb::BLACK,
            0.42 * alpha,
        );
        let (x0, y0, x1, y1) = skel::figure_bbox(cam, &fig, 22.0);
        // Clip the scratch layer to the canvas so huge off-screen figures are cheap.
        let x0 = x0.max(-4);
        let y0 = y0.max(-4);
        let x1 = x1.min(cv.w + 4);
        let y1 = y1.min(cv.h + 4);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        layer.begin(x0, y0, x1 - x0, y1 - y0);
        skel::draw_figure(layer, cam, &fig, style, pose, blade);
        let ol = (cam.s * 0.5).round().max(1.0) as i32;
        layer.composite(cv, style.outline, 0.9, ol, alpha);
    }
}
