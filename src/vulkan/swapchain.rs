use crate::vulkan;

pub struct Swapchain {

}

impl Swapchain {
    pub fn new(physical_device: &vulkan::physical_device::PhysicalDevice, surface: &vulkan::surface::Surface) -> Self {
        let surface_properties = physical_device.get_surface_properties(surface);

        Self {}
    }
}
