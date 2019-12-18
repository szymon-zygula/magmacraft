use std::rc::Rc;
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::{
    builder::*,
    vulkan::{
        VulkanError,
        VulkanResult,
        logical_device::LogicalDevice,
        swapchain::Swapchain,
        shader::{
            GeometryShader,
            VertexShader,
            FragmentShader,
            ShaderStageBuilder
        },
        render_pass::RenderPass
    }
};

pub struct Pipeline {
    vk_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    logical_device: Rc<LogicalDevice>
}

impl Pipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder {
            ..Default::default()
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_pipeline(self.vk_pipeline, None);
            self.logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

#[derive(Default)]
pub struct PipelineBuilder {
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,
    geometry_shader: Option<Rc<GeometryShader>>,
    vertex_shader: Option<Rc<VertexShader>>,
    fragment_shader: Option<Rc<FragmentShader>>,
    swapchain: BuilderRequirement<Rc<Swapchain>>,
    render_pass: BuilderRequirement<Rc<RenderPass>>,
    subpass: BuilderRequirement<u32>,

    vertex_binding_descriptions: BuilderInternal<Vec<vk::VertexInputBindingDescription>>,
    vertex_attribute_descriptions: BuilderInternal<Vec<vk::VertexInputAttributeDescription>>,
    vertex_input_state_create_info: BuilderInternal<vk::PipelineVertexInputStateCreateInfo>,

    input_assembly_state_create_info: BuilderInternal<vk::PipelineInputAssemblyStateCreateInfo>,

    viewport: BuilderInternal<vk::Viewport>,
    viewport_scissors: BuilderInternal<vk::Rect2D>,
    viewport_state_create_info: BuilderInternal<vk::PipelineViewportStateCreateInfo>,

    rasterization_state_create_info: BuilderInternal<vk::PipelineRasterizationStateCreateInfo>,

    multisample_state_create_info: BuilderInternal<vk::PipelineMultisampleStateCreateInfo>,

    color_blend_attachment_state: BuilderInternal<vk::PipelineColorBlendAttachmentState>,
    color_blend_state_create_info: BuilderInternal<vk::PipelineColorBlendStateCreateInfo>,

    pipeline_layout: BuilderInternal<vk::PipelineLayout>,

    vk_pipeline: BuilderInternal<vk::Pipeline>,

    pipeline: BuilderProduct<Pipeline>
}

impl PipelineBuilder {
    const MAX_SHADER_STAGES: usize = 3;

    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
        self
    }

    pub fn geometry_shader(mut self, geometry_shader: Rc<GeometryShader>) -> Self {
        self.geometry_shader = Some(geometry_shader);
        self
    }

    pub fn vertex_shader(mut self, vertex_shader: Rc<VertexShader>) -> Self {
        self.vertex_shader = Some(vertex_shader);
        self
    }

    pub fn fragment_shader(mut self, fragment_shader: Rc<FragmentShader>) -> Self {
        self.fragment_shader = Some(fragment_shader);
        self
    }

    pub fn swapchain(mut self, swapchain: Rc<Swapchain>) -> Self {
        self.swapchain.set(swapchain);
        self
    }

    pub fn render_pass(mut self, render_pass: Rc<RenderPass>) -> Self {
        self.render_pass.set(render_pass);
        self
    }

    pub fn subpass(mut self, subpass: u32) -> Self {
        self.subpass.set(subpass);
        self
    }

    pub fn build(mut self) -> VulkanResult<Pipeline> {
        self.get_ready_for_creation()?;
        self.create_pipeline();

        Ok(self.pipeline.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_vertex_input_state();
        self.init_input_assembly_state();
        self.init_viewport_state();
        self.init_rasterization_state();
        self.init_multisample_state();
        self.init_color_blend_state();
        self.init_pipeline_layout()?;
        self.init_vk_pipeline()?;

        Ok(())
    }

    fn init_vertex_input_state(&mut self) {
        self.vertex_binding_descriptions.set(Vec::new());
        self.vertex_attribute_descriptions.set(Vec::new());
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(self.vertex_binding_descriptions.as_slice())
            .vertex_attribute_descriptions(self.vertex_attribute_descriptions.as_slice())
            .build();

        self.vertex_input_state_create_info.set(vertex_input_state_create_info);
    }

    fn init_input_assembly_state(&mut self) {
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        self.input_assembly_state_create_info.set(input_assembly_state_create_info);
    }

    fn init_viewport_state(&mut self) {
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(self.swapchain.extent().width as f32)
            .height(self.swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(0.0)
            .build();

        self.viewport.set(viewport);

        let scissors_offset = vk::Offset2D::builder()
            .x(0)
            .y(0)
            .build();

        let viewport_scissors = vk::Rect2D::builder()
            .offset(scissors_offset)
            .extent(self.swapchain.extent())
            .build();

        self.viewport_scissors.set(viewport_scissors);

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(std::slice::from_ref(&self.viewport))
            .scissors(std::slice::from_ref(&self.viewport_scissors))
            .build();

        self.viewport_state_create_info.set(viewport_state_create_info);
    }

    fn init_rasterization_state(&mut self) {
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .build();

        self.rasterization_state_create_info.set(rasterization_state_create_info);
    }

    fn init_multisample_state(&mut self) {
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        self.multisample_state_create_info.set(multisample_state_create_info);
    }

    fn init_color_blend_state(&mut self) {
        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(
                vk::ColorComponentFlags::R |
                vk::ColorComponentFlags::G |
                vk::ColorComponentFlags::B |
                vk::ColorComponentFlags::A)
            .blend_enable(false)
            .build();

        self.color_blend_attachment_state.set(color_blend_attachment_state);

        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(std::slice::from_ref(&self.color_blend_attachment_state))
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        self.color_blend_state_create_info.set(color_blend_state_create_info);
    }

    fn init_pipeline_layout(&mut self) -> VulkanResult<()> {
        let pipeline_layout_create_info_builder = vk::PipelineLayoutCreateInfo::builder();

        let pipeline_layout = unsafe {
            self.logical_device.create_pipeline_layout(&pipeline_layout_create_info_builder, None)
        }.map_err(|result| VulkanError::PipelineLayoutCreateError {result})?;

        self.pipeline_layout.set(pipeline_layout);
        Ok(())
    }

    fn init_vk_pipeline(&mut self) -> VulkanResult<()> {
        let mut stages_create_infos = Vec::with_capacity(Self::MAX_SHADER_STAGES);
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.geometry_shader.as_ref());
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.vertex_shader.as_ref());
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.fragment_shader.as_ref());

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages_create_infos.as_slice())
            .vertex_input_state(&self.vertex_input_state_create_info)
            .input_assembly_state(&self.input_assembly_state_create_info)
            .viewport_state(&self.viewport_state_create_info)
            .rasterization_state(&self.rasterization_state_create_info)
            .multisample_state(&self.multisample_state_create_info)
            .color_blend_state(&self.color_blend_state_create_info)
            .layout(*self.pipeline_layout)
            .render_pass(self.render_pass.handle())
            .subpass(*self.subpass)
            .build();

        let vk_pipeline = unsafe {
            self.logical_device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_create_info],
                None)
        }.map_err(|err| VulkanError::PipelineCreateError {result: err.1})?;

        self.vk_pipeline.set(vk_pipeline[0]);
        Ok(())
    }

    fn push_shader_stage_if_some<T: ShaderStageBuilder>(
        stages: &mut Vec<vk::PipelineShaderStageCreateInfo>,
        shader: &Option<&Rc<T>>
    ) {
        if let Some(shader) = shader {
            let shader_stage_create_info =
                shader.shader_stage_create_info_builder().build();

            stages.push(shader_stage_create_info);
        }
    }

    fn create_pipeline(&mut self) {
        let pipeline = Pipeline {
            vk_pipeline: self.vk_pipeline.take(),
            pipeline_layout: self.pipeline_layout.take(),
            logical_device: Rc::clone(&self.logical_device)
        };

        self.pipeline.set(pipeline);
    }
}
