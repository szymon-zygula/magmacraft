use std::{
    rc::Rc,
    cell::RefCell
};
use ash::{
    version::DeviceV1_0,
    vk
};
use custom_error::custom_error;
use crate::{
    vulkan::{
        self,
        state::VulkanState,
        logical_device::LogicalDevice,
        surface::Surface,
        swapchain::Swapchain,
        render_pass::RenderPass,
        pipeline::Pipeline,
        framebuffers::Framebuffers,
        command_pool::CommandPool,
        command_buffer::CommandBuffer,
        physical_device::{
            PhysicalDevice,
            PhysicalDeviceExtensions,
            QueueFamily
        },
        synchronization::{
            Semaphore,
            Fence,
            FenceStatus
        }
    },
    window::Window,
    debugging
};

custom_error!{pub RenderError
    VulkanError {source: vulkan::VulkanError} =
        "Vulkan error: {source}",
    AcquireImageError {result: vk::Result} =
        "failed to acquire swapchain image: {result}",
    RenderImageError {result: vk::Result} =
        "failed to submit swapchain image for rendering: {result}",
    PresentImageError {result: vk::Result} =
        "failed to submit swapchain image for presentation: {result}",
    DeviceWaitIdleError {result: vk::Result} =
        "faild to wait for vulkan logical device to become idle: {result}"
}

type RenderResult<T> = Result<T, RenderError>;

pub struct Renderer {
    // Vulkan internals
    vulkan_state: Rc<vulkan::state::VulkanState>,
    physical_device: Rc<PhysicalDevice>,
    logical_device: Rc<LogicalDevice>,
    surface: Rc<Surface>,
    swapchain: Rc<Swapchain>,
    render_pass: Rc<RenderPass>,
    pipeline: Pipeline,
    framebuffers: Framebuffers,
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
    // Vulkan synchronization
    image_acquired_semaphores: Vec<Semaphore>,
    image_rendered_semaphores: Vec<Semaphore>,
    image_rendered_fences: Vec<Fence>,
    current_frame: usize
}

impl Renderer {
    const FRAMES_IN_FLIGHT: usize = 2;

    pub fn new(window: Rc<RefCell<Window>>) -> RenderResult<Renderer> {
        let vulkan_state = Self::create_vulkan_state(&window)?;
        let surface = Self::create_surface(&vulkan_state, &window)?;
        let physical_device = Self::create_physical_device(&vulkan_state, &surface)?;
        let logical_device = Self::create_logical_device(&vulkan_state, &physical_device)?;
        let swapchain = Self::create_swapchain(&physical_device, &logical_device, &surface)?;
        let render_pass = Self::create_render_pass(&logical_device, &swapchain)?;
        let pipeline = Self::create_pipeline(&logical_device, &swapchain, &render_pass)?;
        let framebuffers = Self::create_framebuffers(&logical_device, &swapchain, &render_pass)?;
        let command_pool = Self::create_command_pool(&physical_device, &logical_device)?;
        let command_buffers =
            command_pool.allocate_command_buffers(Self::FRAMES_IN_FLIGHT)?;
        let mut image_acquired_semaphores = Vec::with_capacity(Self::FRAMES_IN_FLIGHT);
        let mut image_rendered_semaphores = Vec::with_capacity(Self::FRAMES_IN_FLIGHT);
        let mut image_rendered_fences = Vec::with_capacity(Self::FRAMES_IN_FLIGHT);

        for _ in 0..Self::FRAMES_IN_FLIGHT {
            image_acquired_semaphores
                .push(Semaphore::new(Rc::clone(&logical_device))?);
            image_rendered_semaphores
                .push(Semaphore::new(Rc::clone(&logical_device))?);
            image_rendered_fences
                .push(Fence::new(Rc::clone(&logical_device), FenceStatus::Ready)?);
        }

        Ok(Renderer {
            vulkan_state,
            physical_device,
            logical_device,
            surface,
            swapchain,
            render_pass,
            pipeline,
            framebuffers,
            command_pool,
            command_buffers,
            image_acquired_semaphores,
            image_rendered_semaphores,
            image_rendered_fences,
            current_frame: 0,
        })
    }

    fn create_vulkan_state(window: &Rc<RefCell<Window>>) -> RenderResult<Rc<VulkanState>> {
        let window = window.borrow();
        let glfw_extensions = window.get_required_vulkan_extensions();
        let vulkan_state = VulkanState::builder()
            .debug_mode(debugging::is_in_debug_mode())
            .instance_extensions(glfw_extensions)
            .build()?;

        Ok(Rc::new(vulkan_state))
    }

    fn create_surface(
        vulkan_state: &Rc<VulkanState>,
        window: &Rc<RefCell<Window>>
    ) -> RenderResult<Rc<Surface>> {
        let surface = vulkan::surface::Surface::new(
            Rc::clone(&window),
            Rc::clone(&vulkan_state));

        Ok(Rc::new(surface))
    }

    fn create_physical_device(
        vulkan_state: &Rc<VulkanState>,
        surface: &Rc<Surface>
    ) -> RenderResult<Rc<PhysicalDevice>> {
        let queue_families = [QueueFamily::Graphics, QueueFamily::Transfer];
        let physical_device_extensions = c_string_collection!(PhysicalDeviceExtensions:
            [ash::extensions::khr::Swapchain::name().to_str().unwrap()]);

        let physical_device = vulkan::physical_device::PhysicalDevice::selector()
            .vulkan_state(Rc::clone(&vulkan_state))
            .queue_families(&queue_families)
            .surface_compatible(Rc::clone(&surface))
            .device_extensions(physical_device_extensions)
            .select()?;

        Ok(Rc::new(physical_device))
    }

    fn create_logical_device(
        vulkan_state: &Rc<VulkanState>,
        physical_device: &Rc<PhysicalDevice>,
    ) -> RenderResult<Rc<LogicalDevice>> {
        let queue_families = [
            QueueFamily::Graphics,
            QueueFamily::Transfer,
            QueueFamily::Presentation
        ];

        let logical_device = vulkan::logical_device::LogicalDevice::builder()
            .vulkan_state(Rc::clone(&vulkan_state))
            .physical_device(Rc::clone(&physical_device))
            .queue_families(&queue_families)
            .build()?;

        Ok(Rc::new(logical_device))
    }

    fn create_swapchain(
        physical_device: &Rc<PhysicalDevice>,
        logical_device: &Rc<LogicalDevice>,
        surface: &Rc<Surface>
    ) -> RenderResult<Rc<Swapchain>> {
        let swapchain = vulkan::swapchain::Swapchain::builder()
            .physical_device(Rc::clone(&physical_device))
            .logical_device(Rc::clone(&logical_device))
            .surface(Rc::clone(&surface))
            .vsync(false)
            .build()?;

        Ok(Rc::new(swapchain))
    }

    fn create_render_pass(
        logical_device: &Rc<LogicalDevice>,
        swapchain: &Rc<Swapchain>
    ) -> RenderResult<Rc<RenderPass>> {
        let render_pass = vulkan::render_pass::RenderPass::builder()
            .logical_device(Rc::clone(&logical_device))
            .swapchain(Rc::clone(&swapchain))
            .build()?;

        Ok(Rc::new(render_pass))
    }

    fn create_pipeline(
        logical_device: &Rc<LogicalDevice>,
        swapchain: &Rc<Swapchain>,
        render_pass: &Rc<RenderPass>
    ) -> RenderResult<Pipeline> {
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

        Ok(pipeline)
    }

    fn create_framebuffers(
        logical_device: &Rc<LogicalDevice>,
        swapchain: &Rc<Swapchain>,
        render_pass: &Rc<RenderPass>
    ) -> RenderResult<Framebuffers> {
        let framebuffers = vulkan::framebuffers::Framebuffers::builder()
            .logical_device(Rc::clone(&logical_device))
            .swapchain(Rc::clone(&swapchain))
            .render_pass(Rc::clone(&render_pass))
            .build()?;

        Ok(framebuffers)
    }

    fn create_command_pool(
        physical_device: &Rc<PhysicalDevice>,
        logical_device: &Rc<LogicalDevice>
    ) -> RenderResult<CommandPool> {
        let command_pool = vulkan::command_pool::CommandPool::builder()
            .physical_device(Rc::clone(&physical_device))
            .logical_device(Rc::clone(&logical_device))
            .queue_family(QueueFamily::Graphics)
            .submit_buffers_once(true)
            .build()?;

        Ok(command_pool)
    }

    pub fn render(&mut self) -> RenderResult<()> {
        self.wait_for_current_frame_to_complete()?;
        let image_index = self.acquire_next_image()?;
        self.rerecord_command_buffer(image_index)?;
        self.submit_for_rendering()?;
        self.submit_for_presentation(image_index)?;
        self.advance_frame();

        Ok(())
    }

    fn wait_for_current_frame_to_complete(&self) -> RenderResult<()> {
        self.image_rendered_fences[self.current_frame].wait(
            std::time::Duration::from_nanos(u64::max_value()))?;
        self.image_rendered_fences[self.current_frame].reset()?;

        Ok(())
    }

    fn acquire_next_image(&self) -> RenderResult<usize> {
        let swapchain_loader = self.logical_device.get_swapchain_loader();
        let image_index = unsafe {
            swapchain_loader.acquire_next_image(
                self.swapchain.handle(),
                u64::max_value(),
                self.image_acquired_semaphores[self.current_frame].handle(),
                vk::Fence::null())
        }.map_err(|result| RenderError::AcquireImageError {result})?.0;

        Ok(image_index as usize)
    }

    fn rerecord_command_buffer(&self, image_index: usize) -> RenderResult<()> {
        self.command_buffers[self.current_frame].record()?
            .begin_render_pass(&self.render_pass, &self.framebuffers, image_index)
            .bind_pipeline(&self.pipeline)
            .draw(3)
            .end_render_pass()
            .end_recording()?;

        Ok(())
    }

    fn submit_for_rendering(&self) -> RenderResult<()> {
        let graphics_queue = self.logical_device.device_queue(QueueFamily::Graphics)?;
        let wait_semaphores = [self.image_acquired_semaphores[self.current_frame].handle()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [self.command_buffers[self.current_frame].handle()];
        let signal_semaphores = [self.image_rendered_semaphores[self.current_frame].handle()];
        let submit_infos = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores)
            .build()];

        unsafe {
            self.logical_device.queue_submit(
                graphics_queue,
                &submit_infos,
                self.image_rendered_fences[self.current_frame].handle())
        }.map_err(|result| RenderError::RenderImageError {result})?;
        Ok(())
    }

    fn submit_for_presentation(&self, image_index: usize) -> RenderResult<()> {
        let presentation_queue = self.logical_device.device_queue(QueueFamily::Presentation)?;
        let wait_semaphores = [self.image_rendered_semaphores[self.current_frame].handle()];
        let swapchains = [self.swapchain.handle()];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .build();

        let swapchain_loader = self.logical_device.get_swapchain_loader();

        unsafe {
            swapchain_loader.queue_present(presentation_queue, &present_info)
        }.map_err(|result| RenderError::PresentImageError {result})?;
        Ok(())
    }

    fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % Self::FRAMES_IN_FLIGHT;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let wait_result = unsafe {
            self.logical_device.device_wait_idle()
        };

        wait_result
            .map_err(|result| RenderError::DeviceWaitIdleError {result})
            .unwrap();
    }
}
