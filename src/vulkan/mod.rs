use custom_error::custom_error;
use ash::{self, vk};
use crate::builder::BuilderError;

custom_error!{pub VulkanError
    OperationFailed {source: vk::Result, operation: String} = "operation failed: {operation} ({source})",
    LibraryLoadError {source: ash::LoadingError} = "failed to load Vulkan library: {source}",
    ValidationLayersNotAvailable = "specified validation layers are not available",
    InstanceCreateError {source: ash::InstanceError} = "failed to create vulkan instance: {source}",
    InstanceBuildError {source: BuilderError} = "failed to build instance",
    InstanceExtensionsCreationError {source: std::ffi::NulError} = "failed to create C-like nul-terminated string: {source}",
    SuitableDeviceNotFound = "faled to find a physical device fulfilling all criteria"
}

pub mod state;
pub mod instance;
pub mod debug_utils;
pub mod physical_device;
pub mod logical_device;
pub mod surface;