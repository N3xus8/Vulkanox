use std::{cell::RefCell, sync::Arc};

use vulkano::image::SampleCount;

use crate::{
    camera::{Camera, CameraUniform},
    error::Result,
};

pub struct VulkanContext {
    pub camera: Arc<RefCell<Camera>>,
    pub camera_uniform: Arc<RefCell<CameraUniform>>,
    pub samples: SampleCount,
}

impl VulkanContext {
    pub fn new(
        camera: Arc<RefCell<Camera>>,
        camera_uniform: Arc<RefCell<CameraUniform>>,
        samples: SampleCount,
    ) -> Result<Self> {
        Ok(Self {
            camera,
            camera_uniform,
            samples,
        })
    }

    pub fn camera(&self) -> &Arc<RefCell<Camera>> {
        &self.camera
    }

    pub fn camera_uniform(&self) -> &Arc<RefCell<CameraUniform>> {
        &self.camera_uniform
    }
}
