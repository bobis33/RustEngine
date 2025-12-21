use ash::vk;
use thiserror::Error;

use super::depth::VulkanDepthBuffer;
use super::device::VulkanDevice;
use super::render_pass::VulkanRenderPass;
use super::swapchain::VulkanSwapchain;

#[derive(Debug, Error)]
pub enum FramebufferError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct VulkanFramebuffers {
    pub framebuffers: Vec<vk::Framebuffer>,
    device:           ash::Device,
}

impl VulkanFramebuffers {
    pub fn new(
        device:       &VulkanDevice,
        render_pass:  &VulkanRenderPass,
        swapchain:    &VulkanSwapchain,
        depth_buffer: &VulkanDepthBuffer,
    ) -> Result<Self, FramebufferError> {
        let extent = swapchain.config.extent;

        let framebuffers = swapchain
            .image_views
            .iter()
            .map(|&color_view| {
                // Must match render pass attachment order: [color, depth]
                let attachments = [color_view, depth_buffer.image.view];

                let info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass.render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1);

                unsafe { device.device.create_framebuffer(&info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { framebuffers, device: device.device.clone() })
    }

    pub fn recreate(
        &mut self,
        device:       &VulkanDevice,
        render_pass:  &VulkanRenderPass,
        swapchain:    &VulkanSwapchain,
        depth_buffer: &VulkanDepthBuffer,
    ) -> Result<(), FramebufferError> {
        self.destroy();
        let new = Self::new(device, render_pass, swapchain, depth_buffer)?;
        self.framebuffers = new.framebuffers.clone();
        std::mem::forget(new);
        Ok(())
    }

    fn destroy(&self) {
        for &fb in &self.framebuffers {
            unsafe { self.device.destroy_framebuffer(fb, None) };
        }
    }
}

impl Drop for VulkanFramebuffers {
    fn drop(&mut self) { self.destroy(); }
}