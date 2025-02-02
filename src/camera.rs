use bytemuck::{Pod, Zeroable};
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: nalgebra::Matrix4<f32> = nalgebra::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
#[rustfmt::skip]
pub const GLTF_TO_VULKAN_MATRIX: nalgebra::Matrix4<f32> = nalgebra::Matrix4::new(
1. , 0. ,  0. , 0.,
0., -1.,  0.,  0.,
0.,  0.,  1.,  0.,
0.,  0.,  0.,  1.,
);

#[derive(Debug)]
pub struct Camera {
    pub eye: nalgebra::Point3<f32>,
    pub target: nalgebra::Point3<f32>,
    pub up: nalgebra::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> nalgebra::Matrix4<f32> {
        // 1.
        let view = nalgebra::Matrix4::look_at_rh(&self.eye, &self.target, &self.up);
        // 2.
        let projection = nalgebra::Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar);

        // 3.
        // Convert Perspective3 to Matrix4
        GLTF_TO_VULKAN_MATRIX * projection.as_matrix() * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_projection: nalgebra::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_projection = camera.build_view_projection_matrix().into();
    }
}
