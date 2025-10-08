#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    // Instance attributes
    @location(3) i_pos_scale: vec4<f32>,  // xyz = position, w = scale
    @location(4) i_color: vec4<f32>,      // rgba = color
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // Extract instance data
    let instance_pos = vertex.i_pos_scale.xyz;
    let instance_scale = vertex.i_pos_scale.w;

    // Apply scale and position to vertex
    let scaled_pos = vertex.position * instance_scale;
    let world_position = scaled_pos + instance_pos;

    // Transform to clip space
    out.clip_position = position_world_to_clip(world_position);
    out.color = vertex.i_color;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
