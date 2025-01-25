// Note: Logical Device

use std::sync::Arc;

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceCreateInfo, Features, Queue, QueueCreateInfo},
    memory::allocator::StandardMemoryAllocator,
};

use crate::{error::Result, vulkan_instance::VulkanInstance};
pub struct VulkanDevice {
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_allocator: Arc<StandardCommandBufferAllocator>,
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

        Ok(Self {
            queue,
            memory_allocator,
            command_allocator,
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
}
