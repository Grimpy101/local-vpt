@group(0) @binding(0)
var input_texture: texture_2d<f32>;
@group(0) @binding(1)
var input_sampler: sampler;
@group(0) @binding(2)
var<uniform> low: f32;
@group(0) @binding(3)
var<uniform> mid: f32;
@group(0) @binding(4)
var<uniform> high: f32;
@group(0) @binding(5)
var<uniform> saturation: f32;
@group(0) @binding(6)
var<uniform> gamma: f32;

@fragment
fn main(@builtin(position) in_position: vec4<f32>) -> @location(0) vec4<f32> {
    let texture_dims = textureDimensions(input_texture);
    let position = vec2<f32>(
        in_position.x / f32(texture_dims.x),
        in_position.y / f32(texture_dims.y)
    );

    var color = textureSample(input_texture, input_sampler, position);
    color = (color - low) / (high - low);
    let gray = normalize(vec3<f32>(1.0));
    color = vec4<f32>(
        mix(
            dot(color.rgb, gray) * gray,
            color.rgb,
            saturation
        ),
        1.0
    );
    let midpoint = (mid - low) / (high - low);
    let exponent = -log(midpoint) / log(2.0);
    color = pow(color, vec4(exponent / gamma));
    color = vec4<f32>(color.rgb, 1.0);
    return color;
}