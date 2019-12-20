use custom_error::custom_error;
use ash::{self, vk};

custom_error!{pub VulkanError
    LibraryLoadError {source: ash::LoadingError} =
        "failed to load Vulkan library: {source}",
    CreateDebugMessengerError {result: vk::Result} =
        "failed to create debug messenger: {result}",
    ValidationLayersNotAvailable =
        "specified validation layers are not available",
    ValidationLayersError {result: vk::Result} =
        "failed to get a list of validation layers: {result}",
    InstanceCreateError {source: ash::InstanceError} =
        "failed to create vulkan instance: {source}",
    InstanceExtensionsCreationError {source: std::ffi::NulError} =
        "failed to create C-like nul-terminated string (invalid extension name): {source}",
    EnumeratePhysicalDevicesError {result: vk::Result}=
        "failed to enumerate GPUs",
    PhysicalDevicePropertiesError {result: vk::Result}=
        "failed to get vulkan physical device properties",
    QueueFamilyNotSupported {queue_family: physical_device::QueueFamily} =
        "physical device was asked about an index of a queue family that it does not support",
    EnumeratePhysicalDeviceExtensionsError {result: vk::Result} = 
        "failed to enumerate physical device extensions",
    PhysicalDeviceSelectError =
        "failed to select a GPU",
    SuitableDeviceNotFound =
        "failed to find a GPU fulfilling all criteria",
    LogicalDeviceCreateError {result: vk::Result} =
        "failed to create vulkan device: {result}",
    SwapchainCreateError {result: vk::Result} =
        "failed to create vulkan swapchain: {result}",
    SwapchainGetImagesError {result: vk::Result} =
        "failed to acquire swapchain images: {result}",
    ShaderCreateError {result: vk::Result} =
        "failed to create shader: {result}",
    ShaderOpenFileError {error: std::io::Error} =
        "failed to open shader file: {error}",
    RenderPassCreateError {result: vk::Result} =
        "failed to create render pass: {result}",
    PipelineCreateError {result: vk::Result} =
        "failed to create pipeline: {result}",
    PipelineLayoutCreateError {result: vk::Result} =
        "failed to create pipeline layout: {result}",
    ImageViewCreateError {result: vk::Result} =
        "failed to create image view: {result}",
    FramebuffersCreateError {result: vk::Result} =
        "failed to create framebuffers: {result}",
    CommandPoolCreateError {result: vk::Result} =
        "failed to create command pool: {result}",
    CommandBufferAllocateError {result: vk::Result} =
        "failed to allocate command buffer: {result}",
    CommandBufferRecordError {result: vk::Result} =
        "failed to record command buffer: {result}"
}

type VulkanResult<T> = Result<T, VulkanError>;

pub mod state;
pub mod instance;
pub mod debug_utils;
pub mod physical_device;
pub mod logical_device;
pub mod surface;
pub mod swapchain;
pub mod shader;
pub mod render_pass;
pub mod pipeline;
pub mod framebuffers;
pub mod command_pool;
pub mod command_buffer;
