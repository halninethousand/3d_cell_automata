use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, CursorOptions};

#[derive(Component)]
pub struct FlyCamera {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 50.0,
            sensitivity: 0.0005,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl FlyCamera {
    pub fn new(speed: f32, sensitivity: f32, pitch: f32, yaw: f32) -> Self {
        Self {
            speed,
            sensitivity,
            pitch,
            yaw,
        }
    }
}

/// Movement with WASD + Space (up) / LShift (down)
pub fn camera_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &FlyCamera)>,
) {
    if let Ok((mut transform, cam)) = query.single_mut() {
        let mut direction = Vec3::ZERO;

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

/// Mouse look with cursor grab
pub fn camera_look(
    mut motion_events: MessageReader<MouseMotion>,
    windows: Query<&mut Window>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
) {
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

        // Allow full 360 degree vertical rotation - no clamp
        // flycam.pitch = flycam.pitch.clamp(-1.54, 1.54);

        let yaw_rotation = Quat::from_rotation_y(flycam.yaw);
        let pitch_rotation = Quat::from_rotation_x(flycam.pitch);
        transform.rotation = yaw_rotation * pitch_rotation;
    }

    // Lock cursor
    if let Ok(_window) = windows.single() {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
}

/// Press Escape to exit
pub fn handle_exit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn toggle_wireframe(
    mut wireframe_config: ResMut<bevy::pbr::wireframe::WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        wireframe_config.global = !wireframe_config.global;
    }
}
