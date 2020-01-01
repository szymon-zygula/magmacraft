use std::rc::Rc;
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::{
    builder::*,
    vulkan::{
        VulkanResult,
        VulkanError,
        logical_device::LogicalDevice,
        swapchain::Swapchain,
        render_pass::RenderPass
    }
};

pub struct Framebuffers {
    vk_framebuffers: Vec<vk::Framebuffer>,
    logical_device: Rc<LogicalDevice>,
    swapchain: Rc<Swapchain>
}

impl Framebuffers {
    pub fn builder() -> FramebuffersBuilder {
        FramebuffersBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self, index: usize) -> vk::Framebuffer {
        self.vk_framebuffers[index]
    }

    pub fn image_extent(&self) -> vk::Extent2D {
        self.swapchain.extent()
    }
}

impl Drop for Framebuffers {
    fn drop(&mut self) {
        unsafe {
            for framebuffer in self.vk_framebuffers.as_slice() {
                self.logical_device.destroy_framebuffer(*framebuffer, None);
            }
        }
    }
}

#[derive(Default)]
pub struct FramebuffersBuilder {
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,
    swapchain: BuilderRequirement<Rc<Swapchain>>,
    render_pass: BuilderRequirement<Rc<RenderPass>>,

    vk_framebuffers: BuilderInternal<Vec<vk::Framebuffer>>,

    framebuffers: BuilderProduct<Framebuffers>
}

impl FramebuffersBuilder {
    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
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

    pub fn build(mut self) -> VulkanResult<Framebuffers> {
        self.init_vk_framebuffers()?;
        self.create_framebuffers();

        Ok(self.framebuffers.unwrap())
    }

    fn init_vk_framebuffers(&mut self) -> VulkanResult<()> {
        let image_views = self.swapchain.image_views();
        let extent = self.swapchain.extent();
        let mut vk_framebuffers = Vec::with_capacity(image_views.len());

        for image_view in image_views {
            self.push_framebuffer_with_image_to_vec(
                *image_view, &extent, &mut vk_framebuffers)?;
        }

        self.vk_framebuffers.set(vk_framebuffers);
        Ok(())
    }

    fn push_framebuffer_with_image_to_vec(
        &self,
        image_view: vk::ImageView,
        extent: &vk::Extent2D,
        vk_framebuffers: &mut Vec<vk::Framebuffer>
    ) -> VulkanResult<()> {
        let attachments = [image_view];

        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(self.render_pass.handle())
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let vk_framebuffer = unsafe {
            self.logical_device.create_framebuffer(&framebuffer_create_info, None)
        }.map_err(|result| VulkanError::FramebuffersCreateError {result})?;

        vk_framebuffers.push(vk_framebuffer);
        Ok(())
    }

    fn create_framebuffers(&mut self) {
        let framebuffers = Framebuffers {
            vk_framebuffers: self.vk_framebuffers.take(),
            logical_device: self.logical_device.take(),
            swapchain: self.swapchain.take(),
        };

        self.framebuffers.set(framebuffers);
    }
}
