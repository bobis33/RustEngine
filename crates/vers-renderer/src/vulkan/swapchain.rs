use ash::{khr, vk};
use thiserror::Error;
use log::{debug, info};
use super::device::VulkanDevice;
use super::instance::VulkanInstance;
use super::physical_device::VulkanPhysicalDevice;
use super::surface::VulkanSurface;

#[derive(Debug, Error)]
pub enum SwapchainError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("No suitable surface format found")]
    NoSuitableFormat,
    #[error("Surface has zero extent (window minimized?)")]
    ZeroExtent,
}

#[derive(Debug, Clone, Copy)]
pub struct SwapchainConfig {
    pub format:       vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent:       vk::Extent2D,
    pub image_count:  u32,
}

pub struct VulkanSwapchain {
    pub loader:      khr::swapchain::Device,
    pub swapchain:   vk::SwapchainKHR,
    pub images:      Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub config:      SwapchainConfig,
    device:          ash::Device,
}

impl VulkanSwapchain {
    pub fn new(
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        surface:         &VulkanSurface,
        window_size:     (u32, u32),
    ) -> Result<Self, SwapchainError> {
        Self::create(instance, physical_device, device, surface, window_size, vk::SwapchainKHR::null())
    }

    /// Recreate the swapchain in-place after a resize.
    /// Skips silently when the window is minimized (zero extent).
    pub fn recreate(
        &mut self,
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        surface:         &VulkanSurface,
        window_size:     (u32, u32),
    ) -> Result<(), SwapchainError> {
        if window_size.0 == 0 || window_size.1 == 0 {
            return Ok(());
        }

        // Build the new swapchain, handing the old handle to the driver
        // so it can reuse resources underneath.
        let new = Self::create(
            instance,
            physical_device,
            device,
            surface,
            window_size,
            self.swapchain, // old_swapchain hint
        )?;

        // Now tear down the OLD resources (image views first, then handle).
        // `self` still holds the old data at this point.
        self.destroy_image_views();
        unsafe { self.loader.destroy_swapchain(self.swapchain, None) };

        // Move new data into self — no swap, no leftover struct to worry about.
        self.loader      = new.loader.clone();
        self.swapchain   = new.swapchain;
        self.images      = new.images.clone();
        self.image_views = new.image_views.clone();
        self.config      = new.config;

        // Prevent new's Drop from destroying what we just moved out
        std::mem::forget(new);

        Ok(())
    }

    fn create(
        instance:        &VulkanInstance,
        physical_device: &VulkanPhysicalDevice,
        device:          &VulkanDevice,
        surface:         &VulkanSurface,
        window_size:     (u32, u32),
        old_swapchain:   vk::SwapchainKHR,
    ) -> Result<Self, SwapchainError> {
        let capabilities = unsafe {
            surface.loader.get_physical_device_surface_capabilities(
                physical_device.physical_device,
                surface.surface,
            )?
        };
        let formats = unsafe {
            surface.loader.get_physical_device_surface_formats(
                physical_device.physical_device,
                surface.surface,
            )?
        };
        let present_modes = unsafe {
            surface.loader.get_physical_device_surface_present_modes(
                physical_device.physical_device,
                surface.surface,
            )?
        };

        let format       = choose_format(&formats)?;
        let present_mode = choose_present_mode(&present_modes);
        let extent       = choose_extent(&capabilities, window_size)?;

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 {
            image_count = image_count.min(capabilities.max_image_count);
        }

        let indices = physical_device.queue_families;
        let families = [indices.graphics, indices.present];
        let (sharing, families_slice): (vk::SharingMode, &[u32]) =
            if indices.graphics == indices.present {
                (vk::SharingMode::EXCLUSIVE, &[])
            } else {
                (vk::SharingMode::CONCURRENT, &families)
            };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing)
            .queue_family_indices(families_slice)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let loader    = khr::swapchain::Device::new(&instance.instance, &device.device);
        let swapchain = unsafe { loader.create_swapchain(&create_info, None)? };
        let images    = unsafe { loader.get_swapchain_images(swapchain)? };
        let image_views = create_image_views(&device.device, &images, format.format)?;

        let config = SwapchainConfig {
            format,
            present_mode,
            extent,
            image_count: images.len() as u32,
        };

        debug!(
            "==> Swapchain: {}x{} | {:?} | {:?} | {} images",
            extent.width, extent.height, format.format, present_mode, images.len(),
        );

        Ok(Self {
            loader,
            swapchain,
            images,
            image_views,
            config,
            device: device.device.clone(),
        })
    }

    fn destroy_image_views(&self) {
        for &view in &self.image_views {
            unsafe { self.device.destroy_image_view(view, None) };
        }
    }
}

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        self.destroy_image_views();
        unsafe { self.loader.destroy_swapchain(self.swapchain, None) };
    }
}

// ---------------------------------------------------------------------------
// Selection helpers
// ---------------------------------------------------------------------------

fn choose_format(formats: &[vk::SurfaceFormatKHR]) -> Result<vk::SurfaceFormatKHR, SwapchainError> {
    if formats.is_empty() {
        return Err(SwapchainError::NoSuitableFormat);
    }
    formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .copied()
        .or_else(|| formats.first().copied())
        .ok_or(SwapchainError::NoSuitableFormat)
}

fn choose_present_mode(modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    if modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    }
}

fn choose_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    (width, height): (u32, u32),
) -> Result<vk::Extent2D, SwapchainError> {
    if capabilities.current_extent.width != u32::MAX {
        let e = capabilities.current_extent;
        if e.width == 0 || e.height == 0 {
            return Err(SwapchainError::ZeroExtent);
        }
        return Ok(e);
    }
    if width == 0 || height == 0 {
        return Err(SwapchainError::ZeroExtent);
    }
    Ok(vk::Extent2D {
        width:  width.clamp(capabilities.min_image_extent.width,  capabilities.max_image_extent.width),
        height: height.clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
    })
}

fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>, vk::Result> {
    images
        .iter()
        .map(|&image| {
            let info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask:      vk::ImageAspectFlags::COLOR,
                    base_mip_level:   0,
                    level_count:      1,
                    base_array_layer: 0,
                    layer_count:      1,
                });
            unsafe { device.create_image_view(&info, None) }
        })
        .collect()
}