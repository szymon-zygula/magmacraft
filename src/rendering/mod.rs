use ash::vk;
use custom_error::custom_error;
use crate::vulkan;

custom_error!{pub RenderingError
    VulkanError {source: vulkan::VulkanError} =
        "encountered a vulkan error while rendering: {source}",
    AcquireImageError {result: vk::Result} =
        "failed to acquire swapchain image: {result}",
    RenderImageError {result: vk::Result} =
        "failed to submit swapchain image for rendering: {result}",
    PresentImageError {result: vk::Result} =
        "failed to submit swapchain image for presentation: {result}",
    DeviceWaitIdleError {result: vk::Result} =
        "faild to wait for vulkan logical device to become idle: {result}"
}

pub type RenderingResult<T> = Result<T, RenderingError>;

pub mod renderer;
pub mod render_state;
