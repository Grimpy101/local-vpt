struct Photon {
    position: vec3<f32>,
    direction: vec3<f32>,
    transmittance: vec3<f32>,
    radiance: vec3<f32>,
    bounces: u32,
    samples: u32
}

@group(0) @binding(0)
var<uniform> mvp_inverse: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> resolution: vec2<u32>;
@group(0) @binding(2)
var<uniform> inverse_resolution: vec2<f32>;
@group(0) @binding(3)
var<uniform> random_seed: f32;
@group(0) @binding(4)
var<uniform> extinction: f32;
@group(0) @binding(5)
var<uniform> anisotropy: f32;
@group(0) @binding(6)
var<uniform> max_bounces: u32;
@group(0) @binding(7)
var<uniform> steps: u32;

@group(1) @binding(0)
var volume_texture: texture_3d<f32>;
@group(1) @binding(1)
var volume_sampler: sampler;
@group(1) @binding(2)
var transfer_function_texture: texture_2d<f32>;
@group(1) @binding(3)
var transfer_function_sampler: sampler;

@group(2) @binding(0)
var<storage, read_write> result: array<vec4<f32>>;


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

fn random_exponential(state: ptr<function, u32>, rate: f32) -> f32 {
    return -log(random_uniform(state)) / rate;
}

fn sample_volume_color(position: vec3<f32>) -> vec4<f32> {
    let volume_sample = textureSample(volume_texture, volume_sampler, position).r;
    let location = vec2<f32>(volume_sample, 0.5);
    let transfer_sample = textureSample(transfer_function_texture, transfer_function_sampler, location);
    return transfer_sample;
}

fn random_square(state: ptr<function, u32>) -> vec2<f32> {
    let x = random_uniform(state);
    let y = random_uniform(state);

    return vec2<f32>(x, y);
}

fn random_sphere(state: ptr<function, u32>) -> vec3<f32> {
    let disk = random_disk(state);
    let norm = dot(disk, disk);
    let radius = 2.0 * sqrt(1.0 - norm);
    let z = 1.0 - 2.0 * norm;
    return vec3<f32>(radius * disk, z);
}

fn intersect_cube(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let t_min = (vec3<f32>(0.0) - origin) / direction;
    let t_max = (vec3<f32>(1.0) - origin) / direction;
    
    let t1 = min(t_min, t_max);
    let t2 = max(t_min, t_max);

    let t_near = max(max(t1.x, t1.y), t1.z);
    let t_far = min(min(t2.x, t2.y), t2.z);

    return vec2<f32>(t_near, t_far);
}

fn unproject_rand(
    state: ptr<function, u32>,
    position: vec2<f32>,
    inverse_mvp: mat4x4<f32>,
    inverse_res: vec2<f32>,
    fr: ptr<function, vec3<f32>>,
    to: ptr<function, vec3<f32>>
) {
    let near_position = vec4<f32>(position, -1.0, 1.0);
    let antialiasing = (random_square(state) * 2.0 - 1.0) * inverse_res;
    let far_position = vec4<f32>(position + antialiasing, 1.0, 1.0);
    
    let fr_dirty = inverse_mvp * near_position;
    let to_dirty = inverse_mvp * far_position;

    *fr = fr_dirty.xyz / fr_dirty.w;
    *to = to_dirty.xyz / to_dirty.w;
}

fn reset_photon(state: ptr<function, u32>, position: vec2<f32>, photon: ptr<function, Photon>) {
    var fr: vec3<f32>;
    var to: vec3<f32>;

    unproject_rand(state, position, mvp_inverse, inverse_resolution, &fr, &to);

    (*photon).direction = normalize(to - fr);
    (*photon).bounces = 0u;
    let t_bounds = max(intersect_cube(fr, (*photon).direction), vec2<f32>(0.0));
    (*photon).position = fr + t_bounds.x * (*photon).direction;
    (*photon).transmittance = vec3<f32>(1.0);
}

fn sample_henyey_greenstein_angle_cosine(state: ptr<function, u32>, g: f32) -> f32 {
    let g2 = g * g;
    let c = (1.0 - g2) / (1.0 - g + 2.0 * g * random_uniform(state));
    return (1.0 + g2 - c * c) / (2.0 * g);
}

fn sample_henyey_greenstein(state: ptr<function, u32>, g: f32, direction: vec3<f32>) -> vec3<f32> {
    let u = random_sphere(state);
    if abs(g) < 1e-5 {
        return u;
    }

    let hg_cos = sample_henyey_greenstein_angle_cosine(state, g);
    let lambda = hg_cos - dot(direction, u);
    return normalize(u + lambda * direction);
}

fn max3(v: vec3<f32>) -> f32 {
    return max(max(v.x, v.y), v.z);
}

fn mean3(v: vec3<f32>) -> f32 {
    return dot(v, vec3<f32>(1.0 / 3.0));
}

@fragment
fn main(@builtin(position) in_position: vec4<f32>) -> @location(0) vec4<f32> {
    let res_x_f32 = f32(resolution.x);
    let res_y_f32 = f32(resolution.y);

    let x = u32(in_position.x);
    let y = u32(in_position.y);

    let v0 = vec3<f32>(0.0);
    let v1 = vec3<f32>(1.0);

    let index = x + y * resolution.x;

    let position = vec2<f32>(
        (in_position.x / res_x_f32) * 2.0 - 1.0,
        (in_position.y / res_y_f32) * 2.0 - 1.0
    );

    var photon: Photon;
    var fr: vec3<f32>;
    var to: vec3<f32>;

    var state = squash_linear(vec3<u32>(
        bitcast<u32>(position.x),
        bitcast<u32>(position.y),
        bitcast<u32>(random_seed)
    ));

    unproject_rand(&state, position, mvp_inverse, inverse_resolution, &fr, &to);

    photon.direction = normalize(to - fr);
    let t_bounds = max(intersect_cube(fr, photon.direction), vec2<f32>(0.0));
    photon.position = fr + t_bounds.x * photon.direction;
    photon.transmittance = vec3<f32>(1.0);
    photon.radiance = vec3<f32>(1.0);
    photon.bounces = 0u;
    photon.samples = 0u;

    for (var i = 0u; i < steps; i++) {
        let dist = random_exponential(&state, extinction);
        photon.position += dist * photon.direction;

        let volume_sample = sample_volume_color(photon.position);

        let p_null = 1.0 - volume_sample.a;
        var p_scattering: f32;
        if photon.bounces >= max_bounces {
            p_scattering = 0.0;
        } else {
            p_scattering = volume_sample.a * max3(volume_sample.rgb);
        }
        let p_absorption = 1.0 - p_null - p_scattering;

        let fortune_wheel = random_uniform(&state);
        if any(photon.position > v1) || any(photon.position < v0) {
            let env_sample = vec3<f32>(1.0);
            let radiance = photon.transmittance * env_sample;
            photon.samples++;
            photon.radiance += (radiance - photon.radiance) / f32(photon.samples);
            reset_photon(&state, position, &photon);
        } else if fortune_wheel < p_absorption {
            let radiance = vec3<f32>(0.0);
            photon.samples++;
            photon.radiance += (radiance - photon.radiance) / f32(photon.samples);
            reset_photon(&state, position, &photon);
        } else if fortune_wheel < p_absorption + p_scattering {
            photon.transmittance *= volume_sample.rgb;
            photon.direction = sample_henyey_greenstein(&state, anisotropy, photon.direction);
            photon.bounces++;
        }
    }

    result[index] = vec4<f32>(photon.radiance, 1.0);

    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}