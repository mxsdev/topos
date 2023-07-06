struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<u32>,
    @location(2) color: vec4<f32>,
    @location(3) content_type: u32,
    @location(4) depth: f32,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) content_type: u32,
};

struct Params {
    screen_resolution: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> params: Params;

@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

@vertex
fn vs_main(
    vertex_in: VertexInput
) -> VertexOutput {
    var vertex_out: VertexOutput;

    vertex_out.color = vertex_in.color;

    let texDims = textureDimensions(atlas_texture);

    vertex_out.uv = vec2<f32>(vertex_in.uv) / vec2<f32>(texDims);

    vertex_out.position = vec4<f32>(
        2.0 * vertex_in.pos / vec2<f32>(params.screen_resolution) - 1.0,
        vertex_in.depth,
        1.0
    );

    vertex_out.position.y *= -1.;

    // let v = vertex_in.vertex_idx % 4u;

    // switch v {
    //     case 0u: {
    //         vertex_out.uv = vec2<f32>(0.0, 0.0);
    //     }

    //     case 1u: {
    //         vertex_out.uv = vec2<f32>(1.0, 0.0);
    //     }

    //     case 2u: {
    //         vertex_out.uv = vec2<f32>(0.0, 1.0);
    //     }

    //     case 3u: {
    //         vertex_out.uv = vec2<f32>(1.0, 1.0);
    //     }

    //     default: { }
    // }

    // switch vertex_in.vertex_idx {
    //     case 0u: {
    //         vertex_out.position = vec4<f32>(
    //             -0.5, 0.5, 1.0, 1.0
    //         );
    //     }

    //     case 1u: {
    //         vertex_out.position = vec4<f32>(
    //             0.5, 0.5, 1.0, 1.0
    //         );
    //     }

    //     case 2u: {
    //         vertex_out.position = vec4<f32>(
    //             -0.5, -0.5, 1.0, 1.0
    //         );
    //     }

    //     default: {
    //         vertex_out.position = vec4<f32>(
    //             0.5, -0.5, 1.0, 1.0
    //         );
    //     }
    // }

    vertex_out.content_type = vertex_in.content_type;

    return vertex_out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    switch in.content_type {
        // color
        case 0u {
            return textureSampleLevel(atlas_texture, atlas_sampler, in.uv, 0.0);
        }

        // mask
        case 1u: {
            // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            var alpha = textureSampleLevel(atlas_texture, atlas_sampler, in.uv, 0.0).x;
            
            // return mix(vec4<f32>(1.0, 0.0, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), alpha);
            return vec4<f32>(in.color.rgb, in.color.a * alpha);
        }

        default: {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }
}
