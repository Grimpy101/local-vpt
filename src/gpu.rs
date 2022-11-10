use std::mem;

use wgpu::{util::DeviceExt, include_wgsl};

use crate::{math::{Vector3f, Matrix4f}, camera::Camera};


pub struct RenderData {
    pub output_resolution: u32,
    pub volume: Vec<u8>,
    pub volume_dims: (u32, u32, u32),
    pub transfer_function: Vec<u8>,
    pub transfer_function_len: u32
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Photon {
    position: [f32; 3],
    direction: [f32; 3],
    transmittance: [f32; 3],
    samples: u32
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UniformPack {
    view_proj: [[f32; 4]; 4],

}

static CLIP_QUAD: &[f64; 8] = &[-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];
static CLIP_QUAD_INDICES: &[u16; 6] = &[0, 1, 2, 0, 2, 3];

pub async fn render(input: RenderData, output: &mut Vec<u8>) {
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

    // ------------ Declarations --------------//

    let vol_dims = input.volume_dims;
    let tf_len = input.transfer_function_len;
    let out_res = input.output_resolution;
    let volume_scale = vec![1.0, 1.0, 1.0];

    // ------------ Camera --------------//

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

    // ------------ Textures --------------//

    let out_texture_desc = wgpu::TextureDescriptor {
        label: Some("OutTexture"),
        size: wgpu::Extent3d {
            width: out_res,
            height: out_res,
            depth_or_array_layers: 1
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
    };
    
    let out_texture = device.create_texture(&out_texture_desc);

    let volume_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: vol_dims.0,
                height: vol_dims.1,
                depth_or_array_layers: vol_dims.2
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("VolumeTexture"),
        }
    );

    let transfer_function_texture = device.create_texture(
        &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: tf_len,
                height: 1,
                depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("TFTexture"),
        }
    );

    // ------------ Buffers --------------//

    let f32_size = std::mem::size_of::<f32>() as u32;

    let out_buffer_size = (f32_size * 4 * out_res * out_res) as wgpu::BufferAddress;
    let out_buffer_desc = wgpu::BufferDescriptor {
        label: Some("OutputBuffer"),
        size: out_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    };
    let out_buffer = device.create_buffer(&out_buffer_desc);

    // ------------ Work --------------//

    let shader = device.create_shader_module(
        include_wgsl!("shaders/reset.wgsl")
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ResetEncoder")
    });

    let mvp_inverse_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("MVPInverseBuffer"),
            contents: bytemuck::cast_slice(&pvm_inverse.m),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
    );

    let mvp_bind_group_layout = device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            label: Some("MVPBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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

    let 

    let render_pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("ResetPipelineLayout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        }
    );

    let render_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some("ResetPipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[]
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview: None
        }
    );

    let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ResetRenderPass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0
                            }
                        ),
                        store: true,
                    }
                })
            ],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.draw(0..6, 0..1);
    }

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTextureBase {
            texture: &out_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        },
        wgpu::ImageCopyBuffer {
            buffer: &out_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(f32_size * out_res * 4).unwrap()),
                rows_per_image: Some(std::num::NonZeroU32::new(out_res).unwrap())
            }
        },
        out_texture_desc.size
    );

    queue.submit(Some(encoder.finish()));

    {
        let buffer_slice = out_buffer.slice(..);
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
            let (_, floats, _) = data.align_to::<f32>();
            for f in floats {
                let i = (f * 255.0) as u8;
                output.push(i);
            }
        }
    }


}