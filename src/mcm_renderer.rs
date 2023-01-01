use std::num::NonZeroU32;

use wgpu::{util::DeviceExt, include_wgsl};

use crate::{pipeline::RenderData, math::Matrix4f};

struct TextureViewSampler {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler
}

struct RenderPassTextures {
    pub position: [TextureViewSampler; 2],
    pub direction: [TextureViewSampler; 2],
    pub transmittance_sampes: [TextureViewSampler; 2],
    pub radiance_bounces: [TextureViewSampler; 2]
}

fn create_texture_view_sampler_pair(device: &wgpu::Device, w: u32, h: u32) -> [TextureViewSampler; 2] {
    let texture1 = device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("Texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT
        }
    );
    let view1 = texture1.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler1 = device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some("Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );

    let texture2 = device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("Texture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT
        }
    );
    let view2 = texture2.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler2 = device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some("Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );

    let tvs1 = TextureViewSampler {
        texture: texture1,
        view: view1,
        sampler: sampler1
    };
    let tvs2 = TextureViewSampler {
        texture: texture2,
        view: view2,
        sampler: sampler2
    };

    return [tvs1, tvs2];
}

fn create_texture_sampler(device: &wgpu::Device, label: &str, linear: bool) -> wgpu::Sampler {
    let mut filter_mode = wgpu::FilterMode::Nearest;
    if linear {
        filter_mode = wgpu::FilterMode::Linear;
    }
    return device.create_sampler(
        &wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }
    );
}

fn create_matrix_uniform_buffer(device: &wgpu::Device, matrix: &Matrix4f, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&matrix.m),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

fn create_vector2_u32_uniform_buffer(device: &wgpu::Device, vector: &[u32; 2], label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(vector),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );
}

fn create_vector2_f32_uniform_buffer(device: &wgpu::Device, vector: &[f32; 2], label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(vector),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );
}

fn create_f32_uniform_buffer(device: &wgpu::Device, num: f32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[num]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}

fn create_u32_uniform_buffer(device: &wgpu::Device, num: u32, label: &str) -> wgpu::Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[num]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        }
    );
}


fn reset(device: &wgpu::Device, render_pass_textures: &RenderPassTextures, global_uniforms_layout: &wgpu::BindGroupLayout,
    global_uniforms_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
    /* -------------- Global Uniforms --------------- */

    let random_seed = rand::random::<f32>();
    let random_seed_buffer = create_f32_uniform_buffer(&device, random_seed, "RandSeedBuffer");

    /* -------------- Local Bind Groups --------------- */

    let local_uniforms_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("LocalUniformsGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    }
                }
            ]
        }
    );

    let local_uniforms_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("LocalUniformsBindGroup"),
            layout: &local_uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: random_seed_buffer.as_entire_binding()
                }
            ]
        }
    );

    /* -------------- Pipeline --------------- */

    let vertex_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_reset_vertex.wgsl")
    );
    let fragment_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_reset_fragment.wgsl")
    );

    let render_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("ResetRenderPipelineLayout"),
            bind_group_layouts: &[
                global_uniforms_layout,
                &local_uniforms_bind_group_layout
            ],
            push_constant_ranges: &[]
        }
    );

    let render_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("ResetRenderPipeline"),
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
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    })
                ],
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
            },
        }
    );

    /* -------------- Rendering --------------- */

    {
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("RenderPass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &render_pass_textures.position[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color::TRANSPARENT
                        ),
                        store: true
                    }
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &render_pass_textures.direction[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color::TRANSPARENT
                        ),
                        store: true
                    }
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &render_pass_textures.transmittance_sampes[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color::TRANSPARENT
                        ),
                        store: true
                    }
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &render_pass_textures.radiance_bounces[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color::TRANSPARENT
                        ),
                        store: true
                    }
                }),
            ],
            depth_stencil_attachment: None,
        };

        let mut render_pass = encoder.begin_render_pass(&render_pass_descriptor);

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &global_uniforms_group, &[]);
        render_pass.set_bind_group(1, &local_uniforms_bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
    println!("Reset image.");
}

pub async fn render(device: &wgpu::Device, queue: &wgpu::Queue, data: &RenderData, camera_matrix: &Matrix4f, output: &mut Vec<u8>) {
    /* -------------- Global Textures --------------- */
    let bytes_alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let res_x = (data.output_resolution[0] as f32 / bytes_alignment as f32).ceil() as u32 * bytes_alignment;
    let res_y = data.output_resolution[1];

    let position_texture_pair = create_texture_view_sampler_pair(&device, res_x, res_y);
    let direction_texture_pair = create_texture_view_sampler_pair(&device, res_x, res_y);
    let transmittance_samples_texture_pair = create_texture_view_sampler_pair(&device, res_x, res_y);
    let radiance_bounces_texture_pair = create_texture_view_sampler_pair(&device, res_x, res_y);

    let render_pass_textures = RenderPassTextures {
        position: position_texture_pair,
        direction: direction_texture_pair,
        transmittance_sampes: transmittance_samples_texture_pair,
        radiance_bounces: radiance_bounces_texture_pair,
    };

    /* -------------- Global Uniforms --------------- */

    let inverse_resolution_x = 1.0 / res_x as f32;
    let inverse_resolution_y = 1.0 / res_y as f32;

    let mvp_inverse_buffer = create_matrix_uniform_buffer(&device, &camera_matrix, "MVPInverseBuffer");
    let resolution_buffer = create_vector2_u32_uniform_buffer(&device, &[res_x, res_y], "ResolutionBuffer");
    let inverse_resolution_buffer = create_vector2_f32_uniform_buffer(&device, &[inverse_resolution_x, inverse_resolution_y], "InvResBuffer");

    /* -------------- Global Bind Groups --------------- */

    let global_uniforms_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("GlobalUniformsBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    }
                }
            ]
        }
    );

    let global_uniforms_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("GlobalUniformsBindGroup"),
            layout: &global_uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mvp_inverse_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resolution_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inverse_resolution_buffer.as_entire_binding()
                },
            ]
        }
    );

    /* -------------- Rendering --------------- */

    let mut encoder = device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor {
            label: Some("MCMRendererCommandEncoder"),
        }
    );

    reset(device, &render_pass_textures, &global_uniforms_bind_group_layout, &global_uniforms_bind_group, &mut encoder);

    let extinction_buffer = create_f32_uniform_buffer(&device, data.extinction, "ExtinctionBuffer");
    let anisotropy_buffer = create_f32_uniform_buffer(&device, data.anisotropy, "AnisotropyBuffer");
    let max_bounces_buffer = create_u32_uniform_buffer(&device, data.max_bounces, "MaxBouncesBuffer");
    let steps_buffer = create_u32_uniform_buffer(&device, data.steps, "StepsBuffer");

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
                width: data.volume_dims[0],
                height: data.volume_dims[1],
                depth_or_array_layers: data.volume_dims[2]
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
            bytes_per_row: std::num::NonZeroU32::new(data.volume_dims[0]),
            rows_per_image: std::num::NonZeroU32::new(data.volume_dims[1])
        },
        wgpu::Extent3d {
            width: data.volume_dims[0],
            height: data.volume_dims[1],
            depth_or_array_layers: data.volume_dims[2]
        }
    );

    let volume_view = volume_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let volume_sampler = create_texture_sampler(&device, "VolumeSampler", data.linear);

    let tf_view = tf_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let tf_sampler = create_texture_sampler(&device, "TFSampler", false);

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
                }
            ]
        }
    );

    let textures_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("TexturesGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true
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
                        wgpu::SamplerBindingType::Filtering
                    ),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true
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
                        wgpu::SamplerBindingType::Filtering
                    ),
                    count: None,
                },
            ]
        }
    );

    let old_data_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("TexturesGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
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
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        wgpu::SamplerBindingType::NonFiltering
                    ),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
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
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(
                        wgpu::SamplerBindingType::NonFiltering
                    ),
                    count: None,
                },
            ]
        }
    );

    let random_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("RandomGroupLayout"),
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
                }
            ]
        }
    );

    let uniforms_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("UniformsGroup"),
            layout: &uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mvp_inverse_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: resolution_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inverse_resolution_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: extinction_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: anisotropy_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: max_bounces_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: steps_buffer.as_entire_binding()
                },
            ]
        }
    );

    let textures_bind_group = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("TexturesGroup"),
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

    let vertex_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_main_vertex.wgsl")
    );
    let fragment_shader = device.create_shader_module(
        include_wgsl!("shaders/mcm_main_fragment.wgsl")
    );

    let render_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("RenderPipelineLayout"),
            bind_group_layouts: &[
                &random_bind_group_layout,
                &uniforms_bind_group_layout,
                &textures_bind_group_layout,
                &old_data_bind_group_layout
            ],
            push_constant_ranges: &[]
        }
    );

    let render_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("RenderPipeline"),
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
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL
                    })
                ],
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
            },
        }
    );

    let old_data_bind_group1 = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("OldDataBindGroup"),
            layout: &old_data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.position[0].view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.position[0].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.direction[0].view)
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.direction[0].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.transmittance_sampes[0].view)
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.transmittance_sampes[0].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.radiance_bounces[0].view)
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.radiance_bounces[0].sampler)
                }
            ]
        }
    );

    let old_data_bind_group2 = device.create_bind_group(
        &wgpu::BindGroupDescriptor {
            label: Some("OldDataBindGroup"),
            layout: &old_data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.position[1].view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.position[1].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.direction[1].view)
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.direction[1].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.transmittance_sampes[1].view)
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.transmittance_sampes[1].sampler)
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&render_pass_textures.radiance_bounces[1].view)
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&render_pass_textures.radiance_bounces[1].sampler)
                }
            ]
        }
    );

    let render_pass_descriptor1 = wgpu::RenderPassDescriptor {
        label: Some("RenderPassDesc"),
        color_attachments: &[
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.position[1].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.direction[1].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.transmittance_sampes[1].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.radiance_bounces[1].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
        ],
        depth_stencil_attachment: None,
    };

    let render_pass_descriptor2 = wgpu::RenderPassDescriptor {
        label: Some("RenderPassDesc"),
        color_attachments: &[
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.position[0].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.direction[0].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.transmittance_sampes[0].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &render_pass_textures.radiance_bounces[0].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(
                        wgpu::Color::TRANSPARENT
                    ),
                    store: true
                }
            }),
        ],
        depth_stencil_attachment: None,
    };

    for i in 0..data.iterations {
        let random_seed = rand::random::<f32>();
        let random_seed_buffer = create_f32_uniform_buffer(&device, random_seed, "RandSeedBuffer");
        let random_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("RandomBindGroup"),
                layout: &random_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: random_seed_buffer.as_entire_binding()
                    }
                ]
            }
        );

        {
            let mut render_pass = if i % 2 == 0 {
                encoder.begin_render_pass(&render_pass_descriptor1)
            } else {
                encoder.begin_render_pass(&render_pass_descriptor2)
            };
            
            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &random_bind_group, &[]);
            render_pass.set_bind_group(1, &uniforms_bind_group, &[]);
            render_pass.set_bind_group(2, &textures_bind_group, &[]);
            if i % 2 == 0 {
                render_pass.set_bind_group(3, &old_data_bind_group1, &[]);
            } else {
                render_pass.set_bind_group(3, &old_data_bind_group2, &[]);
            }
            render_pass.draw(0..4, 0..1);
        }
    }

    let f32_size = std::mem::size_of::<f32>() as u32;
    let result_buffer_size = (f32_size * 4 * res_x * res_y) as u64;
    let result_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("ResultBuffer"),
            size: result_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }
    );

    let result_index = (data.iterations % 2) as usize;

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTextureBase {
            texture: &render_pass_textures.radiance_bounces[result_index].texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        wgpu::ImageCopyBuffer {
            buffer: &result_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(f32_size * 4 * res_x),
                rows_per_image: NonZeroU32::new(res_y)
            },
        },
        wgpu::Extent3d {
            width: res_x,
            height: res_y,
            depth_or_array_layers: 1,
        }
    );

    queue.submit([encoder.finish()]);

    {
        let buffer_slice = result_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();
        let buffer_data = buffer_slice.get_mapped_range();

        unsafe {
            let (_, colors, _) = buffer_data.align_to::<f32>();
            let colors_width = res_x;
            let real_width = data.output_resolution[0];
            let real_hight = data.output_resolution[1];

            for y in 0..real_hight {
                for x in 0..real_width {
                    let index = ((y * colors_width + x) * 4) as usize;
                    let r = (colors[index] * 255.0) as u8;
                    let g = (colors[index+1] * 255.0) as u8;
                    let b = (colors[index+2] * 255.0) as u8;
                    output.push(r);
                    output.push(g);
                    output.push(b);
                }
            }
        }
    }
}