use ash::vk;
use thiserror::Error;

use super::device::VulkanDevice;
use super::instance::VulkanInstance;
use super::physical_device::VulkanPhysicalDevice;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}

/// A GPU-allocated buffer with its backing memory.
pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size:   vk::DeviceSize,
    device:     ash::Device,
}

impl VulkanBuffer {
    pub fn new(
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        size:            vk::DeviceSize,
        usage:           vk::BufferUsageFlags,
        properties:      vk::MemoryPropertyFlags,
    ) -> Result<Self, MemoryError> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.device.create_buffer(&buffer_info, None)? };

        let mem_requirements = unsafe { device.device.get_buffer_memory_requirements(buffer) };

        let mem_type_index = find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            properties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type_index);

        let memory = unsafe { device.device.allocate_memory(&alloc_info, None)? };

        unsafe { device.device.bind_buffer_memory(buffer, memory, 0)? };

        Ok(Self { buffer, memory, size, device: device.device.clone() })
    }

    /// Map memory, write data, unmap. Only valid for HOST_VISIBLE memory.
    pub fn upload<T: Copy>(&self, data: &[T]) -> Result<(), MemoryError> {
        let size = (std::mem::size_of_val(data)) as vk::DeviceSize;
        unsafe {
            let ptr = self.device.map_memory(self.memory, 0, size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut T, data.len());
            self.device.unmap_memory(self.memory);
        }
        Ok(())
    }

    /// Copy this buffer into `dst` using a one-shot command buffer.
    /// Used to upload from a staging (CPU) buffer to a device-local (GPU) buffer.
    pub fn copy_to(
        &self,
        device:    &VulkanDevice,
        cmd_pool:  vk::CommandPool,
        dst:       &VulkanBuffer,
    ) -> Result<(), MemoryError> {
        let cmd = begin_one_shot(device, cmd_pool)?;

        let region = vk::BufferCopy::default().size(self.size);
        unsafe { device.device.cmd_copy_buffer(cmd, self.buffer, dst.buffer, &[region]) };

        end_one_shot(device, cmd, cmd_pool)?;
        Ok(())
    }
}

impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

/// A GPU image with its backing memory and view.
pub struct VulkanImage {
    pub image:  vk::Image,
    pub view:   vk::ImageView,
    pub memory: vk::DeviceMemory,
    device:     ash::Device,
}

impl VulkanImage {
    pub fn new(
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        width:           u32,
        height:          u32,
        format:          vk::Format,
        tiling:          vk::ImageTiling,
        usage:           vk::ImageUsageFlags,
        properties:      vk::MemoryPropertyFlags,
        aspect:          vk::ImageAspectFlags,
    ) -> Result<Self, MemoryError> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe { device.device.create_image(&image_info, None)? };

        let mem_requirements = unsafe { device.device.get_image_memory_requirements(image) };

        let mem_type_index = find_memory_type(
            instance,
            physical_device,
            mem_requirements.memory_type_bits,
            properties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(mem_type_index);

        let memory = unsafe { device.device.allocate_memory(&alloc_info, None)? };
        unsafe { device.device.bind_image_memory(image, memory, 0)? };

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask:      aspect,
                base_mip_level:   0,
                level_count:      1,
                base_array_layer: 0,
                layer_count:      1,
            });

        let view = unsafe { device.device.create_image_view(&view_info, None)? };

        Ok(Self { image, view, memory, device: device.device.clone() })
    }
}

impl Drop for VulkanImage {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn find_memory_type(
    instance:        &VulkanInstance,
    physical_device: &VulkanPhysicalDevice,
    type_filter:     u32,
    properties:      vk::MemoryPropertyFlags,
) -> Result<u32, MemoryError> {
    let mem_props = unsafe {
        instance.instance.get_physical_device_memory_properties(physical_device.physical_device)
    };

    (0..mem_props.memory_type_count)
        .find(|&i| {
            let type_match = (type_filter & (1 << i)) != 0;
            let prop_match = mem_props.memory_types[i as usize]
                .property_flags
                .contains(properties);
            type_match && prop_match
        })
        .ok_or(MemoryError::NoSuitableMemoryType)
}

fn begin_one_shot(device: &VulkanDevice, pool: vk::CommandPool) -> Result<vk::CommandBuffer, MemoryError> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let cmd = unsafe { device.device.allocate_command_buffers(&alloc_info)?[0] };

    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe { device.device.begin_command_buffer(cmd, &begin_info)? };
    Ok(cmd)
}

fn end_one_shot(device: &VulkanDevice, cmd: vk::CommandBuffer, pool: vk::CommandPool) -> Result<(), MemoryError> {
    unsafe { device.device.end_command_buffer(cmd)? };

    let cmds = [cmd];
    let submit = vk::SubmitInfo::default().command_buffers(&cmds);

    unsafe {
        device.device.queue_submit(device.graphics_queue, &[submit], vk::Fence::null())?;
        device.device.queue_wait_idle(device.graphics_queue)?;
        device.device.free_command_buffers(pool, &cmds);
    }
    Ok(())
}