use std::{
    cell::RefCell,
    rc::Rc
};
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::{
    rendering::{
        RenderingError,
        RenderingResult,
        render_state::RenderStateTrait,
    },
    vulkan::{
        self,
        state::VulkanState,
        logical_device::LogicalDevice,
        surface::Surface,
        swapchain::Swapchain,
        render_pass::RenderPass,
        framebuffers::Framebuffers,
        command_pool::CommandPool,
        command_buffer::{
            CommandBuffer,
            CommandBufferRecorder
        },
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

pub struct Renderer {
    // Vulkan internals
    vulkan_state: Rc<vulkan::state::VulkanState>,
    physical_device: Rc<PhysicalDevice>,
    logical_device: Rc<LogicalDevice>,
    surface: Rc<Surface>,
    swapchain: Rc<Swapchain>,
    render_pass: Rc<RenderPass>,
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

    pub fn new(window: Rc<RefCell<Window>>) -> RenderingResult<Renderer> {
        let vulkan_state = Self::create_vulkan_state(&window)?;
        let surface = Self::create_surface(&vulkan_state, &window)?;
        let physical_device = Self::create_physical_device(&vulkan_state, &surface)?;
        let logical_device = Self::create_logical_device(&vulkan_state, &physical_device)?;
        let swapchain = Self::create_swapchain(&physical_device, &logical_device, &surface)?;
        let render_pass = Self::create_render_pass(&logical_device, &swapchain)?;
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
            framebuffers,
            command_pool,
            command_buffers,
            image_acquired_semaphores,
            image_rendered_semaphores,
            image_rendered_fences,
            current_frame: 0
        })
    }

    fn create_vulkan_state(window: &Rc<RefCell<Window>>) -> RenderingResult<Rc<VulkanState>> {
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
    ) -> RenderingResult<Rc<Surface>> {
        let surface = vulkan::surface::Surface::new(
            Rc::clone(&window),
            Rc::clone(&vulkan_state));

        Ok(Rc::new(surface))
    }

    fn create_physical_device(
        vulkan_state: &Rc<VulkanState>,
        surface: &Rc<Surface>
    ) -> RenderingResult<Rc<PhysicalDevice>> {
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
    ) -> RenderingResult<Rc<LogicalDevice>> {
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
    ) -> RenderingResult<Rc<Swapchain>> {
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
    ) -> RenderingResult<Rc<RenderPass>> {
        let render_pass = vulkan::render_pass::RenderPass::builder()
            .logical_device(Rc::clone(&logical_device))
            .swapchain(Rc::clone(&swapchain))
            .build()?;

        Ok(Rc::new(render_pass))
    }

    fn create_framebuffers(
        logical_device: &Rc<LogicalDevice>,
        swapchain: &Rc<Swapchain>,
        render_pass: &Rc<RenderPass>
    ) -> RenderingResult<Framebuffers> {
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
    ) -> RenderingResult<CommandPool> {
        let command_pool = vulkan::command_pool::CommandPool::builder()
            .physical_device(Rc::clone(&physical_device))
            .logical_device(Rc::clone(&logical_device))
            .queue_family(QueueFamily::Graphics)
            .submit_buffers_once(true)
            .build()?;

        Ok(command_pool)
    }

    pub fn render(&mut self, render_states: &[&dyn RenderStateTrait]) -> RenderingResult<()> {
        self.wait_for_current_frame_to_complete()?;
        let image_index = self.acquire_next_image()?;
        self.rerecord_command_buffer(image_index, render_states)?;
        self.submit_for_rendering()?;
        self.submit_for_presentation(image_index)?;
        self.advance_frame();

        Ok(())
    }

    fn wait_for_current_frame_to_complete(&self) -> RenderingResult<()> {
        self.image_rendered_fences[self.current_frame].wait(
            std::time::Duration::from_nanos(u64::max_value()))?;
        self.image_rendered_fences[self.current_frame].reset()?;

        Ok(())
    }

    fn acquire_next_image(&self) -> RenderingResult<usize> {
        let swapchain_loader = self.logical_device.get_swapchain_loader();
        let image_index = unsafe {
            swapchain_loader.acquire_next_image(
                self.swapchain.handle(),
                u64::max_value(),
                self.image_acquired_semaphores[self.current_frame].handle(),
                vk::Fence::null())
        }.map_err(|result| RenderingError::AcquireImageError {result})?.0;

        Ok(image_index as usize)
    }

    fn rerecord_command_buffer(
        &mut self,
        image_index: usize,
        render_states: &[&dyn RenderStateTrait] 
    ) -> RenderingResult<()> {
        let mut recorder = self.command_buffers[self.current_frame].record()?
            .begin_render_pass(&self.render_pass, &self.framebuffers, image_index);

        for render_state in render_states {
            recorder = Self::record_render_state_to_buffer(*render_state, recorder);
        }

        recorder
            .end_render_pass()
            .end_recording()?;

        Ok(())
    }

    fn record_render_state_to_buffer<'a>(
        render_state: &dyn RenderStateTrait,
        mut recorder: CommandBufferRecorder<'a>
    ) -> CommandBufferRecorder<'a> {
        recorder = recorder
            .bind_pipeline(Rc::clone(render_state.pipeline()));

        for (stage, constants) in render_state.iterate_shaders() {
            recorder = recorder
                .push_constant(render_state.pipeline(), stage, constants);
        }

        recorder.draw(3)
    }

    fn submit_for_rendering(&self) -> RenderingResult<()> {
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
        }.map_err(|result| RenderingError::RenderImageError {result})?;

        Ok(())
    }

    fn submit_for_presentation(&self, image_index: usize) -> RenderingResult<()> {
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
        }.map_err(|result| RenderingError::PresentImageError {result})?;
        Ok(())
    }

    fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % Self::FRAMES_IN_FLIGHT;
    }

    pub fn logical_device(&self) -> &Rc<LogicalDevice> {
        &self.logical_device
    }

    pub fn swapchain(&self) -> &Rc<Swapchain> {
        &self.swapchain
    }

    pub fn render_pass(&self) -> &Rc<RenderPass> {
        &self.render_pass
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let wait_result = unsafe {
            self.logical_device.device_wait_idle()
        };

        wait_result
            .map_err(|result| RenderingError::DeviceWaitIdleError {result})
            .unwrap();
    }
}
