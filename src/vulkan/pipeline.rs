use std::{
    collections::HashMap,
    convert::TryFrom,
    rc::Rc
};
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::{
    builder::{
        BuilderRequirement,
        BuilderInternal,
        BuilderProduct
    },
    vulkan::{
        VulkanError,
        VulkanResult,
        logical_device::LogicalDevice,
        swapchain::Swapchain,
        shader::{
            GeometryShader,
            VertexShader,
            FragmentShader,
            ShaderStageBuilder,
            ShaderStage
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
    pub fn builder<'a>() -> PipelineBuilder<'a> {
        PipelineBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self) -> vk::Pipeline {
        self.vk_pipeline
    }

    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
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
pub struct PipelineBuilder<'a> {
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,
    geometry_shader: Option<&'a GeometryShader>,
    vertex_shader: Option<&'a VertexShader>,
    fragment_shader: Option<&'a FragmentShader>,
    swapchain: BuilderRequirement<Rc<Swapchain>>,
    render_pass: BuilderRequirement<Rc<RenderPass>>,
    subpass: BuilderRequirement<u32>,
    push_constants_sizes: Option<HashMap<ShaderStage, usize>>,
    vertex_binding_description_strides: Vec<usize>,
    vertex_attribute_description_infos: Vec<VertexAttributeDescriptionInfo>,

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

impl<'a> PipelineBuilder<'a> {
    const MAX_SHADER_STAGES: usize = 3;

    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
        self
    }

    pub fn geometry_shader(mut self, geometry_shader: &'a GeometryShader) -> Self {
        self.geometry_shader = Some(geometry_shader);
        self
    }

    pub fn vertex_shader(mut self, vertex_shader: &'a VertexShader) -> Self {
        self.vertex_shader = Some(vertex_shader);
        self
    }

    pub fn fragment_shader(mut self, fragment_shader: &'a FragmentShader) -> Self {
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

    pub fn vertex_binding_stride(mut self, vertex_binding_description_stride: usize) -> Self {
        self.vertex_binding_description_strides.push(vertex_binding_description_stride);
        self
    }

    pub fn vertex_attribute_description(
        mut self,
        format: VertexAttributeFormat,
        offset: usize
    ) -> Self {
        let description_info = VertexAttributeDescriptionInfo {
            binding: self.vertex_binding_description_strides.len(),
            format,
            offset
        };

        self.vertex_attribute_description_infos.push(description_info);
        self
    }

    pub fn push_constants_size(mut self, shader: ShaderStage, size: usize) -> Self {
        match self.push_constants_sizes.as_mut() {
            Some(sizes) => {
                sizes.insert(shader, size);
            },
            None => {
                let mut push_constants_sizes = HashMap::with_capacity(1);
                push_constants_sizes.insert(shader, size);
                self.push_constants_sizes = Some(push_constants_sizes);
            }
        }

        self
    }

    pub fn build(mut self) -> VulkanResult<Pipeline> {
        self.get_ready_for_creation()?;
        self.create_pipeline();

        Ok(self.pipeline.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_vertex_input_state()?;
        self.init_input_assembly_state();
        self.init_viewport_state();
        self.init_rasterization_state();
        self.init_multisample_state();
        self.init_color_blend_state();
        self.init_pipeline_layout()?;
        self.init_vk_pipeline()?;

        Ok(())
    }

    fn init_vertex_input_state(&mut self) -> VulkanResult<()> {
        self.init_vertex_binding_descriptions();
        self.init_vertex_attribute_descriptions()?;
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(self.vertex_binding_descriptions.as_slice())
            .vertex_attribute_descriptions(self.vertex_attribute_descriptions.as_slice())
            .build();

        self.vertex_input_state_create_info.set(vertex_input_state_create_info);
        Ok(())
    }

    fn init_vertex_binding_descriptions(&mut self) {
        let binding_descriptions_count = self.vertex_binding_description_strides.len();
        let mut binding_descriptions = Vec::with_capacity(binding_descriptions_count);
        for (i, size) in self.vertex_binding_description_strides.iter().enumerate() {
            let binding_description = Self::create_vertex_binding_description(i, *size);
            binding_descriptions.push(binding_description);
        }

        self.vertex_binding_descriptions.set(binding_descriptions);
    }

    fn create_vertex_binding_description(
        binding_index: usize,
        stride: usize
    ) -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(binding_index as u32)
            .stride(stride as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn init_vertex_attribute_descriptions(&mut self) -> VulkanResult<()> {
        let attribute_descriptions_count = self.vertex_attribute_description_infos.len();
        let mut attribute_descriptions = Vec::with_capacity(attribute_descriptions_count);
        for (i, info) in self.vertex_attribute_description_infos.iter().enumerate() {
            let vertex_attribute_description =
                Self::create_vertex_attribute_description(i, info)?;
            attribute_descriptions.push(vertex_attribute_description);
        }

        self.vertex_attribute_descriptions.set(attribute_descriptions);
        Ok(())
    }

    fn create_vertex_attribute_description(
        location: usize,
        info: &VertexAttributeDescriptionInfo
    ) -> VulkanResult<vk::VertexInputAttributeDescription> {
        Ok(vk::VertexInputAttributeDescription::builder()
            .binding(info.binding as u32)
            .location(location as u32)
            .format(vk::Format::try_from(info.format)?)
            .offset(info.offset as u32)
            .build())
    }

    fn init_input_assembly_state(&mut self) {
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        self.input_assembly_state_create_info.set(input_assembly_state_create_info);
    }

    fn init_viewport_state(&mut self) {
        let swapchain_extent = self.swapchain.extent();

        let viewport = Self::viewport(swapchain_extent);
        self.viewport.set(viewport);

        let viewport_scissors = Self::viewport_scissors(swapchain_extent);
        self.viewport_scissors.set(viewport_scissors);

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(std::slice::from_ref(&self.viewport))
            .scissors(std::slice::from_ref(&self.viewport_scissors))
            .build();

        self.viewport_state_create_info.set(viewport_state_create_info);
    }

    fn viewport(extent: vk::Extent2D) -> vk::Viewport {
        vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(0.0)
            .build()
    }

    fn viewport_scissors(extent: vk::Extent2D) -> vk::Rect2D {
        let scissors_offset = vk::Offset2D::builder()
            .x(0)
            .y(0)
            .build();

        vk::Rect2D::builder()
            .offset(scissors_offset)
            .extent(extent)
            .build()
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
        let push_constant_ranges = Self::push_constant_ranges(&self.push_constants_sizes);
        let pipeline_layout_create_info_builder = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe {
            self.logical_device.create_pipeline_layout(&pipeline_layout_create_info_builder, None)
        }.map_err(|result| VulkanError::PipelineLayoutCreateError {result})?;

        self.pipeline_layout.set(pipeline_layout);
        Ok(())
    }

    fn push_constant_ranges(push_constants_sizes: &Option<HashMap<ShaderStage, usize>>) -> Vec<vk::PushConstantRange> {
        match push_constants_sizes {
            Some(sizes) => {
                let mut sizes = sizes.clone();
                sizes.retain(|_, size| *size > 0);
                sizes.iter().map(|(shader, size)| {
                    vk::PushConstantRange::builder()
                        .stage_flags((*shader).into())
                        .offset(0)
                        .size(*size as u32)
                        .build()
                }).collect()
            },
            None => Vec::with_capacity(0)
        }
    }

    fn init_vk_pipeline(&mut self) -> VulkanResult<()> {
        let mut stages_create_infos = Vec::with_capacity(Self::MAX_SHADER_STAGES);
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.geometry_shader);
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.vertex_shader);
        Self::push_shader_stage_if_some(&mut stages_create_infos, &self.fragment_shader);

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
        shader: &Option<&T>
    ) {
        if let Some(shader) = shader {
            let shader_stage_create_info = shader
                .shader_stage_create_info_builder()
                .build();

            stages.push(shader_stage_create_info);
        }
    }

    fn create_pipeline(&mut self) {
        let pipeline = Pipeline {
            vk_pipeline: self.vk_pipeline.take(),
            pipeline_layout: self.pipeline_layout.take(),
            logical_device: self.logical_device.take()
        };

        self.pipeline.set(pipeline);
    }
}

struct VertexAttributeDescriptionInfo {
    binding: usize,
    format: VertexAttributeFormat,
    offset: usize
}

#[derive(Clone, Copy)]
pub enum VertexAttributeFormat {
    I32(u8),
    U32(u8),
    F32(u8),
    F64(u8)
}

impl TryFrom<VertexAttributeFormat> for vk::Format {
    type Error = VulkanError;
    fn try_from(value: VertexAttributeFormat) -> Result<vk::Format, Self::Error> {
        match value {
            VertexAttributeFormat::I32(1) => Ok(vk::Format::R32_SINT),
            VertexAttributeFormat::I32(2) => Ok(vk::Format::R32G32_SINT),
            VertexAttributeFormat::I32(3) => Ok(vk::Format::R32G32B32_SINT),
            VertexAttributeFormat::I32(4) => Ok(vk::Format::R32G32B32A32_SINT),
            VertexAttributeFormat::U32(1) => Ok(vk::Format::R32_UINT),
            VertexAttributeFormat::U32(2) => Ok(vk::Format::R32G32_UINT),
            VertexAttributeFormat::U32(3) => Ok(vk::Format::R32G32B32_UINT),
            VertexAttributeFormat::U32(4) => Ok(vk::Format::R32G32B32A32_UINT),
            VertexAttributeFormat::F32(1) => Ok(vk::Format::R32_SFLOAT),
            VertexAttributeFormat::F32(2) => Ok(vk::Format::R32G32_SFLOAT),
            VertexAttributeFormat::F32(3) => Ok(vk::Format::R32G32B32_SFLOAT),
            VertexAttributeFormat::F32(4) => Ok(vk::Format::R32G32B32A32_SFLOAT),
            VertexAttributeFormat::F64(1) => Ok(vk::Format::R32_SFLOAT),
            VertexAttributeFormat::F64(2) => Ok(vk::Format::R32G32_SFLOAT),
            VertexAttributeFormat::F64(3) => Ok(vk::Format::R32G32B32_SFLOAT),
            VertexAttributeFormat::F64(4) => Ok(vk::Format::R32G32B32A32_SFLOAT),
            _ => Err(VulkanError::PipelineCreateVertexAttributeDescriptionError)
        }
    }
}
