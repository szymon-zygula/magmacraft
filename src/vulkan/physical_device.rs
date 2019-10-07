use std::{
    collections::HashSet,
    rc::Rc
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
    vk_physical_device: vk::PhysicalDevice
}

impl PhysicalDevice {
    pub fn selector() -> PhysicalDeviceSelector {
        PhysicalDeviceSelector {
            ..Default::default()
        }
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

    physical_device: BuilderProduct<PhysicalDevice>
}

impl PhysicalDeviceSelector {
    pub fn vulkan_state(mut self, state: Rc<VulkanState>) -> Self {
        self.vulkan_state.set(state);
        self
    }

    pub fn queue_families(mut self, families: HashSet<QueueFamily>) -> Self {
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
        self.create_physical_device();

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
            .map_err(|e| {
                VulkanError::OperationFailed {
                    source: e,
                    operation: String::from("enumerate physical devices")
                }
            })?
        };

        self.devices.set(devices);

        Ok(())
    }

    fn select_suitable_device(&mut self) -> Result<(), VulkanError> {
        for device in self.devices.get() {
            let properties = self.get_device_properties(*device)?;
            let features = self.get_device_features(*device)?;

            let is_device_suitable = 
                self.are_required_queue_families_supported(*device)? &&
                self.are_required_extensions_supported(*device)?;

            if !is_device_suitable {
                continue;
            }

        }

        Ok(())
    }

    fn get_device_properties(&self, device: vk::PhysicalDevice) -> Result<vk::PhysicalDeviceProperties, VulkanError> {
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

    fn are_required_queue_families_supported(&self, device: vk::PhysicalDevice) -> Result<bool, VulkanError> {
        let queue_families = self.get_device_queue_families(device)?;

        let queue_family_indices = QueueFamilyIndices::from_properties(
            queue_families,
            device,
            Some(self.compatible_surface.get()?),
        );

       Ok(queue_family_indices.does_support_families(self.required_queue_families.get()?))
    }

    fn get_device_queue_families(&self, device: vk::PhysicalDevice) -> Result<Vec<vk::QueueFamilyProperties>, VulkanError> {
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

    fn get_device_extensions_properties(&self, device: vk::PhysicalDevice) -> Result<Vec<vk::ExtensionProperties>, VulkanError> {
        let extension_properties = unsafe {
            self.vulkan_state.get()?.get_instance()
                .enumerate_device_extension_properties(device)
                .map_err(|e| {
                    VulkanError::OperationFailed {
                        source: e,
                        operation: String::from("enumerate physical device extensions")
                    }
                })?
        };

        Ok(extension_properties)
    }

    fn is_extension_supported(device_extension_properties: &Vec<vk::ExtensionProperties>, required_extension_name: &std::ffi::CStr) -> bool {
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

    fn create_physical_device(&mut self) {
        self.physical_device.set(PhysicalDevice {
            vk_physical_device: self.selected_device.take()
        });
    }
}

#[derive(Hash, PartialEq, Eq)]
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
    graphics: Option<u32>,
    compute: Option<u32>,
    transfer: Option<u32>,
    sparse_binding: Option<u32>,
    presentation: Option<u32>
}

impl QueueFamilyIndices {
    pub fn from_properties(families: Vec<vk::QueueFamilyProperties>, physical_device: vk::PhysicalDevice, surface: Option<&vulkan::surface::Surface>) -> Self {
        let mut indices = QueueFamilyIndices {
            ..Default::default()
        };

        for (i, queue_family) in families.iter().enumerate() {
            indices.update_family_support(physical_device, queue_family.queue_flags, i as u32, surface);
        }

        indices
    }

    fn update_family_support(&mut self, physical_device: vk::PhysicalDevice, queue_family_flags: vk::QueueFlags, index: u32, surface: Option<&vulkan::surface::Surface>) {
        self.update_vulkan_family_support(queue_family_flags, index);

        if let Some(surface) = surface {
            self.update_surface_support(physical_device, index, surface);
        }
    }

    fn update_vulkan_family_support(&mut self, flags: vk::QueueFlags, index: u32) {
        self.try_set_transfer(flags, index);
        self.try_set_graphics(flags, index);
        self.try_set_compute(flags, index);
        self.try_set_sparse_binding(flags, index);
    }

    fn try_set_transfer(&mut self, flags: vk::QueueFlags, index: u32) {
        if flags.contains(vk::QueueFlags::TRANSFER) && !self.strongly_dedicated_transfer {
            self.transfer = Some(index);
            self.weakly_dedicated_transfer = true;

            if flags == vk::QueueFlags::TRANSFER {
                self.strongly_dedicated_transfer = true;
            }
        }
    }

    fn try_set_graphics(&mut self, flags: vk::QueueFlags, index: u32) {
        if flags.contains(vk::QueueFlags::GRAPHICS) {
            self.graphics = Some(index);
            self.try_set_not_dedicated_transfer(index);
        }
    }

    fn try_set_compute(&mut self, flags: vk::QueueFlags, index: u32) {
        if flags.contains(vk::QueueFlags::COMPUTE) {
            self.compute = Some(index);
            self.try_set_not_dedicated_transfer(index);
        }
    }

    fn try_set_sparse_binding(&mut self, flags: vk::QueueFlags, index: u32) {
        if flags.contains(vk::QueueFlags::SPARSE_BINDING) {
            self.sparse_binding = Some(index);
        }
    }

    fn try_set_not_dedicated_transfer(&mut self, index: u32) {
        if !self.weakly_dedicated_transfer {
            self.transfer = Some(index);
        }
    }

    fn update_surface_support(&mut self, physical_device: vk::PhysicalDevice, queue_family_index: u32, surface: &vulkan::surface::Surface) {
        let presentation_supported = unsafe {
            surface.is_supported_by_vk_device(physical_device, queue_family_index)
        };

        if presentation_supported {
            self.presentation = Some(queue_family_index);
        }
    }

    pub fn get_indice(&self, family: QueueFamily) -> Option<u32> {
        match family {
            QueueFamily::Graphics => self.graphics,
            QueueFamily::Compute => self.compute,
            QueueFamily::Transfer => self.transfer,
            QueueFamily::SparseBinding => self.sparse_binding,
            QueueFamily::Presentation => self.presentation
        }
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
