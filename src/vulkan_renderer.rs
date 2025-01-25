// Note: Renderer

use std::{default, sync::Arc, u32};

use vulkano::{
    image::{view::ImageView, Image, ImageUsage},
    pipeline::graphics::viewport::Viewport,
    swapchain::{Surface, Swapchain, SwapchainCreateInfo},
};
use winit::window::Window;

use crate::{error::Result, vulkan_device::VulkanDevice};

pub struct VulkanRenderer {
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<Image>>,
    pub swapchain_image_views: Vec<Arc<ImageView>>,
}

impl VulkanRenderer {
    pub fn new(
        vulkan_device: Arc<VulkanDevice>,
        window: Arc<Window>,
        image_usage: ImageUsage,
    ) -> Result<Self> {
        let device = vulkan_device.queue().device();
        let physical_device = device.physical_device();
        let instance = device.instance();

        // create the surface of the window
        let surface = Surface::from_window(Arc::clone(&instance), Arc::clone(&window))?;

        // SWAPCHAIN
        // Before we can draw on the surface, we have to create what is called a swapchain. Creating a
        // swapchain allocates the color buffers that will contain the image that will ultimately be
        // visible on the screen. These images are returned alongside the swapchain.

        // Querying the capabilities of the surface. When we create the swapchain we can only pass
        // values that are allowed by the capabilities.
        let surface_capabilities =
            physical_device.surface_capabilities(&surface, Default::default())?;

        // Choosing the internal format that the images will have.
        let image_format = device
            .physical_device()
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        // create the swapchain

        let (mut swapchain, swapchain_images) = Swapchain::new(
            Arc::clone(&device),
            surface,
            SwapchainCreateInfo {
                image_extent: surface_capabilities
                    .current_extent
                    .unwrap_or(window.inner_size().into()),
                image_format: image_format,
                min_image_count: (surface_capabilities.min_image_count + 1)
                    .min(surface_capabilities.max_image_count.unwrap_or(u32::MAX)),
                pre_transform: surface_capabilities.current_transform,
                image_usage: image_usage,
                ..Default::default()
            },
        )?;

        // Dynamic viewports allow us to recreate just the viewport when the window is resized.
        // Otherwise we would have to recreate the whole pipeline.
        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        // When creating the swapchain, we only created plain images. To use them as an attachment for
        // rendering, we must wrap then in an image view.
        //
        // Since we need to draw to multiple images, we are going to create a different image view for
        // each image.
        let mut swapchain_image_views =
            window_size_dependent_setup(&swapchain_images, &mut viewport);

        Ok(Self {
            swapchain,
            swapchain_images,
            swapchain_image_views,
        })
    }
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(
    images: &[Arc<Image>],
    viewport: &mut Viewport,
) -> Vec<Arc<ImageView>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| ImageView::new_default(Arc::clone(image)).unwrap())
        .collect::<Vec<_>>()
}
