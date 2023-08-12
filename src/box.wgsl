const FEATHERING = 1.;

/// Rounded rectangle
const shapeRect = 0;
/// Triangle mesh
const shapeMesh = 1;

const fillModeColor = 0;
const fillModeTexture = 1;
const fillModeTextureMaskColor = 2;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,

    @location(0) @interpolate(flat) shapeType: u32,
    @location(1) @interpolate(flat) fillMode: u32,

    @location(2) depth: f32,

    @location(3) pos: vec2<f32>,

    @location(4) dims: vec2<f32>,
    @location(5) origin: vec2<f32>,

    @location(6) uv: vec2<u32>,
    @location(7) atlas_idx: u32,

    @location(8) color: vec4<f32>,

    @location(9) rounding: f32,
    @location(10) stroke_width: f32,
    @location(11) blur_radius: f32,
}

struct VertexOutput {
    @invariant @builtin(position) position: vec4<f32>,

    @location(0) @interpolate(flat) shapeType: u32,
    @location(1) @interpolate(flat) fillMode: u32,

    @location(2) depth: f32,

    @location(3) pos: vec2<f32>,

    @location(4) dims: vec2<f32>,
    @location(5) origin: vec2<f32>,

    @location(6) uv: vec2<f32>,
    @location(7) atlas_idx: u32,

    @location(8) color: vec4<f32>,

    @location(9) rounding: f32,
    @location(10) stroke_width: f32,
    @location(11) blur_radius: f32,
};

var<private> pi: f32 = 3.141592653589793;

// A standard gaussian function, used for weighting samples
fn gaussian(x: f32, sigma: f32) -> f32 {
  return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * pi) * sigma);
}

// This approximates the error function, needed for the gaussian integral
fn erf(_x: vec2<f32>) -> vec2<f32> {
  let s = sign(_x);
  let a = abs(_x);
  var x = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
  x *= x;
  return s - s / (x * x);
}

// Return the blurred mask along the x dimension
fn roundedBoxShadowX(x: f32, y: f32, sigma: f32, corner: f32, halfSize: vec2<f32>) -> f32 {
  let delta = min(halfSize.y - corner - abs(y), 0.0);
  let curved = halfSize.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
  let integral = 0.5 + 0.5 * erf((x + vec2<f32>(-curved, curved)) * (sqrt(0.5) / sigma));
  return integral.y - integral.x;
}

// Return the mask for the shadow of a box from lower to upper
fn roundedBoxShadow(halfSize: vec2<f32>, _pt: vec2<f32>, sigma: f32, corner: f32) -> f32 {
  let pt = _pt;

  // The signal is only non-zero in a limited range, so don't waste samples
  let low = pt.y - halfSize.y;
  let high = pt.y + halfSize.y;
  let start = clamp(-3.0 * sigma, low, high);
  let end = clamp(3.0 * sigma, low, high);

  // Accumulate samples (we can get away with surprisingly few samples)
  let step = (end - start) / 4.0;
  var y = start + step * 0.5;
  var value = 0.0;
  for (var i = 0; i < 4; i += 1) {
    value += roundedBoxShadowX(pt.x, pt.y - y, sigma, corner, halfSize) * gaussian(y, sigma) * step;
    y += step;
  }

  return value;
}

struct Params {
    screen_resolution: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> params: Params;

{{#times num_atlas_textures}}
@group(1) @binding({{index}})
var atlas_texture_{{index}}: texture_2d<f32>;
{{/times}}

@group(2) @binding(0)
var atlas_sampler: sampler;

@vertex
fn vs_main(
    vertex_in: VertexInput
) -> VertexOutput {
    var vertex_out: VertexOutput;

    vertex_out.shapeType = vertex_in.shapeType;
    vertex_out.fillMode = vertex_in.fillMode;

    vertex_out.depth = vertex_in.depth;

    vertex_out.dims = vertex_in.dims;
    vertex_out.origin = vertex_in.origin;

    vertex_out.color = vertex_in.color;

    vertex_out.rounding = vertex_in.rounding;
    vertex_out.stroke_width = vertex_in.stroke_width;
    vertex_out.blur_radius = vertex_in.blur_radius / 3.;

    var out_pos = vertex_in.pos;

    vertex_out.atlas_idx = vertex_in.atlas_idx;

    var tex_dims: vec2<u32>;
    switch (vertex_in.atlas_idx) {
        {{#times num_atlas_textures}}
        case {{index}}u: {
            tex_dims = textureDimensions(atlas_texture_{{index}});
        }
        {{/times}}

        default: { }
    }

    vertex_out.uv = vec2<f32>(vertex_in.uv) / vec2<f32>(tex_dims);

    switch (vertex_in.shapeType) {
        case 0u: { // shapeRect
            var padding = FEATHERING + vertex_in.stroke_width + vertex_in.blur_radius;
            var rel_pos = vertex_in.pos - vertex_in.origin;
            var padding_quantity = sign(rel_pos) * padding;

            out_pos += padding_quantity;
        }

        case 1u: { // shapeMesh

        }

        default: { }
    }

    vertex_out.pos = out_pos;

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
    var col = in.color;

    switch (in.fillMode) {
        case 0u: { // fillModeColor

        }

        case 1u: { // fillModeTexture
            switch (in.atlas_idx) {
                {{#times num_atlas_textures}}
                case {{index}}u: {
                    col = textureSampleLevel(atlas_texture_{{index}}, atlas_sampler, in.uv, 0.0);
                }
                {{/times}}

                default: { }
            }
        }

        case 2u: { // fillModeTextureMaskColor
            switch (in.atlas_idx) {
                {{#times num_atlas_textures}}
                case {{index}}u: {
                    var alpha = textureSampleLevel(atlas_texture_{{index}}, atlas_sampler, in.uv, 0.0).x;
                    col = vec4<f32>(col.rgb, in.color.a * alpha);
                }
                {{/times}}

                default: { }
            }
        }

        default: { }
    }
    
    switch (in.shapeType) {
        case 0u: { // shapeRect
            var rel_pos = in.pos - in.origin;
        
            // draw blur
            if in.blur_radius > 0. {
                var alpha = roundedBoxShadow(in.dims, rel_pos, in.blur_radius, in.rounding);
                return vec4<f32>(in.color.rgb, alpha * in.color.a);
            }

            if in.rounding <= 0. && in.stroke_width <= 0. {
                // TODO: strokes for non-rounded rects
                return col;
            }

            var dist = sdRoundBox(rel_pos, in.dims, in.rounding);

            // draw fill
            if in.stroke_width <= 0. {
                var alpha = smoothstep(0., 1., -dist+0.5);
                return vec4<f32>(in.color.rgb, alpha * in.color.a);
            } 

            // draw stroke
            var alpha = 1. - (smoothstep(0., 0.5, abs(dist) - in.stroke_width / 2.) * 2.);
            return vec4<f32>(in.color.rgb, alpha * in.color.a);
        }

        case 1u: { // shapeMesh
            return col;
        }

        default: {
            return col;
        }
    }
}