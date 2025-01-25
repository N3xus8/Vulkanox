// Note: Physical Instance
use std::sync::Arc;

use tracing::info;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{DeviceExtensions, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use vulkano::{Version, VulkanLibrary};
use winit::window::Window;

use crate::error::Result;

#[derive(Clone)]
pub struct VulkanInstance {
    pub physical_device: Arc<PhysicalDevice>,
    pub queue_family_index: u32,
    pub device_extensions: DeviceExtensions,
}

impl VulkanInstance {
    pub fn new(compatible_window: Arc<Window>) -> Result<Self> {
        let library = VulkanLibrary::new()?;

        let required_extensions = Surface::required_extensions(&compatible_window);

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_extensions,
                ..Default::default()
            },
        )?;

        let surface = Surface::from_window(Arc::clone(&instance), compatible_window)?;

        // device extension to render to a window
        let mut device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()?
            .filter(|phys_dev| {
                phys_dev.api_version() >= Version::V1_3
                    || phys_dev.supported_extensions().khr_dynamic_rendering
            })
            .filter(|phys_dev| phys_dev.supported_extensions().contains(&device_extensions))
            .filter_map(|phys_dev| {
                phys_dev
                    .queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(idx, queue)| {
                        queue.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && phys_dev
                                .surface_support(idx as u32, &surface)
                                .unwrap_or(false)
                    })
                    .map(|idx| (phys_dev, idx as u32))
            })
            .min_by_key(|(phys_dev, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match phys_dev.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("no suitable physical device found");

        // Some little debug infos.
        info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );
        // If the selected device doesn't have Vulkan 1.3 available, then we need to enable the
        // `khr_dynamic_rendering` extension manually. This extension became a core part of Vulkan
        // in version 1.3 and later, so it's always available then and it does not need to be enabled.
        // We can be sure that this extension will be available on the selected physical device,
        // because we filtered out unsuitable devices in the device selection code above.

        device_extensions.khr_dynamic_rendering = physical_device.api_version() < Version::V1_3;

        Ok(Self {
            physical_device,
            queue_family_index,
            device_extensions,
        })
    }

    pub fn physical_device(&self) -> &Arc<PhysicalDevice> {
        &self.physical_device
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    pub fn device_extensions(&self) -> &DeviceExtensions {
        &self.device_extensions
    }
}
