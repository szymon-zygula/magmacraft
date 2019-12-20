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

        let logical_device = Rc::new(vulkan::logical_device::LogicalDevice::builder()
            .vulkan_state(Rc::clone(&vulkan_state))
            .physical_device(Rc::clone(&physical_device))
            .queue_families(&queue_families)
            .build()?);

        let swapchain = Rc::new(vulkan::swapchain::Swapchain::builder()
            .physical_device(Rc::clone(&physical_device))
            .logical_device(Rc::clone(&logical_device))
            .surface(Rc::clone(&surface))
            .vsync(false)
            .build()?);

        let render_pass = Rc::new(vulkan::render_pass::RenderPass::builder()
            .logical_device(Rc::clone(&logical_device))
            .swapchain(Rc::clone(&swapchain))
            .build()?);

        let vertex_shader = Rc::new(vulkan::shader::VertexShader::from_file(
                Rc::clone(&logical_device), std::path::Path::new("shaders/triangle.vert.spv"))?);
        let fragment_shader = Rc::new(vulkan::shader::FragmentShader::from_file(
                Rc::clone(&logical_device), std::path::Path::new("shaders/triangle.frag.spv"))?);

        let pipeline = vulkan::pipeline::Pipeline::builder()
            .vertex_shader(Rc::clone(&vertex_shader))
            .fragment_shader(Rc::clone(&fragment_shader))
            .logical_device(Rc::clone(&logical_device))
            .swapchain(Rc::clone(&swapchain))
            .render_pass(Rc::clone(&render_pass))
            .subpass(0)
            .build()?;

        let framebuffers = vulkan::framebuffers::Framebuffers::builder()
            .logical_device(Rc::clone(&logical_device))
            .render_pass(Rc::clone(&render_pass))
            .swapchain(Rc::clone(&swapchain))
            .build()?;

        let command_pool = vulkan::command_pool::CommandPool::builder()
            .physical_device(Rc::clone(&physical_device))
            .logical_device(Rc::clone(&logical_device))
            .queue_family(QueueFamily::Graphics)
            .submit_buffers_once(true)
            .build()?;

        let command_buffers =
            command_pool.allocate_command_buffers(swapchain.image_count())?;

        for (i, command_buffer) in command_buffers.iter().enumerate() {
            command_buffer.record()?
                .begin_render_pass(&render_pass, &framebuffers, i)
                .bind_pipeline(&pipeline)
                .draw(3)
                .end_render_pass()
                .end_recording()?;
        }

        Ok(Renderer {
            vulkan_state
        })
    }
}
