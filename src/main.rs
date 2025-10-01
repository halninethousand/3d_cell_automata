//! Here we use shape primitives to generate meshes for 3d objects as well as attaching a runtime-generated patterned texture to each 3d object.
//!
//! "Shape primitives" here are just the mathematical definition of certain shapes, they're not meshes on their own! A sphere with radius `1.0` can be defined with [`Sphere::new(1.0)`][Sphere::new] but all this does is store the radius. So we need to turn these descriptions of shapes into meshes.
//!
//! While a shape is not a mesh, turning it into one in Bevy is easy. In this example we call [`meshes.add(/* Shape here! */)`][Assets<A>::add] on the shape, which works because the [`Assets<A>::add`] method takes anything that can be turned into the asset type it stores. There's an implementation for [`From`] on shape primitives into [`Mesh`], so that will get called internally by [`Assets<A>::add`].
//!
//! [`Extrusion`] lets us turn 2D shape primitives into versions of those shapes that have volume by extruding them. A 1x1 square that gets wrapped in this with an extrusion depth of 2 will give us a rectangular prism of size 1x1x2, but here we're just extruding these 2d shapes by depth 1.
//!
//! The material applied to these shapes is a texture that we generate at run time by looping through a "palette" of RGBA values (stored adjacent to each other in the array) and writing values to positions in another array that represents the buffer for an 8x8 texture. This texture is then registered with the assets system just one time, with that [`Handle<StandardMaterial>`] then applied to all the shapes in this example.
//!
//! The mesh and material are [`Handle<Mesh>`] and [`Handle<StandardMaterial>`] at the moment, neither of which implement `Component` on their own. Handles are put behind "newtypes" to prevent ambiguity, as some entities might want to have handles to meshes (or images, or materials etc.) for different purposes! All we need to do to make them rendering-relevant components is wrap the mesh handle and the material handle in [`Mesh3d`] and [`MeshMaterial3d`] respectively.
//!
//! You can toggle wireframes with the space bar except on wasm. Wasm does not support3
//! `POLYGON_MODE_LINE` on the gpu.

use std::f32::consts::PI;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::basic::SILVER,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy::app::AppExit;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            #[cfg(not(target_arch = "wasm32"))]
            WireframePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                #[cfg(not(target_arch = "wasm32"))]
                toggle_wireframe,
                (camera_movement, camera_look, handle_exit),
                simulate_step,
            ),
        )
        .run();
}

#[derive(Resource)]
struct Grid {
    cells: Vec<Vec<Vec<i32>>>,
    size: usize,
}

#[derive(Resource)]
struct SimulationTimer {
    timer: Timer,
}

enum Rule {
    Single(u8),
    Range(std::ops::RangeInclusive<u8>),
    Singles(u8),
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

#[derive(Component)]
struct Cell {
    x: usize,
    y: usize,
    z: usize,
    state: u8,
}

#[derive(Component)]
struct FlyCamera {
    speed: f32,
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}


fn simulate_step(
    time: Res<Time>,
    mut timer: ResMut<SimulationTimer>,
    mut grid: ResMut<Grid>,
    mut cell_query: Query<(&mut Cell, &mut MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !timer.timer.tick(time.delta()).just_finished() {
        return;
    }

    let size = grid.size;
    let mut new_grid = vec![vec![vec![0; size]; size]; size];

    // Calculate next state for each cell
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let neighbors = count_alive_neighbors(&grid.cells, x, y, z);
                let current_state = grid.cells[x][y][z];

                // 3D Conway's Game of Life rules (more permissive for 3D)
                // Alive cells survive with 4-6 neighbors, dead cells born with 4-5 neighbors
                new_grid[x][y][z] = match current_state {
                    1 => if neighbors >= 4 && neighbors <= 6 { 1 } else { 0 }, // alive cell
                    0 => if neighbors >= 4 && neighbors <= 5 { 1 } else { 0 }, // dead cell
                    _ => 0,
                };
            }
        }
    }

    // Update the grid
    grid.cells = new_grid;

    for (mut cell, mut material_handle) in cell_query.iter_mut() {
        let is_alive = grid.cells[cell.x][cell.y][cell.z] == 1;

        if is_alive {
            // Green for alive cells
            *material_handle = MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0)));
        } else {
            // Red for dead cells (or make them invisible)
            *material_handle = MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0)));
        }
    }
}

fn count_alive_neighbors(grid: &[Vec<Vec<i32>>], x: usize, y: usize, z: usize) -> usize {
    let size = grid.len();
    let mut count = 0;

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }

                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                let nz = z as i32 + dz;

                if nx >= 0 && ny >= 0 && nz >= 0
                    && (nx as usize) < size
                    && (ny as usize) < size
                    && (nz as usize) < size
                {
                    if grid[nx as usize][ny as usize][nz as usize] == 1 {
                        count += 1;
                    }
                }
            }
        }
    }

    count
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = 10;
    let mut grid_cells = vec![vec![vec![0; size]; size]; size];

    // Create truly random initial pattern
    let mut rng = rand::thread_rng();
    let alive_probability = 0.15; // 15% chance for each cell to be alive

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                if rng.gen::<f64>() < alive_probability {
                    grid_cells[x][y][z] = 1;
                }
            }
        }
    }

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_material = materials.add(Color::srgb(0.63, 1.0, 0.0));

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let is_alive = grid_cells[x][y][z] == 1;
                let material = if is_alive {
                    materials.add(Color::srgb(0.0, 1.0, 0.0)) // Green for alive
                } else {
                    materials.add(Color::srgb(1.0, 0.0, 0.0)) // Red for dead
                };

                // Spawn cube in world
                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_xyz(
                        x as f32 * 1.2,
                        y as f32 * 1.2,
                        z as f32 * 1.2,
                    ),
                    Cell { x, y, z, state: grid_cells[x][y][z] as u8 },
                ));
            }
        }
    }

    // Insert the grid as a resource
    commands.insert_resource(Grid {
        cells: grid_cells,
        size,
    });

    // Insert simulation timer (runs every 1 second)
    commands.insert_resource(SimulationTimer {
        timer: Timer::from_seconds(1.0, TimerMode::Repeating),
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            shadows_enabled: false,
            color: Color::WHITE,
            shadow_depth_bias: 1.0,
            shadow_normal_bias: 1.0,
            affects_lightmapped_mesh_diffuse: false,
        },
        Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::ZERO),
    ));

    // // ground plane
    // commands.spawn((
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
    //     MeshMaterial3d(materials.add(Color::from(SILVER))),
    // ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCamera {
            speed: 10.0,
            sensitivity: 0.002,
            pitch: 0.0,
            yaw: -90.0_f32.to_radians(), // facing -Z by default
        },
    ));

    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Text::new("Press space to toggle wireframes"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(22.0), 
            left: Val::Px(22.0),
            ..default()
        },
    ));
}

// fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
//     for mut transform in &mut query {
//         transform.rotate_y(time.delta_secs() / 2.);
//     }
// }


#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        wireframe_config.global = !wireframe_config.global;
    }
}


/// Movement with WASD + Space (up) / LShift (down)
fn camera_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &FlyCamera)>,
) {
    if let Ok((mut transform, cam)) = query.single_mut() {
        let mut direction = Vec3::ZERO;

        // forward/back/right vectors relative to the camera orientation
        let forward = transform.forward();
        let right = transform.right();

        if keys.pressed(KeyCode::KeyW) {
            direction += *forward;
        }
        if keys.pressed(KeyCode::KeyS) {
            direction -= *forward;
        }
        if keys.pressed(KeyCode::KeyA) {
            direction -= *right;
        }
        if keys.pressed(KeyCode::KeyD) {
            direction += *right;
        }
        if keys.pressed(KeyCode::Space) {
            direction += Vec3::Y;
        }
        if keys.pressed(KeyCode::ShiftLeft) {
            direction -= Vec3::Y;
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * cam.speed * time.delta_secs();
        }
    }
}

/// Mouse look. Uses MouseMotion events and writes to the camera's rotation.
/// Also grabs & hides the cursor while there is mouse motion (and sets it initially).
fn camera_look(
    mut motion_events: MessageReader<MouseMotion>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
) {
    // Accumulate mouse delta for the frame
    let mut delta = Vec2::ZERO;
    for ev in motion_events.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    if let Ok((mut transform, mut flycam)) = query.single_mut() {
        flycam.yaw -= delta.x * flycam.sensitivity;
        flycam.pitch -= delta.y * flycam.sensitivity;

        // clamp pitch so camera doesn't flip
        flycam.pitch = flycam.pitch.clamp(-1.54, 1.54);

        let yaw_rotation = Quat::from_rotation_y(flycam.yaw);
        let pitch_rotation = Quat::from_rotation_x(flycam.pitch);
        transform.rotation = yaw_rotation * pitch_rotation;
    }

    // lock & hide cursor
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;
}

/// Press Escape to exit the program
fn handle_exit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
