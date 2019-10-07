use custom_error::custom_error;
use crate::{
    vulkan::{
        self,
        state::VulkanState
    },
    window::Window,
    debugging
};

custom_error!{pub RendererError
    VulkanError {source: vulkan::VulkanError} = "Vulkan error: {source}"
}

pub struct Renderer {
    vulkan_state: vulkan::state::VulkanState,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Renderer, RendererError> {
        let glfw_extensions = window.get_required_vulkan_extensions();

        let vulkan_state = VulkanState::builder()
            .debug_mode(debugging::is_in_debug_mode())
            .instance_extensions(glfw_extensions)
            .build()?;

        let surface = vulkan::surface::Surface::new(&window, &vulkan_state);

        Ok(Renderer {
            vulkan_state
        })
    }
}
