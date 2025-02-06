// Note: Renderer

use std::{sync::Arc, time::Instant};

use palette::Srgba;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo,
        RenderingAttachmentResolveInfo, RenderingInfo,
    },
    device::DeviceOwned,
    format::{ClearValue, Format},
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::AllocationCreateInfo,
    pipeline::{graphics::viewport::Viewport, Pipeline, PipelineBindPoint},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    swapchain::{
        acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{self, GpuFuture},
    Validated, VulkanError,
};
use winit::window::Window;

use crate::{error::Result, shader::vs, vulkan_device::VulkanDevice};

pub struct VulkanRenderer {
    pub vulkan_device: Arc<VulkanDevice>,
    pub window: Arc<Window>,
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<Image>>,
    pub swapchain_image_views: Vec<Arc<ImageView>>,
    pub intermediary_image: Arc<ImageView>, // for msaa (multi-sample anti-aliasing)
    pub depth_view: Arc<ImageView>,         // Depth
    pub previous_frame_end: Option<Box<dyn GpuFuture>>, // synchro
    pub start_time: Instant,
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
        let surface = Surface::from_window(Arc::clone(instance), Arc::clone(&window))?;

        // SWAPCHAIN
        // Before we can draw on the surface, we have to create what is called a swapchain. Creating a
        // swapchain allocates the color buffers that will contain the image that will ultimately be
        // visible on the screen. These images are returned alongside the swapchain.

        // Querying the capabilities of the surface. When we create the swapchain we can only pass
        // values that are allowed by the capabilities.
        let surface_capabilities =
            physical_device.surface_capabilities(&surface, Default::default())?;

        // Choosing the internal format that the images will have.
        /*  let image_format = device
        .physical_device()
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0; */

        // create the swapchain

        let (swapchain, swapchain_images) = Swapchain::new(
            Arc::clone(device),
            surface,
            SwapchainCreateInfo {
                image_extent: surface_capabilities
                    .current_extent
                    .unwrap_or(window.inner_size().into()),
                image_format: Format::B8G8R8A8_SRGB,
                min_image_count: (surface_capabilities.min_image_count + 1)
                    .min(surface_capabilities.max_image_count.unwrap_or(u32::MAX)),
                pre_transform: surface_capabilities.current_transform,
                image_usage,
                ..Default::default()
            },
        )?;

        // When creating the swapchain, we only created plain images. To use them as an attachment for
        // rendering, we must wrap then in an image view.
        //
        // Since we need to draw to multiple images, we are going to create a different image view for
        // each image.
        let swapchain_image_views = window_size_dependent_setup(&swapchain_images);

        // Creating our intermediate multisampled image.
        //
        // MSAA  We pass the same extent and format as for the final
        // image. But we also pass the number of samples-per-pixel, which is 4 here.

        let intermediary_image = ImageView::new_default(Image::new(
            vulkan_device.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: swapchain.image_format(),
                extent: [swapchain.image_extent()[0], swapchain.image_extent()[1], 1],
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT, // transient image
                samples: vulkan_device
                    .vulkan_context.borrow()
                    .samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?)?;

        // Depth buffer

        // Depth image view
        let depth_view: Arc<ImageView> = ImageView::new_default(Image::new(
            vulkan_device.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D16_UNORM,
                extent: [swapchain.image_extent()[0], swapchain.image_extent()[1], 1],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                samples: vulkan_device
                    .vulkan_context.borrow()
                    .samples, // Match intermediary
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?)?;

        // In the event loop  we are going to submit commands to the GPU. Submitting a command produces
        // an object that implements the `GpuFuture` trait, which holds the resources for as long as
        // they are in use by the GPU.
        //
        // Destroying the `GpuFuture` blocks until the GPU is finished executing it. In order to avoid
        // that, we store the submission of the previous frame here.
        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Ok(Self {
            vulkan_device,
            window,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            intermediary_image,
            previous_frame_end,
            start_time: std::time::Instant::now(),
            depth_view,
        })
    }

    pub fn recreate(&mut self) -> Result<()> {
        let surface_capabilities = self
            .swapchain
            .device()
            .physical_device()
            .surface_capabilities(self.swapchain.surface(), Default::default())?;

        self.swapchain_images.clear();
        self.swapchain_image_views.clear();

        let (new_swapchain, new_swapchain_images) =
            self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: surface_capabilities
                    .current_extent
                    .unwrap_or(self.window.inner_size().into()),
                ..self.swapchain.create_info()
            })?;

        let new_swapchain_image_views = window_size_dependent_setup(&new_swapchain_images);

        self.swapchain = new_swapchain;
        self.swapchain_images = new_swapchain_images;
        self.swapchain_image_views = new_swapchain_image_views;
        self.intermediary_image = ImageView::new_default(Image::new(
            self.vulkan_device.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: self.swapchain.image_format(),
                extent: [
                    self.swapchain.image_extent()[0],
                    self.swapchain.image_extent()[1],
                    1,
                ],
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT, // transient image
                samples: self
                    .vulkan_device
                    .vulkan_context.borrow()
                    .samples,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?)?;

        self.depth_view = ImageView::new_default(Image::new(
            self.vulkan_device.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D16_UNORM,
                extent: [
                    self.swapchain.image_extent()[0],
                    self.swapchain.image_extent()[1],
                    1,
                ],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                samples: self
                    .vulkan_device
                    .vulkan_context.borrow()
                    .samples, // Match intermediary
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?)?;

        Ok(())
    }

    pub fn render(&mut self) -> Result<()> {
        // Do not draw the frame when the screen size is zero. On Windows, this can
        // occur when minimizing the application.
        let image_extent: [u32; 2] = self.window.inner_size().into();

        if image_extent.contains(&0) {
            return Ok(());
        }

        // It is important to call this function from time to time, otherwise resources
        // will keep accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU
        // has already processed, and frees the resources that are no longer needed.
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // Before we can draw on the output, we have to *acquire* an image from the
        // swapchain. If no image is available (which happens if you submit draw commands
        // too quickly), then the function will block. This operation returns the index of
        // the image that we are allowed to draw upon.
        //
        // This function can block if no image is available. The parameter is an optional
        // timeout after which the function call will return an error.
        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    todo!();
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        // `acquire_next_image` can be successful, but suboptimal. This means that the
        // swapchain image will still work, but it may not display correctly. With some
        // drivers this can be when the window resizes, but it may not cause the swapchain
        // to become out of date.
        if suboptimal {
            todo!();
        }

        // In order to draw, we have to build a *command buffer*. The command buffer object
        // holds the list of commands that are going to be executed.
        //
        // Building a command buffer is an expensive operation (usually a few hundred
        // microseconds), but it is known to be a hot path in the driver and is expected to
        // be optimized.
        //
        // Note that we have to pass a queue family when we create the command buffer. The
        // command buffer will only be executable on that given queue family.
        let mut builder = AutoCommandBufferBuilder::primary(
            self.vulkan_device.command_allocator(),
            self.vulkan_device.queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        /*         builder.clear_color_image(ClearColorImageInfo {
                    clear_value: ClearColorValue::Float([0.2, 0.2, 0.3, 1.]),
                    ..ClearColorImageInfo::image(Arc::clone(&self.swapchain_images[image_index as usize]))
                })?;
        */

        //

        let clear_color_srgba = Srgba::new(0.2, 0.2, 0.3, 1.);

        let extent = self.swapchain.image_extent();

        // push constant uniform to pass the time to the shader
        let push_constants = vs::PushConstantData {
            time: (Instant::now() - self.start_time).as_secs_f32(),
        };

        //

        // Dynamic viewports allow us to recreate just the viewport when the window is resized.
        // Otherwise we would have to recreate the whole pipeline.
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [extent[0] as f32, extent[1] as f32],
            depth_range: 0.0..=1.0,
        };

        // ----->
        // Command buffer builder
        // <-----
        //println!("DEBUG INDEX BUFFER: {:} ", self.vulkan_device.index_buffer().len());

        // Before we can draw, we have to *enter a render pass*. We specify which
        // attachments we are going to use for rendering here, which needs to match
        // what was previously specified when creating the pipeline.
        builder
            .begin_rendering(RenderingInfo {
                // As before, we specify one color attachment, but now we specify the image
                // view to use as well as how it should be used.
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    // `Clear` means that we ask the GPU to clear the content of this
                    // attachment at the start of rendering.
                    load_op: AttachmentLoadOp::Clear,
                    // `Store` means that we ask the GPU to store the rendered output in
                    // the attachment image. We could also ask it to discard the result.
                    store_op: AttachmentStoreOp::Store,
                    // The value to clear the attachment with. Here we clear it with a blue
                    // color.
                    //
                    // Only attachments that have `AttachmentLoadOp::Clear` are provided
                    // with clear values, any others should use `None` as the clear value.
                    clear_value: Some(ClearValue::Float(clear_color_srgba.into_linear().into())),

                    // MSAA Resolve
                    resolve_info: Some(RenderingAttachmentResolveInfo::image_view(Arc::clone(
                        &self.swapchain_image_views[image_index as usize],
                    ))),
                    // Instead of rendering directly to the swapchain image rendering to the intermediary image with multi-sample: 4
                    // And then resolving into the swapchain image which only have 1 sample (see above)

                    // intermediary image for MSAA
                    ..RenderingAttachmentInfo::image_view(
                        Arc::clone(&self.intermediary_image), // We specify image view corresponding to the currently acquired
                                                              // swapchain image, to use for this attachment.
                                                              // Original without MSAA 👉  Arc::clone(&self.swapchain_image_views[image_index as usize]),
                    )
                })],
                // {---- Depth attachment
                depth_attachment: Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    clear_value: Some(1.0f32.into()),
                    ..RenderingAttachmentInfo::image_view(Arc::clone(&self.depth_view))
                }),
                // -----}
                ..Default::default()
            })?
            // We are now inside the first subpass of the render pass.
            //
            // TODO: Document state setting and how it affects subsequent draw commands.
            .set_viewport(0, [viewport.clone()].into_iter().collect())?
            .bind_pipeline_graphics(Arc::clone(self.vulkan_device.graphics_pipeline()))?
            .bind_vertex_buffers(0, self.vulkan_device.vertex_buffer.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                Arc::clone(self.vulkan_device.graphics_pipeline().layout()),
                0,
                Arc::clone(self.vulkan_device.descriptor_set()),
            )?
            .push_constants(
                Arc::clone(self.vulkan_device.graphics_pipeline().layout()),
                0,
                push_constants,
            )?;
        // We add a draw command.
        // Condition whether index buffers are present or not
        match self.vulkan_device.index_buffer() {
            Some(index_buffer) => builder
                .bind_index_buffer(index_buffer.clone())?
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)?,
            None => builder.draw(self.vulkan_device.vertex_buffer.len() as u32, 1, 0, 0)?,
        }
        // We leave the render pass.
        .end_rendering()?;

        let command_buffer = builder.build()?;

        // ------>
        // Vulkan synchronization
        // <------

        //
        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(Arc::clone(self.vulkan_device.queue()), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to
            // show it on the screen, we have to *present* the image by calling
            // `then_swapchain_present`.
            //
            // This function does not actually present the image immediately. Instead it
            // submits a present command at the end of the queue. This means that it will
            // only be presented once the GPU has finished executing the command buffer
            // that draws the triangle.
            .then_swapchain_present(
                Arc::clone(self.vulkan_device.queue()),
                SwapchainPresentInfo::swapchain_image_index(
                    Arc::clone(&self.swapchain),
                    image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate()?;
                self.previous_frame_end =
                    Some(sync::now(Arc::clone(self.swapchain.device())).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end =
                    Some(sync::now(Arc::clone(self.swapchain.device())).boxed());
            }
        }

        Ok(())
    }
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(images: &[Arc<Image>]) -> Vec<Arc<ImageView>> {
    images
        .iter()
        .map(|image| ImageView::new_default(Arc::clone(image)).unwrap())
        .collect::<Vec<_>>()
}
