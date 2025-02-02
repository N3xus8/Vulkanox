// Note: Logical Device

use std::{f32::consts::{FRAC_PI_2, FRAC_PI_4}, sync::Arc};

use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        Buffer, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{
        self, allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferUsage, CopyBufferInfo,
    },
    descriptor_set::{
        self,
        allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo},
        layout::DescriptorSetLayout,
        DescriptorSet, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, DeviceCreateInfo, Features, Queue, QueueCreateInfo},
    format::Format,
    image::SampleCount,
    memory::{
        allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
        MemoryPropertyFlags,
    },
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            subpass::PipelineRenderingCreateInfo,
            vertex_input::{Vertex as VertexInput, VertexDefinition},
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        }, layout::PipelineDescriptorSetLayoutCreateInfo, DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo
    },
    sync::{self, GpuFuture},
    DeviceSize,
};

use crate::{
    camera::{Camera, CameraUniform},
    error::Result,
    mesh::MeshBuilder,
    shader::{self, fs, vs},
    vulkan_instance::VulkanInstance,
};
pub struct VulkanDevice {
    pub queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    command_allocator: Arc<StandardCommandBufferAllocator>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    pub vertex_buffer: Subbuffer<[shader::Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub descriptor_set: Arc<PersistentDescriptorSet>,
}

impl VulkanDevice {
    pub fn new(instance: Arc<VulkanInstance>) -> Result<Self> {
        let physical_device = instance.physical_device();
        let queue_family_index = instance.queue_family_index();
        let device_extensions = instance.device_extensions();

        // Now initializing the device. This is probably the most important object of Vulkan.
        //
        // An iterator of created queues is returned by the function alongside the device.
        let (device, mut queues) = Device::new(
            // Which physical device to connect to.
            Arc::clone(&physical_device),
            DeviceCreateInfo {
                // The list of queues that we are going to use. Here we only use one queue, from the
                // previously chosen queue family.
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],

                // A list of optional features and extensions that our program needs to work correctly.
                // Some parts of the Vulkan specs are optional and must be enabled manually at device
                // creation. In this example the only things we are going to need are the
                // `khr_swapchain` extension that allows us to draw to a window, and
                // `khr_dynamic_rendering` if we don't have Vulkan 1.3 available.
                enabled_extensions: *device_extensions,

                // In order to render with Vulkan 1.3's dynamic rendering, we need to enable it here.
                // Otherwise, we are only allowed to render with a render pass object, as in the
                // standard triangle example. The feature is required to be supported by the device if
                // it supports Vulkan 1.3 and higher, or if the `khr_dynamic_rendering` extension is
                // available, so we don't need to check for support.
                enabled_features: Features {
                    dynamic_rendering: true,
                    ..Features::empty()
                },

                ..Default::default()
            },
        )?;

        // Since we can request multiple queues, the `queues` variable is in fact an iterator. We only
        // use one queue in this example, so we just retrieve the first and only element of the
        // iterator.
        let queue = queues.next().unwrap();

        // Vulkano allocator for both Host and Device
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(Arc::clone(&device)));

        // Before we can start creating and recording command buffers, we need a way of allocating
        // them. Vulkano provides a command buffer allocator, which manages raw Vulkan command pools
        // underneath and provides a safe interface for them.
        let command_allocator = Arc::new(StandardCommandBufferAllocator::new(
            Arc::clone(&device),
            Default::default(),
        ));

        //<----
        // Decriptor set Allocator
        //---->
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            Arc::clone(&device),
            StandardDescriptorSetAllocatorCreateInfo::default(),
        ));

        // ---->
        //
        let gltf_mesh = MeshBuilder::read_gltf("assets/Box.gltf")?;
        let vertices = gltf_mesh.vertices()?;
        let indices = gltf_mesh.indices();
        let vertices_length = vertices.len();
        let indices_length = indices.len();

        let indices: Vec<u32> = indices.iter().map(|id| { *id as u32}).collect();

        // <---  -S T A G I N G  B U F F E R S-
        // Create a Staging Vertex buffer  : subbuffer<[Vertex]>

        let vertex_staging_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            vertices,
        )?;

        // Create an Staging index buffer : subbuffer<[u32]>

        let index_staging_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            indices,
        )?;
        // --->

        // Create a Vertex buffer  : subbuffer<[Vertex]>

        let vertex_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter {
                    required_flags: MemoryPropertyFlags::DEVICE_LOCAL, // Make sure this buffer is on the Device=GPU
                    ..Default::default()
                },
                ..Default::default()
            },
            vertices_length as DeviceSize,
        )?;

        // Create an index buffer : subbuffer<[u32]>

        let index_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter {
                    required_flags: MemoryPropertyFlags::DEVICE_LOCAL, // Make sure this buffer is on the Device=GPU
                    ..Default::default()
                },
                ..Default::default()
            },
            indices_length as DeviceSize,
        )?;

        // <----
        // Camera
        // ----->
        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: nalgebra::Point3::new(0.77, -0.67, 0.9),
            // have it look at the origin
            target: nalgebra::Point3::new(0.0, 0.0, 0.0),
            // which way is "up"
            up: nalgebra::Vector3::y(),
            aspect: 800 as f32 / 600 as f32, // ⚠ Caution! Hard Coded , Bad ! Bad !
            fovy: FRAC_PI_2,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let uniform_staging_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::TRANSFER_SRC,
                memory_type_filter: MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
        );

        let uniform_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::UNIFORM_BUFFER | BufferUsage::TRANSFER_DST,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let uniform_staging_buffer: Subbuffer<CameraUniform> =
            uniform_staging_buffer_allocator.allocate_sized()?;
        *uniform_staging_buffer.write()? = camera_uniform;

        let uniform_buffer: Subbuffer<CameraUniform> = uniform_buffer_allocator.allocate_sized().unwrap();

        // ---->
        // Staging buffers to Device buffers
        // <-----

        // command to copy buffer on host to  buffer on device
        // command builder:
        let mut command_builder = AutoCommandBufferBuilder::primary(
            &command_allocator,
            queue_family_index,
            CommandBufferUsage::OneTimeSubmit,
        )?;

        // build copy command
        command_builder.copy_buffer(CopyBufferInfo::buffers(
            vertex_staging_buffer,
            vertex_buffer.clone(),
        ))?;
        command_builder.copy_buffer(CopyBufferInfo::buffers(
            index_staging_buffer,
            index_buffer.clone(),
        ))?;

        command_builder.copy_buffer(CopyBufferInfo::buffers(
            uniform_staging_buffer,
            uniform_buffer.clone(),
        ))?;

        let command_buffer = command_builder.build()?;

        // submit command
        let buffers_upload_future = sync::now(Arc::clone(&device))
            .then_execute(Arc::clone(&queue), command_buffer)?
            .then_signal_fence_and_flush()?;

        //

        // ---->
        // Graphics Pipeline - Shader
        // ---->

        let graphics_pipeline = {
            // 👈 scope to make sure shaders are dropped once pipelines are created.

            let vertex_shader = vs::load(Arc::clone(&device))?.entry_point("main").unwrap();
            let fragment_shader = fs::load(Arc::clone(&device))?.entry_point("main").unwrap();

            // Automatically generate a vertex input state from the vertex shader's input interface,
            // that takes a single vertex buffer containing `Vertex` structs.
            let vertex_input_state =
                shader::Vertex::per_vertex().definition(&vertex_shader.info().input_interface)?;

            let stages: [PipelineShaderStageCreateInfo; 2] = [
                PipelineShaderStageCreateInfo::new(vertex_shader),
                PipelineShaderStageCreateInfo::new(fragment_shader),
            ];

            // We must now create a **pipeline layout** object, which describes the locations and types of
            // descriptor sets and push constants used by the shaders in the pipeline.
            //
            // Multiple pipelines can share a common layout object, which is more efficient.
            // The shaders in a pipeline must use a subset of the resources described in its pipeline
            // layout, but the pipeline layout is allowed to contain resources that are not present in the
            // shaders; they can be used by shaders in other pipelines that share the same layout.
            // Thus, it is a good idea to design shaders so that many pipelines have common resource
            // locations, which allows them to share pipeline layouts.
            let layout = PipelineLayout::new(
                Arc::clone(&device),
                // Since we only have one pipeline in this example, and thus one pipeline layout,
                // we automatically generate the creation info for it from the resources used in the
                // shaders. In a real application, you would specify this information manually so that you
                // can re-use one layout in multiple pipelines.
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(Arc::clone(&device))?,
            )?;

            // We describe the formats of attachment images where the colors, depth and/or stencil
            // information will be written. The pipeline will only be usable with this particular
            // configuration of the attachment images.
            let subpass = PipelineRenderingCreateInfo {
                // We specify a single color attachment that will be rendered to. When we begin
                // rendering, we will specify a swapchain image to be used as this attachment, so here
                // we set its format to be the same format as the swapchain.
                color_attachment_formats: vec![Some(Format::B8G8R8A8_SRGB)], // ⚠ Caution! Hard coded
                ..Default::default()
            };

            GraphicsPipeline::new(
                Arc::clone(&device),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    // How vertex data is read from the vertex buffers into the vertex shader.
                    vertex_input_state: Some(vertex_input_state),
                    // How vertices are arranged into primitive shapes.
                    // The default primitive shape is a triangle.
                    input_assembly_state: Some(InputAssemblyState::default()),
                    // How primitives are transformed and clipped to fit the framebuffer.
                    // We use a resizable viewport, set to draw over the entire window.
                    viewport_state: Some(ViewportState::default()),
                    // How polygons are culled and converted into a raster of pixels.
                    // The default value does not perform any culling.
                    rasterization_state: Some(RasterizationState {
                        cull_mode: CullMode::Back,
                        ..Default::default()
                    }),
                    // How multiple fragment shader samples are converted to a single pixel value.
                    // The default value does not perform any multisampling.
                    //Original without MSAA 👉 multisample_state: Some(MultisampleState::default()),
                    multisample_state: Some(MultisampleState {
                        // MSAA
                        rasterization_samples: SampleCount::Sample4,
                        ..Default::default()
                    }),
                    // How pixel values are combined with the values already present in the framebuffer.
                    // The default value overwrites the old value with the new one, without any blending.
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.color_attachment_formats.len() as u32,
                        ColorBlendAttachmentState::default(),
                    )),
                    // Dynamic states allows us to specify parts of the pipeline settings when
                    // recording the command buffer, before we perform drawing.
                    // Here, we specify that the viewport should be dynamic.
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        let descriptor_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            Arc::clone(
                graphics_pipeline.layout()
                    .set_layouts()
                    .get(0)
                    .expect("error getting the layout"),
            ),
            [WriteDescriptorSet::buffer(0, uniform_buffer)],
            [],
        )?;

        buffers_upload_future.wait(None)?; // Not sure this works? Is this needed

        Ok(Self {
            queue,
            memory_allocator,
            command_allocator,
            graphics_pipeline,
            vertex_buffer,
            index_buffer,
            descriptor_set,
        })
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    pub fn memory_allocator(&self) -> &Arc<StandardMemoryAllocator> {
        &self.memory_allocator
    }

    pub fn command_allocator(&self) -> &Arc<StandardCommandBufferAllocator> {
        &self.command_allocator
    }

    pub fn graphics_pipeline(&self) -> &Arc<GraphicsPipeline> {
        &self.graphics_pipeline
    }

    pub fn index_buffer(&self) -> &Subbuffer<[u32]> {
        &self.index_buffer
    }

    pub fn descriptor_set(&self) -> &Arc<PersistentDescriptorSet> {
        &self.descriptor_set
    }
}
