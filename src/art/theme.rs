//! Per-level colour themes. Each one retints the entire environment: bricks,
//! mortar, floor slabs, the distant back wall, metalwork and the ambient light.

use crate::gfx::color::{rgb, Rgb};

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    /// Brick body, light and dark ends of the random range.
    pub brick: Rgb,
    pub brick_dk: Rgb,
    pub mortar: Rgb,
    /// Floor slab: top face, front face, shadow under it.
    pub slab_top: Rgb,
    pub slab_face: Rgb,
    pub slab_dk: Rgb,
    /// Distant back wall behind open space.
    pub back: Rgb,
    pub back_dk: Rgb,
    /// Trim / decorative accents (gold in the palace, iron in the dungeon).
    pub accent: Rgb,
    pub metal: Rgb,
    /// Ambient light multiplier applied to the whole room.
    pub ambient: [f32; 3],
    /// Colour of torch light in this level.
    pub torch: Rgb,
    /// Tint used by the vignette.
    pub vignette: Rgb,
}

pub const DUNGEON: Theme = Theme {
    name: "dungeon",
    brick: rgb(96, 92, 104),
    brick_dk: rgb(58, 56, 70),
    mortar: rgb(34, 33, 42),
    slab_top: rgb(140, 134, 140),
    slab_face: rgb(88, 84, 94),
    slab_dk: rgb(40, 38, 48),
    back: rgb(66, 62, 82),
    back_dk: rgb(32, 30, 43),
    accent: rgb(120, 104, 72),
    metal: rgb(158, 162, 174),
    ambient: [0.74, 0.72, 0.84],
    torch: rgb(255, 168, 78),
    vignette: rgb(8, 8, 16),
};

pub const CISTERN: Theme = Theme {
    name: "cistern",
    brick: rgb(76, 100, 104),
    brick_dk: rgb(42, 62, 68),
    mortar: rgb(24, 38, 44),
    slab_top: rgb(122, 148, 148),
    slab_face: rgb(70, 94, 98),
    slab_dk: rgb(30, 46, 52),
    back: rgb(44, 68, 78),
    back_dk: rgb(22, 34, 44),
    accent: rgb(102, 132, 118),
    metal: rgb(150, 168, 172),
    ambient: [0.66, 0.76, 0.84],
    torch: rgb(255, 176, 96),
    vignette: rgb(6, 12, 18),
};

pub const PALACE: Theme = Theme {
    name: "palace",
    brick: rgb(178, 148, 106),
    brick_dk: rgb(128, 102, 70),
    mortar: rgb(92, 72, 50),
    slab_top: rgb(226, 200, 156),
    slab_face: rgb(170, 140, 100),
    slab_dk: rgb(96, 74, 52),
    back: rgb(112, 88, 68),
    back_dk: rgb(62, 48, 38),
    accent: rgb(226, 184, 84),
    metal: rgb(206, 200, 190),
    ambient: [0.74, 0.70, 0.62],
    torch: rgb(255, 190, 110),
    vignette: rgb(22, 14, 10),
};

pub const TOWER: Theme = Theme {
    name: "tower",
    brick: rgb(112, 96, 118),
    brick_dk: rgb(64, 54, 76),
    mortar: rgb(38, 30, 48),
    slab_top: rgb(166, 148, 166),
    slab_face: rgb(104, 88, 112),
    slab_dk: rgb(46, 36, 56),
    back: rgb(64, 50, 84),
    back_dk: rgb(32, 24, 46),
    accent: rgb(150, 122, 190),
    metal: rgb(176, 168, 196),
    ambient: [0.70, 0.64, 0.86],
    torch: rgb(190, 150, 255),
    vignette: rgb(10, 6, 20),
};

pub const GARDEN: Theme = Theme {
    name: "garden",
    brick: rgb(158, 152, 116),
    brick_dk: rgb(104, 104, 76),
    mortar: rgb(72, 76, 56),
    slab_top: rgb(212, 208, 168),
    slab_face: rgb(150, 148, 112),
    slab_dk: rgb(80, 84, 60),
    back: rgb(92, 116, 110),
    back_dk: rgb(46, 64, 66),
    accent: rgb(120, 176, 116),
    metal: rgb(196, 198, 186),
    ambient: [0.80, 0.80, 0.70],
    torch: rgb(255, 196, 128),
    vignette: rgb(14, 20, 16),
};

pub const SANCTUM: Theme = Theme {
    name: "sanctum",
    brick: rgb(84, 62, 78),
    brick_dk: rgb(48, 32, 46),
    mortar: rgb(28, 18, 28),
    slab_top: rgb(148, 116, 132),
    slab_face: rgb(88, 64, 80),
    slab_dk: rgb(38, 24, 36),
    back: rgb(52, 32, 52),
    back_dk: rgb(26, 15, 28),
    accent: rgb(214, 168, 88),
    metal: rgb(182, 172, 190),
    ambient: [0.64, 0.54, 0.74],
    torch: rgb(255, 132, 96),
    vignette: rgb(12, 4, 12),
};

pub fn by_name(n: &str) -> Theme {
    match n {
        "cistern" => CISTERN,
        "palace" => PALACE,
        "tower" => TOWER,
        "garden" => GARDEN,
        "sanctum" => SANCTUM,
        _ => DUNGEON,
    }
}
