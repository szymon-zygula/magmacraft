use std::rc::Rc;
use ash::{
    self,
    vk
};
use crate::{
    builder::*,
    vulkan::{
        VulkanError,
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
    swapchain_loader: Rc<ash::extensions::khr::Swapchain>,
    // lifetime extenders
    _logical_device: Rc<LogicalDevice>,
    _surface: Rc<Surface>
}

impl Swapchain {
    pub fn builder() -> SwapchainBuilder {
        SwapchainBuilder {
            ..Default::default()
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.vk_swapchain, None);
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
    image_format: BuilderInternal<vk::SurfaceFormatKHR>,
    present_mode: BuilderInternal<vk::PresentModeKHR>,
    optimal_image_count: BuilderInternal<u32>,
    image_sharing_mode: BuilderInternal<vk::SharingMode>,
    concurrent_queue_families: BuilderInternal<Vec<u32>>,
    swapchain_create_info: BuilderInternal<vk::SwapchainCreateInfoKHR>,

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

    pub fn build(mut self) -> Result<Swapchain, VulkanError> {
        self.get_ready_for_creation()?;
        self.create_swapchain()?;

        Ok(self.swapchain.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> Result<(), VulkanError> {
        self.init_surface_properties()?;
        self.init_image_format();
        self.init_image_extent()?;
        self.init_present_mode()?;
        self.init_optimal_image_count()?;
        self.init_image_sharing_info()?;
        self.init_swapchain_create_info()?;

        Ok(())
    }

    fn init_surface_properties(&mut self) -> Result<(), VulkanError> {
        let surface = self.surface.get()?;
        let physical_device = self.physical_device.get()?;
        let surface_properties = physical_device.get_surface_properties(surface)?;
        self.surface_properties.set(surface_properties);

        Ok(())
    }

    fn init_image_format(&mut self) {
        let surface_properties = self.surface_properties.get();
        let image_format = surface_properties.formats[0];
        // TODO: select this based on gamma and other things
        self.image_format.set(image_format);
    }

    fn init_image_extent(&mut self) -> Result<(), VulkanError> {
        let surface_properties = self.surface_properties.get();
        let capabilities = surface_properties.capabilities;
        let current_extent = capabilities.current_extent;

        // TODO: support custom resolutions
        let image_extent = if Self::is_extent_undefined(&current_extent) {
            let surface = self.surface.get()?;
            surface.get_framebuffer_extent()
        }
        else {
            current_extent
        };

        self.image_extent.set(image_extent);

        Ok(())
    }

    fn is_extent_undefined(extent: &vk::Extent2D) -> bool {
        extent.width == u32::max_value()
    }

    fn init_present_mode(&mut self) -> Result<(), VulkanError> {
        let surface_properties = self.surface_properties.get();

        for present_mode in &surface_properties.present_modes {
            if self.is_present_mode_suitable(*present_mode)? {
                self.present_mode.set(*present_mode);
                return Ok(());
            }
        }

        self.present_mode.set(vk::PresentModeKHR::FIFO);
        Ok(())
    }

    fn is_present_mode_suitable(
        &self, present_mode: vk::PresentModeKHR
    ) -> Result<bool, VulkanError> {
        let vsync = *self.vsync.get()?;

        Ok(vsync && present_mode == Self::PRESENT_MODE_WITH_VSYNC ||
        !vsync && present_mode == Self::PRESENT_MODE_WITHOUT_VSYNC)
    }

    fn init_optimal_image_count(&mut self) -> Result<(), VulkanError> {
        let surface_properties = self.surface_properties.get();
        let min_image_count = surface_properties.capabilities.min_image_count;
        let max_image_count = surface_properties.capabilities.max_image_count;
        let mut optimal_image_count =min_image_count + Self::ADDITIONAL_IMAGES_COUNT;

        if max_image_count != 0 && optimal_image_count > max_image_count {
            optimal_image_count = max_image_count;
        };

        self.optimal_image_count.set(optimal_image_count);

        Ok(())
    }

    fn init_image_sharing_info(&mut self) -> Result<(), VulkanError> {
        let physical_device = self.physical_device.get()?;
        let multiple_queue_family_usage = physical_device.is_transfer_queue_family_dedicated();
        let graphics_index = physical_device.get_queue_family_index(QueueFamily::Graphics)?;
        let transfer_index = physical_device.get_queue_family_index(QueueFamily::Transfer)?;

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

    fn init_swapchain_create_info(&mut self) -> Result<(), VulkanError> {
        let surface = self.surface.get()?;
        let optimal_image_count = self.optimal_image_count.take();
        let image_format = self.image_format.take();
        let image_extent = self.image_extent.take();
        let image_sharing_mode = self.image_sharing_mode.take();
        // Dereferencing `swapchain_create_info` gets rid of lifetime information,
        // but it depends on memory owned by `self.concurrent_queue_families` after return,
        // so it cannot be taken.
        let concurrent_queue_families = self.concurrent_queue_families.get();
        let present_mode = self.present_mode.take();
        let surface_properties = self.surface_properties.get();

        let swapchain_create_info_builder = vk::SwapchainCreateInfoKHR::builder()
            .surface(***surface)
            .min_image_count(optimal_image_count)
            .image_format(image_format.format)
            .image_color_space(image_format.color_space)
            .image_extent(image_extent)
            .image_array_layers(Self::IMAGE_ARRAY_LAYERS)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(concurrent_queue_families.as_slice())
            .present_mode(present_mode)
            .pre_transform(surface_properties.capabilities.current_transform)
            .clipped(true)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            // TODO: Allow swapchain recreation
            .old_swapchain(vk::SwapchainKHR::null());

        self.swapchain_create_info.set(*swapchain_create_info_builder);

        Ok(())
    }

    fn create_swapchain(&mut self) -> Result<(), VulkanError> {
        let swapchain_loader = self.logical_device.get()?.get_swapchain_loader();
        let vk_swapchain = unsafe {
            swapchain_loader.create_swapchain(
                self.swapchain_create_info.get(),
                None
            ).map_err(VulkanError::operation_failed_mapping("create swapchain"))?
        };

        self.swapchain.set(Swapchain {
            vk_swapchain,
            swapchain_loader: Rc::clone(&swapchain_loader),
            _logical_device: Rc::clone(self.logical_device.get()?),
            _surface: Rc::clone(self.surface.get()?)
        });

        Ok(())
    }
}
