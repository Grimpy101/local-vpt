use crate::math::{Quaternion, Vector3f, Matrix4f};

pub struct Camera {
    position: Vector3f,
    rotation: Quaternion,
    fov_x: f32,
    fov_y: f32,
    near: f32,
    far: f32,
    view_matrix: Matrix4f,
    proj_matrix: Matrix4f
}

impl Camera {
    pub fn new() -> Self {
        return Self {
            position: Vector3f::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 1.0),
            fov_x: 1.0,
            fov_y: 1.0,
            near: 0.1,
            far: 50.0,
            view_matrix: Matrix4f::new(),
            proj_matrix: Matrix4f::new()
        }
    }

    pub fn set_position(&mut self, pos: Vector3f) {
        self.position = pos;
    }

    pub fn look_at(&mut self, focus: Vector3f) {
        let a = Vector3f::new(0.0, 0.0, -1.0);
        let mut b = focus - self.position;
        b.normalize();

        let phi = -Vector3f::dot(&a, &b).acos();
        let cos_hphi = (phi / 2.0).cos();
        let sin_hphi = (phi / 2.0).sin();

        let mut axis = Vector3f::cross(&a, &b);
        axis.normalize();

        let mut q = Quaternion::new(
            axis.x * sin_hphi,
            axis.y * sin_hphi,
            axis.z * sin_hphi,
            cos_hphi
        );
        q.normalize();
        self.rotation = q;
    }

    pub fn set_fov_x(&mut self, fov: f32) {
        self.fov_x = fov;
    }

    pub fn set_fov_y(&mut self, fov: f32) {
        self.fov_y = fov;
    }

    pub fn set_fov(&mut self, focal_length: f32, aspect_ratio: f32) {
        let w = 1.0 * aspect_ratio;
        let h = 1.0;
        let fov_x = (w / (2.0 * focal_length)).tan() * 2.0;
        let fov_y = (h / (2.0 * focal_length)).tan() * 2.0;

        self.fov_x = fov_x;
        self.fov_y = fov_y;
    }

    pub fn update_view_matrix(&mut self) {
        let mut view_matrix = self.rotation.to_rotation_matrix();
        view_matrix.m[0][3] = self.position.x;
        view_matrix.m[1][3] = self.position.y;
        view_matrix.m[2][3] = self.position.z;
        self.view_matrix = view_matrix.inverse();
    }

    pub fn update_projection_matrix(&mut self) {
        let w = self.fov_x * self.near;
        let h = self.fov_y * self.near;

        self.proj_matrix = Matrix4f::from_frustum(
            -w, w, -h, h, self.near, self.far
        );
    }

    pub fn update_matrices(&mut self) {
        self.update_view_matrix();
        self.update_projection_matrix();
    }

    pub fn get_view_matrix(&self) -> &Matrix4f {
        return &self.view_matrix;
    }

    pub fn get_projection_matrix(&self) -> &Matrix4f {
        return &self.proj_matrix;
    }
}