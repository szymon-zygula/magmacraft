use std::rc::Rc;
use std::cell::RefCell;
use custom_error::custom_error;
use crate::{
    vulkan::{
        self,
        state::VulkanState,
        physical_device::{
            PhysicalDeviceExtensions,
            QueueFamily
        }
    },
    window::Window,
    debugging
};

custom_error!{pub RendererError
    VulkanError {source: vulkan::VulkanError} = "Vulkan error: {source}"
}

pub struct Renderer {
    vulkan_state: Rc<vulkan::state::VulkanState>,
}

impl Renderer {
    pub fn new(window: Rc<RefCell<Window>>) -> Result<Renderer, RendererError> {
        let glfw_extensions = window.borrow().get_required_vulkan_extensions();

        let vulkan_state = Rc::new(VulkanState::builder()
            .debug_mode(debugging::is_in_debug_mode())
            .instance_extensions(glfw_extensions)
            .build()?);

        let surface = Rc::new(vulkan::surface::Surface::new(Rc::clone(&window), Rc::clone(&vulkan_state)));

        let mut physical_device_extensions = PhysicalDeviceExtensions::new();
        physical_device_extensions.push(
            ash::extensions::khr::Swapchain::name().to_str().unwrap()
        );
        let queue_families = vec![QueueFamily::Graphics, QueueFamily::Transfer];
        let physical_device = Rc::new(vulkan::physical_device::PhysicalDevice::selector()
            .vulkan_state(Rc::clone(&vulkan_state))
            .queue_families(&queue_families)
            .surface_compatible(Rc::clone(&surface))
            .device_extensions(physical_device_extensions)
            .select()?);

        let _logical_device = vulkan::logical_device::LogicalDevice::builder()
            .vulkan_state(Rc::clone(&vulkan_state))
            .physical_device(Rc::clone(&physical_device))
            .queue_families(&queue_families)
            .build();

        Ok(Renderer {
            vulkan_state
        })
    }
}
