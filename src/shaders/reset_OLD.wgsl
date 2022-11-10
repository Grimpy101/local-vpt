struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) index: u32)
    -> VertexOutput {
        var out: VertexOutput;
        if index == 0u || index == 4u {
            out.clip_position = vec4<f32>(-1.0, -1.0, 0.0, 1.0);
        }
        else if index == 1u {
            out.clip_position = vec4<f32>(1.0, -1.0, 0.0, 1.0);
        }
        else if index == 2u || index == 5u {
            out.clip_position = vec4<f32>(1.0, 1.0, 0.0, 1.0);
        }
        else if index == 3u || index == 6u {
            out.clip_position = vec4<f32>(-1.0, 1.0, 0.0, 1.0);
        }
        return out;
}

@fragment
struct FragmentUniform {
    mvp_inverse: mat4x4<f32>,
    inverse_res: vec2<f32>,
    random_seed: f32,
    blur: f32
}

@group(1) @binding(0)
var<uniform> fradment_uniform: FragmentUniform;

fn fragment_main(in: VertexOutput)
    -> @location(0) vec4<f32> {
        return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}