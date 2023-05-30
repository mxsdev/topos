struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) dims: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) rounding: f32,
    @location(4) depth: f32,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4<f32>,
    @location(0) dims: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct Params {
    screen_resolution: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> params: Params;

@vertex
fn vs_main(
    vertex_in: VertexInput
) -> VertexOutput {
    var vertex_out: VertexOutput;

    vertex_out.color = vertex_in.color;
    vertex_out.dims = vertex_in.dims;

    vertex_out.position = vec4<f32>(
        2.0 * vertex_in.pos / vec2<f32>(params.screen_resolution) - 1.0,
        vertex_in.depth,
        1.0
    );

    vertex_out.position.y *= -1.;

    return vertex_out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}