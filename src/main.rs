use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{lifetimeless::*, SystemParamItem},
    input::mouse::MouseMotion,
    pbr::{MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline,
            TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        sync_world::MainEntity,
        view::ExtractedView,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
    window::{CursorGrabMode, CursorOptions},
};
use bevy_mesh::VertexBufferLayout;
use bytemuck::{Pod, Zeroable};
use rand::Rng;
use std::mem::size_of;

#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};

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
                toggle_wireframe,
            ),
        )
        .run();
}

#[derive(Resource)]
struct Grid {
    cells: Vec<Vec<Vec<u8>>>,
    size: usize,
    max_state: u8,  // Maximum state value (newly born cells start here)
}

#[derive(Resource)]
struct CellColors {
    birth_color: Color,   // Color for newly born cells (max_state)
    death_color: Color,   // Color for dying cells (state 1)
    materials_cache: Vec<Handle<StandardMaterial>>,  // No longer used with instancing
}

// Instance data that will be sent to the GPU
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct InstanceData {
    position: Vec3,
    scale: f32,
    color: [f32; 4],
}

// Component that holds all instance data
#[derive(Component, Deref)]
struct InstanceMaterialData(Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
    type QueryData = &'static InstanceMaterialData;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(InstanceMaterialData(item.0.clone()))
    }
}

// GPU buffer that holds instance data
#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

// System that prepares instance buffers for rendering
fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstanceMaterialData)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instance_data.0.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.0.len(),
        });
    }
}

// Custom render pipeline for instanced cells
#[derive(Resource)]
struct CellPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

fn init_cell_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    commands.insert_resource(CellPipeline {
        shader: asset_server.load("shaders/instancing.wgsl"),
        mesh_pipeline: mesh_pipeline.clone(),
    });
}

impl SpecializedMeshPipeline for CellPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &bevy_mesh::MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();

        // Create vertex buffer layout for instance data
        let instance_attrs = [
            // Position + scale
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 3,
            },
            // Color
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: VertexFormat::Float32x4.size(),
                shader_location: 4,
            },
        ];

        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: instance_attrs.to_vec(),
        });

        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();

        Ok(descriptor)
    }
}

// Custom draw command for instanced rendering
struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMeshInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<InstanceBuffer>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w InstanceBuffer>,
        (meshes, render_mesh_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // A borrow check workaround.
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) =
            mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id)
        else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) =
                    mesh_allocator.mesh_index_slice(&mesh_instance.mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_buffer.length as u32,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    DrawMeshInstanced,
);

// Queue system to add our entities to the render phase
fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CellPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CellPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<(Entity, &MainEntity), With<InstanceMaterialData>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(&ExtractedView, &Msaa)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawCustom>();

    for (view, msaa) in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();

        for (entity, main_entity) in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .unwrap();
            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw_custom,
                distance: rangefinder.distance_translation(&mesh_instance.translation),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

// Plugin that sets up our custom rendering pipeline
struct CellMaterialPlugin;

impl Plugin for CellMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<SpecializedMeshPipelines<CellPipeline>>()
            .add_systems(RenderStartup, init_cell_pipeline)
            .add_systems(
                Render,
                (
                    queue_custom.in_set(RenderSystems::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

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
) {
    let size = 50;  // Much bigger grid
    let max_state = 5;  // Rule 445 uses 5 states
    let mut grid_cells = vec![vec![vec![0; size]; size]; size];

    // Spawn dense cluster in center like the reference repo
    let mut rng = rand::rng();
    let center = size as i32 / 2;
    let radius = 6;
    let amount = 12 * 12 * 12;  // 1,728 cells

    for _ in 0..amount {
        let x = (center + rng.random_range(-radius..=radius)) as usize;
        let y = (center + rng.random_range(-radius..=radius)) as usize;
        let z = (center + rng.random_range(-radius..=radius)) as usize;

        // Make sure we're in bounds
        if x < size && y < size && z < size {
            grid_cells[x][y][z] = max_state;
        }
    }

    // Create color interpolation info
    let birth_color = Color::srgb(0.67, 1.0, 1.0); // Cyan-ish for max state
    let death_color = Color::srgb(0.1, 0.1, 0.1);  // Very dark for dying state

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let grid_center = Vec3::new(
        (size - 1) as f32 * 1.0 / 2.0,
        (size - 1) as f32 * 1.0 / 2.0,
        (size - 1) as f32 * 1.0 / 2.0,
    );

    // Build instance data for all living cells
    let mut instance_data = Vec::new();
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let state = grid_cells[x][y][z];
                if state > 0 {
                    // Interpolate color based on state
                    let t = state as f32 / max_state as f32;
                    let color = Color::srgb(
                        death_color.to_srgba().red * (1.0 - t) + birth_color.to_srgba().red * t,
                        death_color.to_srgba().green * (1.0 - t) + birth_color.to_srgba().green * t,
                        death_color.to_srgba().blue * (1.0 - t) + birth_color.to_srgba().blue * t,
                    );

                    let position = Vec3::new(
                        x as f32 * 1.0 - grid_center.x,
                        y as f32 * 1.0 - grid_center.y,
                        z as f32 * 1.0 - grid_center.z,
                    );

                    instance_data.push(InstanceData {
                        position,
                        scale: 1.0,
                        color: color.to_srgba().to_f32_array(),
                    });
                }
            }
        }
    }

    // Spawn single entity with all instances
    commands.spawn((
        Mesh3d(cube_mesh),
        Transform::IDENTITY,
        Visibility::default(),
        InstanceMaterialData(instance_data),
    ));

    commands.insert_resource(Grid {
        cells: grid_cells,
        size,
        max_state,
    });

    commands.insert_resource(CellColors {
        birth_color,
        death_color,
        materials_cache: Vec::new(), // No longer needed with instancing
    });

    // Camera looks at origin (grid is centered around origin now)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(50.0, 50.0, 120.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCamera {
            speed: 50.0,
            sensitivity: 0.0005,
            pitch: 0.0,
            yaw: -90.0_f32.to_radians(),
        },
    ));
}

fn simulate_step(
    mut grid: ResMut<Grid>,
    colors: Res<CellColors>,
    mut instance_query: Query<&mut InstanceMaterialData>,
    time: Res<Time>,
    mut last_update: Local<f32>,
) {
    if time.elapsed_secs() - *last_update < 0.5 {
        return;
    }
    *last_update = time.elapsed_secs();

    let size = grid.size;
    let max_state = grid.max_state;
    let mut new_grid = vec![vec![vec![0; size]; size]; size];

    // Rule 445: survival=4, birth=4, states=5
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let neighbors = count_neighbors(&grid.cells, x, y, z);
                let current = grid.cells[x][y][z];

                new_grid[x][y][z] = if current == 0 {
                    // Dead cell - can be born if exactly 4 neighbors
                    if neighbors == 4 {
                        max_state  // Born at maximum state
                    } else {
                        0
                    }
                } else {
                    // Living cell (state 1 to max_state)
                    if current == 1 {
                        // At state 1, check survival rule
                        if neighbors == 4 {
                            max_state  // Survived, refresh to max_state
                        } else {
                            0  // Die
                        }
                    } else {
                        // States 2-max_state: decrement (fade toward death)
                        current - 1
                    }
                };
            }
        }
    }

    grid.cells = new_grid;

    // Rebuild instance data for all living cells
    let grid_center = Vec3::new(
        (size - 1) as f32 * 1.0 / 2.0,
        (size - 1) as f32 * 1.0 / 2.0,
        (size - 1) as f32 * 1.0 / 2.0,
    );

    let mut instance_data = Vec::new();
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let state = grid.cells[x][y][z];
                if state > 0 {
                    // Interpolate color based on state
                    let t = state as f32 / max_state as f32;
                    let color = Color::srgb(
                        colors.death_color.to_srgba().red * (1.0 - t) + colors.birth_color.to_srgba().red * t,
                        colors.death_color.to_srgba().green * (1.0 - t) + colors.birth_color.to_srgba().green * t,
                        colors.death_color.to_srgba().blue * (1.0 - t) + colors.birth_color.to_srgba().blue * t,
                    );

                    let position = Vec3::new(
                        x as f32 * 1.0 - grid_center.x,
                        y as f32 * 1.0 - grid_center.y,
                        z as f32 * 1.0 - grid_center.z,
                    );

                    instance_data.push(InstanceData {
                        position,
                        scale: 1.0,
                        color: color.to_srgba().to_f32_array(),
                    });
                }
            }
        }
    }

    // Update the instance buffer
    if let Ok(mut instances) = instance_query.single_mut() {
        instances.0 = instance_data;
    }
}

fn count_neighbors(grid: &[Vec<Vec<u8>>], x: usize, y: usize, z: usize) -> usize {
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
                    && grid[nx as usize][ny as usize][nz as usize] >= 1  // Any living state counts
                {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Movement with WASD + Space (up) / LShift (down)
fn camera_movement(
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
fn camera_look(
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
fn handle_exit(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        wireframe_config.global = !wireframe_config.global;
    }
}
