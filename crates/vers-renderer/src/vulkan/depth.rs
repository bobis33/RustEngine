use ash::vk;
use thiserror::Error;

use super::device::VulkanDevice;
use super::instance::VulkanInstance;
use super::memory::{MemoryError, VulkanImage};
use super::physical_device::VulkanPhysicalDevice;
use super::swapchain::VulkanSwapchain;

#[derive(Debug, Error)]
pub enum DepthError {
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
    #[error("No suitable depth format found")]
    NoFormat,
}

pub struct VulkanDepthBuffer {
    pub image:  VulkanImage,
    pub format: vk::Format,
}

impl VulkanDepthBuffer {
    pub fn new(
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        swapchain:       &VulkanSwapchain,
    ) -> Result<Self, DepthError> {
        let format = choose_depth_format(instance, physical_device)?;
        let extent = swapchain.config.extent;

        let image = VulkanImage::new(
            instance,
            physical_device,
            device,
            extent.width,
            extent.height,
            format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vk::ImageAspectFlags::DEPTH,
        )?;

        Ok(Self { image, format })
    }

    pub fn recreate(
        &mut self,
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        swapchain:       &VulkanSwapchain,
    ) -> Result<(), DepthError> {
        *self = Self::new(instance, physical_device, device, swapchain)?;
        Ok(())
    }
}

/// Pick the best available depth format (with stencil if possible).
fn choose_depth_format(
    instance:        &VulkanInstance,
    physical_device: &VulkanPhysicalDevice,
) -> Result<vk::Format, DepthError> {
    let candidates = [
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    candidates
        .iter()
        .find(|&&format| {
            let props = unsafe {
                instance.instance.get_physical_device_format_properties(
                    physical_device.physical_device,
                    format,
                )
            };
            props.optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        })
        .copied()
        .ok_or(DepthError::NoFormat)
}