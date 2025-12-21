use std::time::Instant;

use ash::vk;
use tracing::{debug, error, info};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use vers_engine::renderer::vulkan::{
    command::VulkanCommandPool,
    cube::{INDICES, VERTICES},
    depth::{DepthError, VulkanDepthBuffer},
    device::VulkanDevice,
    entry::VulkanEntry,
    framebuffer::VulkanFramebuffers,
    instance::VulkanInstance,
    memory::{MemoryError, VulkanBuffer},
    physical_device::VulkanPhysicalDevice,
    pipeline::{PipelineError, VulkanPipeline},
    render_pass::VulkanRenderPass,
    surface::VulkanSurface,
    swapchain::VulkanSwapchain,
    sync::VulkanSync,
};
use vers_engine::renderer::vulkan::camera::Camera;

#[repr(C)]
#[derive(Clone, Copy)]
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view:  [[f32; 4]; 4],
    proj:  [[f32; 4]; 4],
}

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

struct VulkanContext {
    // Geometry
    vertex_buffer:  VulkanBuffer,
    index_buffer:   VulkanBuffer,
    // One UBO per frame in flight
    uniform_buffers: Vec<VulkanBuffer>,
    // Descriptors
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    // Pipeline
    pipeline:        VulkanPipeline,
    // Render
    command_pool:    VulkanCommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    sync:            VulkanSync,
    // Swapchain & co
    framebuffers:    VulkanFramebuffers,
    depth_buffer:    VulkanDepthBuffer,
    render_pass:     VulkanRenderPass,
    swapchain:       VulkanSwapchain,
    // Core (destruction order: device last)
    device:          VulkanDevice,
    physical_device: VulkanPhysicalDevice,
    surface:         VulkanSurface,
    instance:        VulkanInstance,
    entry:           VulkanEntry,
    // State
    current_frame:   usize,
    camera:          Camera,
    start_time:      Instant,
}

impl VulkanContext {
    fn new(window: &Window) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let entry           = VulkanEntry::new()?;
        let instance        = VulkanInstance::new(&entry, window)?;
        let surface         = VulkanSurface::new(&entry, &instance, window, window)?;
        let physical_device = VulkanPhysicalDevice::select(&instance, &surface)?;
        let device          = VulkanDevice::new(&instance, &physical_device)?;

        // Swapchain + depth + render pass + framebuffers
        let swapchain = VulkanSwapchain::new(
            &instance, &physical_device, &device, &surface,
            (size.width, size.height),
        )?;
        let depth_buffer = VulkanDepthBuffer::new(&instance, &physical_device, &device, &swapchain)?;
        let render_pass  = VulkanRenderPass::new(&device, &swapchain, &depth_buffer)?;
        let framebuffers = VulkanFramebuffers::new(&device, &render_pass, &swapchain, &depth_buffer)?;

        // Command pool
        let command_pool = VulkanCommandPool::new(&device, &physical_device)?;

        // Geometry buffers
        let vertex_size = (std::mem::size_of_val(VERTICES)) as vk::DeviceSize;
        let index_size  = (std::mem::size_of_val(INDICES))  as vk::DeviceSize;

        // Upload via staging buffer → device-local buffer
        let vertex_buffer = upload_buffer(
            &instance, &physical_device, &device, command_pool.pool,
            bytemuck::cast_slice(VERTICES), vertex_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;

        let index_buffer = upload_buffer(
            &instance, &physical_device, &device, command_pool.pool,
            bytemuck::cast_slice(INDICES), index_size,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        // Uniform buffers (one per frame)
        let frames_in_flight = swapchain.config.image_count as usize;
        let ubo_size = std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize;

        let uniform_buffers: Vec<VulkanBuffer> = (0..frames_in_flight)
            .map(|_| VulkanBuffer::new(
                &instance, &physical_device, &device,
                ubo_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ))
            .collect::<Result<_, _>>()?;

        let pipeline = VulkanPipeline::new(&device, &render_pass)?;

        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(frames_in_flight as u32);

        let pool_sizes = [pool_size];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(frames_in_flight as u32);

        let descriptor_pool = unsafe {
            device.device.create_descriptor_pool(&pool_info, None)?
        };

        let layouts = vec![pipeline.descriptor_layout; frames_in_flight];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.device.allocate_descriptor_sets(&alloc_info)? };

        for (i, &set) in descriptor_sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(uniform_buffers[i].buffer)
                .offset(0)
                .range(ubo_size);

            let buffer_infos = [buffer_info];
            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos);

            unsafe { device.device.update_descriptor_sets(&[write], &[]) };
        }

        // Command buffers + sync
        let command_buffers = command_pool.allocate(frames_in_flight as u32)?;
        let sync            = VulkanSync::new(&device, frames_in_flight as u32)?;

        info!(
            gpu = %physical_device.name(),
            frames_in_flight,
            "Vulkan initialized"
        );

        Ok(Self {
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            descriptor_pool,
            descriptor_sets,
            pipeline,
            command_pool,
            command_buffers,
            sync,
            framebuffers,
            depth_buffer,
            render_pass,
            swapchain,
            device,
            physical_device,
            surface,
            instance,
            entry,
            current_frame: 0,
            camera:     Camera {
                position: [0.0, -0.5, -2.5],
                target:   [0.0,  0.0,  0.0],
                ..Camera::default()
            },
            start_time: Instant::now(),
        })
    }

    fn draw(&mut self, window_size: (u32, u32)) {
        if let Err(e) = self.draw_frame(window_size) {
            error!("draw_frame: {e}");
        }
    }

    fn draw_frame(&mut self, window_size: (u32, u32)) -> anyhow::Result<()> {
        let frame = &self.sync.frames[self.current_frame];
        let cmd   = self.command_buffers[self.current_frame];

        unsafe { self.device.device.wait_for_fences(&[frame.in_flight], true, u64::MAX)? };

        // Acquire
        let acquire = unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.swapchain, u64::MAX,
                frame.image_available, vk::Fence::null(),
            )
        };

        let image_index = match acquire {
            Ok((idx, false)) => idx,
            Ok((_, true)) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                debug!("Swapchain out-of-date on acquire");
                self.recreate(window_size)?;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        unsafe { self.device.device.reset_fences(&[frame.in_flight])? };

        // Update UBO
        self.update_ubo(self.current_frame)?;

        // Record
        self.record(cmd, image_index as usize)?;

        // Submit
        let waits   = [frame.image_available];
        let signals = [frame.render_finished];
        let stages  = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let cmds    = [cmd];

        unsafe {
            self.device.device.queue_submit(
                self.device.graphics_queue,
                &[vk::SubmitInfo::default()
                    .wait_semaphores(&waits)
                    .wait_dst_stage_mask(&stages)
                    .command_buffers(&cmds)
                    .signal_semaphores(&signals)],
                frame.in_flight,
            )?;
        }

        // Present
        let swapchains = [self.swapchain.swapchain];
        let indices    = [image_index];
        let present = unsafe {
            self.swapchain.loader.queue_present(
                self.device.present_queue,
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(&signals)
                    .swapchains(&swapchains)
                    .image_indices(&indices),
            )
        };

        match present {
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                debug!("Swapchain suboptimal/out-of-date on present");
                self.recreate(window_size)?;
            }
            Ok(false) => {}
            Err(e) => return Err(e.into()),
        }

        self.current_frame = (self.current_frame + 1) % self.sync.frames.len();
        Ok(())
    }

    fn update_ubo(&self, frame: usize) -> anyhow::Result<()> {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let extent  = self.swapchain.config.extent;
        let aspect  = extent.width as f32 / extent.height as f32;

        let angle   = elapsed * std::f32::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();

        let model: [[f32; 4]; 4] = [
            [ cos, 0.0, sin, 0.0],
            [ 0.0, 1.0, 0.0, 0.0],
            [-sin, 0.0, cos, 0.0],
            [ 0.0, 0.0, 0.0, 1.0],
        ];

        let ubo = UniformBufferObject {
            model,
            view: self.camera.view(),
            proj: self.camera.projection(aspect),
        };
        self.uniform_buffers[frame].upload(std::slice::from_ref(&ubo))?;
        Ok(())
    }

    fn record(&self, cmd: vk::CommandBuffer, image_index: usize) -> anyhow::Result<()> {
        let device = &self.device.device;
        let extent = self.swapchain.config.extent;

        unsafe {
            device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;
            device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let clear_values = [
                vk::ClearValue { color: vk::ClearColorValue { float32: CLEAR_COLOR } },
                vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
            ];

            device.cmd_begin_render_pass(
                cmd,
                &vk::RenderPassBeginInfo::default()
                    .render_pass(self.render_pass.render_pass)
                    .framebuffer(self.framebuffers.framebuffers[image_index])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    })
                    .clear_values(&clear_values),
                vk::SubpassContents::INLINE,
            );

            // Pipeline + viewport + scissor
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);

            device.cmd_set_viewport(cmd, 0, &[vk::Viewport {
                x: 0.0, y: 0.0,
                width: extent.width as f32, height: extent.height as f32,
                min_depth: 0.0, max_depth: 1.0,
            }]);

            device.cmd_set_scissor(cmd, 0, &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            }]);

            // Bind vertex + index buffers
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.buffer], &[0]);
            device.cmd_bind_index_buffer(cmd, self.index_buffer.buffer, 0, vk::IndexType::UINT16);

            // Bind descriptor set (UBO)
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[self.descriptor_sets[self.current_frame]],
                &[],
            );

            // Draw!
            device.cmd_draw_indexed(cmd, INDICES.len() as u32, 1, 0, 0, 0);

            device.cmd_end_render_pass(cmd);
            device.end_command_buffer(cmd)?;
        }
        Ok(())
    }

    fn recreate(&mut self, window_size: (u32, u32)) -> anyhow::Result<()> {
        unsafe { self.device.device.device_wait_idle()? };

        self.swapchain.recreate(
            &self.instance, &self.physical_device, &self.device, &self.surface, window_size,
        )?;
        self.depth_buffer.recreate(
            &self.instance, &self.physical_device, &self.device, &self.swapchain,
        )?;
        self.framebuffers.recreate(
            &self.device, &self.render_pass, &self.swapchain, &self.depth_buffer,
        )?;
        Ok(())
    }

    fn wait_idle(&self) {
        unsafe { self.device.device.device_wait_idle().ok() };
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        self.wait_idle();
        unsafe {
            self.device.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

fn upload_buffer(
    instance:        &VulkanInstance,
    physical_device: &VulkanPhysicalDevice,
    device:          &VulkanDevice,
    cmd_pool:        vk::CommandPool,
    data:            &[u8],
    size:            vk::DeviceSize,
    usage:           vk::BufferUsageFlags,
) -> anyhow::Result<VulkanBuffer> {
    // CPU-visible staging buffer
    let staging = VulkanBuffer::new(
        instance, physical_device, device, size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    staging.upload(data)?;

    // GPU-local buffer
    let gpu = VulkanBuffer::new(
        instance, physical_device, device, size,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    staging.copy_to(device, cmd_pool, &gpu)?;
    Ok(gpu)
}

#[derive(Default)]
struct App {
    window:      Option<Window>,
    vulkan:      Option<VulkanContext>,
    window_size: (u32, u32),
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.vulkan.is_some() { return; }

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title(format!("VERS v{}", env!("CARGO_PKG_VERSION"))),
            )
            .expect("Failed to create window");

        let size = window.inner_size();
        self.window_size = (size.width, size.height);

        let vulkan = VulkanContext::new(&window).expect("Failed to initialize Vulkan");
        self.window = Some(window);
        self.vulkan = Some(vulkan);

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(v) = &self.vulkan { v.wait_idle(); }
                self.vulkan = None;
                event_loop.exit();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                self.window_size = (width, height);
            }
            WindowEvent::RedrawRequested => {
                if let Some(v) = &mut self.vulkan {
                    v.draw(self.window_size);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window { w.request_redraw(); }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    EventLoop::new().unwrap().run_app(&mut App::default()).unwrap();
}