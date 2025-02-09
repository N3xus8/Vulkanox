// Note: Logical Device

use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        Buffer, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyBufferInfo,
    },
    descriptor_set::{
        allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo},
        layout::{DescriptorBindingFlags, DescriptorSetLayoutBinding, DescriptorType},
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, DeviceCreateInfo, Features, Queue, QueueCreateInfo},
    format::Format,
    memory::{
        allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
        MemoryPropertyFlags,
    },
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            subpass::PipelineRenderingCreateInfo,
            vertex_input::{Vertex as VertexInput, VertexDefinition},
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    shader::ShaderStages,
    sync::{self, GpuFuture},
    DeviceSize, NonExhaustive,
};

use crate::{
    camera::{CameraUniform, MVP},
    error::Result,
    index_buffer::setup_index_buffers,
    instance_buffer::{self, Instance, InstanceRaw},
    lighting::{AmbientLight, DirectionalLight, WHITE_AMBIENT_LIGHT},
    mesh::MeshBuilder,
    shader::{self, fs, vs, Vertex},
    vulkan_context::VulkanContext,
    vulkan_instance::VulkanInstance,
};
pub struct VulkanDevice {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    command_allocator: Arc<StandardCommandBufferAllocator>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    pub vertex_buffer: Subbuffer<[shader::Vertex]>,
    pub instance_buffer: Subbuffer<[InstanceRaw]>,
    pub index_buffer: Option<Subbuffer<[u32]>>,
    pub descriptor_set: Arc<PersistentDescriptorSet>,
    pub vulkan_context: Arc<RefCell<VulkanContext>>,
    pub uniform_staging_buffer: Subbuffer<MVP>,
    pub uniform_buffer: Subbuffer<MVP>,
}

impl VulkanDevice {
    pub fn new(
        instance: Arc<VulkanInstance>,
        vulkan_context: Arc<RefCell<VulkanContext>>,
    ) -> Result<Self> {
        let physical_device = instance.physical_device();
        let queue_family_index = instance.queue_family_index();
        let device_extensions = instance.device_extensions();

        // Now initializing the device. This is probably the most important object of Vulkan.
        //
        // An iterator of created queues is returned by the function alongside the device.
        let (device, mut queues) = Device::new(
            // Which physical device to connect to.
            Arc::clone(physical_device),
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
        // let indices_length = indices.len();

        // let indices: Vec<u32> = indices.iter().map(|id| *id as u32).collect();


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

        // Condition: whether the GTLF contains indices or not?
        // Option for index staging buffer and index buffer
        let (index_staging_buffer, index_buffer) =
            setup_index_buffers(indices, memory_allocator.clone())?;

        // Instances for vertex model
        // Create a Vertex buffer  : subbuffer<[InstanceRaw]>

        let instances = Instance::new()
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();

        let instances_length = instances.len();

        println!("INSTANCES NUMBER: {:}", instances_length);

        let instance_buffer = Buffer::new_slice(
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
            instances_length as DeviceSize,
        )?;

        // <---  -S T A G I N G  B U F F E R S-

        // Create a Staging Vertex buffer  : subbuffer<[Vertex]>

        // let vertex_staging_buffer = Buffer::from_iter(
        //     memory_allocator.clone(),
        //     BufferCreateInfo {
        //         usage: BufferUsage::TRANSFER_SRC,
        //         ..Default::default()
        //     },
        //     AllocationCreateInfo {
        //         memory_type_filter: MemoryTypeFilter::PREFER_HOST
        //             | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        //         ..Default::default()
        //     },
        //     vertices,
        // )?;

        let subbuffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                arena_size: vertex_buffer.size() + instance_buffer.size(),
                buffer_usage: BufferUsage::TRANSFER_SRC,
                memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let vertex_staging_buffer = subbuffer_allocator.allocate_slice::<Vertex>(vertices_length as DeviceSize)?;
        let instances_staging_buffer = subbuffer_allocator.allocate_slice::<InstanceRaw>(instances_length as DeviceSize)?;
        

        {
            let mut vertex_writer = vertex_staging_buffer.write()?;
            vertex_writer.copy_from_slice(&vertices);
            let mut instance_writer = instances_staging_buffer.write()?;
            instance_writer.copy_from_slice(&instances);

        }

        // <----
        // Camera
        // ----->

        let mvp_uniform = vulkan_context.borrow().mvp_uniform().clone();

        // Camera setup

        let uniform_staging_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::TRANSFER_SRC,
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
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

        // let uniform_staging_buffer: Subbuffer<CameraUniform> =
        //     uniform_staging_buffer_allocator.allocate_sized()?;
        // *uniform_staging_buffer.write()? = *camera_uniform.lock().unwrap();

        // let uniform_buffer: Subbuffer<CameraUniform> =
        //     uniform_buffer_allocator.allocate_sized().unwrap();

        let uniform_staging_buffer: Subbuffer<MVP> =
            uniform_staging_buffer_allocator.allocate_sized()?;
        *uniform_staging_buffer.write()? = *mvp_uniform.lock().unwrap();

        let uniform_buffer: Subbuffer<MVP> = uniform_buffer_allocator.allocate_sized().unwrap();
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
        instances_staging_buffer,
        instance_buffer.clone(),
        ))?;

        // Condition on index buffer existence
        // 2 "actions" here
        // if yes copy_buffer command index staging buffer and index_buffer is Some
        // otherwise no copy_buffer command and index_buffer option = None
        let index_buffer = match index_buffer {
            Some(index_buffer) => match index_staging_buffer {
                Some(index_staging_buffer) => {
                    command_builder.copy_buffer(CopyBufferInfo::buffers(
                        index_staging_buffer,
                        index_buffer.clone(),
                    ))?;

                    Some(index_buffer)
                }
                None => None,
            },
            None => None,
        };

        command_builder.copy_buffer(CopyBufferInfo::buffers(
            uniform_staging_buffer.clone(),
            uniform_buffer.clone(),
        ))?;

        let command_buffer = command_builder.build()?;

        // submit command
        let buffers_upload_future = sync::now(Arc::clone(&device))
            .then_execute(Arc::clone(&queue), command_buffer)?
            .then_signal_fence_and_flush()?;

        //

        //  Lights

        // Ambient Light

        let ambient_light = WHITE_AMBIENT_LIGHT;
        //let ambient_light = AmbientLight { color: [0.0, 0.5 , 0.5], intensity: 0.7};

        let ambient_light_subbuffer =
            AmbientLight::setup_ambient_light_buffers(ambient_light, memory_allocator.clone())?;

        // Directional Light

        let directional_light = DirectionalLight {
            position: [0.0, 0.2, 1.5],
            color: [1.0, 1.0, 0.0],
        };

        //let directional_light = vec![directional_light.clone()];

        let directional_lights_subbuffer = DirectionalLight::setup_directional_light_buffers(
            directional_light,
            memory_allocator.clone(),
        )?;

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
                [shader::Vertex::per_vertex(), instance_buffer::InstanceRaw::per_instance()].definition(&vertex_shader.info().input_interface)?;

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
            // let layout = PipelineLayout::new(
            //     Arc::clone(&device),
            //     // Since we only have one pipeline in this example, and thus one pipeline layout,
            //     // we automatically generate the creation info for it from the resources used in the
            //     // shaders. In a real application, you would specify this information manually so that you
            //     // can re-use one layout in multiple pipelines.
            //     PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            //         .into_pipeline_layout_create_info(Arc::clone(&device))?,
            // )?;

            let layout = {
                let mut layout_create_info =
                    PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages);

                let set_layout = &mut layout_create_info.set_layouts[0];
                set_layout.bindings.insert(
                    1,
                    DescriptorSetLayoutBinding {
                        descriptor_type: DescriptorType::UniformBuffer,
                        descriptor_count: 1,
                        stages: ShaderStages::FRAGMENT,
                        ..DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer)
                    },
                );

                set_layout.bindings.insert(
                    2,
                    DescriptorSetLayoutBinding {
                        descriptor_type: DescriptorType::UniformBuffer,
                        descriptor_count: 1,
                        stages: ShaderStages::FRAGMENT,
                        ..DescriptorSetLayoutBinding::descriptor_type(DescriptorType::UniformBuffer)
                    },
                );

                PipelineLayout::new(
                    Arc::clone(&device),
                    layout_create_info.into_pipeline_layout_create_info(Arc::clone(&device))?,
                )?
            };

            // We describe the formats of attachment images where the colors, depth and/or stencil
            // information will be written. The pipeline will only be usable with this particular
            // configuration of the attachment images.
            let subpass = PipelineRenderingCreateInfo {
                // We specify a single color attachment that will be rendered to. When we begin
                // rendering, we will specify a swapchain image to be used as this attachment, so here
                // we set its format to be the same format as the swapchain.
                color_attachment_formats: vec![Some(Format::B8G8R8A8_SRGB)], // ⚠ Caution! Hard coded
                depth_attachment_format: Some(Format::D16_UNORM),
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
                    // Depth
                    depth_stencil_state: Some(DepthStencilState {
                        // Simple = CompareOp::Less,
                        depth: Some(DepthState::simple()),
                        ..Default::default()
                    }),
                    // How multiple fragment shader samples are converted to a single pixel value.
                    // The default value does not perform any multisampling.
                    //Original without MSAA 👉 multisample_state: Some(MultisampleState::default()),
                    multisample_state: Some(MultisampleState {
                        // MSAA
                        rasterization_samples: vulkan_context.borrow().samples, //SampleCount::Sample4,
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
                graphics_pipeline
                    .layout()
                    .set_layouts()
                    .first()
                    .expect("error getting the layout"),
            ),
            [
                WriteDescriptorSet::buffer(0, uniform_buffer.clone()),
                WriteDescriptorSet::buffer(1, ambient_light_subbuffer.clone()),
                WriteDescriptorSet::buffer(2, directional_lights_subbuffer.clone()),
            ],
            [],
        )?;

        buffers_upload_future.wait(None)?; // Not sure this works? Is this needed

        Ok(Self {
            device,
            queue,
            memory_allocator,
            command_allocator,
            graphics_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            descriptor_set,
            vulkan_context,
            uniform_staging_buffer,
            uniform_buffer,
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

    pub fn index_buffer(&self) -> &Option<Subbuffer<[u32]>> {
        &self.index_buffer
    }

    pub fn descriptor_set(&self) -> &Arc<PersistentDescriptorSet> {
        &self.descriptor_set
    }

    pub fn vulkan_context(&self) -> &Arc<VulkanContext> {
        &self.vulkan_context()
    }

    pub fn update_uniform_buffer(&self) -> Result<()> {
        *self.uniform_staging_buffer.write()? =
            *self.vulkan_context.borrow().mvp_uniform().lock().unwrap();

        let mut command_builder = AutoCommandBufferBuilder::primary(
            &self.command_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        command_builder.copy_buffer(CopyBufferInfo::buffers(
            self.uniform_staging_buffer.clone(),
            self.uniform_buffer.clone(),
        ))?;

        let command_buffer = command_builder.build()?;

        // submit command
        let buffers_upload_future = sync::now(Arc::clone(&self.device))
            .then_execute(Arc::clone(&self.queue), command_buffer)?
            .then_signal_fence_and_flush()?;

        buffers_upload_future.wait(None)?;
        Ok(())
    }
}
