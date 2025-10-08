use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::WireframePlugin;

mod camera;
mod grid;
mod rendering;
mod rule;

use camera::{camera_look, camera_movement, handle_exit, FlyCamera};
use grid::{simulate_step, CellColors, Grid};
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
    // Create the cellular automaton rule - try different rules!
    // let rule = Rule::rule_445();
    let rule = Rule::builder();
    // let rule = Rule::pretty_crystals();  // Try this one - it's visually interesting!
    // let rule = Rule::fancy_snancy();
    // let rule = Rule::expanding_blob();

    println!("Using rule with {} states", rule.states);
    let max_state = rule.states;

    // Initialize grid
    let size = 50;
    let mut grid = Grid::new(size);

    // Spawn dense cluster in center like the reference repo
    grid.spawn_center_cluster(&rule, max_state, 6, 12 * 12 * 12);

    // Create color interpolation info
    let colors = CellColors {
        birth_color: Color::srgb(1.0, 1.0, 0.0),
        death_color: Color::srgb(1.0, 0.0, 0.0),
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
