use ash::Entry;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntryError {
    #[error("Failed to load Vulkan: {0}")]
    Load(#[from] ash::LoadingError),
}

pub struct VulkanEntry {
    pub(crate) entry: Entry,
}

impl VulkanEntry {
    pub fn new() -> Result<Self, EntryError> {
        let entry = unsafe { Entry::load()? };
        Ok(Self { entry })
    }

    pub unsafe fn version(&self) -> Option<u32> {
        unsafe {
            self.entry.try_enumerate_instance_version().ok().flatten()
        }
    }
}