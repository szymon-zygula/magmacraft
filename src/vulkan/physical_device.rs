use std::{
    collections::HashSet,
    rc::Rc,
    iter::FromIterator,
    clone::Clone
};
use ash::{
    self,
    vk,
    version::{
        InstanceV1_0
    }
};
use crate::{
    builder::*,
    vulkan::{
        self,
        VulkanError,
        state::VulkanState
    }
};

pub struct PhysicalDevice {
    vulkan_state: Rc<VulkanState>,
    vk_physical_device: vk::PhysicalDevice,
    queue_family_indices: QueueFamilyIndices,
    requested_extensions: PhysicalDeviceExtensions
}

impl PhysicalDevice {
    pub fn selector() -> PhysicalDeviceSelector {
        PhysicalDeviceSelector {
            ..Default::default()
        }
    }

    pub fn get_queue_family_index(&self, queue_family: QueueFamily) -> Result<QueueFamilyIndex, VulkanError> {
        let indice = self.queue_family_indices.get_index(queue_family);
        indice.ok_or(VulkanError::QueueFamilyNotSupported)
    }

    pub fn is_transfer_queue_family_dedicated(&self) -> bool {
        self.queue_family_indices.is_transfer_dedicated()
    }

    pub fn get_requested_extensions(&self) -> &PhysicalDeviceExtensions {
        &self.requested_extensions
    }

    pub fn get_surface_properties(
        &self,
        surface: &vulkan::surface::Surface
    ) -> Result<PhysicalDeviceSurfaceProperties, VulkanError> {
        let surface_loader = self.vulkan_state.get_surface_loader();

        let capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(self.vk_physical_device, **surface)
        }.map_err(Self::surface_properties_error_mapping)?;

        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(self.vk_physical_device, **surface)
        }.map_err(Self::surface_properties_error_mapping)?;

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(self.vk_physical_device, **surface)
        }.map_err(Self::surface_properties_error_mapping)?;

        Ok(PhysicalDeviceSurfaceProperties {
            capabilities,
            formats,
            present_modes
        })
    }

    fn surface_properties_error_mapping(source: vk::Result) -> VulkanError {
        VulkanError::OperationFailed {
            source,
            operation: String::from("get physical device surface properties")
        }
    }
}

impl std::ops::Deref for PhysicalDevice {
    type Target = vk::PhysicalDevice;

    fn deref(&self) -> &Self::Target {
        &self.vk_physical_device
    }
}

#[derive(Default)]
pub struct PhysicalDeviceSelector {
    vulkan_state: BuilderRequirement<Rc<VulkanState>>,
    required_queue_families: BuilderRequirement<HashSet<QueueFamily>>,
    compatible_surface: BuilderRequirement<Rc<vulkan::surface::Surface>>,
    required_extensions: PhysicalDeviceExtensions,

    devices: BuilderInternal<Vec<vk::PhysicalDevice>>,
    selected_device: BuilderInternal<vk::PhysicalDevice>,
    queue_family_indices: BuilderInternal<QueueFamilyIndices>,

    physical_device: BuilderProduct<PhysicalDevice>
}

impl PhysicalDeviceSelector {
    pub fn vulkan_state(mut self, state: Rc<VulkanState>) -> Self {
        self.vulkan_state.set(state);
        self
    }

    pub fn queue_families(mut self, families: &Vec<QueueFamily>) -> Self {
        let families = HashSet::from_iter(families.clone().into_iter());
        self.required_queue_families.set(families);
        self
    }

    pub fn surface_compatible(mut self, surface: Rc<vulkan::surface::Surface>) -> Self {
        self.compatible_surface.set(surface);
        self
    }

    pub fn device_extensions(mut self, extensions: PhysicalDeviceExtensions) -> Self {
        self.required_extensions = extensions;
        self
    }

    pub fn select(mut self) -> Result<PhysicalDevice, VulkanError> {
        self.get_ready_for_physical_device_creation()?;
        self.create_physical_device()?;

        Ok(self.physical_device.unwrap())
    }

    pub fn get_ready_for_physical_device_creation(&mut self) -> Result<(), VulkanError> {
        self.init_available_devices()?;
        self.select_suitable_device()?;

        Ok(())
    }

    fn init_available_devices(&mut self) -> Result<(), VulkanError> {
        let devices = unsafe { self.vulkan_state
            .get()?.get_instance()
            .enumerate_physical_devices()
            .map_err(VulkanError::operation_failed_mapping("enumerate physical devices"))?
        };

        self.devices.set(devices);

        Ok(())
    }

    fn select_suitable_device(&mut self) -> Result<(), VulkanError> {
        for device in self.devices.get() {
            if self.is_device_suitable(*device)? {
                self.selected_device.set(*device);
                let queue_family_indices = self.get_queue_family_indices(*device)?;
                self.queue_family_indices.set(queue_family_indices);
            }

            // If selected device is a discrete GPU, it's good enough
            if self.is_device_discrete(*device)? {
                return Ok(());
            }
        }

        Ok(())
    }

    fn is_device_suitable(&self, device: vk::PhysicalDevice) -> Result<bool, VulkanError> {
        let is_suitable =
            self.are_required_queue_families_supported(device)? &&
            self.are_required_extensions_supported(device)?;

        Ok(is_suitable)
    }

    fn are_required_queue_families_supported(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<bool, VulkanError> {
        let queue_family_indices = self.get_queue_family_indices(device)?;
        Ok(queue_family_indices.does_support_families(self.required_queue_families.get()?))
    }

    fn get_queue_family_indices(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<QueueFamilyIndices, VulkanError> {
        let queue_families = self.get_device_queue_families(device)?;

        Ok(QueueFamilyIndices::from_properties(
            queue_families,
            device,
            Some(self.compatible_surface.get()?),
        ))
    }

    fn get_device_queue_families(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<Vec<vk::QueueFamilyProperties>, VulkanError> {
        let queue_family_properties = unsafe {
            self.vulkan_state.get()?.get_instance()
                .get_physical_device_queue_family_properties(device)
        };

        Ok(queue_family_properties)
    }

    fn are_required_extensions_supported(&self, device: vk::PhysicalDevice) -> Result<bool, VulkanError> {
        let device_extension_properties = self.get_device_extensions_properties(device)?;

        for required_extension in self.required_extensions.get_strings() {
            if !Self::is_extension_supported(&device_extension_properties, &required_extension) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_device_extensions_properties(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<Vec<vk::ExtensionProperties>, VulkanError> {
        let extension_properties = unsafe {
            self.vulkan_state.get()?.get_instance()
                .enumerate_device_extension_properties(device)
                .map_err(VulkanError::operation_failed_mapping("enumerate physical device extensions"))?
        };

        Ok(extension_properties)
    }

    fn is_extension_supported(
        device_extension_properties: &Vec<vk::ExtensionProperties>,
        required_extension_name: &std::ffi::CStr
    ) -> bool {
        let mut found = false;
        for extension_properties in device_extension_properties {
            let device_extension_name = unsafe { std::ffi::CStr::from_ptr(
                &extension_properties.extension_name as *const std::os::raw::c_char
            )};

            if device_extension_name == required_extension_name {
                found = true;
                break;
            }
        }

        found
    }

    fn is_device_discrete(&self, device: vk::PhysicalDevice) -> Result<bool, VulkanError> {
        let properties = self.get_device_properties(device)?;

        Ok(properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
    }

    fn get_device_properties(
        &self,
        device: vk::PhysicalDevice
    ) -> Result<vk::PhysicalDeviceProperties, VulkanError> {
        let properties = unsafe {
            self.vulkan_state.get()?.get_instance()
                .get_physical_device_properties(device)
        };

        Ok(properties)
    }

    fn get_device_features(&self, device: vk::PhysicalDevice) -> Result<vk::PhysicalDeviceFeatures, VulkanError> {
        let features = unsafe {
            self.vulkan_state.get()?.get_instance()
                .get_physical_device_features(device)
        };

        Ok(features)
    }

    fn create_physical_device(&mut self) -> Result<(), VulkanError> {
        self.physical_device.set(PhysicalDevice {
            vulkan_state: Rc::clone(self.vulkan_state.get()?),
            vk_physical_device: self.selected_device.take(),
            queue_family_indices: self.queue_family_indices.take(),
            requested_extensions: self.required_extensions.clone()
        });

        Ok(())
    }
}

pub struct PhysicalDeviceSurfaceProperties {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>
}

pub type QueueFamilyIndex = u32;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum QueueFamily {
    Graphics,
    Compute,
    Transfer,
    SparseBinding,
    Presentation
}

#[derive(Default)]
pub struct QueueFamilyIndices {
    weakly_dedicated_transfer: bool,
    strongly_dedicated_transfer: bool,
    graphics: Option<QueueFamilyIndex>,
    compute: Option<QueueFamilyIndex>,
    transfer: Option<QueueFamilyIndex>,
    sparse_binding: Option<QueueFamilyIndex>,
    presentation: Option<QueueFamilyIndex>
}

impl QueueFamilyIndices {
    pub fn from_properties(
        families: Vec<vk::QueueFamilyProperties>,
        physical_device: vk::PhysicalDevice,
        surface: Option<&vulkan::surface::Surface>
    ) -> Self {
        let mut indices = QueueFamilyIndices {
            ..Default::default()
        };

        for (i, queue_family) in families.iter().enumerate() {
            indices.update_family_support(physical_device, queue_family.queue_flags, i as QueueFamilyIndex, surface);
        }

        indices
    }

    fn update_family_support(
        &mut self,
        physical_device: vk::PhysicalDevice,
        queue_family_flags: vk::QueueFlags,
        index: QueueFamilyIndex,
        surface: Option<&vulkan::surface::Surface>
    ) {
        self.update_vulkan_family_support(queue_family_flags, index);

        if let Some(surface) = surface {
            self.update_surface_support(physical_device, index, surface);
        }
    }

    fn update_vulkan_family_support(&mut self, flags: vk::QueueFlags, index: QueueFamilyIndex) {
        self.try_set_transfer(flags, index);
        self.try_set_graphics(flags, index);
        self.try_set_compute(flags, index);
        self.try_set_sparse_binding(flags, index);
    }

    fn try_set_transfer(&mut self, flags: vk::QueueFlags, index: QueueFamilyIndex) {
        if flags.contains(vk::QueueFlags::TRANSFER) && !self.strongly_dedicated_transfer {
            self.transfer = Some(index);
            self.weakly_dedicated_transfer = true;

            if flags == vk::QueueFlags::TRANSFER {
                self.strongly_dedicated_transfer = true;
            }
        }
    }

    fn try_set_graphics(&mut self, flags: vk::QueueFlags, index: QueueFamilyIndex) {
        if flags.contains(vk::QueueFlags::GRAPHICS) {
            self.graphics = Some(index);
            self.try_set_not_dedicated_transfer(index);
        }
    }

    fn try_set_compute(&mut self, flags: vk::QueueFlags, index: QueueFamilyIndex) {
        if flags.contains(vk::QueueFlags::COMPUTE) {
            self.compute = Some(index);
            self.try_set_not_dedicated_transfer(index);
        }
    }

    fn try_set_sparse_binding(&mut self, flags: vk::QueueFlags, index: QueueFamilyIndex) {
        if flags.contains(vk::QueueFlags::SPARSE_BINDING) {
            self.sparse_binding = Some(index);
        }
    }

    fn try_set_not_dedicated_transfer(&mut self, index: QueueFamilyIndex) {
        if !self.weakly_dedicated_transfer {
            self.transfer = Some(index);
        }
    }

    fn update_surface_support(
        &mut self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: QueueFamilyIndex,
        surface: &vulkan::surface::Surface
    ) {
        let presentation_supported = unsafe {
            surface.is_supported_by_vk_device(physical_device, queue_family_index)
        };

        if presentation_supported {
            self.presentation = Some(queue_family_index);
        }
    }

    pub fn get_index(&self, family: QueueFamily) -> Option<QueueFamilyIndex> {
        match family {
            QueueFamily::Graphics => self.graphics,
            QueueFamily::Compute => self.compute,
            QueueFamily::Transfer => self.transfer,
            QueueFamily::SparseBinding => self.sparse_binding,
            QueueFamily::Presentation => self.presentation
        }
    }

    pub fn is_transfer_dedicated(&self) -> bool {
        self.strongly_dedicated_transfer
    }

    pub fn does_support_families(&self, required_families: &HashSet<QueueFamily>) -> bool {
        let supported_families = self.get_family_hash_set();
        supported_families.is_superset(required_families)
    }

    fn get_family_hash_set(&self) -> HashSet<QueueFamily> {
        let mut family_set = HashSet::new();

        if self.graphics.is_some() {
            family_set.insert(QueueFamily::Graphics);
        }

        if self.compute.is_some() {
            family_set.insert(QueueFamily::Compute);
        }

        if self.transfer.is_some() {
            family_set.insert(QueueFamily::Transfer);
        }

        if self.sparse_binding.is_some() {
            family_set.insert(QueueFamily::SparseBinding);
        }

        family_set
    }
}

create_c_string_collection_type!(PhysicalDeviceExtensions);
