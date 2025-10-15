use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::WireframePlugin;

mod camera;
mod grid;
mod rendering;
mod rule;

use camera::{camera_look, camera_movement, handle_exit, FlyCamera};
use grid::{simulate_step, CellColors, ColorMethod, Grid};
use rendering::{CellMaterialPlugin, InstanceMaterialData};
use rule::Rule;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CellMaterialPlugin,
            #[cfg(not(target_arch = "wasm32"))]
            WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                simulate_step,
                camera_movement,
                camera_look,
                handle_exit,
                #[cfg(not(target_arch = "wasm32"))]
                camera::toggle_wireframe,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Preset rules from various sources:
    // let rule = Rule::rule_445();           // Classic 4/4/5 rule
    // let rule = Rule::builder();            // Complex expanding structures
    // let rule = Rule::pretty_crystals();    // Crystalline formations
    // let rule = Rule::fancy_snancy();       // Chaotic patterns
    // let rule = Rule::expanding_blob();     // Slowly growing blob

    // Rules from Softology blog (https://softologyblog.wordpress.com/2019/12/28/3d-cellular-automata-3/)
    // let rule = Rule::clouds_1();           // Cloud-like wispy structures
    // let rule = Rule::amoeba();             // Morphing blob organism
    // let rule = Rule::architecture();       // Architectural structures
    // let rule = Rule::brain();              // Brain-like tissue
    // let rule = Rule::builder_2();          // Builder variant
    // let rule = Rule::coral();              // Coral-like branching
    // let rule = Rule::crystal_growth_1();   // Growing crystals
    // let rule = Rule::diamond_growth();     // Diamond-like crystals
    // let rule = Rule::pulse_waves();        // Wave-like pulses
    // let rule = Rule::pyroclastic();        // Explosive volcanic patterns
    // let rule = Rule::spiky_growth();       // Spiky protrusions
    // let rule = Rule::shells();             // Shell-like layers

    // let rule = Rule::vn_pyramid();         // Von Neumann pyramid structure
    let rule = Rule::swapping_structures(); // Constantly morphing patterns
    // let rule = Rule::expand_then_die();    // Explosive growth → collapse
    // let rule = Rule::spikey_growth_complex(); // Complex spikey patterns
    // let rule = Rule::large_lines();        // Large linear structures (35 states!)

    // Rule notation: survival/birth/states/method
    // 4-7/6-8/10/M means: survive with 4-7 neighbors, birth with 6-8, 10 states, Moore
    // let rule = Rule::from_ranges(4, 6, 5, 6, 11, rule::NeighborMethod::Moore);

    println!("Using rule with {} states", rule.states);
    let max_state = rule.states;

    // Initialize grid
    let size = 64;
    let mut grid = Grid::new(size);

    // Spawn dense cluster in center like the reference repo
    grid.spawn_center_cluster(&rule, max_state, 6, 12 * 12 * 12);

    // Create color interpolation info
    // Try different color methods: StateLerp, DistToCenter, Neighbor, Single
    let colors = CellColors {
        birth_color: Color::srgb(1.0, 1.0, 0.0),
        death_color: Color::srgb(1.0, 0.0, 0.0),
        method: ColorMethod::DistToCenter,        // Shows depth/3D structure nicely!
    };

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Build initial instance data from spawned cells
    let instance_data = grid.build_instances(&colors, max_state);

    // Spawn single entity with all instances
    commands.spawn((
        Mesh3d(cube_mesh),
        Transform::IDENTITY,
        Visibility::default(),
        InstanceMaterialData(instance_data),
    ));

    commands.insert_resource(grid);
    commands.insert_resource(rule);
    commands.insert_resource(colors);

    // Camera looks at origin (grid is centered around origin now)
    let camera_pos = Vec3::new(50.0, 50.0, 120.0);
    let target = Vec3::ZERO;
    let direction = (target - camera_pos).normalize();

    // Calculate yaw and pitch from the direction vector
    let yaw = -direction.x.atan2(-direction.z);
    let pitch = direction.y.asin();

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(camera_pos.x, camera_pos.y, camera_pos.z).looking_at(target, Vec3::Y),
        FlyCamera::new(50.0, 0.0005, pitch, yaw),
    ));
}
