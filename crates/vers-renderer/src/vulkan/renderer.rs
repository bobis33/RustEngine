use ash::vk;
use thiserror::Error;
use log::debug;
use crate::vulkan::depth::{DepthError, VulkanDepthBuffer};
use super::command::{CommandError, VulkanCommandPool};
use super::device::VulkanDevice;
use super::framebuffer::{FramebufferError, VulkanFramebuffers};
use super::instance::VulkanInstance;
use super::physical_device::VulkanPhysicalDevice;
use super::render_pass::VulkanRenderPass;
use super::surface::VulkanSurface;
use super::swapchain::{SwapchainError, VulkanSwapchain};
use super::sync::{SyncError, VulkanSync};

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("Command error: {0}")]
    Command(#[from] CommandError),
    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),
    #[error("Swapchain error: {0}")]
    Swapchain(#[from] SwapchainError),
    #[error("Depth error: {0}")]
    Depth(#[from] DepthError),
    #[error("Framebuffer error: {0}")]
    Framebuffer(#[from] FramebufferError),
}

pub struct VulkanRenderer {
    pub command_pool:    VulkanCommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync:            VulkanSync,
    current_frame:       usize,
}

impl VulkanRenderer {
    pub fn new(
        device:          &VulkanDevice,
        physical_device: &VulkanPhysicalDevice,
        swapchain:       &VulkanSwapchain,
    ) -> Result<Self, RendererError> {
        let command_pool    = VulkanCommandPool::new(device, physical_device)?;
        let command_buffers = command_pool.allocate(swapchain.config.image_count)?;
        let sync            = VulkanSync::new(device, swapchain.config.image_count)?;

        Ok(Self {
            command_pool,
            command_buffers,
            sync,
            current_frame: 0,
        })
    }

    /// Draw one frame. Returns `true` if the swapchain needs to be recreated.
    pub fn draw_frame(
        &mut self,
        device:          &VulkanDevice,
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        surface:         &VulkanSurface,
        swapchain:       &mut VulkanSwapchain,
        depth_buffer:    &mut VulkanDepthBuffer,   // <-- ajouté
        framebuffers:    &mut VulkanFramebuffers,
        render_pass:     &VulkanRenderPass,
        window_size:     (u32, u32),
        clear_color:     [f32; 4],
    ) -> Result<bool, RendererError> {
        let frame = &self.sync.frames[self.current_frame];
        let cmd   = self.command_buffers[self.current_frame];

        unsafe { device.device.wait_for_fences(&[frame.in_flight], true, u64::MAX)?; }

        let acquire_result = unsafe {
            swapchain.loader.acquire_next_image(
                swapchain.swapchain, u64::MAX,
                frame.image_available, vk::Fence::null(),
            )
        };

        let image_index = match acquire_result {
            Ok((index, false)) => index,
            Ok((_, true)) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                debug!("Swapchain out of date on acquire — recreating");
                self.recreate_swapchain(
                    device, instance, physical_device, surface,
                    swapchain, depth_buffer, framebuffers, render_pass, window_size,
                )?;
                return Ok(false);
            }
            Err(e) => return Err(e.into()),
        };

        unsafe { device.device.reset_fences(&[frame.in_flight])?; }

        unsafe {
            device.device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            device.device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let clear_values = [
                vk::ClearValue { color: vk::ClearColorValue { float32: clear_color } },
                vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
            ];

            device.device.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(render_pass.render_pass)
                    .framebuffer(framebuffers.framebuffers[image_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: swapchain.config.extent,
                    })
                    .clear_values(&clear_values),
                vk::SubpassContents::INLINE,
            );

            // ---- draw calls will go here ----

            device.device.cmd_end_render_pass(cmd);
            device.device.end_command_buffer(cmd)?;
        }

        let wait_semaphores   = [frame.image_available];
        let signal_semaphores = [frame.render_finished];
        let wait_stages       = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let cmds              = [cmd];

        unsafe {
            device.device.queue_submit(
                device.graphics_queue,
                &[vk::SubmitInfo::default()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_stages)
                    .command_buffers(&cmds)
                    .signal_semaphores(&signal_semaphores)],
                frame.in_flight,
            )?;
        }

        let swapchains    = [swapchain.swapchain];
        let image_indices = [image_index];

        let present_result = unsafe {
            swapchain.loader.queue_present(
                device.present_queue,
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(&signal_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&image_indices),
            )
        };

        match present_result {
            Ok(false) => {}
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                debug!("Swapchain suboptimal/out-of-date on present — recreating");
                self.recreate_swapchain(
                    device, instance, physical_device, surface,
                    swapchain, depth_buffer, framebuffers, render_pass, window_size,
                )?;
            }
            Err(e) => return Err(e.into()),
        }

        self.current_frame = (self.current_frame + 1) % self.sync.frames.len();
        Ok(false)
    }

    /// Recreate swapchain + framebuffers + re-allocate command buffers.
    fn recreate_swapchain(
        &mut self,
        device:          &VulkanDevice,
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        surface:         &VulkanSurface,
        swapchain:       &mut VulkanSwapchain,
        depth_buffer:    &mut VulkanDepthBuffer,
        framebuffers:    &mut VulkanFramebuffers,
        render_pass:     &VulkanRenderPass,
        window_size:     (u32, u32),
    ) -> Result<(), RendererError> {
        unsafe { device.device.device_wait_idle()? };

        swapchain.recreate(instance, physical_device, device, surface, window_size)?;
        depth_buffer.recreate(instance, physical_device, device, swapchain)?;
        framebuffers.recreate(device, render_pass, swapchain, depth_buffer)?;

        if self.command_buffers.len() != swapchain.config.image_count as usize {
            self.command_buffers = self.command_pool.allocate(swapchain.config.image_count)?;
        }

        Ok(())
    }
}