const FEATHERING = 1.;

/// Rounded rectangle
const shapeRect = 0;
/// Triangle mesh
const shapeMesh = 1;

const fillModeColor = 0;
const fillModeTexture = 1;
const fillModeTextureMaskColor = 2;
const fillModeTextureMaskTexture = 2;

struct Params {
    screen_resolution: vec2<u32>,
    scale_factor: f32,
};

struct ClipRect {
    origin: vec2<f32>,
    half_size: vec2<f32>,
    rounding: f32,
    transformation_idx: u32,
};

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,

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

    @location(12) clip_rect_idx: u32,

    @location(13) transformation_idx: u32,

    @location(14) uv_alt: vec2<f32>,
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
    @location(8) atlas_idx_alt: u32,

    @location(9) color: vec4<f32>,

    @location(10) rounding: f32,
    @location(11) stroke_width: f32,
    @location(12) blur_radius: f32,

    @location(14) @interpolate(flat) clip_rect_idx: u32,
    @location(15) original_pos: vec2<f32>,

    @location(16) scale_factor: f32,

    @location(17) uv_alt: vec2<f32>,
};

var<private> pi: f32 = 3.141592653589793;

var<private> rotation90: mat2x2<f32> = mat2x2<f32>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(-1.0, 0.0),
);

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

// color space transformations

// Constants
var<private> HCV_EPSILON: f32 = 1e-10;
var<private> HSL_EPSILON: f32 = 1e-10;
var<private> HCY_EPSILON: f32 = 1e-10;

// var<private> SRGB_GAMMA: f32 = 1.0 / 2.2;
var<private> SRGB_GAMMA: f32 = 0.45454545454;
var<private> SRGB_INVERSE_GAMMA: f32 = 2.2;
var<private> SRGB_ALPHA: f32 = 0.055;

// Converts from pure Hue to linear RGB
fn hue_to_rgb(hue: f32) -> vec3<f32>
{
    var R = abs(hue * 6.0 - 3.0) - 1.0;
    var G = 2.0 - abs(hue * 6.0 - 2.0);
    var B = 2.0 - abs(hue * 6.0 - 4.0);
    return clamp(vec3(R,G,B), vec3<f32>(0.), vec3<f32>(1.));
}

// Converts a value from linear RGB to HCV (Hue, Chroma, Value)
fn rgb_to_hcv(rgb: vec3<f32>) -> vec3<f32>
{
    // Based on work by Sam Hocevar and Emil Persson
    var P = select(vec4(rgb.gb, 0.0, -1.0/3.0), vec4(rgb.bg, -1.0, 2.0/3.0), rgb.g < rgb.b);
    // vec4 Q = (rgb.r < P.x) ? vec4(P.xyw, rgb.r) : vec4(rgb.r, P.yzx);
    var Q = select(vec4(rgb.r, P.yzx), vec4(P.xyw, rgb.r), rgb.r < P.x);
    var C = Q.x - min(Q.w, Q.y);
    var H = abs((Q.w - Q.y) / (6.0 * C + HCV_EPSILON) + Q.z);
    return vec3<f32>(H, C, Q.x);
}

// Converts from linear RGB to HSV
fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32>
{
    var HCV = rgb_to_hcv(rgb);
    var S = HCV.y / (HCV.z + HCV_EPSILON);
    return vec3<f32>(HCV.x, S, HCV.z);
}

// Converts from HSV to linear RGB
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32>
{
    var rgb = hue_to_rgb(hsv.x);
    return ((rgb - vec3<f32>(1.0)) * hsv.y + vec3<f32>(1.0)) * hsv.z;
}

// Converts from linear rgb to HSL
fn rgb_to_hsl(rgb: vec3<f32>) -> vec3<f32>
{
    // vec3 HCV = rgb_to_hcv(rgb);
    var HCV = rgb_to_hcv(rgb);
    // float L = HCV.z - HCV.y * 0.5;
    var L = HCV.z - HCV.y * 0.5;
    // float S = HCV.y / (1.0 - abs(L * 2.0 - 1.0) + HSL_EPSILON);
    var S = HCV.y / (1.0 - abs(L * 2.0 - 1.0) + HSL_EPSILON);
    // return vec3(HCV.x, S, L);
    return vec3<f32>(HCV.x, S, L);
}

// Converts a single srgb channel to rgb
fn srgb_to_linear(channel: f32) -> f32 {
    if (channel <= 0.04045) {
        return channel / 12.92;
    } else {
        return pow((channel + SRGB_ALPHA) / (1.0 + SRGB_ALPHA), 2.4);
    }
}

@group(0) @binding(0)
var<uniform> params: Params;

@group(0) @binding(1)
var<storage, read> clip_rects: array<ClipRect>;

@group(0) @binding(2)
var<storage, read> transformations: array<mat3x2<f32>>;

@group(0) @binding(3)
var<storage, read> transformation_inversions: array<mat3x2<f32>>;

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

    var transformation_cols = transpose(transformations[vertex_in.transformation_idx]);

    var transformation = transpose(mat3x3<f32>(
        transformation_cols[0],
        transformation_cols[1],
        vec3<f32>(0.0, 0.0, 1.0),
    )) * params.scale_factor;

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

    vertex_out.atlas_idx = vertex_in.atlas_idx >> 16u;
    vertex_out.atlas_idx_alt = vertex_in.atlas_idx & u32(0xFFFF);

    var tex_dims: vec2<u32>;
    switch (vertex_out.atlas_idx) {
        {{#times num_atlas_textures}}
        case {{index}}u: {
            tex_dims = textureDimensions(atlas_texture_{{index}});
        }
        {{/times}}

        default: { }
    }

    vertex_out.uv = vertex_in.uv / vec2<f32>(tex_dims);

    var tex_dims_alt: vec2<u32>;
    switch (vertex_out.atlas_idx_alt) {
        {{#times num_atlas_textures}}
        case {{index}}u: {
            tex_dims_alt = textureDimensions(atlas_texture_{{index}});
        }
        {{/times}}

        default: { }
    }

    vertex_out.uv_alt = vertex_in.uv_alt / vec2<f32>(tex_dims_alt);

    let scale_fac = determinant(transformation);

    switch (vertex_in.shapeType) {
        case 0u: { // shapeRect
            var padding = (FEATHERING / scale_fac) + vertex_in.stroke_width + vertex_in.blur_radius;
            var rel_pos = vertex_in.pos - vertex_in.origin;
            var padding_quantity = sign(rel_pos) * padding;

            out_pos += padding_quantity;
        }

        case 1u: { // shapeMesh

        }

        default: { }
    }

    vertex_out.pos = out_pos;

    out_pos = (transformation * vec3<f32>(out_pos, 1.)).xy;

    vertex_out.original_pos = out_pos;

    vertex_out.position = vec4<f32>(
        2.0 * out_pos / vec2<f32>(params.screen_resolution) - 1.0,
        vertex_in.depth,
        1.0
    );

    vertex_out.position.y *= -1.;

    vertex_out.clip_rect_idx = vertex_in.clip_rect_idx;

    vertex_out.scale_factor = scale_fac;

    return vertex_out;
}

fn sdRoundBox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    var q = abs(p) - (b - vec2<f32>(r));
    return length(max(q, vec2<f32>(0.))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdSmoothStep(dist: f32) -> f32 {
    return smoothstep(0., 1., -dist + 0.5);
}

// fn sdSharpBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
//     var q = abs(p) - b;
//     return min(max(q, vec<f32>(0.)))
// }

// Converts a color from sRGB gamma to linear light gamma
fn toLinear(sRGB: vec4<f32>) -> vec4<f32>
{
    return vec4<f32>(pow(sRGB.rgb, vec3<f32>(2.2)), sRGB.a);
    
    // var cutoff = sRGB < vec3<f32>(0.04045);
    // var higher = pow((sRGB + vec3<f32>(0.055))/vec3<f32>(1.055), vec3<f32>(2.4));
    // var lower = sRGB/vec3<f32>(12.92);

    // return mix(higher, lower, cutoff);
}

fn toSrgb(linear: vec4<f32>) -> vec4<f32>
{
    return vec4<f32>(pow(linear.rgb, vec3<f32>(1.0/2.2)), linear.a);
    
    // var cutoff = linear < vec3<f32>(0.0031308);
    // var higher = vec3<f32>(1.055) * pow(linear, vec3<f32>(1.0/2.4)) - vec3<f32>(0.055);
    // var lower = linear * vec3<f32>(12.92);

    // return mix(higher, lower, cutoff);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var col = in.color;

    var mask_atlas = in.atlas_idx;
    var mask_uv = in.uv;
    var do_mask_texture = false;

    var color_atlas = in.atlas_idx;
    var color_uv = in.uv;
    var do_color_texture = false;

    switch (in.fillMode) {
        case 0u: { // fillModeColor

        }

        case 1u: { // fillModeTexture
            do_color_texture = true;
        }

        case 2u: { // fillModeTextureMaskColor
            do_mask_texture = true;
        }

        case 3u: { // fillModeTextureMaskTexture
            do_color_texture = true;
            do_mask_texture = true;

            color_atlas = in.atlas_idx_alt;
            color_uv = in.uv_alt;
        }

        default: { }
    }

    if do_color_texture {
        switch (color_atlas) {
            {{#times num_atlas_textures}}
            case {{index}}u: {
                var sampled_col = textureSampleLevel(atlas_texture_{{index}}, atlas_sampler, color_uv, 0.0);

                // for some reason, the hue/saturation are linear, but the value is sRGB
                // so we need to convert it to linear like so
                var sampled_hsv = rgb_to_hsv(sampled_col.rgb);
                var col_fixed = hsv_to_rgb(vec3<f32>(sampled_hsv.x, sampled_hsv.y, srgb_to_linear(sampled_hsv.z)));

                col = vec4<f32>(col_fixed, sampled_col.a);
            }
            {{/times}}

            default: { }
        }
    }

    if do_mask_texture {
        switch (mask_atlas) {
            {{#times num_atlas_textures}}
            case {{index}}u: {
                var alpha = textureSampleLevel(atlas_texture_{{index}}, atlas_sampler, mask_uv, 0.0).x;
                col = vec4<f32>(col.rgb, col.a * alpha);
            }
            {{/times}}

            default: { }
        }
    }

    var alpha: f32 = col.a;

    if alpha == 0. { discard; }

    if (in.clip_rect_idx != 0u) {
        let clip_rect = clip_rects[in.clip_rect_idx];

        // TODO: pass clip rect instead of clip_rect_idx
        var clip_transform_cols = transpose(transformation_inversions[clip_rect.transformation_idx]);
        var clip_transform = transpose(mat3x3<f32>(
            clip_transform_cols[0], 
            clip_transform_cols[1], 
            vec3<f32>(0.0, 0.0, 1.0),
        )) * (1. / params.scale_factor);

        var clip_pos = (clip_transform * vec3<f32>(in.original_pos, 1.)).xy;

        var clip_dist = sdRoundBox(clip_pos - clip_rect.origin, clip_rect.half_size, clip_rect.rounding);

        alpha *= sdSmoothStep(clip_dist);
    }

    if alpha == 0. { discard; }
    
    switch (in.shapeType) {
        case 0u: { // shapeRect
            var rel_pos = in.pos - in.origin;
        
            // draw blur
            if in.blur_radius > 0. {
                alpha *= roundedBoxShadow(in.dims, rel_pos, in.blur_radius, in.rounding);
                break;
            }

            if in.rounding <= 0. && in.stroke_width <= 0. {
                // TODO: strokes for non-rounded rects
                break;
            }

            var dist = sdRoundBox(rel_pos, in.dims, in.rounding);

            // draw fill
            if in.stroke_width <= 0. {
                alpha *= smoothstep(0., 1., -dist+0.5);
                break;
            } 

            // draw stroke
            alpha *= 1. - (smoothstep(0., 0.5 / in.scale_factor, abs(dist) - in.stroke_width / 2.) * 2.);
        }

        case 1u: { // shapeMesh

        }

        default: { }
    }

    var res = vec4<f32>(col.rgb, alpha);

    return res;
}

// 24 x 16