use ash::vk;
use thiserror::Error;

use super::device::VulkanDevice;
use super::physical_device::VulkanPhysicalDevice;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct VulkanCommandPool {
    pub pool:    vk::CommandPool,
    device:      ash::Device,
}

impl VulkanCommandPool {
    pub fn new(
        device:          &VulkanDevice,
        physical_device: &VulkanPhysicalDevice,
    ) -> Result<Self, CommandError> {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(physical_device.queue_families.graphics)
            // RESET_COMMAND_BUFFER: allows re-recording individual buffers
            // without resetting the whole pool each frame
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let pool = unsafe { device.device.create_command_pool(&create_info, None)? };

        Ok(Self { pool, device: device.device.clone() })
    }

    /// Allocate `count` primary command buffers from this pool.
    pub fn allocate(&self, count: u32) -> Result<Vec<vk::CommandBuffer>, CommandError> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        Ok(unsafe { self.device.allocate_command_buffers(&alloc_info)? })
    }
}

impl Drop for VulkanCommandPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.pool, None) };
    }
}