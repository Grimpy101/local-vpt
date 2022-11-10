use std::mem;

use wgpu::{include_wgsl, util::DeviceExt};

use crate::{camera::Camera, math::{Vector3f, Matrix4f}};

pub struct RenderData {
    pub output_resolution: u32,
    pub volume: Vec<u8>,
    pub volume_dims: (u32, u32, u32),
    pub transfer_function: Vec<u8>,
    pub transfer_function_len: u32,
    pub extinction: f32,
    pub anisotropy: f32,
    pub max_bounces: u32,
    pub steps: u32
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Photon {
    position: [f32; 4],
    direction: [f32; 4],
    transmittance: [f32; 4],
    radiance: [f32; 4],
    samples: u32,
    bounces: u32,
    _padding1: u32,
    _padding2: u32
}

const WORKGROUP_GRID_SIZE: u32 = 8;

fn create_f32_uniform_buffer(device: &wgpu::Device, data: f32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

fn create_u32_uniform_buffer(device: &wgpu::Device, data: u32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

pub async fn render(data: RenderData, output: &mut Vec<u8>) {
    let vol_dims = data.volume_dims;
    let tf_len = data.transfer_function_len;
    let volume_scale = vec![1.0, 1.0, 1.0];

    let mut camera = Camera::new();
    camera.set_position(
        Vector3f::new(0.0, 0.0, 1.5)
    );
    camera.look_at(Vector3f::new(0.0, 0.0, 0.0));
    camera.set_fov_x(0.512);
    camera.set_fov_y(0.512);
    camera.update_matrices();

    let model_matrix = Matrix4f::from_values(vec![
        volume_scale[0], 0.0, 0.0, -0.5,
        0.0, volume_scale[1], 0.0, -0.5,
        0.0, 0.0, volume_scale[2], -0.5,
        0.0, 0.0, 0.0, 1.0
    ]);

    let vm_matrix = Matrix4f::mutiply(
        camera.get_view_matrix(), &model_matrix
    );

    let pvm_matrix = Matrix4f::mutiply(
        camera.get_projection_matrix(), &vm_matrix
    );

    let pvm_inverse = pvm_matrix.inverse().transpose();

    // ------------ Initialization ------------ //

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let adapter = instance.request_adapter(
        &wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }
    ).await.unwrap();
    let (device, queue) = adapter.request_device(
        &Default::default(), None
    ).await.unwrap();

    let pixel_amount = data.output_resolution * data.output_resolution;
    let u32_size = mem::size_of::<u32>() as u32;
    let f32_size = mem::size_of::<u32>() as u32;

    let photon_buffer_size = pixel_amount * (16 * f32_size + 4 * u32_size);

    let photon_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("PhotonBuffer"),
            size: photon_buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }
    );

    let photon_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("PhotonBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                }
            ]
        }
    );

    let photon_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("PhotonBindGroup"),
            layout: &photon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: photon_buffer.as_entire_binding()
                }
            ]
        }
    );

    let dims = &[data.output_resolution, data.output_resolution];
    let dims_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("DimsBuffer"),
            contents: bytemuck::cast_slice(dims),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );

    let mvp_inverse_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("MVPInverseBuffer"),
            contents: bytemuck::cast_slice(&pvm_inverse.m),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );

    let inverse_res = 1.0 / data.output_resolution as f32;
    let inverse_res_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("InverseResBuffer"),
            contents: bytemuck::cast_slice(&[inverse_res, inverse_res]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );

    let random_seed = rand::random::<f32>();
    let random_seed_buffer = create_f32_uniform_buffer(&device, random_seed, "RandomSeedBuffer");

    let uniforms_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("UniformsBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }
            ]
        }
    );

    let uniforms_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("UniformsBindGroup"),
            layout: &uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: dims_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mvp_inverse_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inverse_res_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: random_seed_buffer.as_entire_binding()
                }
            ]
        }
    );

    let shader = device.create_shader_module(include_wgsl!("shaders/reset.wgsl"));

    let compute_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("ResetPipelineLayout"),
            bind_group_layouts: &[
                &photon_bind_group_layout,
                &uniforms_bind_group_layout
            ],
            push_constant_ranges: &[]
        }
    );

    let compute_pipeline = device.create_compute_pipeline(
        &wgpu::ComputePipelineDescriptor {
            label: Some("ResetPipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "main",
        }
    );

    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor {
            label: None,
        }
    );

    {
        let mut pass_encoder = encoder.begin_compute_pass(
            &wgpu::ComputePassDescriptor {
                label: Some("ComputePass"),
            }
        );
    
        pass_encoder.set_pipeline(&compute_pipeline);
        pass_encoder.set_bind_group(0, &photon_bind_group, &[]);
        pass_encoder.set_bind_group(1, &uniforms_bind_group, &[]);
        let work_count_x = (data.output_resolution as f32 / WORKGROUP_GRID_SIZE as f32).ceil() as u32;
        let work_count_y = (data.output_resolution as f32 / WORKGROUP_GRID_SIZE as f32).ceil() as u32;
        pass_encoder.dispatch_workgroups(work_count_x, work_count_y, 1);
    }

    let commands = encoder.finish();
    queue.submit([commands]);


    // MCM

    let shader = device.create_shader_module(include_wgsl!("shaders/MCM.wgsl"));

    let extinction = data.extinction;
    let extinction_buffer = create_f32_uniform_buffer(&device, extinction, "ExtinctionBuffer");

    let anisotropy = data.anisotropy;
    let anisotropy_buffer = create_f32_uniform_buffer(&device, anisotropy, "AnisotropyBuffer");

    let max_bounces = data.max_bounces;
    let max_bounces_buffer = create_u32_uniform_buffer(&device, max_bounces, "MaxBouncesBuffer");

    let steps = data.steps;
    let steps_buffer = create_u32_uniform_buffer(&device, steps, "StepsBuffer");

    let bind_group_3_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("BindGroupLayout3"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }
            ]
        }
    );

    let bind_group_3 = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("BindGroup3"),
            layout: &bind_group_3_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: extinction_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: anisotropy_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: max_bounces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: steps_buffer.as_entire_binding(),
                },
            ]
        }
    );

    println!("{} {}", data.volume.len(), data.transfer_function.len());

    let volume_texture_size = wgpu::Extent3d {
        width: vol_dims.0,
        height: vol_dims.1,
        depth_or_array_layers: vol_dims.2
    };
    let volume_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: volume_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("VolumeTexture")
        }
    );

    let tf_texture_size = wgpu::Extent3d {
        width: tf_len,
        height: 1,
        depth_or_array_layers: 1
    };
    let tf_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: tf_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("TFTexture")
        }
    );

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &volume_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        &data.volume,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(1 * vol_dims.0),
            rows_per_image: std::num::NonZeroU32::new(vol_dims.1)
        },
        volume_texture_size
    );

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &tf_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        &data.transfer_function,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(4 * tf_len),
            rows_per_image: std::num::NonZeroU32::new(1)
        },
        tf_texture_size
    );

    let volume_texture_view = volume_texture.create_view(
        &wgpu::TextureViewDescriptor::default()
    );
    let volume_sampler = device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some("VolumeSampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );

    let tf_texture_view = tf_texture.create_view(
        &wgpu::TextureViewDescriptor::default()
    );
    let tf_sampler = device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some("TFSampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );

    let textures_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("TexturesBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                },
            ]
        }
    );

    let textures_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("TexturesBindGroup"),
            layout: &textures_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&volume_texture_view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&volume_sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&tf_texture_view)
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&tf_sampler)
                }
            ]
        }
    );

    let compute_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("MCMPipelineLayout"),
            bind_group_layouts: &[
                &photon_bind_group_layout,
                &uniforms_bind_group_layout,
                &bind_group_3_layout,
                &textures_bind_group_layout
            ],
            push_constant_ranges: &[]
        }
    );

    let compute_pipeline = device.create_compute_pipeline(
        &wgpu::ComputePipelineDescriptor {
            label: Some("MCMPipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: "main",
        }
    );

    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor {
            label: None,
        }
    );

    {
        let mut pass_encoder = encoder.begin_compute_pass(
            &wgpu::ComputePassDescriptor {
                label: Some("ComputePass"),
            }
        );
    
        pass_encoder.set_pipeline(&compute_pipeline);

        pass_encoder.set_bind_group(0, &photon_bind_group, &[]);
        pass_encoder.set_bind_group(1, &uniforms_bind_group, &[]);
        pass_encoder.set_bind_group(2, &bind_group_3, &[]);
        pass_encoder.set_bind_group(3, &textures_bind_group, &[]);

        let work_count_x = (data.output_resolution as f32 / WORKGROUP_GRID_SIZE as f32).ceil() as u32;
        let work_count_y = (data.output_resolution as f32 / WORKGROUP_GRID_SIZE as f32).ceil() as u32;
        pass_encoder.dispatch_workgroups(work_count_x, work_count_y, 1);
    }

    let output_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("OutputBuffer"),
            size: photon_buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }
    );

    encoder.copy_buffer_to_buffer(
        &photon_buffer,
        0,
        &output_buffer,
        0,
        photon_buffer_size as u64
    );

    let commands = encoder.finish();
    queue.submit([commands]);


    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(
        wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        }
    );
    device.poll(wgpu::Maintain::Wait);
    rx.receive().await.unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    
    unsafe {
        let (b, photons, e) = data.align_to::<Photon>();
        println!("{}-{}", b.len(), e.len());
        for photon in photons {
            let r = (photon.radiance[0] * 255.0) as u8;
            let g = (photon.radiance[1] * 255.0) as u8;
            let b = (photon.radiance[2] * 255.0) as u8;
            output.push(r);
            output.push(g);
            output.push(b);
        }
    }
}