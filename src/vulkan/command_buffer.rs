use std::rc::Rc;
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::vulkan::{
    VulkanError,
    VulkanResult,
    logical_device::LogicalDevice,
    render_pass::RenderPass,
    framebuffers::Framebuffers,
    pipeline::Pipeline
};


pub struct CommandBuffer {
    vk_command_buffer: vk::CommandBuffer,
    logical_device: Rc<LogicalDevice>,
    submit_once: bool
}

impl CommandBuffer {
    pub fn from_handle(
        vk_command_buffer: vk::CommandBuffer,
        logical_device: Rc<LogicalDevice>,
        submit_once: bool
    ) -> CommandBuffer {
        Self {
            vk_command_buffer,
            logical_device,
            submit_once
        }
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.vk_command_buffer
    }

    pub fn record(&self) -> VulkanResult<CommandBufferRecorder> {
        CommandBufferRecorder::new(&self, self.submit_once)
    }
}

pub struct CommandBufferRecorder<'a> {
    command_buffer: &'a CommandBuffer,
    recording: bool
}

impl<'a> CommandBufferRecorder<'a> {
    fn new(command_buffer: &'a CommandBuffer, submit_once: bool) -> VulkanResult<Self> {
        let flags = Self::begin_info_flags(submit_once);
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(flags);

        unsafe {
            command_buffer.logical_device
                .begin_command_buffer(command_buffer.handle(), &begin_info)
        }.map_err(|result| VulkanError::CommandBufferRecordError {result})?;

        Ok(CommandBufferRecorder {
            command_buffer,
            recording: true
        })
    }

    fn begin_info_flags(submit_once: bool) -> vk::CommandBufferUsageFlags {
        if submit_once {
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
        }
        else {
            vk::CommandBufferUsageFlags::empty()
        }
    }

    pub fn begin_render_pass(
        self,
        render_pass: &RenderPass,
        framebuffers: &Framebuffers,
        framebuffer_index: usize
    ) -> Self {
        let render_area = Self::render_area(framebuffers);
        let render_clear_values = Self::render_clear_values();

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle())
            .framebuffer(framebuffers.handle(framebuffer_index))
            .render_area(render_area)
            .clear_values(&render_clear_values);

        unsafe {
            self.command_buffer.logical_device
                .cmd_begin_render_pass(
                    self.command_buffer.handle(),
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE);
        }

        self
    }

    pub fn render_area(framebuffers: &Framebuffers) -> vk::Rect2D {
        let render_area_extent = framebuffers.image_extent();
        let render_area_offset = vk::Offset2D::builder()
            .x(0)
            .y(0)
            .build();

        vk::Rect2D::builder()
            .extent(render_area_extent)
            .offset(render_area_offset)
            .build()
    }

    pub fn render_clear_values() -> [vk::ClearValue; 1] {
        [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0]
            }
        }]
    }

    pub fn end_render_pass(self) -> Self {
        unsafe {
            self.command_buffer.logical_device
                .cmd_end_render_pass(self.command_buffer.handle());
        }

        self
    }

    pub fn bind_pipeline(self, pipeline: &Pipeline) -> Self {
        unsafe {
            self.command_buffer.logical_device
                .cmd_bind_pipeline(
                    self.command_buffer.handle(),
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.handle());
        }

        self
    }

    pub fn draw(self, vertex_count: u32) -> Self {
        unsafe {
            self.command_buffer.logical_device
                .cmd_draw(self.command_buffer.handle(), vertex_count, 1, 0, 0);
        };

        self
    }

    pub fn end_recording(mut self) -> VulkanResult<()> {
        unsafe {
            self.command_buffer.logical_device
                .end_command_buffer(self.command_buffer.handle())
        }.map_err(|result| VulkanError::CommandBufferRecordError {result})?;

        self.recording = false;
        Ok(())
    }
}

impl Drop for CommandBufferRecorder<'_> {
    fn drop(&mut self) {
        if self.recording {
            panic!("Error: vulkan command buffer recorder went out of scope while recording");
        }
    }
}
