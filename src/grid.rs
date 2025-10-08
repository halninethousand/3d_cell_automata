use bevy::prelude::*;
use bevy::math::IVec3;
use rand::Rng;
use crate::rule::Rule;
use crate::rendering::InstanceMaterialData;

/// Cell data with persistent neighbor count for fast simulation
#[derive(Clone, Copy)]
struct Cell {
    value: u8,      // Current state (0 = dead, 1..max_state = alive)
    neighbors: u8,  // Cached count of neighbors at max_state
}

impl Cell {
    fn is_dead(self) -> bool {
        self.value == 0
    }
}

#[derive(Resource)]
pub struct Grid {
    cells: Vec<Cell>,  // Flat 1D array for cache efficiency
    pub size: i32,     // Grid size in each dimension
}

impl Grid {
    pub fn new(size: i32) -> Self {
        let total = (size * size * size) as usize;
        Self {
            cells: vec![Cell { value: 0, neighbors: 0 }; total],
            size,
        }
    }

    /// Convert 3D position to 1D index
    #[inline]
    fn pos_to_index(&self, pos: IVec3) -> usize {
        let x = pos.x as usize;
        let y = pos.y as usize;
        let z = pos.z as usize;
        let size = self.size as usize;
        x + y * size + z * size * size
    }

    /// Convert 1D index to 3D position
    #[inline]
    fn index_to_pos(&self, index: usize) -> IVec3 {
        let size = self.size;
        IVec3::new(
            (index as i32) % size,
            (index as i32) / size % size,
            (index as i32) / size / size
        )
    }

    /// Wrap position to handle toroidal boundaries
    #[inline]
    fn wrap(&self, pos: IVec3) -> IVec3 {
        let size = self.size;
        IVec3::new(
            ((pos.x % size) + size) % size,
            ((pos.y % size) + size) % size,
            ((pos.z % size) + size) % size,
        )
    }

    /// Update neighbor counts when a cell transitions to/from max_state
    fn update_neighbors(&mut self, rule: &Rule, index: usize, increment: bool) {
        let pos = self.index_to_pos(index);

        for &offset in rule.neighbor_method.get_neighbors() {
            let neighbor_pos = self.wrap(pos + offset);
            let neighbor_index = self.pos_to_index(neighbor_pos);

            if increment {
                self.cells[neighbor_index].neighbors += 1;
            } else {
                self.cells[neighbor_index].neighbors -= 1;
            }
        }
    }

    /// Spawn a dense cluster of cells in the center
    pub fn spawn_center_cluster(&mut self, rule: &Rule, max_state: u8, radius: i32, amount: usize) {
        let mut rng = rand::rng();
        let center = self.size / 2;

        for _ in 0..amount {
            let pos = IVec3::new(
                center + rng.random_range(-radius..=radius),
                center + rng.random_range(-radius..=radius),
                center + rng.random_range(-radius..=radius),
            );

            let wrapped_pos = self.wrap(pos);
            let index = self.pos_to_index(wrapped_pos);

            if self.cells[index].is_dead() {
                self.cells[index].value = max_state;
                // Update neighbor counts for surrounding cells
                self.update_neighbors(rule, index, true);
            }
        }
    }

    /// Build instance data for rendering
    pub fn build_instances(&self, colors: &CellColors, max_state: u8) -> Vec<crate::rendering::InstanceData> {
        let grid_center = Vec3::splat((self.size - 1) as f32 * 0.5);
        let mut instance_data = Vec::new();

        for (index, cell) in self.cells.iter().enumerate() {
            if cell.value > 0 {
                let pos = self.index_to_pos(index);

                // Interpolate color based on state
                let t = cell.value as f32 / max_state as f32;
                let color = Color::srgb(
                    colors.death_color.to_srgba().red * (1.0 - t) + colors.birth_color.to_srgba().red * t,
                    colors.death_color.to_srgba().green * (1.0 - t) + colors.birth_color.to_srgba().green * t,
                    colors.death_color.to_srgba().blue * (1.0 - t) + colors.birth_color.to_srgba().blue * t,
                );

                let position = pos.as_vec3() - grid_center;

                instance_data.push(crate::rendering::InstanceData {
                    position,
                    scale: 1.0,
                    color: color.to_srgba().to_f32_array(),
                });
            }
        }

        instance_data
    }

    /// Count living cells
    pub fn cell_count(&self) -> usize {
        self.cells.iter().filter(|c| !c.is_dead()).count()
    }
}

#[derive(Resource)]
pub struct CellColors {
    pub birth_color: Color,
    pub death_color: Color,
}

impl Default for CellColors {
    fn default() -> Self {
        Self {
            birth_color: Color::srgb(1.0, 0.0, 0.0),
            death_color: Color::srgb(0.0, 1.0, 0.0),
        }
    }
}

/// Optimized simulation step using persistent neighbor counts
pub fn simulate_step(
    mut grid: ResMut<Grid>,
    rule: Res<Rule>,
    colors: Res<CellColors>,
    mut instance_query: Query<&mut InstanceMaterialData>,
    time: Res<Time>,
    mut last_update: Local<f32>,
) {
    // Adjust this to control simulation speed (in seconds between updates)
    const UPDATE_INTERVAL: f32 = 0.05;  // 10 updates/sec (0.0 = as fast as possible)

    if UPDATE_INTERVAL > 0.0 && time.elapsed_secs() - *last_update < UPDATE_INTERVAL {
        return;
    }
    *last_update = time.elapsed_secs();

    let frame_start = std::time::Instant::now();
    let max_state = rule.states;

    // Track which cells spawned (transitioned to max_state) or died (left max_state)
    let mut spawns = Vec::new();
    let mut deaths = Vec::new();

    // === PHASE 1: Update cell values ===
    let phase1_start = std::time::Instant::now();
    for (index, cell) in grid.cells.iter_mut().enumerate() {
        if cell.is_dead() {
            // Dead cell - check birth rule using CACHED neighbor count
            if rule.should_birth(cell.neighbors) {
                cell.value = max_state;
                spawns.push(index);
            }
        } else {
            // Living cell
            // Only cells at max_state can survive if they meet the survival rule
            if cell.value < max_state || !rule.should_survive(cell.neighbors) {
                // Track if this cell is leaving max_state (affects neighbor counts)
                if cell.value == max_state {
                    deaths.push(index);
                }
                // Decay
                cell.value -= 1;
            }
        }
    }
    let phase1_time = phase1_start.elapsed();

    // === PHASE 2: Update neighbor counts ===
    let phase2_start = std::time::Instant::now();
    for index in spawns.iter() {
        grid.update_neighbors(&rule, *index, true);
    }
    for index in deaths.iter() {
        grid.update_neighbors(&rule, *index, false);
    }
    let phase2_time = phase2_start.elapsed();

    // === PHASE 3: Rebuild instance data ===
    let phase3_start = std::time::Instant::now();
    let instance_data = grid.build_instances(&colors, max_state);
    let phase3_time = phase3_start.elapsed();

    // === PHASE 4: Update GPU buffer ===
    let phase4_start = std::time::Instant::now();
    if let Ok(mut instances) = instance_query.single_mut() {
        instances.0 = instance_data;
    }
    let phase4_time = phase4_start.elapsed();

    let total_time = frame_start.elapsed();
    let living_cells = grid.cells.iter().filter(|c| !c.is_dead()).count();

    // Calculate FPS based on actual elapsed time since last frame
    let delta_secs = time.delta_secs();
    let fps = if delta_secs > 0.0 { 1.0 / delta_secs } else { 0.0 };

    // Print performance stats every update
    println!("=== Performance Profile ({:.0} FPS) ===", fps);
    println!("Total:      {:6.2}ms", total_time.as_secs_f64() * 1000.0);
    println!("Phase 1:    {:6.2}ms  (update {} cells)", phase1_time.as_secs_f64() * 1000.0, grid.cells.len());
    println!("Phase 2:    {:6.2}ms  (update neighbors: {} spawns, {} deaths)",
             phase2_time.as_secs_f64() * 1000.0, spawns.len(), deaths.len());
    println!("Phase 3:    {:6.2}ms  (build {} instances)", phase3_time.as_secs_f64() * 1000.0, living_cells);
    println!("Phase 4:    {:6.2}ms  (upload to GPU)", phase4_time.as_secs_f64() * 1000.0);
    println!("Frame time: {:6.2}ms (render + overhead)", delta_secs * 1000.0);
    println!();
}
