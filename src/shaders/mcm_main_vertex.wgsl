@vertex
fn main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    switch index {
        case 0u {
            return vec4<f32>(-1.0, -1.0, 0.0, 1.0);
        }
        case 1u {
            return vec4<f32>(1.0, -1.0, 0.0, 1.0);
        }
        case 2u {
            return vec4<f32>(-1.0, 1.0, 0.0, 1.0);
        }
        case 3u {
            return vec4<f32>(1.0, 1.0, 0.0, 1.0);
        }
        default {
            return vec4<f32>(1.0, 1.0, 0.0, 1.0);
        }
    }

    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}