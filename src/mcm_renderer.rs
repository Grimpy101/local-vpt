use std::mem;

use wgpu::{Device, util::DeviceExt, include_wgsl};

use crate::{pipeline::RenderData, math::Matrix4f};


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Photon {
    pub position: [f32; 4],
    pub direction: [f32; 4],
    pub transmittance: [f32; 4],
    pub radiance: [f32; 4],
    samples: u32,
    bounces: u32,
    _padding1: u32,
    _padding2: u32
}

fn create_2d_render_texture(device: &Device, res: u32, label: &str) -> wgpu::Texture {
    return device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: res,
                height: res,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        }
    );
}

fn create_texture_sampler(device: &Device, label: &str) -> wgpu::Sampler {
    return device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );
}

fn create_matrix_uniform_buffer(device: &Device, matrix: &Matrix4f, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&matrix.m),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

fn create_vector2_u32_uniform_buffer(device: &Device, vector: &[u32; 2], label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(vector),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );
}

fn create_vector2_f32_uniform_buffer(device: &Device, vector: &[f32; 2], label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(vector),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );
}

fn create_f32_uniform_buffer(device: &Device, num: f32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[num]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

fn create_u32_uniform_buffer(device: &Device, num: u32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[num]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

pub async fn render(device: &Device, queue: &wgpu::Queue, data: &RenderData, camera_matrix: &Matrix4f, output: &mut Vec<u8>) {
    // ------------------ Output Buffer ------------------ //

    let pixel_amount = data.output_resolution * data.output_resolution;
    let f32_size = mem::size_of::<u32>() as u32;

    let result_buffer_size = pixel_amount * 4 * f32_size;
    let result_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("PhotonBuffer"),
            size: result_buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }
    );

    // ------------------ Uniforms ------------------ //

    let inverse_resolution = 1.0 / data.output_resolution as f32;
    let random_seed = rand::random::<f32>();

    let mvp_inverse_buffer = create_matrix_uniform_buffer(&device, &camera_matrix, "MVPInverseBuffer");
    let resolution_buffer = create_vector2_u32_uniform_buffer(&device, &[data.output_resolution, data.output_resolution], "ResolutionBuffer");
    let inverse_resolution_buffer = create_vector2_f32_uniform_buffer(&device, &[inverse_resolution, inverse_resolution], "InvResBuffer");
    let random_seed_buffer = create_f32_uniform_buffer(&device, random_seed, "RandSeedBuffer");

    let extinction_buffer = create_f32_uniform_buffer(&device, data.extinction, "ExtinctionBuffer");
    let anisotropy_buffer = create_f32_uniform_buffer(&device, data.anisotropy, "AnisotropyBuffer");
    let max_bounces_buffer = create_u32_uniform_buffer(&device, data.max_bounces, "MaxBouncesBuffer");
    let steps_buffer = create_u32_uniform_buffer(&device, data.steps, "StepsBuffer");

    // ------------------ Textures ------------------ //

    let foo_texture = create_2d_render_texture(&device, data.output_resolution, "FooTexture");
    let foo_texture_view = foo_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let tf_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("TFTexture"),
            size: wgpu::Extent3d {
                width: data.transfer_function_len,
                height: 1,
                depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        }
    );

    let volume_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("VolumeTexture"),
            size: wgpu::Extent3d {
                width: data.volume_dims.0,
                height: data.volume_dims.1,
                depth_or_array_layers: data.volume_dims.2
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        }
    );

    queue.write_texture(
        wgpu::ImageCopyTextureBase {
            texture: &tf_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        &data.transfer_function,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(data.transfer_function_len * 4),
            rows_per_image: std::num::NonZeroU32::new(1)
        },
        wgpu::Extent3d {
            width: data.transfer_function_len,
            height: 1,
            depth_or_array_layers: 1
        }
    );

    queue.write_texture(
        wgpu::ImageCopyTextureBase {
            texture: &volume_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        &data.volume,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(data.volume_dims.0),
            rows_per_image: std::num::NonZeroU32::new(data.volume_dims.1)
        },
        wgpu::Extent3d {
            width: data.volume_dims.0,
            height: data.volume_dims.1,
            depth_or_array_layers: data.volume_dims.2
        }
    );

    let volume_view = volume_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let volume_sampler = create_texture_sampler(&device, "VolumeSampler");

    let tf_view = tf_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let tf_sampler = create_texture_sampler(&device, "TFSampler");

    // ------------------ Bind Group Layouts ------------------- //

    let uniforms_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("UniformsGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                }
            ]
        }
    );

    let textures_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("PhotonGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false
                        },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        wgpu::SamplerBindingType::NonFiltering
                    ),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        wgpu::SamplerBindingType::NonFiltering
                    ),
                    count: None,
                }
            ]
        }
    );

    let result_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("ResultGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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

    // ------------------ Bind Groups ------------------- //

    let uniforms_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("UniformsBindGroup"),
            layout: &uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mvp_inverse_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resolution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inverse_resolution_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: random_seed_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: extinction_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: anisotropy_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: max_bounces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: steps_buffer.as_entire_binding(),
                }
            ]
        }
    );

    let textures_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("TextureBindGroup"),
            layout: &textures_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&volume_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&volume_sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&tf_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&tf_sampler)
                }
            ]
        }
    );

    let result_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("ResultBindGroup"),
            layout: &result_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: result_buffer.as_entire_binding(),
                }
            ]
        }
    );

    // ------------------ Pipeline ------------------- //

    let vertex_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_main_vertex.wgsl")
    );
    let fragment_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_main_fragment.wgsl")
    );

    let render_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("InitRenderPipelineLayout"),
            bind_group_layouts: &[
                &uniforms_bind_group_layout,
                &textures_bind_group_layout,
                &result_bind_group_layout
            ],
            push_constant_ranges: &[]
        }
    );

    let render_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("InitRenderPipeline"),
            layout: Some(&render_pipeline_layout),
            multiview: None,
            depth_stencil: None,
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "main",
                buffers: &[]
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                ]
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            }
        }
    );

    // ------------------ Rendering ------------------- //

    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor {
            label: Some("CommandEncoder"),
        }
    );

    {
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("RenderPassDesc"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &foo_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: true
                    },
                })
            ],
            depth_stencil_attachment: None,
        };

        let mut render_pass = encoder.begin_render_pass(&render_pass_descriptor);

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &uniforms_bind_group, &[]);
        render_pass.set_bind_group(1, &textures_bind_group, &[]);
        render_pass.set_bind_group(2, &result_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }

    let output_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("OutputBuffer"),
            size: result_buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }
    );

    encoder.copy_buffer_to_buffer(
        &result_buffer,
        0,
        &output_buffer,
        0,
        result_buffer_size as u64
    );

    queue.submit([
        encoder.finish()
    ]);

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
        let (_, colors, _) = data.align_to::<f32>();
        for i in (0..colors.len()).step_by(4) {
            let r = (colors[i] * 255.0) as u8;
            let g = (colors[i + 1] * 255.0) as u8;
            let b = (colors[i + 2] * 255.0) as u8;
            output.push(r);
            output.push(g);
            output.push(b);
        }
    }
}
