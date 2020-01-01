use std::rc::Rc;
use ash::{
    self,
    version::DeviceV1_0,
    vk
};
use crate::{
    builder::*,
    vulkan::{
        VulkanError,
        VulkanResult,
        physical_device::{
            PhysicalDevice,
            PhysicalDeviceSurfaceProperties,
            QueueFamily
        },
        logical_device::LogicalDevice,
        surface::Surface
    }
};

pub struct Swapchain {
    vk_swapchain: vk::SwapchainKHR,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    swapchain_loader: Rc<ash::extensions::khr::Swapchain>,
    logical_device: Rc<LogicalDevice>,
    // lifetime extenders
    _surface: Rc<Surface>
}

impl Swapchain {
    pub fn builder() -> SwapchainBuilder {
        SwapchainBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self) -> vk::SwapchainKHR {
        self.vk_swapchain
    }

    pub fn image_format(&self) -> vk::Format {
        self.surface_format.format
    }

    pub fn image_color_space(&self) -> vk::ColorSpaceKHR {
        self.surface_format.color_space
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn image_views(&self) -> &Vec<vk::ImageView> {
        &self.image_views
    }

    pub fn image_count(&self) -> usize {
        self.image_views.len()
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.vk_swapchain, None);
        }

        for image_view in self.image_views.as_slice() {
            unsafe {
                self.logical_device.destroy_image_view(*image_view, None);
            }
        }
    }
}

#[derive(Default)]
pub struct SwapchainBuilder {
    physical_device: BuilderRequirement<Rc<PhysicalDevice>>,
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,
    surface: BuilderRequirement<Rc<Surface>>,
    vsync: BuilderRequirement<bool>,

    surface_properties: BuilderInternal<PhysicalDeviceSurfaceProperties>,
    image_extent: BuilderInternal<vk::Extent2D>,
    surface_format: BuilderInternal<vk::SurfaceFormatKHR>,
    present_mode: BuilderInternal<vk::PresentModeKHR>,
    optimal_image_count: BuilderInternal<u32>,
    image_sharing_mode: BuilderInternal<vk::SharingMode>,
    concurrent_queue_families: BuilderInternal<Vec<u32>>,
    swapchain_create_info: BuilderInternal<vk::SwapchainCreateInfoKHR>,
    vk_swapchain: BuilderInternal<vk::SwapchainKHR>,
    images: BuilderInternal<Vec<vk::Image>>,
    image_views: BuilderInternal<Vec<vk::ImageView>>,

    swapchain: BuilderProduct<Swapchain>
}

impl SwapchainBuilder {
    const IMAGE_ARRAY_LAYERS: u32 = 1;
    const ADDITIONAL_IMAGES_COUNT: u32 = 1;

    const PRESENT_MODE_WITH_VSYNC: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;
    const PRESENT_MODE_WITHOUT_VSYNC: vk::PresentModeKHR = vk::PresentModeKHR::IMMEDIATE;

    pub fn physical_device(mut self, physical_device: Rc<PhysicalDevice>) -> Self {
        self.physical_device.set(physical_device);
        self
    }

    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
        self
    }

    pub fn surface(mut self, surface: Rc<Surface>) -> Self {
        self.surface.set(surface);
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync.set(vsync);
        self
    }

    pub fn build(mut self) -> VulkanResult<Swapchain> {
        self.get_ready_for_creation()?;
        self.create_swapchain();

        Ok(self.swapchain.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_surface_properties()?;
        self.init_surface_format();
        self.init_image_extent();
        self.init_present_mode();
        self.init_optimal_image_count();
        self.init_image_sharing_info()?;
        self.init_swapchain_create_info();
        self.init_vk_swapchain()?;
        self.init_images()?;
        self.init_image_views()?;

        Ok(())
    }

    fn init_surface_properties(&mut self) -> VulkanResult<()> {
        let surface_properties = self.physical_device.get_surface_properties(&self.surface)?;
        self.surface_properties.set(surface_properties);

        Ok(())
    }

    fn init_surface_format(&mut self) {
        let surface_format = self.surface_properties.formats[0];
        // TODO: select this based on gamma and other things
        self.surface_format.set(surface_format);
    }

    fn init_image_extent(&mut self) {
        let capabilities = self.surface_properties.capabilities;
        let current_extent = capabilities.current_extent;

        // TODO: support custom resolutions
        let image_extent = if Self::is_extent_undefined(&current_extent) {
            self.surface.get_framebuffer_extent()
        }
        else {
            current_extent
        };

        self.image_extent.set(image_extent);
    }

    fn is_extent_undefined(extent: &vk::Extent2D) -> bool {
        extent.width == u32::max_value()
    }

    fn init_present_mode(&mut self) {
        for present_mode in &self.surface_properties.present_modes {
            if self.is_present_mode_suitable(*present_mode) {
                self.present_mode.set(*present_mode);
                return;
            }
        }

        self.present_mode.set(vk::PresentModeKHR::FIFO);
    }

    fn is_present_mode_suitable(&self, present_mode: vk::PresentModeKHR) -> bool {
        *self.vsync && present_mode == Self::PRESENT_MODE_WITH_VSYNC ||
        !*self.vsync && present_mode == Self::PRESENT_MODE_WITHOUT_VSYNC
    }

    fn init_optimal_image_count(&mut self) {
        let min_image_count = self.surface_properties.capabilities.min_image_count;
        let max_image_count = self.surface_properties.capabilities.max_image_count;
        let mut optimal_image_count = min_image_count + Self::ADDITIONAL_IMAGES_COUNT;

        if max_image_count != 0 && optimal_image_count > max_image_count {
            optimal_image_count = max_image_count;
        };

        self.optimal_image_count.set(optimal_image_count);
    }

    fn init_image_sharing_info(&mut self) -> VulkanResult<()> {
        let multiple_queue_family_usage = self.physical_device.is_transfer_queue_family_dedicated();
        let graphics_index = self.physical_device.get_queue_family_index(QueueFamily::Graphics)?;
        let transfer_index = self.physical_device.get_queue_family_index(QueueFamily::Transfer)?;

        let (image_sharing_mode, concurrent_queue_families) = 
            if multiple_queue_family_usage {
                (vk::SharingMode::CONCURRENT, vec![graphics_index, transfer_index])
            }
            else {
                (vk::SharingMode::EXCLUSIVE, vec![])
            };

        self.image_sharing_mode.set(image_sharing_mode);
        self.concurrent_queue_families.set(concurrent_queue_families);

        Ok(())
    }

    fn init_swapchain_create_info(&mut self) {
        let surface_format = *self.surface_format;
        // Dereferencing `swapchain_create_info` gets rid of lifetime information,
        // but it depends on memory owned by `self.concurrent_queue_families` after return,
        // so it cannot be taken.

        let swapchain_create_info_builder = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface.get_handle())
            .min_image_count(*self.optimal_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(*self.image_extent)
            .image_array_layers(Self::IMAGE_ARRAY_LAYERS)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(*self.image_sharing_mode)
            .queue_family_indices(&self.concurrent_queue_families)
            .present_mode(*self.present_mode)
            .pre_transform(self.surface_properties.capabilities.current_transform)
            .clipped(true)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            // TODO: Allow swapchain recreation
            .old_swapchain(vk::SwapchainKHR::null());

        self.swapchain_create_info.set(*swapchain_create_info_builder);
    }

    fn init_vk_swapchain(&mut self) -> VulkanResult<()> {
        let swapchain_loader = self.logical_device.get_swapchain_loader();
        let vk_swapchain = unsafe {
            swapchain_loader.create_swapchain(
                &self.swapchain_create_info,
                None
            ).map_err(|result| VulkanError::SwapchainCreateError {result})?
        };

        self.vk_swapchain.set(vk_swapchain);
        Ok(())
    }

    fn init_images(&mut self) -> VulkanResult<()> {
        let images = unsafe {
            self.logical_device.get_swapchain_loader()
                .get_swapchain_images(*self.vk_swapchain)
        }.map_err(|result| VulkanError::SwapchainGetImagesError {result})?;

        self.images.set(images);
        Ok(())
    }

    fn init_image_views(&mut self) -> VulkanResult<()> {
        let component_mapping = Self::image_view_component_mapping();
        let subresource_range = Self::image_view_subresource_range();
        let mut image_views = Vec::with_capacity(self.images.len());

        for image in self.images.as_slice() {
            let image_view = self.create_image_view(
                *image, component_mapping, subresource_range)?;

            image_views.push(image_view);
        }

        self.image_views.set(image_views);
        Ok(())
    }

    fn image_view_component_mapping() -> vk::ComponentMapping {
        vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::IDENTITY)
            .g(vk::ComponentSwizzle::IDENTITY)
            .b(vk::ComponentSwizzle::IDENTITY)
            .a(vk::ComponentSwizzle::IDENTITY)
            .build()
    }

    fn image_view_subresource_range() -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build()
    }

    fn create_image_view(
        &self,
        image: vk::Image,
        component_mapping: vk::ComponentMapping,
        subresource_range: vk::ImageSubresourceRange
    ) -> VulkanResult<vk::ImageView> {
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.surface_format.format)
            .components(component_mapping)
            .subresource_range(subresource_range);

        let image_view = unsafe {
            self.logical_device.create_image_view(&image_view_create_info, None)
        }.map_err(|result| VulkanError::ImageViewCreateError {result})?;

        Ok(image_view)
    }

    fn create_swapchain(&mut self) {
        self.swapchain.set(Swapchain {
            vk_swapchain: self.vk_swapchain.take(),
            surface_format: self.surface_format.take(),
            extent: self.image_extent.take(),
            images: self.images.take(),
            image_views: self.image_views.take(),
            swapchain_loader: self.logical_device.get_swapchain_loader(),
            logical_device: self.logical_device.take(),
            _surface: self.surface.take()
        });
    }
}
