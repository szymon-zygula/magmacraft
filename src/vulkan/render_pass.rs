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
        swapchain::Swapchain
    }
};

pub struct RenderPass {
    vk_render_pass: vk::RenderPass,
    logical_device: Rc<LogicalDevice>
}

impl RenderPass {
    pub fn builder() -> RenderPassBuilder {
        RenderPassBuilder {
            ..Default::default()
        }
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_render_pass(self.vk_render_pass, None);
        }
    }
}

#[derive(Default)]
pub struct RenderPassBuilder {
    swapchain: BuilderRequirement<Rc<Swapchain>>,
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,

    attachment_descriptions: BuilderInternal<Vec<vk::AttachmentDescription>>,
    attachment_references: BuilderInternal<Vec<vk::AttachmentReference>>,
    subpass_descriptions: BuilderInternal<Vec<vk::SubpassDescription>>,
    subpass_dependencies: BuilderInternal<Vec<vk::SubpassDependency>>,

    vk_render_pass: BuilderInternal<vk::RenderPass>,

    render_pass: BuilderProduct<RenderPass>
}

impl RenderPassBuilder {
    pub fn swapchain(mut self, swapchain: Rc<Swapchain>) -> Self {
        self.swapchain.set(swapchain);
        self
    }

    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
        self
    }

    pub fn build(mut self) -> VulkanResult<RenderPass> {
        self.get_ready_for_creation()?;
        self.create_render_pass();

        Ok(self.render_pass.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_attachment_descriptions();
        self.init_attachment_references();
        self.init_subpass_descriptions();
        self.init_subpass_dependencies();
        self.init_vk_render_pass()?;

        Ok(())
    }

    fn init_attachment_descriptions(&mut self) {
        let attachment_description = vk::AttachmentDescription::builder()
            .format(self.swapchain.image_format())
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let attachment_descriptions = vec![attachment_description];
        self.attachment_descriptions.set(attachment_descriptions);
    }

    fn init_attachment_references(&mut self) {
        let attachment_reference = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let attachment_references = vec![attachment_reference];
        self.attachment_references.set(attachment_references);
    }

    fn init_subpass_descriptions(&mut self) {
        let subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&self.attachment_references)
            .build();

        let subpass_descriptions = vec![subpass_description];
        self.subpass_descriptions.set(subpass_descriptions);
    }

    fn init_subpass_dependencies(&mut self) {
        let subpass_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let subpass_dependencies = vec![subpass_dependency];
        self.subpass_dependencies.set(subpass_dependencies);
    }

    fn init_vk_render_pass(&mut self) -> VulkanResult<()> {
        let render_pass_create_info_builder = vk::RenderPassCreateInfo::builder()
            .attachments(&self.attachment_descriptions)
            .subpasses(&self.subpass_descriptions)
            .dependencies(&self.subpass_dependencies);

        let vk_render_pass = unsafe {
            self.logical_device.create_render_pass(&render_pass_create_info_builder, None)
                .map_err(|result| VulkanError::RenderPassCreateError {result})?
        };

        self.vk_render_pass.set(vk_render_pass);

        Ok(())
    }

    fn create_render_pass(&mut self) {
        self.render_pass.set(RenderPass {
            vk_render_pass: self.vk_render_pass.take(),
            logical_device: Rc::clone(&self.logical_device)
        });
    }
}
