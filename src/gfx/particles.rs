//! Particles: dust, sparks, blood, flame, smoke and stone debris.
//!
//! Split into two draw passes. Opaque-ish matter (dust, blood, debris) is drawn
//! before the lighting pass so it is lit like everything else; emitters (flame,
//! sparks, glow) are drawn after it, additively, so they actually look like they
//! are giving off light.

use crate::gfx::canvas::{Blend, Canvas, Cam};
use crate::gfx::color::{rgb, Rgb};
use crate::gfx::target::{fill_circle, fill_rect, radial_glow};
use crate::util::{clampf, v2, Rng, V2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PKind {
    Dust,
    Blood,
    Debris,
    Spark,
    Flame,
    Smoke,
}

#[derive(Clone, Copy)]
pub struct Particle {
    pub kind: PKind,
    pub p: V2,
    pub v: V2,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub col: Rgb,
    pub spin: f32,
    pub grav: f32,
    /// Bottom of the tile the particle should rest on, if it can settle.
    pub floor: f32,
}

pub struct Particles {
    pub v: Vec<Particle>,
    pub rng: Rng,
}

impl Particles {
    pub fn new(seed: u64) -> Self {
        Particles {
            v: Vec::with_capacity(512),
            rng: Rng::new(seed),
        }
    }

    pub fn clear(&mut self) {
        self.v.clear();
    }

    fn push(&mut self, p: Particle) {
        if self.v.len() < 1400 {
            self.v.push(p);
        }
    }

    /// Puff of dust — landing, running, a body hitting the floor.
    pub fn dust(&mut self, at: V2, n: i32, power: f32, tint: Rgb) {
        for _ in 0..n {
            let a = self.rng.range(-2.6, -0.5);
            let sp = self.rng.range(6.0, 26.0) * power;
            let life = self.rng.range(0.35, 0.95);
            let __p = Particle {
                kind: PKind::Dust,
                p: v2(at.x + self.rng.sym() * 3.0, at.y + self.rng.sym() * 1.5),
                v: v2(a.cos() * sp * self.rng.range(0.6, 1.6), a.sin() * sp * 0.5),
                life,
                max: life,
                size: self.rng.range(0.8, 2.4),
                col: tint,
                spin: 0.0,
                grav: -6.0,
                floor: f32::MAX,
            };
            self.push(__p);
        }
    }

    pub fn blood(&mut self, at: V2, dir: f32, n: i32, floor: f32) {
        for _ in 0..n {
            let life = self.rng.range(0.5, 1.4);
            let __p = Particle {
                kind: PKind::Blood,
                p: v2(at.x, at.y),
                v: v2(
                    dir * self.rng.range(10.0, 55.0),
                    self.rng.range(-46.0, -6.0),
                ),
                life,
                max: life,
                size: self.rng.range(0.7, 1.8),
                col: rgb(
                    self.rng.irange(140, 200) as u8,
                    self.rng.irange(12, 34) as u8,
                    self.rng.irange(18, 40) as u8,
                ),
                spin: 0.0,
                grav: 210.0,
                floor,
            };
            self.push(__p);
        }
    }

    pub fn debris(&mut self, at: V2, n: i32, col: Rgb, floor: f32) {
        for _ in 0..n {
            let life = self.rng.range(0.7, 1.7);
            let __p = Particle {
                kind: PKind::Debris,
                p: v2(at.x + self.rng.sym() * 9.0, at.y + self.rng.sym() * 3.0),
                v: v2(self.rng.sym() * 38.0, self.rng.range(-52.0, 4.0)),
                life,
                max: life,
                size: self.rng.range(1.0, 3.0),
                col: col.scale(self.rng.range(0.75, 1.15)),
                spin: self.rng.sym() * 8.0,
                grav: 260.0,
                floor,
            };
            self.push(__p);
        }
    }

    pub fn sparks(&mut self, at: V2, n: i32, power: f32) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let sp = self.rng.range(20.0, 95.0) * power;
            let life = self.rng.range(0.12, 0.4);
            let __p = Particle {
                kind: PKind::Spark,
                p: at,
                v: v2(a.cos() * sp, a.sin() * sp * 0.8),
                life,
                max: life,
                size: self.rng.range(0.5, 1.3),
                col: rgb(255, self.rng.irange(190, 245) as u8, self.rng.irange(90, 170) as u8),
                spin: 0.0,
                grav: 120.0,
                floor: f32::MAX,
            };
            self.push(__p);
        }
    }

    /// One tick of flame for a torch or a burning projectile.
    pub fn flame(&mut self, at: V2, up: f32, scale: f32, hue: Rgb) {
        let life = self.rng.range(0.22, 0.55);
        let __p = Particle {
            kind: PKind::Flame,
            p: v2(at.x + self.rng.sym() * 1.6 * scale, at.y + self.rng.sym() * 1.0),
            v: v2(self.rng.sym() * 5.0, -up * self.rng.range(0.7, 1.3)),
            life,
            max: life,
            size: self.rng.range(1.5, 3.4) * scale,
            col: hue,
            spin: 0.0,
            grav: -18.0,
            floor: f32::MAX,
        };
            self.push(__p);
    }

    pub fn smoke(&mut self, at: V2, scale: f32) {
        let life = self.rng.range(0.7, 1.6);
        let __p = Particle {
            kind: PKind::Smoke,
            p: v2(at.x + self.rng.sym() * 2.0, at.y),
            v: v2(self.rng.sym() * 4.0, -self.rng.range(6.0, 16.0)),
            life,
            max: life,
            size: self.rng.range(2.0, 4.5) * scale,
            col: rgb(70, 66, 72),
            spin: 0.0,
            grav: -4.0,
            floor: f32::MAX,
        };
            self.push(__p);
    }

    pub fn update(&mut self, dt: f32) {
        for p in self.v.iter_mut() {
            p.life -= dt;
            p.v.y += p.grav * dt;
            if p.kind == PKind::Dust || p.kind == PKind::Smoke {
                p.v.x *= 1.0 - 2.0 * dt;
            }
            p.p.x += p.v.x * dt;
            p.p.y += p.v.y * dt;
            if p.p.y > p.floor {
                // Settle: stains and rubble stop where they land.
                p.p.y = p.floor;
                p.v = v2(p.v.x * 0.25, 0.0);
                p.grav = 0.0;
            }
        }
        self.v.retain(|p| p.life > 0.0);
    }

    /// Matter pass — before lighting.
    pub fn draw_matter(&self, cv: &mut Canvas, cam: &Cam) {
        cv.blend = Blend::Alpha;
        for p in &self.v {
            let t = clampf(p.life / p.max.max(0.001), 0.0, 1.0);
            match p.kind {
                PKind::Dust => {
                    let a = t * 0.5;
                    let r = cam.l(p.size * (1.0 + (1.0 - t) * 1.8));
                    fill_circle(cv, cam.p(p.p), r, p.col, a);
                }
                PKind::Smoke => {
                    let a = t * 0.42;
                    let r = cam.l(p.size * (1.0 + (1.0 - t) * 1.4));
                    fill_circle(cv, cam.p(p.p), r, p.col, a);
                }
                PKind::Blood => {
                    let d = cam.p(p.p);
                    let r = cam.l(p.size);
                    fill_circle(cv, d, r, p.col, (t * 1.6).min(1.0));
                }
                PKind::Debris => {
                    let d = cam.p(p.p);
                    let s = cam.l(p.size);
                    fill_rect(cv, d.x - s, d.y - s, d.x + s, d.y + s, p.col, (t * 2.0).min(1.0));
                }
                _ => {}
            }
        }
    }

    /// Emissive pass — after lighting, additive.
    pub fn draw_light(&self, cv: &mut Canvas, cam: &Cam) {
        cv.blend = Blend::Add;
        for p in &self.v {
            let t = clampf(p.life / p.max.max(0.001), 0.0, 1.0);
            match p.kind {
                PKind::Flame => {
                    // Fade from white-hot through the emitter's hue to embers.
                    let core = rgb(255, 240, 200).lerp(p.col, 1.0 - t);
                    let col = core.lerp(rgb(120, 30, 8), (1.0 - t) * 0.85);
                    let r = cam.l(p.size * (0.55 + t * 0.65));
                    fill_circle(cv, cam.p(p.p), r, col, t * 0.62);
                }
                PKind::Spark => {
                    let d = cam.p(p.p);
                    let tail = cam.p(p.p.sub(p.v.mul(0.02)));
                    crate::gfx::target::fill_capsule(
                        cv,
                        tail,
                        d,
                        cam.l(p.size * 0.4),
                        cam.l(p.size * 0.7),
                        p.col,
                        t,
                    );
                    radial_glow(cv, d, cam.l(p.size * 3.0), p.col, t * 0.2);
                }
                _ => {}
            }
        }
        cv.blend = Blend::Alpha;
    }

    /// Light contributed by emitters, so flames illuminate their surroundings.
    pub fn emitter_lights(&self, cam: &Cam, f: &mut dyn FnMut(V2, f32, Rgb, f32)) {
        for p in &self.v {
            if p.kind == PKind::Flame {
                let t = clampf(p.life / p.max.max(0.001), 0.0, 1.0);
                f(cam.p(p.p), cam.l(p.size * 4.0), p.col, t * 0.09);
            }
        }
    }
}
