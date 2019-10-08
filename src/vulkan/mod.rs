use custom_error::custom_error;
use ash::{self, vk};
use crate::builder::BuilderError;

custom_error!{pub VulkanError
    OperationFailed {source: vk::Result, operation: String} = "operation failed: {operation} ({source})",
    LibraryLoadError {source: ash::LoadingError} = "failed to load Vulkan library: {source}",
    ValidationLayersNotAvailable = "specified validation layers are not available",
    InstanceCreateError {source: ash::InstanceError} = "failed to create vulkan instance: {source}",
    VulkanBuildError {source: BuilderError} = "failed to build a Vulkan structure",
    InstanceExtensionsCreationError {source: std::ffi::NulError} = "failed to create C-like nul-terminated string: {source}",
    SuitableDeviceNotFound = "failed to find a physical device fulfilling all criteria",
    QueueFamilyNotSupported = "physical device was asked about an index of a queue family that it does not support"
}

pub mod state;
pub mod instance;
pub mod debug_utils;
pub mod physical_device;
pub mod logical_device;
pub mod surface;
pub mod swapchain;
