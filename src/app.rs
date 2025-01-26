use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

use vulkano::image::ImageUsage;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId},
};

use crate::{
    error::Result, vulkan_device::VulkanDevice, vulkan_instance::VulkanInstance,
    vulkan_renderer::VulkanRenderer,
};

pub struct VisualSystem {
    primary_window_id: WindowId,
    windows: BTreeMap<WindowId, Arc<Window>>,
    vulkan_instance: Arc<VulkanInstance>,
    vulkan_device: Arc<VulkanDevice>,
    vulkan_renderers: BTreeMap<WindowId, Arc<RefCell<VulkanRenderer>>>,
}

impl VisualSystem {
    pub fn new<T>(window_target: &EventLoopWindowTarget<T>) -> Result<Self> {
        // Support Multi windows
        let primary_window = Arc::new(
            WindowBuilder::new()
                .with_title("🌋VULKANO ♣")
                .with_visible(false)
                .build(&window_target)?,
        );
        let primary_window_id = primary_window.id();

        let vulkan_instance = Arc::new(VulkanInstance::new(Arc::clone(&primary_window))?);

        let vulkan_device = Arc::new(VulkanDevice::new(Arc::clone(&vulkan_instance))?);

        // Store the windows in a BTreeMap
        let mut windows = BTreeMap::from([(primary_window_id, Arc::clone(&primary_window))]);

        let symbol_list = "♔♕♖♗♘♙☚★";

        for idx in 0..3 {
            let char_at_index = symbol_list
                .chars()
                .nth(idx % (symbol_list.chars().count()))
                .unwrap_or('✔');
            let window = Arc::new(
                WindowBuilder::new()
                    .with_visible(false)
                    .with_title(format!("🌋VULKANO {char_at_index} {idx}"))
                    .build(window_target)?,
            );
            windows.insert(window.id(), window);
        }

        // Each window has its own renderer
        let mut vulkan_renderers = BTreeMap::new();

        for (window_id, window) in &windows {
            vulkan_renderers.insert(
                *window_id,
                Arc::new(RefCell::new(VulkanRenderer::new(
                    Arc::clone(&vulkan_device),
                    Arc::clone(&window),
                    ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                )?)),
            );
        }

        windows
            .iter()
            .for_each(|(_, window)| window.set_visible(true)); // visible when ready to avoid seeing garbage in the window during setup

        Ok(Self {
            primary_window_id,
            windows,
            vulkan_instance,
            vulkan_device,
            vulkan_renderers,
        })
    }

    // Resume create a new renderer. Keep device and window
    pub fn resume<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        for (window_id, window) in &self.windows {
            self.vulkan_renderers.insert(
                *window_id,
                Arc::new(RefCell::new(VulkanRenderer::new(
                    // Use RefCell fo interior mutability
                    Arc::clone(&self.vulkan_device),
                    Arc::clone(&window),
                    ImageUsage::COLOR_ATTACHMENT,
                )?)),
            );
        }
        Ok(())
    }

    pub fn suspend(&mut self) {
        self.vulkan_renderers.clear(); // Clear the renderers in the BTreeMap
    }

    pub fn resize(&mut self, window_id: WindowId) -> Result<()> {
        self.vulkan_renderers[&window_id].borrow_mut().recreate() // Use RefCell fo interior mutability
    }

    pub fn draw(&mut self, window_id: WindowId) -> Result<()> {
        self.vulkan_renderers[&window_id].borrow_mut().render()
    }

    pub fn request_redraw(&mut self) -> Result<()> {
        self.windows.iter().for_each(|(_, window)| {
            window.request_redraw();
        });
        Ok(())
    }
}

pub struct App {
    is_app_started: bool,
    visual_system: Option<VisualSystem>,
}

impl App {
    pub fn new<T>(event_loop: &EventLoop<T>) -> Result<Self> {
        Ok(Self {
            is_app_started: false,
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

    pub fn process_event<T>(
        &mut self,
        event: Event<()>,
        window_target: &EventLoopWindowTarget<()>,
    ) -> Result<()> {
        match event {
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::CloseRequested => {
                    if self.visual_system.as_ref().unwrap().primary_window_id == window_id {
                        window_target.exit()
                    }
                }
                WindowEvent::Resized(_) => {
                    self.visual_system.as_mut().unwrap().resize(window_id)?
                }

                WindowEvent::RedrawRequested => {
                    self.visual_system.as_mut().unwrap().draw(window_id)?
                }

                _ => {}
            },

            Event::Resumed => {
                if self.is_app_started {
                    self.resume(window_target).unwrap();
                } else {
                    self.is_app_started = true;

                    self.start(window_target).unwrap();
                }
            }

            Event::Suspended => {
                self.suspend();
            }

            Event::AboutToWait => self.visual_system.as_mut().unwrap().request_redraw()?,
            _ => {}
        }

        Ok(())
    }
}
