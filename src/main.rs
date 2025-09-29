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
                (camera_movement, camera_look, toggle_cursor_unlock)
            ),
        )
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

#[derive(Component)]
struct Cell {
    x: usize,
    y: usize,
    z: usize,
}

#[derive(Component)]
struct FlyCamera {
    speed: f32,
    sensitivity: f32,
    pitch: f32,
    yaw: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // let debug_material = materials.add(StandardMaterial {
    //     base_color: Color::srgb(0.63, 1.0, 0.0),
    //     ..default()
    // });

    let size = 10;

    let mut grid = vec![vec![vec![0; size]; size]; size];

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cube_material = materials.add(Color::srgb(0.63, 1.0, 0.0));

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                // Mark this cell as alive
                grid[x][y][z] = 1;

                // Spawn cube in world
                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(cube_material.clone()),
                    Transform::from_xyz(
                        x as f32 * 1.2,
                        y as f32 * 1.2,
                        z as f32 * 1.2,
                    ),
                    Cell { x, y, z }, // mark which grid cell this entity belongs to
                ));
            }
        }
    }


    // commands.spawn((
    //     PointLight {
    //         shadows_enabled: true,
    //         intensity: 10_000_000.,
    //         range: 100.0,
    //         shadow_depth_bias: 0.2,
    //         ..default()
    //     },
    //     Transform::from_xyz(8.0, 16.0, 8.0),
    // ));
    commands.spawn((
        AmbientLight {
            color: Color::WHITE,
            brightness: 10.0,
            affects_lightmapped_meshes: true,
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // commands.insert_resource(AmbientLight {
    //     color: Color::WHITE,
    //     brightness: 1.0,
    //     affects_lightmapped_meshes: true,
    // });


    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
    ));

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
    mut motion_events: EventReader<MouseMotion>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
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

    // lock & hide cursor for the primary window
    if let Ok(mut window) = windows.single_mut() {
        window.cursor_options = CursorOptions {
            visible: false,
            grab_mode: CursorGrabMode::Locked,
            ..default()
        };
    }
}

/// Press Escape to toggle cursor lock/visibility
fn toggle_cursor_unlock(keys: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = windows.single_mut() {
            let currently_locked = match window.cursor_options.grab_mode {
                CursorGrabMode::Locked | CursorGrabMode::Confined => true,
                CursorGrabMode::None => false,
            };
            if currently_locked {
                window.cursor_options = CursorOptions {
                    visible: true,
                    grab_mode: CursorGrabMode::None,
                    ..default()
                };
            } else {
                window.cursor_options = CursorOptions {
                    visible: false,
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                };
            }
        }
    }
}
