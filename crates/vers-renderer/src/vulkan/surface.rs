use ash::{khr, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use thiserror::Error;

use super::entry::VulkanEntry;
use super::instance::VulkanInstance;

#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("Invalid window handle")]
    InvalidHandle,
}

pub struct VulkanSurface {
    pub(crate) loader: khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
}

impl VulkanSurface {
    pub fn new(
        entry: &VulkanEntry,
        instance: &VulkanInstance,
        window: &dyn HasWindowHandle,
        display: &dyn HasDisplayHandle,
    ) -> Result<Self, SurfaceError> {
        let raw_display = display
            .display_handle()
            .map_err(|_| SurfaceError::InvalidHandle)?
            .as_raw();

        let raw_window = window
            .window_handle()
            .map_err(|_| SurfaceError::InvalidHandle)?
            .as_raw();

        let loader = khr::surface::Instance::new(&entry.entry, &instance.instance);

        let surface = unsafe {
            ash_window::create_surface(
                &entry.entry,
                &instance.instance,
                raw_display,
                raw_window,
                None,
            )?
        };

        Ok(Self { loader, surface })
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.surface, None) };
    }
}