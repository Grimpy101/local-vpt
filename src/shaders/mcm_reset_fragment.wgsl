struct Photon {
    position: vec4<f32>,
    direction: vec4<f32>,
    transmittance: vec4<f32>,
    radiance: vec4<f32>,
    bounces: u32,
    samples: u32
}

struct UnprojectOut {
    fr: vec3<f32>,
    to: vec3<f32>
}

struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) direction: vec4<f32>,
    @location(2) ts: vec4<f32>,
    @location(3) rb: vec4<f32>
}

@group(0) @binding(0)
var<uniform> mvp_inverse: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> resolution: vec2<u32>;
@group(0) @binding(2)
var<uniform> inverse_resolution: vec2<f32>;
@group(1) @binding(0)
var<uniform> random_seed: f32;

fn hash(x: ptr<function, u32>) -> u32 {
    *x = *x * 747796405u + 2891336453u;
    *x = ((*x >> ((*x >> 28u) + 4u)) ^ *x) * 277803737u;
    return (*x >> 22u) ^ *x;
}

fn squash_linear(x: vec3<u32>) -> u32 {
    var state = 19u*x.x + 47u*x.y + 101u*x.z + 131u;
    return hash(&state);
}

fn random_uniform(state: ptr<function, u32>) -> f32 {
    *state = hash(state);
    return bitcast<f32>((*state & 0x007fffffu) | 0x3f800000u) - 1.0;
}

fn random_disk(state: ptr<function, u32>) -> vec2<f32> {
    let radius = sqrt(random_uniform(state));
    let angle = 6.28318530718 * random_uniform(state);
    return radius * vec2<f32>(cos(angle), sin(angle));
}

fn random_square(state: ptr<function, u32>) -> vec2<f32> {
    let x = random_uniform(state);
    let y = random_uniform(state);

    return vec2<f32>(x, y);
}

fn unproject_rand(
    state: ptr<function, u32>,
    position: vec2<f32>,
    inverse_mvp: mat4x4<f32>,
    inverse_res: vec2<f32>,
    fr: ptr<function, vec3<f32>>,
    to: ptr<function, vec3<f32>>
) {
    let near_position = vec4<f32>(position.x, position.y, -1.0, 1.0);
    let antialiasing = (random_square(state) * 2.0 - 1.0) * inverse_res;
    let far_position = vec4<f32>(
        position.x + antialiasing.x,
        position.y + antialiasing.y,
        1.0,
        1.0);
    
    let fr_dirty = inverse_mvp * near_position;
    let to_dirty = inverse_mvp * far_position;

    *fr = fr_dirty.xyz / fr_dirty.w;
    *to = to_dirty.xyz / to_dirty.w;
}

fn intersect_cube(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let t_min = (vec3<f32>(0.0, 0.0, 0.0) - origin) / direction;
    let t_max = (vec3<f32>(1.0, 1.0, 1.0) - origin) / direction;
    
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);

    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);

    return vec2<f32>(t_near, t_far);
}

@fragment
fn main(@builtin(position) in_position: vec4<f32>) -> FragmentOutput {
    let res_x_f32 = f32(resolution.x);
    let res_y_f32 = f32(resolution.y);

    let x = u32(in_position.x);
    let y = u32(in_position.y);

    let index = x + y * resolution.x;

    let position = vec2<f32>(
        in_position.x / res_x_f32,
        in_position.y / res_y_f32
    );

    var photon: FragmentOutput;
    var fr: vec3<f32>;
    var to: vec3<f32>;

    let hash_arg = vec3<u32>(
        bitcast<u32>(in_position.x),
        bitcast<u32>(in_position.y),
        bitcast<u32>(random_seed)
    );
    var state = squash_linear(hash_arg);

    unproject_rand(&state, position, mvp_inverse, inverse_resolution, &fr, &to);

    photon.direction = vec4<f32>(normalize(to - fr), 0.0);
    let t_bounds = max(intersect_cube(fr, photon.direction.xyz), vec2<f32>(0.0, 0.0));
    photon.position = vec4<f32>(fr - t_bounds.x * photon.direction.xyz, 0.0);
    photon.ts = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    photon.rb = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    return photon;
}