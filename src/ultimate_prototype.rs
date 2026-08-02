//! # Ultimate Modular Prototype - Prince of Persia Ultimate Mechanical Core
//!
//! This module implements an advanced, highly optimized, zero-allocation-in-loop game prototype
//! in idiomatic Rust. It integrates four core mechanical axes:
//!
//! 1. **Time-Echo (Recording & Replay)**: Circular buffer (`VecDeque`) tracking exact player inputs
//!    with constant-time playback to drive a spectral ghost entity (`EchoEntity`).
//! 2. **Destructible Environmental ECS**: Generational index Entity-Component System managing grid tiles,
//!    with structural force propagation (`StressPropagationSystem`) and dynamic pathfinding update.
//! 3. **Camera & Controller Perspective Transition**: Smooth state-machine interpolation between
//!    2D Horizontal Side-Scroller and orthographic/isometric Top-Down grid coordinates, triggered spatially.
//! 4. **Predictive Guard AI**: Pattern learning and real-time adjustment of defense weights based on the
//!    player's recent action sequence history.

use std::collections::VecDeque;

// ============================================================================
// 1. SYSTEM "TIME-ECHO"
// ============================================================================

/// Represents recorded player inputs at a given simulation frame (tick).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputFrame {
    pub tick_id: u64,
    pub movement_vector: [f32; 2],
    pub action_flags: u32,
}

impl InputFrame {
    pub const ACTION_NONE: u32 = 0;
    pub const ACTION_JUMP: u32 = 1 << 0;
    pub const ACTION_DODGE: u32 = 1 << 1;
    pub const ACTION_ATTACK_LOW_RIGHT: u32 = 1 << 2;
    pub const ACTION_ATTACK_HIGH_LEFT: u32 = 1 << 3;
}

/// Circular buffer managing input frames with fixed capacity.
/// Guarantees zero allocation during continuous gameplay by pre-allocating up to the limit.
#[derive(Debug, Clone)]
pub struct CircularInputBuffer {
    buffer: VecDeque<InputFrame>,
    max_capacity: usize,
}

impl CircularInputBuffer {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// Push an input frame. Drops the oldest frame if capacity is reached.
    pub fn push(&mut self, frame: InputFrame) {
        if self.buffer.len() >= self.max_capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(frame);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Access frame by index relative to the oldest stored frame.
    pub fn get(&self, index: usize) -> Option<&InputFrame> {
        self.buffer.get(index)
    }

    /// Get the frame that was recorded exactly `offset_ticks` ago from the newest frame.
    pub fn get_delayed(&self, offset_ticks: usize) -> Option<&InputFrame> {
        if self.buffer.is_empty() || offset_ticks == 0 {
            return self.buffer.back();
        }
        let len = self.buffer.len();
        if offset_ticks >= len {
            self.buffer.front()
        } else {
            self.buffer.get(len - 1 - offset_ticks)
        }
    }
}

// ============================================================================
// 2. ENVIRONMENTAL DESTRUCTION & ECS ENGINE
// ============================================================================

/// Generation-safe Entity Index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialType {
    Space,
    Wood,
    Stone,
    Iron,
    Obsidian,
}

impl MaterialType {
    pub fn max_health(self) -> f32 {
        match self {
            MaterialType::Space => 0.0,
            MaterialType::Wood => 50.0,
            MaterialType::Stone => 150.0,
            MaterialType::Iron => 400.0,
            MaterialType::Obsidian => 1000.0,
        }
    }

    /// Resistance to structural load. Above this load, health deteriorates.
    pub fn stress_resistance(self) -> f32 {
        match self {
            MaterialType::Space => 0.0,
            MaterialType::Wood => 10.0,
            MaterialType::Stone => 40.0,
            MaterialType::Iron => 200.0,
            MaterialType::Obsidian => 500.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileComponent {
    pub grid_x: i32,
    pub grid_y: i32,
    pub health: f32,
    pub material: MaterialType,
    pub accumulated_stress: f32,
}

/// Dynamic pathfinding node connectivity.
#[derive(Clone, Debug)]
pub struct NavigationMap {
    pub width: usize,
    pub height: usize,
    /// grid of boolean flags indicating if a tile is walkable/navigable
    pub grid: Vec<bool>,
}

impl NavigationMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![true; width * height],
        }
    }

    pub fn set_navigable(&mut self, x: usize, y: usize, navigable: bool) {
        if x < self.width && y < self.height {
            self.grid[y * self.width + x] = navigable;
        }
    }

    pub fn is_navigable(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.grid[y * self.width + x]
        } else {
            false
        }
    }
}

/// Standard, clean Entity-Component-System container.
/// Specially optimized with dense arrays and parallel validation indices.
pub struct EcsWorld {
    next_entity_id: u32,
    generations: Vec<u32>,
    free_ids: Vec<u32>,

    // Component storages
    pub positions: Vec<Option<Position>>,
    pub tiles: Vec<Option<TileComponent>>,
    pub echoes: Vec<Option<EchoComponent>>,
    pub camera_controllers: Vec<Option<CameraControllerComponent>>,
    pub guard_ai: Vec<Option<GuardAiComponent>>,
}

#[derive(Clone)]
pub struct EchoComponent {
    pub delayed_playback_offset: usize,
    pub active_spectral: bool,
}

#[derive(Clone)]
pub struct CameraControllerComponent {
    pub mode: CameraMode,
    pub controller_type: ControllerType,
    pub transition_progress: f32, // Interpolator in [0, 1]
    pub position_offset: [f32; 3],
}

#[derive(Clone)]
pub struct GuardAiComponent {
    pub last_actions: VecDeque<InputSummary>,
    /// Weights for defense decisions: [LowRight, HighLeft, DodgePredict]
    pub parry_weights: [f32; 3],
}

impl EcsWorld {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            generations: Vec::new(),
            free_ids: Vec::new(),
            positions: Vec::new(),
            tiles: Vec::new(),
            echoes: Vec::new(),
            camera_controllers: Vec::new(),
            guard_ai: Vec::new(),
        }
    }

    pub fn create_entity(&mut self) -> Entity {
        let id = if let Some(free_id) = self.free_ids.pop() {
            free_id
        } else {
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            id
        };

        let index = id as usize;
        if index >= self.generations.len() {
            self.generations.resize(index + 1, 0);
            self.positions.resize(index + 1, None);
            self.tiles.resize(index + 1, None);
            self.echoes.resize(index + 1, None);
            self.camera_controllers.resize(index + 1, None);
            self.guard_ai.resize(index + 1, None);
        }

        Entity {
            id,
            generation: self.generations[index],
        }
    }

    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        let index = entity.id as usize;
        if index < self.generations.len() && self.generations[index] == entity.generation {
            self.generations[index] += 1;
            self.positions[index] = None;
            self.tiles[index] = None;
            self.echoes[index] = None;
            self.camera_controllers[index] = None;
            self.guard_ai[index] = None;
            self.free_ids.push(entity.id);
            true
        } else {
            false
        }
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let index = entity.id as usize;
        index < self.generations.len() && self.generations[index] == entity.generation
    }
}

// ============================================================================
// 3. CAMERAS & PERSPECTIVES STATE MACHINE
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraMode {
    SideScroller,
    TopDownGrid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControllerType {
    Platformer,
    GridMovement,
}

pub struct PortalZone {
    pub min_coords: [f32; 3],
    pub max_coords: [f32; 3],
    pub trigger_to_mode: CameraMode,
    pub trigger_to_controller: ControllerType,
}

impl PortalZone {
    pub fn contains(&self, p: &Position) -> bool {
        p.x >= self.min_coords[0] && p.x <= self.max_coords[0] &&
        p.y >= self.min_coords[1] && p.y <= self.max_coords[1] &&
        p.z >= self.min_coords[2] && p.z <= self.max_coords[2]
    }
}

// ============================================================================
// 4. PREDICTIVE GUARD AI
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputSummary {
    pub used_dodge: bool,
    pub attack_type: u32, // ACTION_ATTACK_LOW_RIGHT, ACTION_ATTACK_HIGH_LEFT, etc.
}

// ============================================================================
// SYSTEM IMPLEMENTATIONS (PHYSICS, PROPAGATION, TRANSITION, LEARNING)
// ============================================================================

pub struct StressPropagationSystem;

impl StressPropagationSystem {
    /// Propagates structural loads horizontally & vertically on a tile grid.
    /// Deducts health when stress exceeds resistance, collapses tiles,
    /// and dynamically flags navigation blocks on the map.
    pub fn update(
        world: &mut EcsWorld,
        nav_map: &mut NavigationMap,
        impacts: &[(i32, i32, f32)], // (grid_x, grid_y, force)
    ) {
        // Step 1: Apply immediate impact damages
        for &(ix, iy, force) in impacts {
            for maybe_tile in world.tiles.iter_mut() {
                if let Some(tile) = maybe_tile {
                    if tile.grid_x == ix && tile.grid_y == iy {
                        tile.health -= force;
                        tile.accumulated_stress += force * 0.5;
                    }
                }
            }
        }

        // Step 2: Stress propagation and collapse loops (zero allocations inside logic)
        let mut changes = true;
        let mut iterations = 0;
        while changes && iterations < 5 {
            changes = false;
            iterations += 1;

            // Collect collapse markers
            let mut collapse_coords = [(0, 0); 32];
            let mut collapse_count = 0;

            for maybe_tile in world.tiles.iter() {
                if let Some(tile) = maybe_tile {
                    if tile.health <= 0.0 && tile.material != MaterialType::Space {
                        if collapse_count < collapse_coords.len() {
                            collapse_coords[collapse_count] = (tile.grid_x, tile.grid_y);
                            collapse_count += 1;
                        }
                    }
                }
            }

            // Perform collapses and transfer stress to direct neighbors
            for idx in 0..collapse_count {
                let (cx, cy) = collapse_coords[idx];
                let mut collapsed_stress = 0.0;

                // Turn the collapsed tile into Space
                for maybe_tile in world.tiles.iter_mut() {
                    if let Some(tile) = maybe_tile {
                        if tile.grid_x == cx && tile.grid_y == cy && tile.material != MaterialType::Space {
                            collapsed_stress = tile.accumulated_stress;
                            tile.material = MaterialType::Space;
                            tile.health = 0.0;
                            changes = true;

                            // Update navigation map: space is walkable, wall/solids were not
                            if cx >= 0 && (cx as usize) < nav_map.width && cy >= 0 && (cy as usize) < nav_map.height {
                                nav_map.set_navigable(cx as usize, cy as usize, true);
                            }
                        }
                    }
                }

                // Propagate stress to direct neighbors: (up, down, left, right)
                let neighbors = [(cx, cy - 1), (cx, cy + 1), (cx - 1, cy), (cx + 1, cy)];
                for &(nx, ny) in &neighbors {
                    for maybe_tile in world.tiles.iter_mut() {
                        if let Some(tile) = maybe_tile {
                            if tile.grid_x == nx && tile.grid_y == ny && tile.material != MaterialType::Space {
                                let transferred = collapsed_stress * 1.0;
                                tile.accumulated_stress += transferred;
                                if tile.accumulated_stress > tile.material.stress_resistance() {
                                    let damage = tile.accumulated_stress - tile.material.stress_resistance();
                                    tile.health -= damage;
                                    changes = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct CameraTransitionSystem;

impl CameraTransitionSystem {
    /// Smoothly interpolates the viewport properties when spatial zones are entered,
    /// shifting controller input schemes instantly.
    pub fn update(
        world: &mut EcsWorld,
        portals: &[PortalZone],
        dt: f32,
    ) {
        let n = world.positions.len();
        for i in 0..n {
            if let Some(pos) = world.positions[i] {
                // Check if the entity triggered a transition zone
                let mut target_transition = None;
                for portal in portals {
                    if portal.contains(&pos) {
                        target_transition = Some((portal.trigger_to_mode, portal.trigger_to_controller));
                        break;
                    }
                }

                if let Some((tgt_mode, tgt_ctrl)) = target_transition {
                    if let Some(ref mut cc) = world.camera_controllers[i] {
                        if cc.mode != tgt_mode {
                            cc.mode = tgt_mode;
                            cc.controller_type = tgt_ctrl;
                            cc.transition_progress = 0.0;
                        }
                    }
                }
            }

            // Smoothly advance matrix projection interpolations
            if let Some(ref mut cc) = world.camera_controllers[i] {
                if cc.transition_progress < 1.0 {
                    cc.transition_progress = (cc.transition_progress + dt * 2.5).min(1.0);

                    // Interpolate projection viewing offset (simulated 3D matrix change)
                    match cc.mode {
                        CameraMode::SideScroller => {
                            // Interpolate towards 2D lateral view offset
                            cc.position_offset[0] = cc.position_offset[0] * (1.0 - cc.transition_progress);
                            cc.position_offset[1] = cc.position_offset[1] * (1.0 - cc.transition_progress);
                            cc.position_offset[2] = 10.0 * cc.transition_progress + cc.position_offset[2] * (1.0 - cc.transition_progress);
                        }
                        CameraMode::TopDownGrid => {
                            // Interpolate towards orthographic projection tilt [30°, 45°]
                            cc.position_offset[0] = 5.0 * cc.transition_progress + cc.position_offset[0] * (1.0 - cc.transition_progress);
                            cc.position_offset[1] = 8.0 * cc.transition_progress + cc.position_offset[1] * (1.0 - cc.transition_progress);
                            cc.position_offset[2] = 5.0 * cc.transition_progress + cc.position_offset[2] * (1.0 - cc.transition_progress);
                        }
                    }
                }
            }
        }
    }
}

pub struct GuardAiPredictiveSystem;

impl GuardAiPredictiveSystem {
    /// Analyses the player's last inputs stored in history.
    /// If an attack of specific direction is detected after a dodge,
    /// dynamic defense parry weights are automatically boosted in real-time.
    pub fn update(
        world: &mut EcsWorld,
        player_history: &VecDeque<InputFrame>,
    ) {
        // Adjust guard postures dynamically based on sequence detection (fully zero-allocation)
        for maybe_ai in world.guard_ai.iter_mut() {
            if let Some(ai) = maybe_ai {
                // Update local memory of guard directly from history without intermediate Vec
                ai.last_actions.clear();
                for frame in player_history.iter().take(10) {
                    let used_dodge = (frame.action_flags & InputFrame::ACTION_DODGE) != 0;
                    let mut attack_type = 0;
                    if (frame.action_flags & InputFrame::ACTION_ATTACK_LOW_RIGHT) != 0 {
                        attack_type = InputFrame::ACTION_ATTACK_LOW_RIGHT;
                    } else if (frame.action_flags & InputFrame::ACTION_ATTACK_HIGH_LEFT) != 0 {
                        attack_type = InputFrame::ACTION_ATTACK_HIGH_LEFT;
                    }

                    ai.last_actions.push_back(InputSummary {
                        used_dodge,
                        attack_type,
                    });
                }

                // Analyze pattern: check if player frequently dodges and then attacks
                let mut dodge_then_low_right = 0;
                let mut dodge_then_high_left = 0;
                let mut total_patterns = 0;

                for window in ai.last_actions.make_contiguous().windows(2) {
                    if window[0].used_dodge {
                        total_patterns += 1;
                        if window[1].attack_type == InputFrame::ACTION_ATTACK_LOW_RIGHT {
                            dodge_then_low_right += 1;
                        } else if window[1].attack_type == InputFrame::ACTION_ATTACK_HIGH_LEFT {
                            dodge_then_high_left += 1;
                        }
                    }
                }

                // Boost weights in real-time
                if total_patterns > 0 {
                    let lr_ratio = dodge_then_low_right as f32 / total_patterns as f32;
                    let hl_ratio = dodge_then_high_left as f32 / total_patterns as f32;

                    // Standard parry base weights adjusted smoothly
                    ai.parry_weights[0] = 0.2 + lr_ratio * 0.7; // Low Right Defense
                    ai.parry_weights[1] = 0.2 + hl_ratio * 0.7; // High Left Defense
                    ai.parry_weights[2] = 0.1 + (lr_ratio + hl_ratio) * 0.4; // Adaptive counter-attack
                } else {
                    // Reset to baseline weights
                    ai.parry_weights = [0.2, 0.2, 0.1];
                }
            }
        }
    }
}

// ============================================================================
// DRY RUN DEMO (MAIN SUITE)
// ============================================================================

fn main() {
    println!("=== PRINCE OF PERSIA ULTIMATE PROTOTYPE CORE ===");

    // 1. Time-Echo Buffer Initiation
    let mut echo_buffer = CircularInputBuffer::new(600);

    // 2. ECS and Grid Environment Setup
    let mut world = EcsWorld::new();
    let mut nav_map = NavigationMap::new(10, 10);

    // Populate Destructible Tiles in ECS
    for y in 0..10 {
        for x in 0..10 {
            let ent = world.create_entity();
            let material = if x == 5 && y == 5 {
                MaterialType::Stone
            } else if x == 6 && y == 5 {
                MaterialType::Wood
            } else {
                MaterialType::Iron
            };

            world.tiles[ent.id as usize] = Some(TileComponent {
                grid_x: x,
                grid_y: y,
                health: material.max_health(),
                material,
                accumulated_stress: 0.0,
            });

            // Set initially non-navigable where solid tiles exist
            nav_map.set_navigable(x as usize, y as usize, false);
        }
    }

    // Spawn Player Entity
    let player = world.create_entity();
    world.positions[player.id as usize] = Some(Position { x: 4.0, y: 5.0, z: 0.0 });
    world.camera_controllers[player.id as usize] = Some(CameraControllerComponent {
        mode: CameraMode::SideScroller,
        controller_type: ControllerType::Platformer,
        transition_progress: 1.0,
        position_offset: [0.0, 0.0, 10.0],
    });

    // Spawn Echo Entity
    let echo = world.create_entity();
    world.positions[echo.id as usize] = Some(Position { x: 4.0, y: 5.0, z: 0.0 });
    world.echoes[echo.id as usize] = Some(EchoComponent {
        delayed_playback_offset: 180, // 3 seconds delay at 60 FPS
        active_spectral: true,
    });

    // Spawn Guard Entity
    let guard = world.create_entity();
    world.positions[guard.id as usize] = Some(Position { x: 7.0, y: 5.0, z: 0.0 });
    world.guard_ai[guard.id as usize] = Some(GuardAiComponent {
        last_actions: VecDeque::new(),
        parry_weights: [0.2, 0.2, 0.1],
    });

    // Portal Zone Setup
    let portals = vec![PortalZone {
        min_coords: [4.5, 4.5, -1.0],
        max_coords: [5.5, 5.5, 1.0],
        trigger_to_mode: CameraMode::TopDownGrid,
        trigger_to_controller: ControllerType::GridMovement,
    }];

    // -------------------------------------------------------------
    // TICK LOOP DRY-RUN DEMO (Zero allocation in critical section)
    // -------------------------------------------------------------
    println!("\n--- Simulating 5 Active Gameplay Ticks ---");

    for tick in 0..5 {
        println!("\n[Tick #{}]", tick);

        // Simulated inputs from player (alternating sequences to trigger AI adaptations)
        let movement = [1.0, 0.0];
        let action = if tick % 2 == 0 {
            InputFrame::ACTION_DODGE
        } else {
            InputFrame::ACTION_ATTACK_LOW_RIGHT
        };

        let current_input_frame = InputFrame {
            tick_id: tick as u64,
            movement_vector: movement,
            action_flags: action,
        };

        // Record inputs to Time-Echo buffer
        echo_buffer.push(current_input_frame);

        // Update player position based on movement vector (dummy physics step)
        if let Some(ref mut pos) = world.positions[player.id as usize] {
            pos.x += current_input_frame.movement_vector[0] * 0.1;
            pos.y += current_input_frame.movement_vector[1] * 0.1;
            println!("  Player Position: ({:.2}, {:.2})", pos.x, pos.y);
        }

        // Playback system updates Echo position based on delay
        if let Some(ref mut echo_pos) = world.positions[echo.id as usize] {
            if let Some(delayed_frame) = echo_buffer.get_delayed(2) {
                echo_pos.x += delayed_frame.movement_vector[0] * 0.1;
                echo_pos.y += delayed_frame.movement_vector[1] * 0.1;
                println!("  Time-Echo Spectral position (2-ticks delay): ({:.2}, {:.2})", echo_pos.x, echo_pos.y);
            }
        }

        // Environmental impact damage on tile (5, 5) - e.g., Player hits a Stone pillar
        let impacts = if tick == 1 {
            println!("  ! Heavy impact registered on Stone tile (5, 5)");
            vec![(5, 5, 160.0)] // 160 units of damage, Stone max_health is 150.0
        } else {
            vec![]
        };

        // Run system updates
        StressPropagationSystem::update(&mut world, &mut nav_map, &impacts);
        CameraTransitionSystem::update(&mut world, &portals, 0.016);
        GuardAiPredictiveSystem::update(&mut world, &echo_buffer.buffer);

        // Check if Stone (5, 5) collapsed and caused Wood (6, 5) to collapse due to stress
        for maybe_tile in world.tiles.iter() {
            if let Some(tile) = maybe_tile {
                if (tile.grid_x == 5 || tile.grid_x == 6) && tile.grid_y == 5 {
                    println!(
                        "  Tile ({}, {}): Material: {:?}, Health: {:.1}, Stress Load: {:.1}",
                        tile.grid_x, tile.grid_y, tile.material, tile.health, tile.accumulated_stress
                    );
                }
            }
        }

        // Print Guard Parry Weights (Learning and adaptation proof)
        if let Some(ref ai) = world.guard_ai[guard.id as usize] {
            println!(
                "  Guard Parry Weights: Low-Right: {:.2}, High-Left: {:.2}",
                ai.parry_weights[0], ai.parry_weights[1]
            );
        }

        // Print current Camera mode of the player
        if let Some(ref cc) = world.camera_controllers[player.id as usize] {
            println!(
                "  Camera State: Mode: {:?}, Controller Scheme: {:?}, Transition progress: {:.2}",
                cc.mode, cc.controller_type, cc.transition_progress
            );
        }
    }
}

// ============================================================================
// SYSTEM UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_echo_buffer() {
        let mut buffer = CircularInputBuffer::new(5);
        for i in 0..10 {
            buffer.push(InputFrame {
                tick_id: i,
                movement_vector: [i as f32, 0.0],
                action_flags: InputFrame::ACTION_NONE,
            });
        }

        assert_eq!(buffer.len(), 5);
        // The front should be the oldest retained frame, which is tick_id 5
        assert_eq!(buffer.get(0).unwrap().tick_id, 5);
        assert_eq!(buffer.get_delayed(0).unwrap().tick_id, 9);
        assert_eq!(buffer.get_delayed(2).unwrap().tick_id, 7);
        assert_eq!(buffer.get_delayed(10).unwrap().tick_id, 5);
    }

    #[test]
    fn test_stress_propagation_and_navigation_update() {
        let mut world = EcsWorld::new();
        let mut nav_map = NavigationMap::new(10, 10);

        let t1 = world.create_entity();
        world.tiles[t1.id as usize] = Some(TileComponent {
            grid_x: 2,
            grid_y: 2,
            health: MaterialType::Stone.max_health(),
            material: MaterialType::Stone,
            accumulated_stress: 0.0,
        });
        nav_map.set_navigable(2, 2, false);

        let t2 = world.create_entity();
        world.tiles[t2.id as usize] = Some(TileComponent {
            grid_x: 3,
            grid_y: 2,
            health: MaterialType::Wood.max_health(),
            material: MaterialType::Wood,
            accumulated_stress: 0.0,
        });
        nav_map.set_navigable(3, 2, false);

        // Apply heavy impact on Stone (2, 2) that exceeds its max_health (150.0)
        let impacts = vec![(2, 2, 160.0)];
        StressPropagationSystem::update(&mut world, &mut nav_map, &impacts);

        // Stone (2, 2) should collapse, transferring stress to Wood (3, 2).
        // Since Wood's stress_resistance is only 10.0, the transferred load
        // should also trigger the Wood tile's collapse.
        let tile1 = world.tiles[t1.id as usize].unwrap();
        let tile2 = world.tiles[t2.id as usize].unwrap();

        assert_eq!(tile1.material, MaterialType::Space);
        assert_eq!(tile2.material, MaterialType::Space);

        // Dynamic pathfinding map should now be walkable
        assert!(nav_map.is_navigable(2, 2));
        assert!(nav_map.is_navigable(3, 2));
    }

    #[test]
    fn test_camera_mode_and_controller_transition() {
        let mut world = EcsWorld::new();
        let player = world.create_entity();

        world.positions[player.id as usize] = Some(Position { x: 5.0, y: 5.0, z: 0.0 });
        world.camera_controllers[player.id as usize] = Some(CameraControllerComponent {
            mode: CameraMode::SideScroller,
            controller_type: ControllerType::Platformer,
            transition_progress: 1.0,
            position_offset: [0.0, 0.0, 10.0],
        });

        let portals = vec![PortalZone {
            min_coords: [4.5, 4.5, -1.0],
            max_coords: [5.5, 5.5, 1.0],
            trigger_to_mode: CameraMode::TopDownGrid,
            trigger_to_controller: ControllerType::GridMovement,
        }];

        CameraTransitionSystem::update(&mut world, &portals, 0.1);

        let cc = world.camera_controllers[player.id as usize].as_ref().unwrap();
        assert_eq!(cc.mode, CameraMode::TopDownGrid);
        assert_eq!(cc.controller_type, ControllerType::GridMovement);
        assert!(cc.transition_progress < 1.0); // Should be in the middle of smooth transition
    }

    #[test]
    fn test_guard_ai_learning_and_weights() {
        let mut world = EcsWorld::new();
        let guard = world.create_entity();

        world.guard_ai[guard.id as usize] = Some(GuardAiComponent {
            last_actions: VecDeque::new(),
            parry_weights: [0.2, 0.2, 0.1],
        });

        // Simulating sequence where the player consistently attacks low right after dodging
        let mut player_history = VecDeque::new();
        for i in 0..5 {
            // Dodge frame
            player_history.push_back(InputFrame {
                tick_id: i * 2,
                movement_vector: [0.0, 0.0],
                action_flags: InputFrame::ACTION_DODGE,
            });
            // Followed by attack frame
            player_history.push_back(InputFrame {
                tick_id: i * 2 + 1,
                movement_vector: [0.0, 0.0],
                action_flags: InputFrame::ACTION_ATTACK_LOW_RIGHT,
            });
        }

        GuardAiPredictiveSystem::update(&mut world, &player_history);

        let ai = world.guard_ai[guard.id as usize].as_ref().unwrap();
        // Since low right attack happens 100% of the time after dodge in history,
        // low right parry weight should be heavily boosted (from base 0.2 up to ~0.9)
        assert!(ai.parry_weights[0] > 0.8);
        assert_eq!(ai.parry_weights[1], 0.2); // High left remains base
    }
}
