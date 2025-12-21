use ash::vk;
use log::info;
use thiserror::Error;

use super::instance::VulkanInstance;
use super::surface::VulkanSurface;

#[derive(Debug, Error)]
pub enum PhysicalDeviceError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("No suitable GPU found")]
    NoSuitableDevice,
}

/// Queue family indices required by the engine.
/// Both must be `Some` for a device to be considered suitable.
#[derive(Debug, Clone, Copy)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

impl QueueFamilyIndices {
    fn find(
        instance: &VulkanInstance,
        surface: &VulkanSurface,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Option<Self>, vk::Result> {
        let families = unsafe {
            instance
                .instance
                .get_physical_device_queue_family_properties(physical_device)
        };

        let mut graphics: Option<u32> = None;
        let mut present: Option<u32> = None;

        for (i, family) in families.iter().enumerate() {
            let i = i as u32;

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(i);
            }

            let supports_present = unsafe {
                surface.loader.get_physical_device_surface_support(
                    physical_device,
                    i,
                    surface.surface,
                )?
            };

            if supports_present {
                present = Some(i);
            }

            if graphics.is_some() && present.is_some() {
                break;
            }
        }

        Ok(graphics.zip(present).map(|(graphics, present)| Self {
            graphics,
            present,
        }))
    }
}

pub struct VulkanPhysicalDevice {
    pub physical_device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_families: QueueFamilyIndices,
}

impl VulkanPhysicalDevice {
    pub fn select(
        instance: &VulkanInstance,
        surface: &VulkanSurface,
    ) -> Result<Self, PhysicalDeviceError> {
        let devices = unsafe {
            instance.instance.enumerate_physical_devices()?
        };

        let mut best: Option<(u32, vk::PhysicalDevice, vk::PhysicalDeviceProperties, QueueFamilyIndices)> = None;

        for physical_device in devices {
            let properties = unsafe {
                instance.instance.get_physical_device_properties(physical_device)
            };

            let Some(queue_families) =
                QueueFamilyIndices::find(instance, surface, physical_device)?
            else {
                continue; // Missing a required queue family, skip
            };

            let score = score_device(&properties);

            if best.as_ref().map_or(true, |(best_score, ..)| score > *best_score) {
                best = Some((score, physical_device, properties, queue_families));
            }
        }

        let (_, physical_device, properties, queue_families) =
            best.ok_or(PhysicalDeviceError::NoSuitableDevice)?;

        let name = properties.device_name_as_c_str()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "<unknown>".to_string());

        info!("==> Selected GPU: {} ({:?})", name, properties.device_type);

        Ok(Self {
            physical_device,
            properties,
            queue_families,
        })
    }

    pub fn name(&self) -> String {
        self.properties
            .device_name_as_c_str()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "<unknown>".to_string())
    }
}

/// Score a GPU. Higher is better.
/// Discrete GPU > Integrated GPU > everything else.
/// VRAM is used as tiebreaker between two GPUs of the same type.
fn score_device(props: &vk::PhysicalDeviceProperties) -> u32 {
    let type_score = match props.device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU   => 1_000_000,
        vk::PhysicalDeviceType::INTEGRATED_GPU =>   100_000,
        vk::PhysicalDeviceType::VIRTUAL_GPU    =>    10_000,
        vk::PhysicalDeviceType::CPU            =>     1_000,
        _                                      =>         0,
    };

    // VRAM tiebreaker isn't directly in properties — limits give us an
    // indirect signal via max image dimension (bigger usually = more capable).
    let capability_score = props.limits.max_image_dimension2_d;

    type_score + capability_score
}