use std::f32::consts::FRAC_PI_4;

use bytemuck::{Pod, Zeroable};

use nalgebra::Matrix4;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[rustfmt::skip]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
#[rustfmt::skip]
pub const GLTF_TO_VULKAN_MATRIX: Matrix4<f32> = Matrix4::new(
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
    #[allow(unused)]
    pub fn new(
        // position the camera 1 unit up and 2 units back
        // +z is out of the screen
        eye: nalgebra::Point3<f32>,
        // have it look at the origin
        target: nalgebra::Point3<f32>,
        // which way is "up"
        up: nalgebra::Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        // 1.
        let view = Matrix4::look_at_rh(&self.eye, &self.target, &self.up);
        // 2.
        let projection = nalgebra::Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar);

        // 3.
        // Convert Perspective3 to Matrix4
        GLTF_TO_VULKAN_MATRIX * projection.as_matrix() * view
    }

    pub fn build_projection_matrix(&self) -> Matrix4<f32> {
        let projection = nalgebra::Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar);

        GLTF_TO_VULKAN_MATRIX * projection.as_matrix()
    }

    pub fn build_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.eye, &self.target, &self.up)
    }

    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height.max(1) as f32;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: nalgebra::Point3::new(0.97, 0.97, 1.97),
            target: nalgebra::Point3::new(0.0, 0.0, 0.0),
            up: nalgebra::Vector3::y(),
            aspect: 800_f32 / 600_f32,
            fovy: FRAC_PI_4,
            znear: 0.1,
            zfar: 100.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[allow(unused)]
pub struct CameraUniform {
    pub view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            view_projection: Matrix4::identity().into(),
        }
    }
    #[allow(unused)]
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_projection = camera.build_view_projection_matrix().into();
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(&camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

// MVP (Model-View-Projection)
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Mvp {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

impl Mvp {
    pub fn new() -> Mvp {
        Mvp {
            model: Matrix4::identity().into(),
            view: Matrix4::identity().into(),
            projection: Matrix4::identity().into(),
        }
    }

    pub fn update_view(&mut self, camera: &Camera) {
        self.view = camera.build_view_matrix().into();
    }

    pub fn update_projection(&mut self, camera: &Camera) {
        self.projection = camera.build_projection_matrix().into();
    }

    pub fn update_model_translate(&mut self, vector: nalgebra::Vector3<f32>) {
        self.model = nalgebra::Matrix4::new_translation(&vector).into();
    }
}
