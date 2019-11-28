use std::rc::Rc;
use ash::{
    self,
    vk
};
use crate::vulkan::{
    VulkanError,
    physical_device::{self, PhysicalDevice},
    logical_device::LogicalDevice,
    surface::Surface
};

pub struct Swapchain {
    vk_swapchain: vk::SwapchainKHR,
    swapchain_loader: Rc<ash::extensions::khr::Swapchain>,
    // lifetime extenders
    _logical_device: Rc<LogicalDevice>,
    _surface: Rc<Surface>
}

impl Swapchain {
    const IMAGE_ARRAY_LAYERS: u32 = 1;
    const ADDITIONAL_IMAGES_COUNT: u32 = 1;

    const PRESENT_MODE_WITH_VSYNC: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
    const PRESENT_MODE_WITHOUT_VSYNC: vk::PresentModeKHR = vk::PresentModeKHR::IMMEDIATE;

    pub fn new(physical_device: Rc<PhysicalDevice>, logical_device: Rc<LogicalDevice>, surface: Rc<Surface>, vsync: bool) -> Result<Self, VulkanError> {
        let surface_properties = physical_device.get_surface_properties(&*surface)?;
        let min_image_count = surface_properties.capabilities.min_image_count + Self::ADDITIONAL_IMAGES_COUNT;
        let image_format = Self::select_image_format(&surface_properties.formats);
        let image_extent = Self::select_image_extent(&surface_properties.capabilities, &*surface);

        let mut swapchain_create_info_builder = vk::SwapchainCreateInfoKHR::builder()
            .surface(**surface)
            .min_image_count(min_image_count)
            .image_format(image_format.format)
            .image_color_space(image_format.color_space)
            .image_extent(image_extent)
            .image_array_layers(Self::IMAGE_ARRAY_LAYERS)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        let multiple_queue_family_usage = physical_device.is_transfer_queue_family_dedicated();
        let graphics_index = physical_device.get_queue_family_index(
            physical_device::QueueFamily::Graphics)?;
        let transfer_index = physical_device.get_queue_family_index(
            physical_device::QueueFamily::Transfer)?;
        let queue_family_indices = &[graphics_index, transfer_index];

        swapchain_create_info_builder = if multiple_queue_family_usage {
            swapchain_create_info_builder
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(queue_family_indices)
        }
        else {
            swapchain_create_info_builder
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        };

        let present_mode = Self::select_present_mode(&surface_properties.present_modes, vsync);

        swapchain_create_info_builder = swapchain_create_info_builder
            .pre_transform(surface_properties.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            // TODO: Allow swapchain recreation
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain_loader = logical_device.get_swapchain_loader();
        let vk_swapchain = unsafe {
            swapchain_loader.create_swapchain(
                &swapchain_create_info_builder,
                None
            ).map_err(VulkanError::operation_failed_mapping("create swapchain"))?
        };

        Ok(Self {
            vk_swapchain,
            swapchain_loader: Rc::clone(&swapchain_loader),
            _logical_device: Rc::clone(&logical_device),
            _surface: Rc::clone(&surface)
        })
    }

    fn select_image_format(formats: &Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        // TODO: Sort out all things related to gamma correction
        return formats[0];
    }

    fn select_image_extent(
        capabilities: &vk::SurfaceCapabilitiesKHR, surface: &Surface
    ) -> vk::Extent2D {
        if capabilities.current_extent.width == u32::max_value() {
            surface.get_framebuffer_extent()
        }
        else {
            capabilities.current_extent
        }
    }

    fn select_present_mode(present_modes: &Vec<vk::PresentModeKHR>, vsync: bool) -> vk::PresentModeKHR {
        for present_mode in present_modes {
            if vsync && *present_mode == Self::PRESENT_MODE_WITH_VSYNC {
                return vk::PresentModeKHR::MAILBOX;
            }
            else if !vsync && *present_mode == Self::PRESENT_MODE_WITHOUT_VSYNC {
                return vk::PresentModeKHR::IMMEDIATE;
            }
        }

        vk::PresentModeKHR::FIFO
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.vk_swapchain, None);
        }
    }
}

struct SwapchainBuilder {

}

impl SwapchainBuilder {

}
