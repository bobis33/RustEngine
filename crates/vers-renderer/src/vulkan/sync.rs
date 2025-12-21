use ash::vk;
use thiserror::Error;

use super::device::VulkanDevice;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

/// Per-frame synchronization primitives.
///
/// - `image_available`: signaled when the swapchain image is ready to render into
/// - `render_finished`: signaled when rendering is done, waited on before present
/// - `in_flight`:       CPU-side fence — blocks the CPU until the GPU frame is done
pub struct FrameSync {
    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub in_flight:       vk::Fence,
    device:              ash::Device,
}

impl FrameSync {
    pub fn new(device: &VulkanDevice) -> Result<Self, SyncError> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();

        // Create the fence pre-signaled so the first frame doesn't wait forever
        let fence_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let image_available = unsafe { device.device.create_semaphore(&semaphore_info, None)? };
        let render_finished = unsafe { device.device.create_semaphore(&semaphore_info, None)? };
        let in_flight       = unsafe { device.device.create_fence(&fence_info, None)? };

        Ok(Self {
            image_available,
            render_finished,
            in_flight,
            device: device.device.clone(),
        })
    }
}

impl Drop for FrameSync {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.image_available, None);
            self.device.destroy_semaphore(self.render_finished, None);
            self.device.destroy_fence(self.in_flight, None);
        }
    }
}

pub struct VulkanSync {
    pub frames: Vec<FrameSync>,
}

impl VulkanSync {
    pub fn new(device: &VulkanDevice, frames_in_flight: u32) -> Result<Self, SyncError> {
        let frames = (0..frames_in_flight)
            .map(|_| FrameSync::new(device))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { frames })
    }
}