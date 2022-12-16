use crate::{camera::Camera, math::{Vector3f, Matrix4f}, mcm_renderer};

pub struct RenderData {
    pub output_resolution: [u32; 2],
    pub volume: Vec<u8>,
    pub volume_dims: [u32; 3],
    pub transfer_function: Vec<u8>,
    pub transfer_function_len: u32,
    pub extinction: f32,
    pub anisotropy: f32,
    pub max_bounces: u32,
    pub steps: u32,
    pub camera_position: [f32; 3],
    pub linear: bool,
    pub iterations: u32,
    pub mvp_matrix: Option<[f32; 16]>
}

pub async fn render(data: RenderData, output: &mut Vec<u8>) {
    //let vol_dims = data.volume_dims;
    //let tf_len = data.transfer_function_len;
    let volume_scale = vec![1.0, 1.0, 1.0];

    let mut camera = Camera::new();
    camera.set_position(
        Vector3f::new(
            data.camera_position[0],
            data.camera_position[1],
            data.camera_position[2]
        )
    );
    camera.look_at(Vector3f::new(0.0, 0.0, 0.0));
    camera.set_fov_x(0.512);
    camera.set_fov_y(0.512);
    camera.update_matrices();

    let pvm_inverse = if data.mvp_matrix.is_none() {
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
    
        pvm_matrix.inverse().transpose()
    } else {
        Matrix4f::from_values(
            data.mvp_matrix.unwrap().to_vec()
        )
    };

    // -------------- Initialization -------------- //

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


    //mcm_renderer::render(&device, &queue, &data, &pvm_inverse, output).await;
    mcm_renderer::render(&device, &queue, &data, &pvm_inverse, output).await;
}