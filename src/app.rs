use std::{
    cell::RefCell, collections::BTreeMap, rc::Rc, sync::{Arc, Mutex}
};

use tracing::info;
use vulkano::image::{ImageUsage, SampleCount};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId},
};

use crate::{
    camera::{Camera, CameraController, Mvp},
    error::{self, Result},
    utils::load_icon,
    vulkan_context::VulkanContext,
    vulkan_device::VulkanDevice,
    vulkan_instance::VulkanInstance,
    vulkan_renderer::VulkanRenderer,
};

pub struct VisualSystem {
    primary_window_id: WindowId,
    windows: BTreeMap<WindowId, Arc<Window>>,
    #[allow(unused)]
    vulkan_instance: Arc<VulkanInstance>,
    vulkan_device: Rc<VulkanDevice>,
    vulkan_renderers: BTreeMap<WindowId, Rc<Mutex<VulkanRenderer>>>,
}

impl VisualSystem {
    pub fn new<T>(window_target: &EventLoopWindowTarget<T>) -> Result<Self> {
        let window_icon: Option<winit::window::Icon> = Some(load_icon("./assets/icon.png"));

        // Support Multi windows
        let primary_window = Arc::new(
            WindowBuilder::new()
                .with_title("🌋VULKANO ♣")
                .with_window_icon(window_icon)
                .with_visible(false)
                .build(window_target)?,
        );
        let primary_window_id = primary_window.id();

        let vulkan_instance = Arc::new(
            VulkanInstance::new(Arc::clone(&primary_window))
                .map_err(|_| error::VisualSystemError::ErrorCreatingVulkanInstance)?,
        );

        let camera = Arc::new(Mutex::new(Camera::default()));

        let camera_controller = Arc::new(Mutex::new(CameraController::new(0.2)));

        let mut mvp_uniform = Mvp::new();
        mvp_uniform.update_view(&camera.lock().unwrap());
        mvp_uniform.update_projection(&camera.lock().unwrap());

        mvp_uniform.update_model_translate(nalgebra::Vector3::new(0.0, 0.0, -1.0));

        let samples = SampleCount::Sample4;

        let vulkan_context = Rc::new(RefCell::new(VulkanContext::new(
            camera,
            Arc::new(Mutex::new(mvp_uniform)),
            camera_controller,
            samples,
        )?));

        let vulkan_device = Rc::new(
            VulkanDevice::new(Arc::clone(&vulkan_instance), Rc::clone(&vulkan_context))
                .map_err(|_| error::VisualSystemError::ErrorCreatingVulkanDevice)?,
        );

        // Store the windows in a BTreeMap
        let mut windows = BTreeMap::from([(primary_window_id, Arc::clone(&primary_window))]);

        let symbol_list = "♔♕♖♗♘♙☚★";

        for idx in 0..1 {
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
                Rc::new(Mutex::new(
                    VulkanRenderer::new(
                        Rc::clone(&vulkan_device),
                        Arc::clone(window),
                        ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                    )
                    .map_err(|_| error::VisualSystemError::ErrorCreatingVulkanRenderer)?,
                )),
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
    pub fn resume<T>(&mut self, _window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        for (window_id, window) in &self.windows {
            self.vulkan_renderers.insert(
                *window_id,
                Rc::new(Mutex::new(
                    VulkanRenderer::new(
                        // Use Mutex fo interior mutability
                        Rc::clone(&self.vulkan_device),
                        Arc::clone(window),
                        ImageUsage::COLOR_ATTACHMENT,
                    )
                    .map_err(|_| error::VisualSystemError::ErrorCreatingVulkanRenderer)?,
                )),
            );
        }
        Ok(())
    }

    pub fn suspend(&mut self) {
        self.vulkan_renderers.clear(); // Clear the renderers in the BTreeMap
    }

    pub fn resize(&mut self, window_id: WindowId, new_size: PhysicalSize<u32>) -> Result<()> {
        if !(new_size.width == 0 || new_size.height == 0) {
            self.vulkan_renderers[&window_id]
                .lock()
                .expect("failed to get a lock on vulkan renderer")
                .recreate()?; // Use Mutex for interior mutability

            // update camera aspect ratio
            self.vulkan_device
                .vulkan_context
                .borrow()
                .camera
                .lock()
                .expect("failed to get a lock on camera ")
                .update_aspect(new_size.width, new_size.height);

            self.vulkan_device
                .vulkan_context
                .borrow()
                .mvp_uniform
                .lock()
                .expect("failed to get a lock on camera uniform")
                .update_projection(
                    &self
                        .vulkan_device
                        .vulkan_context
                        .borrow()
                        .camera
                        .lock()
                        .unwrap(),
                );

            self.vulkan_device.update_uniform_buffer()?;

            return Ok(());
        }

        Ok(())
    }

    pub fn input(&mut self) -> Result<()> {
        // update camera via camera controller
        self.vulkan_device
            .vulkan_context
            .borrow()
            .camera_controller
            .lock()
            .expect("failed to get a lock on camera controller")
            .update_camera(
                &mut self
                    .vulkan_device
                    .vulkan_context
                    .borrow()
                    .camera
                    .lock()
                    .expect("failed to get a lock on camera"),
            );

        self.vulkan_device
            .vulkan_context
            .borrow()
            .mvp_uniform
            .lock()
            .expect("failed to get a lock on camera uniform")
            .update_view(
                &self
                    .vulkan_device
                    .vulkan_context
                    .borrow()
                    .camera
                    .lock()
                    .unwrap(),
            );

        self.vulkan_device.update_uniform_buffer()?;

        Ok(())
    }

    pub fn draw(&mut self, window_id: WindowId) -> Result<()> {
        self.vulkan_renderers[&window_id].lock().unwrap().render()
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
    pub fn new<T>(_event_loop: &EventLoop<T>) -> Result<Self> {
        Ok(Self {
            is_app_started: false,
            visual_system: None,
        })
    }

    pub fn start<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        self.visual_system = Some(
            VisualSystem::new(window_target)
                .map_err(|_| error::VisualSystemError::ErrorCreatingVisualSystem)?,
        );

        Ok(())
    }

    pub fn resume<T>(&mut self, window_target: &EventLoopWindowTarget<T>) -> Result<()> {
        self.visual_system
            .as_mut()
            .expect("no visual system")
            .resume(window_target)
            .map_err(|_| error::VisualSystemError::ErrorResumingVisualSystem)?;

        Ok(())
    }

    pub fn suspend(&mut self) {
        self.visual_system
            .as_mut()
            .expect("no visual system")
            .suspend();
    }

    pub fn process_event(
        &mut self,
        event: Event<()>,
        window_target: &EventLoopWindowTarget<()>,
    ) -> Result<()> {
        match event {
            Event::WindowEvent { window_id, event } => {
                if !self
                    .visual_system
                    .as_mut()
                    .unwrap()
                    .vulkan_device
                    .vulkan_context
                    .borrow_mut()
                    .input(&event)
                {
                    match event {
                        WindowEvent::CloseRequested => {
                            if self.visual_system.as_ref().unwrap().primary_window_id == window_id {
                                info!("The close button was pressed; stopping \u{2B22}");
                                window_target.exit()
                            }
                        }
                        WindowEvent::Resized(new_size) => {
                            self.visual_system
                                .as_mut()
                                .unwrap()
                                .resize(window_id, new_size)
                                .map_err(|_| error::VisualSystemError::ErrorResizingVisualSystem)?;
                        }

                        WindowEvent::RedrawRequested => self
                            .visual_system
                            .as_mut()
                            .unwrap()
                            .draw(window_id)
                            .map_err(|_| error::VisualSystemError::ErrorDrawingVisualSystem)?,

                        _ => {}
                    }
                } else {
                    self.visual_system
                        .as_mut()
                        .unwrap()
                        .input()
                        .map_err(|_| error::VisualSystemError::ErrorInputVisualSystem)?
                }
            }

            Event::Resumed => {
                if self.is_app_started {
                    self.resume(window_target).unwrap();
                } else {
                    self.is_app_started = true;

                    self.start(window_target).expect("failed to start");
                }
            }

            Event::Suspended => {
                self.suspend();
            }

            Event::AboutToWait => self
                .visual_system
                .as_mut()
                .unwrap()
                .request_redraw()
                .map_err(|_| error::VisualSystemError::ErrorRequestReDrawVisualSystem)?,
            _ => {}
        }

        Ok(())
    }
}
