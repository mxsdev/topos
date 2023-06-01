const FEATHERING = 1.;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) pos: vec2<f32>,
    @location(1) dims: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) rounding: f32,
    @location(4) depth: f32,
    @location(5) stroke_width: f32,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) dims: vec2<f32>,
    @location(2) rel_pos: vec2<f32>,
    @location(3) rounding: f32,
    @location(4) stroke_width: f32,
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

    var padding = FEATHERING + vertex_in.stroke_width;

    vertex_out.color = vertex_in.color;
    vertex_out.rounding = vertex_in.rounding;
    vertex_out.stroke_width = vertex_in.stroke_width;

    var out_pos = vertex_in.pos;

    if vertex_in.rounding > 0. || vertex_in.stroke_width > 0. {
        vertex_out.dims = vertex_in.dims;

        var v = vertex_in.vertex_idx % 4u;

        var px = vertex_in.dims.x + padding;
        var py = vertex_in.dims.y + padding;

        switch v {
            case 0u: {
                vertex_out.rel_pos = vec2<f32>(
                    -px,
                    py,
                );

                out_pos += vec2<f32>(
                    -padding,
                    -padding,
                );
            }

            case 1u: {
                vertex_out.rel_pos = vec2<f32>(
                    px,
                    py,
                );

                out_pos += vec2<f32>(
                    padding,
                    -padding,
                );
            }
            
            case 2u: {
                vertex_out.rel_pos = vec2<f32>(
                    -px,
                    -py,
                );

                out_pos += vec2<f32>(
                    -padding,
                    padding,
                );
            }

            case 3u: {
                vertex_out.rel_pos = vec2<f32>(
                    px,
                    -py,
                );

                out_pos += vec2<f32>(
                    padding,
                    padding,
                );
            }

            default: { }
        }
    }

    vertex_out.position = vec4<f32>(
        2.0 * out_pos / vec2<f32>(params.screen_resolution) - 1.0,
        vertex_in.depth,
        1.0
    );

    vertex_out.position.y *= -1.;

    return vertex_out;
}

fn sdRoundBox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  var q = abs(p) - (b - vec2<f32>(r));
  return length(max(q, vec2<f32>(0.))) + min(max(q.x,q.y),0.0) - r;
}

// fn sdSharpBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
//     var q = abs(p) - b;
//     return min(max(q, vec<f32>(0.)))
// }

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.rounding <= 0. {
        return in.color;
    }

    var dist = sdRoundBox(in.rel_pos, in.dims, in.rounding);

    if in.stroke_width <= 0. {
        var alpha = clamp(-dist, -0.5, 0.5) + 0.5;
        return vec4<f32>(in.color.rgb, alpha);
    } else {
        var alpha = 1. - (clamp(abs(dist) - in.stroke_width / 2., 0., 0.5) * 2.);
        return vec4<f32>(in.color.rgb, alpha);
    }
}