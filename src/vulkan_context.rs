use std::sync::{Arc, Mutex};

use vulkano::image::SampleCount;
use winit::event::WindowEvent;

use crate::{
    camera::{Camera, CameraController, Mvp},
    error::Result,
};

pub struct VulkanContext {
    pub camera: Arc<Mutex<Camera>>,
    pub mvp_uniform: Arc<Mutex<Mvp>>,
    pub camera_controller: Arc<Mutex<CameraController>>,
    pub samples: SampleCount,
}

impl VulkanContext {
    pub fn new(
        camera: Arc<Mutex<Camera>>,
        mvp_uniform: Arc<Mutex<Mvp>>,
        camera_controller: Arc<Mutex<CameraController>>,
        samples: SampleCount,
    ) -> Result<Self> {
        Ok(Self {
            camera,
            mvp_uniform,
            camera_controller,
            samples,
        })
    }

    #[allow(unused)]
    pub fn camera(&self) -> &Arc<Mutex<Camera>> {
        &self.camera
    }

    pub fn mvp_uniform(&self) -> &Arc<Mutex<Mvp>> {
        &self.mvp_uniform
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.lock().unwrap().process_events(event)
    }
}
