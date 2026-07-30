//! The campaign: six new levels, every one of them larger than any level in the
//! 1989 original (which topped out at 24 rooms).
//!
//! | # | name                      | rooms | theme   |
//! |---|---------------------------|-------|---------|
//! | 1 | Les Geôles du Sultan      | 8 x 4 | dungeon |
//! | 2 | Les Citernes              | 9 x 5 | cistern |
//! | 3 | L'Escalier du Palais      |10 x 5 | palace  |
//! | 4 | La Tour de l'Alchimiste   | 9 x 6 | tower   |
//! | 5 | Les Jardins Suspendus     |11 x 5 | garden  |
//! | 6 | Le Sanctuaire de Jaffar   |10 x 6 | sanctum |
//!
//! Map legend — tile layer:
//!
//! ```text
//!   . space      = floor       # rock/wall   | column
//!   b loose      : rubble      ^ spikes      V chomper
//!   G portcullis p plate up    o plate down  X exit
//!   t torch      m mirror      w window      A arch     n bones
//!   @ start
//!   h heal  H vigour  f feather  x poison  q celerity
//!   s sword D daggers F flame wand C buckler M scimitar
//!   g guard z jailer  k skeleton S shadow    J Jaffar
//! ```
//!
//! Link layer: a digit or letter wires plates to the gates and doors that share
//! it. **Lower-case and digit groups are timed** — the gate creeps shut again —
//! while **upper-case groups latch open for good**. On a guard, a digit is its
//! skill (0–9) and `>` makes it face right.

use crate::world::level::{LevelDef, F, S, UPL_A, UPL_B, UPR_A, UPR_B, W};

// =====================================================================
// 1. Les Geôles du Sultan — 8 x 4 rooms
// =====================================================================

const L1_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W],
    &[
        W,
        "....t.....",
        S,
        "......t...",
        S,
        "#####t....",
        "....t.....",
        "...#######",
    ],
    &[
        W,
        F,
        "====g=====",
        "===^^=====",
        "=b======H#",
        "#####p=h==",
        F,
        "===X######",
    ],
    &[W, UPL_A, W, W, W, W, "..====....", W],
    &[W, UPL_B, "....t.....", S, S, "....t.....", "......====", W],
    &[W, F, F, "x=========", "...==G====", "=====g====", F, W],
    &[W, ".....====.", S, "......====", S, "....t.....", W, W],
    &[W, ".====.....", S, "........==", "==........", W, W, W],
    &[W, "======g===", "=======s==", F, "=======p.#", W, W, W],
    &[W, S, "......t...", "====......", "##==.....#", ".....t....", S, W],
    &[W, "....t.....", S, "....====..", "##..==...#", S, S, W],
    &[
        W,
        "==@=======",
        "==b===^^==",
        "======h===",
        "####==n===",
        "=^^===DH==",
        "=n====f==#",
        W,
    ],
];

const L1_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, S, "....3.....", S, S, ".....B....", S, "...B......"],
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, ".....A....", ".....2....", S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, "......1...", S, S, ".......A..", S, S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S],
];

// =====================================================================
// 2. Les Citernes — 9 x 5 rooms
// =====================================================================

const L2_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W, W],
    &[W, W, "....t.....", S, "....t.....", S, "...t......", S, "..t......."],
    &[
        W,
        W,
        F,
        "====p=====",
        "=====^^===",
        F,
        "==...=====",
        "==b=======",
        "=====V=X##",
    ],
    &[W, W, UPL_A, S, S, S, S, W, W],
    &[W, W, UPL_B, S, S, S, S, W, W],
    &[
        W,
        W,
        "=h========",
        "======...=",
        "====g=====",
        "b=========",
        "==g=======",
        W,
        W,
    ],
    &[W, W, S, S, S, S, UPR_A, W, W],
    &[W, W, S, S, S, S, UPR_B, W, W],
    &[
        W,
        F,
        "==V=====V=",
        F,
        "======p===",
        "=.======G=",
        F,
        W,
        W,
    ],
    &[W, UPL_A, S, S, "##==......", S, W, W, W],
    &[W, UPL_B, S, S, "##..==....", S, W, W, W],
    &[
        W,
        F,
        "====g=====",
        "=n========",
        "####======",
        "==^^==C=n=",
        W,
        W,
        W,
    ],
    &[W, S, "...t......", UPR_A, W, W, W, W, W],
    &[W, S, S, UPR_B, W, W, W, W, W],
    &[
        "...@======",
        "==b====^^=",
        "=====V====",
        "===h======",
        W,
        W,
        W,
        W,
        W,
    ],
];

const L2_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, "....C.....", S, S, S, S, ".......C.."],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "....4.....", S, "..3.......", S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "......a...", "........a.", S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, "....3.....", S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
];

// =====================================================================
// 3. L'Escalier du Palais — 10 x 5 rooms
// =====================================================================

const L3_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W, W, W],
    &[
        W,
        W,
        W,
        "...t......",
        S,
        S,
        S,
        "....t.....",
        S,
        "..t.......",
    ],
    &[
        W,
        W,
        W,
        F,
        "======^^==",
        F,
        "==...=====",
        "=b========",
        "====p=====",
        "======X###",
    ],
    &[W, W, W, UPL_A, S, S, S, S, S, W],
    &[W, W, W, UPL_B, S, S, S, S, S, W],
    &[
        W,
        W,
        W,
        "=h========",
        "===g======",
        "=====b====",
        "==g=======",
        F,
        W,
        W,
    ],
    &[W, W, W, S, S, S, S, UPR_A, W, W],
    &[W, W, W, S, S, S, S, UPR_B, W, W],
    &[
        W,
        F,
        "===^^=====",
        "=====b====",
        "==z=======",
        "========.=",
        "=====G====",
        F,
        W,
        W,
    ],
    &[W, UPL_A, S, S, S, "##==......", S, W, W, W],
    &[W, UPL_B, S, S, S, "##..==....", S, W, W, W],
    &[
        W,
        F,
        "====g=====",
        F,
        "=====n====",
        "##====^^==",
        "==H===p===",
        W,
        W,
        W,
    ],
    &[W, S, S, S, UPR_A, W, W, W, W, W],
    &[W, S, "...t......", S, UPR_B, W, W, W, W, W],
    &[
        "...@======",
        "==b====^^=",
        "=====V====",
        "===h======",
        F,
        W,
        W,
        W,
        W,
        W,
    ],
];

const L3_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, "....A.....", "......A..."],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "...4......", S, "..5.......", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "..5.......", S, ".....A....", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, "....3.....", S, S, S, "......A...", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
];

// =====================================================================
// 4. La Tour de l'Alchimiste — 9 x 6 rooms
// =====================================================================

const L4_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W, W],
    &[
        W,
        W,
        "..t.......",
        S,
        "....t.....",
        S,
        "......t...",
        "....t.....",
        "..t.......",
    ],
    &[
        W,
        W,
        "=====H====",
        "==^^======",
        F,
        "===b======",
        "=========#",
        "==p=======",
        "=====X####",
    ],
    &[W, W, S, S, UPR_A, S, S, UPR_A, W],
    &[W, W, S, S, UPR_B, S, S, UPR_B, W],
    &[
        W,
        W,
        "=q========",
        "====^^====",
        "==g=======",
        F,
        "=====b====",
        F,
        W,
    ],
    &[W, W, UPL_A, S, S, S, S, S, W],
    &[W, W, UPL_B, S, S, S, S, S, W],
    &[
        W,
        W,
        F,
        "===k======",
        "======V===",
        "==x=======",
        W,
        W,
        W,
    ],
    &[W, W, S, S, S, UPR_A, W, W, W],
    &[W, W, S, S, S, UPR_B, W, W, W],
    &[
        W,
        F,
        "==V==p====",
        "=====G====",
        "=====F====",
        F,
        W,
        W,
        W,
    ],
    &[W, UPL_A, S, S, S, S, W, W, W],
    &[W, UPL_B, S, S, S, S, W, W, W],
    &[W, F, "====g=====", "==h=======", W, W, W, W, W],
    &[W, S, S, UPR_A, W, W, W, W, W],
    &[W, S, "...t......", UPR_B, W, W, W, W, W],
    &[
        "...@======",
        "==b====^^=",
        "=====V====",
        F,
        W,
        W,
        W,
        W,
        W,
    ],
];

const L4_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, "..A.......", ".....A...."],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "..5.......", S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, "...6......", S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, ".....b....", ".....b....", S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, "....4.....", S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S],
];

// =====================================================================
// 5. Les Jardins Suspendus — 11 x 5 rooms
// =====================================================================

const L5_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W, W, W, W],
    &[
        W,
        W,
        W,
        W,
        W,
        "....t.....",
        S,
        S,
        "....t.....",
        S,
        "..t.......",
    ],
    &[
        W,
        W,
        W,
        W,
        W,
        F,
        "===^^=====",
        F,
        "==...=====",
        "=b====p===",
        "=====X####",
    ],
    &[W, W, W, W, W, UPL_A, S, S, S, S, W],
    &[W, W, W, W, W, UPL_B, S, S, S, S, W],
    &[
        W,
        W,
        W,
        W,
        W,
        "=h========",
        "====g=====",
        "=====b====",
        "==z=======",
        W,
        W,
    ],
    &[W, W, S, S, S, S, S, S, UPR_A, W, W],
    &[W, W, S, S, S, S, S, S, UPR_B, W, W],
    &[
        W,
        W,
        F,
        "==V=======",
        "====g=====",
        "=====^^===",
        "==g=======",
        "==.====G==",
        F,
        W,
        W,
    ],
    &[W, W, UPL_A, S, S, S, "........==", S, W, W, W],
    &[W, W, UPL_B, S, S, S, S, "==........", W, W, W],
    &[
        W,
        W,
        F,
        "====g=====",
        F,
        "=====n====",
        W,
        "===M==p===",
        W,
        W,
        W,
    ],
    &[W, W, S, S, S, UPR_A, W, W, W, W, W],
    &[W, W, "...t......", S, S, UPR_B, W, W, W, W, W],
    &[
        "...@======",
        "==b====^^=",
        "=====V====",
        "===h======",
        "====g=====",
        F,
        W,
        W,
        W,
        W,
        W,
    ],
];

const L5_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, "......A...", ".....A...."],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, "....4.....", S, "..6.......", S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "....4.....", S, "..5.......", ".......A..", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, "....5.....", S, S, S, "......A...", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "....3.....", S, S, S, S, S, S],
];

// =====================================================================
// 6. Le Sanctuaire de Jaffar — 10 x 6 rooms
// =====================================================================

const L6_ROWS: &[&[&str]] = &[
    &[W, W, W, W, W, W, W, W, W, W],
    &[W, W, W, W, W, W, W, W, "....t.....", "..t......."],
    &[W, W, W, W, W, W, W, W, "==J===p===", "=======X##"],
    &[W, W, W, W, W, W, W, W, UPR_A, W],
    &[W, W, W, W, W, W, W, W, UPR_B, W],
    &[
        W,
        W,
        W,
        "=h========",
        "====^^====",
        "==S=======",
        "=====b====",
        "==V=======",
        F,
        W,
    ],
    &[W, W, W, UPL_A, S, S, S, S, S, W],
    &[W, W, W, UPL_B, S, S, S, S, S, W],
    &[
        W,
        W,
        W,
        F,
        "==k=======",
        "=====G====",
        "======p===",
        W,
        W,
        W,
    ],
    &[W, W, W, S, S, S, UPR_A, W, W, W],
    &[W, W, "....m.....", S, S, S, UPR_B, W, W, W],
    &[
        W,
        F,
        "====q=====",
        "=====V====",
        "==g=======",
        "=====^^===",
        F,
        W,
        W,
        W,
    ],
    &[W, UPL_A, S, S, S, S, W, W, W, W],
    &[W, UPL_B, S, S, S, S, W, W, W, W],
    &[W, F, "====g=====", "==h=======", F, W, W, W, W, W],
    &[W, S, S, S, UPR_A, W, W, W, W, W],
    &[W, S, "...t......", S, UPR_B, W, W, W, W, W],
    &[
        "...@======",
        "==b====^^=",
        "=====V====",
        "===n======",
        F,
        W,
        W,
        W,
        W,
        W,
    ],
];

const L6_LINKS: &[&[&str]] = &[
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, "..9...A...", ".......A.."],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, "..7.......", S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "..6.......", ".....b....", "......b...", S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, "..5.......", S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, "....4.....", S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
    &[S, S, S, S, S, S, S, S, S, S],
];

// =====================================================================

pub static CAMPAIGN: &[LevelDef] = &[
    LevelDef {
        name: "Les Geôles du Sultan",
        theme: "dungeon",
        hint: "Ramasse l'épée avant d'affronter le garde. Presse la dalle pour lever la herse.",
        rows: L1_ROWS,
        links: L1_LINKS,
        time: 900,
    },
    LevelDef {
        name: "Les Citernes",
        theme: "cistern",
        hint: "Les lames se referment en cadence : compte avant de passer.",
        rows: L2_ROWS,
        links: L2_LINKS,
        time: 960,
    },
    LevelDef {
        name: "L'Escalier du Palais",
        theme: "palace",
        hint: "Le geôlier frappe fort mais pare mal. Attaque après sa botte.",
        rows: L3_ROWS,
        links: L3_LINKS,
        time: 1020,
    },
    LevelDef {
        name: "La Tour de l'Alchimiste",
        theme: "tower",
        hint: "Le bâton de flamme brûle à distance : garde-le pour le squelette.",
        rows: L4_ROWS,
        links: L4_LINKS,
        time: 1080,
    },
    LevelDef {
        name: "Les Jardins Suspendus",
        theme: "garden",
        hint: "Le cimeterre du Vizir tranche en deux coups. Il dort sous les jardins.",
        rows: L5_ROWS,
        links: L5_LINKS,
        time: 1140,
    },
    LevelDef {
        name: "Le Sanctuaire de Jaffar",
        theme: "sanctum",
        hint: "L'Ombre est toi-même. Rengaine ton épée et va vers elle.",
        rows: L6_ROWS,
        links: L6_LINKS,
        time: 1200,
    },
];

pub fn count() -> usize {
    CAMPAIGN.len()
}
