use ash::{vk, Instance};
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use thiserror::Error;

use super::entry::VulkanEntry;

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("Invalid window handle")]
    InvalidHandle,
}

pub struct VulkanInstance {
    pub(crate) instance: Instance,
}

impl VulkanInstance {
    pub fn new(
        entry: &VulkanEntry,
        display_handle: &dyn HasDisplayHandle,
    ) -> Result<Self, InstanceError> {
        let raw_display = display_handle
            .display_handle()
            .map_err(|_| InstanceError::InvalidHandle)?
            .as_raw();

        let surface_extensions =
            ash_window::enumerate_required_extensions(raw_display)
                .map_err(InstanceError::Vulkan)?;

        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3);

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(surface_extensions);

        let instance = unsafe {
            entry.entry.create_instance(&create_info, None)?
        };

        Ok(Self { instance })
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}