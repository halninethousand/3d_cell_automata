#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct CellMaterialUniform {
    color: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: CellMaterialUniform;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.clip_position = position_world_to_clip(world_position.xyz);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return material.color;
}
