//! The animation library: keyframed joint angles for every action in the game.
//!
//! Clips are built once at start-up (rather than as consts) so the poses can be
//! written with ordinary Rust and tweaked in one place. Durations are seconds
//! from that key to the next.

use std::sync::OnceLock;

use crate::art::skel::{ps, Pose};

pub struct Clip {
    pub keys: Vec<(Pose, f32)>,
    pub looping: bool,
}

impl Clip {
    fn new(keys: Vec<(Pose, f32)>, looping: bool) -> Clip {
        Clip { keys, looping }
    }

    pub fn total(&self) -> f32 {
        let n = self.keys.len();
        if n == 0 {
            return 0.0;
        }
        if self.looping {
            self.keys.iter().map(|k| k.1).sum()
        } else {
            self.keys[..n - 1].iter().map(|k| k.1).sum()
        }
    }

    /// Sample at time `t` (seconds). Non-looping clips hold their last pose.
    pub fn sample(&self, t: f32) -> Pose {
        if self.keys.is_empty() {
            return Pose::REST;
        }
        if self.keys.len() == 1 {
            return self.keys[0].0;
        }
        let total = self.total();
        let mut t = t.max(0.0);
        if self.looping {
            if total > 0.0 {
                t %= total;
            }
        } else if t >= total {
            return self.keys[self.keys.len() - 1].0;
        }
        let mut i = 0usize;
        loop {
            let d = self.keys[i].1;
            if t < d || i + 1 >= self.keys.len() {
                let next = if self.looping {
                    (i + 1) % self.keys.len()
                } else {
                    (i + 1).min(self.keys.len() - 1)
                };
                let f = if d > 0.0 { (t / d).min(1.0) } else { 1.0 };
                return self.keys[i].0.lerp(&self.keys[next].0, f);
            }
            t -= d;
            i += 1;
        }
    }

    /// Normalised progress in 0..1 for a non-looping clip.
    pub fn progress(&self, t: f32) -> f32 {
        let total = self.total();
        if total <= 0.0 {
            1.0
        } else {
            (t / total).min(1.0)
        }
    }
}

pub struct Anims {
    pub stand: Clip,
    pub stand_alert: Clip,
    pub turn: Clip,
    pub run_start: Clip,
    pub run: Clip,
    pub run_stop: Clip,
    pub step: Clip,
    pub crouch_in: Clip,
    pub crouch: Clip,
    pub jump_up: Clip,
    pub jump_run: Clip,
    pub fall: Clip,
    pub land: Clip,
    pub hang: Clip,
    pub climb: Clip,
    pub sword_ready: Clip,
    pub sword_adv: Clip,
    pub sword_ret: Clip,
    pub sword_strike: Clip,
    pub sword_parry: Clip,
    pub hurt: Clip,
    pub dead: Clip,
    pub drink: Clip,
    pub throw: Clip,
    pub cast: Clip,
    pub walk: Clip,
    pub bow: Clip,
}

static ANIMS: OnceLock<Anims> = OnceLock::new();

pub fn anims() -> &'static Anims {
    ANIMS.get_or_init(build)
}

fn build() -> Anims {
    // ---------------------------------------------------------- idle
    let stand_a = ps(13.6, 0.3, -3.0, 3.0, (7.0, 17.0), (-15.0, 15.0), (11.0, 15.0), (-9.0, 8.0));
    let stand_b = ps(13.4, 0.3, -1.5, 4.0, (5.0, 15.0), (-12.0, 13.0), (11.0, 15.0), (-9.0, 8.0));
    let stand = Clip::new(vec![(stand_a, 1.6), (stand_b, 1.6)], true);

    let alert = ps(13.0, 0.9, 6.0, -3.0, (28.0, 44.0), (-20.0, 30.0), (13.0, 14.0), (-13.0, 14.0));
    let alert_b = ps(12.8, 1.0, 8.0, -4.0, (32.0, 48.0), (-22.0, 34.0), (15.0, 16.0), (-15.0, 16.0));
    let stand_alert = Clip::new(vec![(alert, 0.5), (alert_b, 0.5)], true);

    // ---------------------------------------------------------- turning
    let turn = Clip::new(
        vec![
            (stand_a, 0.09),
            (
                ps(12.6, -0.6, 2.0, -14.0, (14.0, 26.0), (10.0, 24.0), (6.0, 8.0), (-4.0, 12.0)),
                0.10,
            ),
            (
                ps(12.8, -0.4, 0.0, -6.0, (9.0, 18.0), (2.0, 16.0), (2.0, 5.0), (-3.0, 8.0)),
                0.09,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- running
    let r0 = ps(13.0, 1.0, 15.0, -7.0, (-40.0, 26.0), (46.0, 60.0), (33.0, 13.0), (-31.0, 40.0));
    let r1 = ps(11.9, 1.3, 17.0, -8.0, (-20.0, 32.0), (28.0, 66.0), (15.0, 22.0), (-14.0, 64.0));
    let r2 = ps(13.5, 1.0, 13.0, -6.0, (6.0, 40.0), (8.0, 70.0), (-14.0, 14.0), (19.0, 76.0));
    let run = Clip::new(
        vec![
            (r0, 0.075),
            (r1, 0.075),
            (r2, 0.075),
            (r0.mirrored(), 0.075),
            (r1.mirrored(), 0.075),
            (r2.mirrored(), 0.075),
        ],
        true,
    );

    // A slower, heavier gait for guards on patrol.
    let w0 = ps(12.9, 0.4, 6.0, -2.0, (-22.0, 18.0), (24.0, 30.0), (22.0, 8.0), (-20.0, 24.0));
    let w1 = ps(12.4, 0.5, 8.0, -3.0, (-10.0, 22.0), (12.0, 34.0), (8.0, 14.0), (-8.0, 44.0));
    let w2 = ps(13.1, 0.4, 5.0, -2.0, (4.0, 26.0), (2.0, 36.0), (-9.0, 9.0), (12.0, 52.0));
    let walk = Clip::new(
        vec![
            (w0, 0.17),
            (w1, 0.17),
            (w2, 0.17),
            (w0.mirrored(), 0.17),
            (w1.mirrored(), 0.17),
            (w2.mirrored(), 0.17),
        ],
        true,
    );

    let run_start = Clip::new(
        vec![
            (
                ps(12.2, 1.4, 20.0, -10.0, (-16.0, 22.0), (26.0, 48.0), (12.0, 34.0), (-22.0, 30.0)),
                0.09,
            ),
            (
                ps(12.6, 1.8, 22.0, -10.0, (-30.0, 24.0), (40.0, 56.0), (26.0, 16.0), (-28.0, 46.0)),
                0.09,
            ),
            (r0, 0.0),
        ],
        false,
    );

    let run_stop = Clip::new(
        vec![
            (
                ps(12.0, -1.6, -8.0, 6.0, (30.0, 40.0), (-24.0, 30.0), (26.0, 10.0), (-16.0, 54.0)),
                0.11,
            ),
            (
                ps(12.6, -0.8, -4.0, 4.0, (18.0, 28.0), (-14.0, 22.0), (12.0, 6.0), (-8.0, 26.0)),
                0.11,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    let step = Clip::new(
        vec![
            (stand_a, 0.12),
            (
                ps(12.8, 0.6, 4.0, -2.0, (-8.0, 16.0), (10.0, 22.0), (17.0, 8.0), (-10.0, 20.0)),
                0.14,
            ),
            (
                ps(13.0, 0.4, 2.0, 0.0, (-4.0, 14.0), (5.0, 18.0), (7.0, 5.0), (-6.0, 12.0)),
                0.12,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- crouch
    let crouch_pose =
        ps(6.6, 1.6, 34.0, -22.0, (34.0, 62.0), (26.0, 70.0), (62.0, 96.0), (48.0, 104.0));
    let crouch = Clip::new(vec![(crouch_pose, 1.0)], false);
    let crouch_in = Clip::new(
        vec![
            (stand_a, 0.07),
            (
                ps(10.0, 1.0, 20.0, -12.0, (20.0, 40.0), (14.0, 44.0), (34.0, 52.0), (26.0, 58.0)),
                0.07,
            ),
            (crouch_pose, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- airborne
    let jump_up = Clip::new(
        vec![
            (
                ps(9.4, 0.4, 16.0, -8.0, (-18.0, 28.0), (-14.0, 30.0), (40.0, 62.0), (34.0, 66.0)),
                0.09,
            ),
            (
                ps(13.6, 0.6, 4.0, -6.0, (120.0, 30.0), (128.0, 26.0), (10.0, 14.0), (4.0, 20.0)),
                0.10,
            ),
            (
                ps(14.4, 0.4, -4.0, -10.0, (162.0, 14.0), (168.0, 12.0), (-6.0, 12.0), (-14.0, 26.0)),
                0.16,
            ),
            (
                ps(14.0, 0.2, 0.0, -6.0, (150.0, 20.0), (156.0, 18.0), (6.0, 22.0), (-4.0, 34.0)),
                0.0,
            ),
        ],
        false,
    );

    let jump_run = Clip::new(
        vec![
            (
                ps(11.6, 1.8, 24.0, -12.0, (-34.0, 26.0), (52.0, 54.0), (26.0, 46.0), (-30.0, 26.0)),
                0.08,
            ),
            (
                ps(13.4, 1.6, 14.0, -8.0, (58.0, 40.0), (-32.0, 36.0), (56.0, 74.0), (-24.0, 48.0)),
                0.14,
            ),
            (
                ps(13.8, 1.2, 6.0, -6.0, (96.0, 30.0), (-40.0, 30.0), (38.0, 30.0), (-32.0, 70.0)),
                0.16,
            ),
            (
                ps(13.4, 1.0, 12.0, -8.0, (60.0, 44.0), (-20.0, 40.0), (44.0, 20.0), (-18.0, 60.0)),
                0.0,
            ),
        ],
        false,
    );

    let fall = Clip::new(
        vec![
            (
                ps(13.2, 0.0, -6.0, -8.0, (144.0, 34.0), (152.0, 30.0), (16.0, 30.0), (-14.0, 44.0)),
                0.18,
            ),
            (
                ps(13.4, 0.0, -10.0, -10.0, (156.0, 28.0), (140.0, 36.0), (24.0, 24.0), (-20.0, 52.0)),
                0.18,
            ),
        ],
        true,
    );

    let mut land_squash =
        ps(7.4, 1.0, 30.0, -18.0, (40.0, 66.0), (32.0, 70.0), (54.0, 88.0), (44.0, 96.0));
    land_squash.squash = 0.9;
    let land = Clip::new(
        vec![
            (land_squash, 0.10),
            (
                ps(11.0, 0.6, 14.0, -8.0, (22.0, 40.0), (16.0, 44.0), (24.0, 40.0), (18.0, 46.0)),
                0.12,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- hanging & climbing
    // Hanging: legs straight, arms straight up. The reach this produces is what
    // HANG_DROP is set to, so the hands land exactly on the ledge lip — a test
    // pins the two together.
    let hang_a =
        ps(13.0, -0.5, -2.0, 6.0, (172.0, 6.0), (176.0, 8.0), (1.0, 2.0), (-2.0, 5.0));
    let hang_b =
        ps(12.8, -0.5, -4.0, 7.0, (174.0, 7.0), (178.0, 9.0), (4.0, 4.0), (-5.0, 8.0));
    let hang = Clip::new(vec![(hang_a, 1.1), (hang_b, 1.1)], true);

    let climb = Clip::new(
        vec![
            (hang_a, 0.16),
            (
                ps(11.0, -0.2, 4.0, 0.0, (152.0, 54.0), (156.0, 56.0), (54.0, 92.0), (44.0, 86.0)),
                0.18,
            ),
            (
                ps(9.0, 1.6, 26.0, -14.0, (118.0, 46.0), (124.0, 48.0), (66.0, 104.0), (30.0, 62.0)),
                0.18,
            ),
            (
                ps(10.4, 1.4, 18.0, -8.0, (60.0, 40.0), (66.0, 42.0), (34.0, 56.0), (12.0, 28.0)),
                0.16,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- sword play
    let ready = ps(12.4, 0.6, 4.0, -4.0, (52.0, 34.0), (-30.0, 46.0), (16.0, 12.0), (-18.0, 30.0))
        .with_sword(-34.0);
    let ready_b = ps(12.2, 0.7, 6.0, -5.0, (56.0, 30.0), (-32.0, 44.0), (18.0, 14.0), (-20.0, 32.0))
        .with_sword(-30.0);
    let sword_ready = Clip::new(vec![(ready, 0.42), (ready_b, 0.42)], true);

    let sword_adv = Clip::new(
        vec![
            (ready, 0.09),
            (
                ps(12.0, 1.6, 10.0, -6.0, (58.0, 26.0), (-28.0, 44.0), (34.0, 14.0), (-24.0, 42.0))
                    .with_sword(-30.0),
                0.10,
            ),
            (ready, 0.0),
        ],
        false,
    );

    let sword_ret = Clip::new(
        vec![
            (ready, 0.09),
            (
                ps(12.2, -0.8, -2.0, 0.0, (46.0, 40.0), (-34.0, 48.0), (-14.0, 22.0), (22.0, 20.0))
                    .with_sword(-40.0),
                0.10,
            ),
            (ready, 0.0),
        ],
        false,
    );

    let sword_strike = Clip::new(
        vec![
            // wind up
            (
                ps(12.4, -0.6, -6.0, -2.0, (24.0, 66.0), (-30.0, 50.0), (-6.0, 16.0), (14.0, 26.0))
                    .with_sword(-58.0),
                0.10,
            ),
            // thrust
            (
                ps(11.8, 2.6, 18.0, -8.0, (86.0, 6.0), (-40.0, 40.0), (46.0, 12.0), (-30.0, 50.0))
                    .with_sword(4.0),
                0.09,
            ),
            // extended
            (
                ps(11.9, 2.8, 20.0, -8.0, (90.0, 2.0), (-42.0, 38.0), (48.0, 10.0), (-32.0, 52.0))
                    .with_sword(6.0),
                0.07,
            ),
            // recover
            (ready, 0.14),
            (ready, 0.0),
        ],
        false,
    );

    let sword_parry = Clip::new(
        vec![
            (ready, 0.06),
            (
                ps(12.4, -0.2, -8.0, 2.0, (74.0, 62.0), (-26.0, 44.0), (4.0, 16.0), (-14.0, 28.0))
                    .with_sword(-96.0),
                0.14,
            ),
            (ready, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- damage & death
    let hurt = Clip::new(
        vec![
            (
                ps(12.0, -1.8, -16.0, 14.0, (-30.0, 24.0), (-38.0, 20.0), (-16.0, 24.0), (18.0, 20.0)),
                0.14,
            ),
            (
                ps(12.4, -0.8, -8.0, 8.0, (-16.0, 20.0), (-20.0, 18.0), (-6.0, 14.0), (8.0, 16.0)),
                0.14,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    let mut fallen =
        ps(3.4, -2.0, 84.0, -34.0, (-56.0, 30.0), (34.0, 40.0), (76.0, 24.0), (58.0, 62.0));
    fallen.tail = 0.2;
    let dead = Clip::new(
        vec![
            (
                ps(9.0, -2.4, 46.0, -20.0, (-40.0, 30.0), (30.0, 36.0), (30.0, 40.0), (24.0, 50.0)),
                0.18,
            ),
            (fallen, 0.0),
        ],
        false,
    );

    // ---------------------------------------------------------- misc actions
    let drink = Clip::new(
        vec![
            (stand_a, 0.14),
            (
                ps(12.8, 0.2, -4.0, 10.0, (128.0, 92.0), (-8.0, 16.0), (4.0, 6.0), (-8.0, 12.0)),
                0.36,
            ),
            (
                ps(12.6, 0.0, -12.0, 22.0, (140.0, 104.0), (-6.0, 14.0), (2.0, 6.0), (-6.0, 12.0)),
                0.30,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    let throw = Clip::new(
        vec![
            (
                ps(12.6, -0.8, -10.0, -2.0, (-46.0, 96.0), (-20.0, 30.0), (-8.0, 14.0), (12.0, 22.0))
                    .with_sword(-70.0),
                0.10,
            ),
            (
                ps(12.2, 2.0, 16.0, -8.0, (104.0, 12.0), (-30.0, 34.0), (30.0, 12.0), (-22.0, 40.0))
                    .with_sword(10.0),
                0.10,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    let cast = Clip::new(
        vec![
            (
                ps(12.6, -0.4, -8.0, -4.0, (120.0, 40.0), (-22.0, 32.0), (-6.0, 14.0), (10.0, 22.0))
                    .with_sword(-20.0),
                0.14,
            ),
            (
                ps(12.4, 1.4, 8.0, -6.0, (96.0, 8.0), (-28.0, 36.0), (22.0, 12.0), (-16.0, 34.0))
                    .with_sword(6.0),
                0.16,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    let bow = Clip::new(
        vec![
            (stand_a, 0.5),
            (
                ps(12.0, 1.0, 34.0, 12.0, (30.0, 30.0), (24.0, 34.0), (10.0, 12.0), (-10.0, 18.0)),
                0.6,
            ),
            (stand_a, 0.0),
        ],
        false,
    );

    Anims {
        stand,
        stand_alert,
        turn,
        run_start,
        run,
        run_stop,
        step,
        crouch_in,
        crouch,
        jump_up,
        jump_run,
        fall,
        land,
        hang,
        climb,
        sword_ready,
        sword_adv,
        sword_ret,
        sword_strike,
        sword_parry,
        hurt,
        dead,
        drink,
        throw,
        cast,
        walk,
        bow,
    }
}
