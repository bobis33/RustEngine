use ash::{vk, Device};
use thiserror::Error;
use std::collections::HashSet;

use super::entry::VulkanEntry;
use super::instance::VulkanInstance;
use super::physical_device::VulkanPhysicalDevice;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct VulkanDevice {
    pub device: Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
}

impl VulkanDevice {
    pub fn new(
        instance: &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
    ) -> Result<Self, DeviceError> {
        let indices = physical_device.queue_families;

        // Deduplicate: graphics and present may be the same family
        let unique_families: HashSet<u32> = [indices.graphics, indices.present]
            .into_iter()
            .collect();

        let queue_priority = [1.0_f32];

        let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = unique_families
            .iter()
            .map(|&family| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family)
                    .queue_priorities(&queue_priority)
            })
            .collect();

        // Swapchain extension is required for presenting to a surface
        let extensions = [ash::khr::swapchain::NAME.as_ptr()];

        // Enable common features for a 3D renderer
        let features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)   // Anisotropic filtering
            .fill_mode_non_solid(true); // Wireframe mode

        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extensions)
            .enabled_features(&features);

        let device = unsafe {
            instance
                .instance
                .create_device(physical_device.physical_device, &create_info, None)?
        };

        // Queue index 0: we only create one queue per family
        let graphics_queue = unsafe { device.get_device_queue(indices.graphics, 0) };
        let present_queue  = unsafe { device.get_device_queue(indices.present,  0) };

        Ok(Self {
            device,
            graphics_queue,
            present_queue,
        })
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) };
    }
}