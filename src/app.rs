use std::sync::Arc;

use vulkano::image::ImageUsage;
use winit::{
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{
    error::Result, vulkan_device::VulkanDevice, vulkan_instance::VulkanInstance,
    vulkan_renderer::VulkanRenderer,
};

pub struct VisualSystem {
    window: Arc<Window>,
    vulkan_instance: Arc<VulkanInstance>,
    vulkan_device: Arc<VulkanDevice>,
    vulkan_renderer: Option<Arc<VulkanRenderer>>,
}

impl VisualSystem {
    pub fn new<T>(window_target: &EventLoopWindowTarget<T>) -> Result<Self> {
        let window = Arc::new(WindowBuilder::new().build(&window_target)?);

        let vulkan_instance = Arc::new(VulkanInstance::new(Arc::clone(&window))?);

        let vulkan_device = Arc::new(VulkanDevice::new(Arc::clone(&vulkan_instance))?);

        let vulkan_renderer = Arc::new(VulkanRenderer::new(
            Arc::clone(&vulkan_device),
            Arc::clone(&window),
            ImageUsage::COLOR_ATTACHMENT,
        )?);

        Ok(Self {
            window,
            vulkan_instance,
            vulkan_device,
            vulkan_renderer: Some(vulkan_renderer),
        })
    }

    pub fn resume<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        self.vulkan_renderer = Some(Arc::new(VulkanRenderer::new(
            Arc::clone(&self.vulkan_device),
            Arc::clone(&self.window),
            ImageUsage::COLOR_ATTACHMENT,
        )?));

        Ok(())
    }

    pub fn suspend(&mut self) {
        self.vulkan_renderer = None;
    }
}

pub struct App {
    visual_system: Option<VisualSystem>,
}

impl App {
    pub fn new<T>(event_loop: &EventLoop<T>) -> Result<Self> {
        Ok(Self {
            visual_system: None,
        })
    }

    pub fn start<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        self.visual_system = Some(VisualSystem::new(window_target)?);

        Ok(())
    }

    pub fn resume<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        self.visual_system
            .as_mut()
            .expect("no visual system")
            .resume(window_target)?;

        Ok(())
    }

    pub fn suspend(&mut self) {
        self.visual_system
            .as_mut()
            .expect("no visual system")
            .suspend();
    }
}
