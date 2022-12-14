struct Photon {
    position: vec3<f32>,
    direction: vec3<f32>,
    transmittance: vec3<f32>,
    radiance: vec3<f32>,
    bounces: u32,
    samples: u32
}

struct FragmentOutput {
    @location(0) position: vec4<f32>,
    @location(1) direction: vec4<f32>,
    @location(2) ts: vec4<f32>,
    @location(3) rb: vec4<f32>
}

@group(0) @binding(0)
var<uniform> random_seed: f32;

@group(1) @binding(0)
var<uniform> mvp_inverse: mat4x4<f32>;
@group(1) @binding(1)
var<uniform> resolution: vec2<u32>;
@group(1) @binding(2)
var<uniform> inverse_resolution: vec2<f32>;
@group(1) @binding(3)
var<uniform> extinction: f32;
@group(1) @binding(4)
var<uniform> anisotropy: f32;
@group(1) @binding(5)
var<uniform> max_bounces: u32;
@group(1) @binding(6)
var<uniform> steps: u32;

@group(2) @binding(0)
var volume_texture: texture_3d<f32>;
@group(2) @binding(1)
var volume_sampler: sampler;
@group(2) @binding(2)
var transfer_function_texture: texture_2d<f32>;
@group(2) @binding(3)
var transfer_function_sampler: sampler;

@group(3) @binding(0)
var position_texture: texture_2d<f32>;
@group(3) @binding(1)
var position_sampler: sampler;
@group(3) @binding(2)
var direction_texture: texture_2d<f32>;
@group(3) @binding(3)
var direction_sampler: sampler;
@group(3) @binding(4)
var ts_texture: texture_2d<f32>;
@group(3) @binding(5)
var ts_sampler: sampler;
@group(3) @binding(6)
var rb_texture: texture_2d<f32>;
@group(3) @binding(7)
var rb_sampler: sampler;

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
fn main(@builtin(position) in_position: vec4<f32>) -> FragmentOutput {
    let res_x_f32 = f32(resolution.x);
    let res_y_f32 = f32(resolution.y);

    let x = u32(in_position.x);

    if x > resolution.x {
        var out: FragmentOutput;
        out.position = vec4<f32>(0.0);
        out.direction = vec4<f32>(0.0);
        out.ts = vec4<f32>(0.0);
        out.rb = vec4<f32>(0.0);
        return out;
    }

    let v0 = vec3<f32>(0.0);
    let v1 = vec3<f32>(1.0);

    let position = vec2<f32>(
        (in_position.x / res_x_f32) * 2.0 - 1.0,
        (in_position.y / res_y_f32) * 2.0 - 1.0
    );
    let mapped_position = position * 0.5 + 0.5;

    var photon: Photon;
    photon.position = textureSampleLevel(position_texture, position_sampler, mapped_position, 0.0).xyz;
    photon.direction = textureSampleLevel(direction_texture, direction_sampler, mapped_position, 0.0).xyz;
    let ts = textureSampleLevel(ts_texture, ts_sampler, mapped_position, 0.0);
    photon.transmittance = ts.xyz;
    photon.samples = u32(ts.w + 0.5);
    let rb = textureSampleLevel(rb_texture, rb_sampler, mapped_position, 0.0);
    photon.radiance = rb.xyz;
    photon.bounces = u32(rb.w + 0.5);

    var state = squash_linear(vec3<u32>(
        bitcast<u32>(position.x),
        bitcast<u32>(position.y),
        bitcast<u32>(random_seed)
    ));

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
    
    var out: FragmentOutput;
    out.position = vec4<f32>(photon.position, 0.0);
    out.direction = vec4<f32>(photon.direction, 0.0);
    out.ts = vec4<f32>(
        photon.transmittance,
        f32(photon.samples)
    );
    out.rb = vec4<f32>(
        photon.radiance,
        f32(photon.bounces)
    );

    return out;
}