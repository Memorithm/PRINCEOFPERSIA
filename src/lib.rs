//! Prince of Persia — a terminal reimplementation in Rust.
//!
//! Module map:
//!
//! * [`util`] — vectors, easing, a deterministic RNG and stable hashing.
//! * [`gfx`] — anti-aliased software rasteriser, colour, lighting, particles,
//!   the half-block terminal front-end and a dependency-free PNG writer.
//! * [`art`] — the look of the game: skeletal character rendering, the animation
//!   library, procedural environment art and per-level colour themes.
//! * [`world`] — tile vocabulary, the ASCII level parser, the campaign data and a
//!   reachability checker that proves each level can be finished.
//! * [`game`] — simulation: the prince's state machine, guards, combat, items.
//! * [`input`], [`app`] — keyboard handling and the terminal application loop.

pub mod app;
pub mod art;
pub mod game;
pub mod gfx;
pub mod input;
pub mod util;
pub mod world;
