use std::{ops::{Sub, Neg}, fmt::Display};

#[derive(Debug)]
pub struct Matrix4f {
    pub m: [[f32; 4]; 4]
}

#[derive(Clone, Copy, Debug)]
pub struct Vector3f {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32
}


impl Vector3f {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        return Self {
            x,
            y,
            z
        };
    }

    /*pub fn clone(&self) -> Self {
        Vector3f {
            x: self.x,
            y: self.y,
            z: self.z
        }
    }*/

    pub fn distance(&self) -> f32 {
        return (self.x*self.x + self.y*self.y + self.z*self.z).sqrt();
    }

    pub fn normalize(&mut self) {
        let dist = self.distance();
        if dist == 0.0 {
            self.x = 0.0;
            self.y = 0.0;
            self.z = 0.0;
            return;
        }
        self.x = self.x / dist;
        self.y = self.y / dist;
        self.z = self.z / dist;
    }

    pub fn cross(vec1: &Self, vec2: &Self) -> Self {
        let x = vec1.y * vec2.z - vec1.z * vec2.y;
        let y = vec1.z * vec2.x - vec1.x * vec2.z;
        let z = vec1.x * vec2.y - vec1.y * vec2.x;
        return Self { x, y, z }
    }

    pub fn dot(vec1: &Self, vec2: &Self) -> f32 {
        return vec1.x*vec2.x + vec1.y*vec2.y + vec1.z*vec2.z;
    }
}

impl Sub for Vector3f {
    type Output = Vector3f;

    fn sub(self, rhs: Self) -> Self::Output {
        return Vector3f {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        };
    }
}

impl Neg for Vector3f {
    type Output = Vector3f;

    fn neg(self) -> Self::Output {
        return Vector3f {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }
}

impl Quaternion {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        return Self {
            x,
            y,
            z,
            w
        };
    }

    pub fn to_rotation_matrix(&self) -> Matrix4f {
        let x = self.x;
        let y = self.y;
        let z = self.z;
        let w = self.w;

        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;

        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        let res = Matrix4f::from_values(vec![
            1.0 - (yy + zz), xy + wz, xz - wy, 0.0,
            xy - wz, 1.0 - (xx + zz), yz + wx, 0.0,
            xz + wy, yz - wx, 1.0 - (xx + yy), 0.0,
            0.0, 0.0, 0.0, 1.0
        ]);
        return res;
    }
}

impl Matrix4f {
    pub fn new() -> Self {
        return Self {
            m: [[1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]]
        }
    }

    pub fn from_values(val: Vec<f32>) -> Self {
        let m = [[val[0], val[1], val[2], val[3]],
                [val[4], val[5], val[6], val[7]],
                [val[8], val[9], val[10], val[11]],
                [val[12], val[13], val[14], val[15]]];
        return Matrix4f { m };
    }

    pub fn mutiply(matrix1: &Matrix4f, matrix2: &Matrix4f) -> Self {
        let mut res = Matrix4f::new();
        let m1 = matrix1.m;
        let m2 = matrix2.m;

        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += m1[i][k] * m2[k][j];
                }
                res.m[i][j] = sum;
            }
        }

        return res;
    }

    pub fn transpose(&self) -> Self {
        let mut res = Matrix4f::new();
        for i in 0..4 {
            for j in 0..4 {
                res.m[j][i] = self.m[i][j];
            }
        }
        return res;
    }

    pub fn det(&self) -> f32 {
        let m = self.m;

        let det = m[0][0] *
                     (m[1][1] * m[2][2] * m[3][3]
                    + m[1][2] * m[2][3] * m[3][1]
                    + m[1][3] * m[2][1] * m[3][2]
                    - m[1][3] * m[2][2] * m[3][1]
                    - m[1][2] * m[1][2] * m[3][3]
                    - m[1][1] * m[2][3] * m[3][2]) -
                  m[1][0] *
                     (m[0][1] * m[2][2] * m[3][3]
                    + m[0][2] * m[2][3] * m[3][1]
                    + m[0][3] * m[2][1] * m[3][2]
                    - m[0][3] * m[2][2] * m[3][1]
                    - m[0][2] * m[2][1] * m[3][3]
                    - m[0][1] * m[2][3] * m[3][2]) +
                  m[2][0] *
                     (m[0][1] * m[1][2] * m[3][3]
                    + m[0][2] * m[1][3] * m[3][1]
                    + m[0][3] * m[1][1] * m[3][2]
                    - m[0][3] * m[1][2] * m[3][1]
                    - m[0][2] * m[1][1] * m[3][3]
                    - m[0][1] * m[1][3] * m[3][2]) -
                  m[3][0] *
                     (m[0][1] * m[1][2] * m[2][3]
                    + m[0][2] * m[1][3] * m[2][1]
                    + m[0][3] * m[1][1] * m[2][2]
                    - m[0][3] * m[1][2] * m[2][1]
                    - m[0][2] * m[1][1] * m[2][3]
                    - m[0][1] * m[1][3] * m[2][2]);
        return det;
    }

    pub fn inverse(&self) -> Self {
        let mut res = Matrix4f::new();
        let m = self.m;
        let det_inv = 1.0 / self.det();

        let m11 = m[0][0]; let m12 = m[0][1]; let m13 = m[0][2]; let m14 = m[0][3];
        let m21 = m[1][0]; let m22 = m[1][1]; let m23 = m[1][2]; let m24 = m[1][3];
        let m31 = m[2][0]; let m32 = m[2][1]; let m33 = m[2][2]; let m34 = m[2][3];
        let m41 = m[3][0]; let m42 = m[3][1]; let m43 = m[3][2]; let m44 = m[3][3];

        res.m[0][0] = (m22 * m33 * m44 + m23 * m34 * m42 + m24 * m32 * m43 - m22 * m34 * m43 - m23 * m32 * m44 - m24 * m33 * m42) * det_inv;
        res.m[0][1] = (m12 * m34 * m43 + m13 * m32 * m44 + m14 * m33 * m42 - m12 * m33 * m44 - m13 * m34 * m42 - m14 * m32 * m43) * det_inv;
        res.m[0][2] = (m12 * m23 * m44 + m13 * m24 * m42 + m14 * m22 * m43 - m12 * m24 * m43 - m13 * m22 * m44 - m14 * m23 * m42) * det_inv;
        res.m[0][3] = (m12 * m24 * m33 + m13 * m22 * m34 + m14 * m23 * m32 - m12 * m23 * m34 - m13 * m24 * m32 - m14 * m22 * m33) * det_inv;

        res.m[1][0] = (m21 * m34 * m43 + m23 * m31 * m44 + m24 * m33 * m41 - m21 * m33 * m44 - m23 * m34 * m41 - m24 * m31 * m43) * det_inv;
        res.m[1][1] = (m11 * m33 * m44 + m13 * m34 * m41 + m14 * m31 * m43 - m11 * m34 * m43 - m13 * m31 * m44 - m14 * m33 * m41) * det_inv;
        res.m[1][2] = (m11 * m24 * m43 + m13 * m21 * m44 + m14 * m23 * m41 - m11 * m23 * m44 - m13 * m24 * m41 - m14 * m21 * m43) * det_inv;
        res.m[1][3] = (m11 * m23 * m34 + m13 * m24 * m31 + m14 * m21 * m33 - m11 * m24 * m33 - m13 * m21 * m34 - m14 * m23 * m31) * det_inv;

        res.m[2][0] = (m21 * m32 * m44 + m22 * m34 * m41 + m24 * m31 * m42 - m21 * m34 * m42 - m22 * m31 * m44 - m24 * m32 * m41) * det_inv;
        res.m[2][1] = (m11 * m34 * m42 + m12 * m31 * m44 + m14 * m32 * m41 - m11 * m32 * m44 - m12 * m34 * m41 - m14 * m31 * m42) * det_inv;
        res.m[2][2] = (m11 * m22 * m44 + m12 * m24 * m41 + m14 * m21 * m42 - m11 * m24 * m42 - m12 * m21 * m44 - m14 * m22 * m41) * det_inv;
        res.m[2][3] = (m11 * m24 * m32 + m12 * m21 * m34 + m14 * m22 * m31 - m11 * m22 * m34 - m12 * m24 * m31 - m14 * m21 * m32) * det_inv;

        res.m[3][0] = (m21 * m33 * m42 + m22 * m31 * m43 + m23 * m32 * m41 - m21 * m32 * m43 - m22 * m33 * m41 - m23 * m31 * m42) * det_inv;
        res.m[3][1] = (m11 * m32 * m43 + m12 * m33 * m41 + m13 * m31 * m42 - m11 * m33 * m42 - m12 * m31 * m43 - m13 * m32 * m41) * det_inv;
        res.m[3][2] = (m11 * m23 * m42 + m12 * m21 * m43 + m13 * m22 * m41 - m11 * m22 * m43 - m12 * m23 * m41 - m13 * m21 * m42) * det_inv;
        res.m[3][3] = (m11 * m22 * m33 + m12 * m23 * m31 + m13 * m21 * m32 - m11 * m23 * m32 - m12 * m21 * m33 - m13 * m22 * m31) * det_inv;

        return res;
    }

    pub fn from_frustum(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut res = Matrix4f::new();
        res.m[0][0] = 2.0 * near / (right - left);
        res.m[0][2] = (right + left) / (right - left);

        res.m[1][1] = 2.0 * near / (top - bottom);
        res.m[1][2] = (top + bottom) / (top - bottom);

        res.m[2][2] = -(far + near) / (far - near);
        res.m[2][3] = -2.0 * far * near / (far - near);

        res.m[3][2] = -1.0;
        res.m[3][3] = 0.0;

        return res;
    }

    /*pub fn from_translation(x: f32, y: f32, z: f32) -> Matrix4f {
        let mut res = Matrix4f::new();

        res.m[0][3] = x;
        res.m[1][3] = y;
        res.m[2][3] = z;
        
        return res;
    }

    pub fn from_rotation_x(angle: f32) -> Matrix4f {
        let mut res = Matrix4f::new();

        let s = f32::sin(angle);
        let c = f32::cos(angle);

        res.m[1][1] = c;
        res.m[1][2] = s;
        res.m[2][1] = -s;
        res.m[2][2] = c;

        return res;
    }

    pub fn from_rotation_y(angle: f32) -> Matrix4f {
        let mut res = Matrix4f::new();

        let s = f32::sin(angle);
        let c = f32::cos(angle);

        res.m[0][0] = c;
        res.m[0][2] = -s;
        res.m[2][0] = s;
        res.m[2][2] = c;

        return res;
    }

    pub fn from_rotation_z(angle: f32) -> Matrix4f {
        let mut res = Matrix4f::new();

        let s = f32::sin(angle);
        let c = f32::cos(angle);

        res.m[0][0] = c;
        res.m[0][1] = s;
        res.m[1][0] = -s;
        res.m[1][1] = c;

        return res;
    }

    pub fn from_scale(x: f32, y: f32, z: f32) -> Matrix4f {
        let mut res = Matrix4f::new();
        res.m[0][0] = x;
        res.m[1][1] = y;
        res.m[2][2] = z;
        return res;
    }*/
}

impl Display for Matrix4f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}\n{:?}\n{:?}\n{:?}",
        self.m[0], self.m[1], self.m[2], self.m[3])
    }
}