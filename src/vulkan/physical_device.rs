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
    builder::{
        BuilderRequirement,
        BuilderInternal,
        BuilderProduct
    },
    vulkan::{
        self,
        VulkanError,
        VulkanResult,
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

    pub fn handle(&self) -> vk::PhysicalDevice {
        self.vk_physical_device
    }

    pub fn queue_family_index(
        &self, queue_family: QueueFamily
    ) -> VulkanResult<QueueFamilyIndex> {
        let indice = self.queue_family_indices.index(queue_family);
        indice.ok_or(VulkanError::QueueFamilyNotSupported {queue_family})
    }

    pub fn is_transfer_queue_family_dedicated(&self) -> bool {
        self.queue_family_indices.is_transfer_dedicated()
    }

    pub fn requested_extensions(&self) -> &PhysicalDeviceExtensions {
        &self.requested_extensions
    }

    pub fn surface_properties(
        &self, surface: &vulkan::surface::Surface
    ) -> VulkanResult<PhysicalDeviceSurfaceProperties> {
        let capabilities = self.surface_capabilities(surface)?;
        let formats = self.surface_formats(surface)?;
        let present_modes = self.surface_present_modes(surface)?;

        Ok(PhysicalDeviceSurfaceProperties {
            capabilities,
            formats,
            present_modes
        })
    }

    fn surface_loader(&self) -> Rc<ash::extensions::khr::Surface> {
        self.vulkan_state.surface_loader()
    }

    fn surface_capabilities(
        &self,
        surface: &vulkan::surface::Surface
    ) -> VulkanResult<vk::SurfaceCapabilitiesKHR> {
        let capabilities = unsafe {
            self.surface_loader()
                .get_physical_device_surface_capabilities(
                    self.vk_physical_device, surface.handle())
                .map_err(|result| VulkanError::PhysicalDevicePropertiesError {result})?
        };

        Ok(capabilities)
    }

    fn surface_formats(
        &self,
        surface: &vulkan::surface::Surface
    ) -> VulkanResult<Vec<vk::SurfaceFormatKHR>> {
        let formats = unsafe {
            self.surface_loader()
                .get_physical_device_surface_formats(
                    self.vk_physical_device, surface.handle())
                .map_err(|result| VulkanError::PhysicalDevicePropertiesError {result})?
        };

        Ok(formats)
    }

    fn surface_present_modes(
        &self,
        surface: &vulkan::surface::Surface
    ) -> VulkanResult<Vec<vk::PresentModeKHR>> {
        let present_modes = unsafe {
            self.surface_loader()
                .get_physical_device_surface_present_modes(
                    self.vk_physical_device, surface.handle())
                .map_err(|result| VulkanError::PhysicalDevicePropertiesError {result})?
        };

        Ok(present_modes)
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
    required_extensions: Option<PhysicalDeviceExtensions>,

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

    pub fn queue_families(mut self, families: &[QueueFamily]) -> Self {
        let families = HashSet::from_iter(families.to_owned().into_iter());
        self.required_queue_families.set(families);
        self
    }

    pub fn surface_compatible(mut self, surface: Rc<vulkan::surface::Surface>) -> Self {
        self.compatible_surface.set(surface);
        self
    }

    pub fn device_extensions(mut self, extensions: PhysicalDeviceExtensions) -> Self {
        self.required_extensions = Some(extensions);
        self
    }

    pub fn select(mut self) -> VulkanResult<PhysicalDevice> {
        self.get_ready_for_physical_device_creation()?;
        self.create_physical_device();

        Ok(self.physical_device.unwrap())
    }

    pub fn get_ready_for_physical_device_creation(&mut self) -> VulkanResult<()> {
        self.init_available_devices()?;
        self.select_suitable_device()?;

        Ok(())
    }

    fn init_available_devices(&mut self) -> VulkanResult<()> {
        let devices = unsafe { self.vulkan_state
            .instance()
            .enumerate_physical_devices()
            .map_err(|result| VulkanError::EnumeratePhysicalDevicesError {result})?
        };

        self.devices.set(devices);

        Ok(())
    }

    fn select_suitable_device(&mut self) -> VulkanResult<()> {
        for device in self.devices.as_ref() {
            if self.is_device_suitable(*device)? {
                self.selected_device.set(*device);
                let queue_family_indices = self.queue_family_indices(*device);
                self.queue_family_indices.set(queue_family_indices);
            }

            // If selected device is a discrete GPU, it's good enough
            if self.is_device_discrete(*device) {
                return Ok(());
            }
        }

        Ok(())
    }

    fn is_device_suitable(&self, device: vk::PhysicalDevice) -> VulkanResult<bool> {
        let is_suitable =
            self.are_required_queue_families_supported(device) &&
            self.are_required_extensions_supported(device)?;

        Ok(is_suitable)
    }

    fn are_required_queue_families_supported(&self, device: vk::PhysicalDevice) -> bool {
        let queue_family_indices = self.queue_family_indices(device);
        queue_family_indices.does_support_families(&self.required_queue_families)
    }

    fn queue_family_indices(&self, device: vk::PhysicalDevice) -> QueueFamilyIndices {
        let queue_families = self.device_queue_families(device);

        QueueFamilyIndices::from_properties(
            queue_families,
            device,
            Some(&self.compatible_surface),
        )
    }

    fn device_queue_families(
        &self, device: vk::PhysicalDevice
    ) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.vulkan_state
                .instance()
                .get_physical_device_queue_family_properties(device)
        }
    }

    fn are_required_extensions_supported(
        &self,
        device: vk::PhysicalDevice
    ) -> VulkanResult<bool> {
        let device_extension_properties = self.device_extensions_properties(device)?;

        if let Some(required_extensions) = &self.required_extensions {
            let are_extensions_supported = Self::are_extensions_supported(
                &device_extension_properties,
                &required_extensions);

            return Ok(are_extensions_supported);
        }

        Ok(true)
    }

    fn device_extensions_properties(
        &self,
        device: vk::PhysicalDevice
    ) -> VulkanResult<Vec<vk::ExtensionProperties>> {
        let extension_properties = unsafe {
            self.vulkan_state.instance()
                .enumerate_device_extension_properties(device)
                .map_err(|result| VulkanError::EnumeratePhysicalDeviceExtensionsError {result})?
        };

        Ok(extension_properties)
    }

    fn are_extensions_supported(
        device_extension_properties: &Vec<vk::ExtensionProperties>,
        required_extensions: &PhysicalDeviceExtensions
    ) -> bool {
        for required_extension in required_extensions.strings() {
            if !Self::is_extension_supported(&device_extension_properties, &required_extension) {
                return false;
            }
        }

        true
    }

    fn is_extension_supported(
        device_extension_properties: &Vec<vk::ExtensionProperties>,
        required_extension_name: &std::ffi::CStr
    ) -> bool {
        for extension_properties in device_extension_properties {
            let device_extension_name = Self::device_extension_name(extension_properties);

            if device_extension_name == required_extension_name {
                return true;
            }
        }

        false
    }

    fn device_extension_name(
        extension_properties: &vk::ExtensionProperties
    ) -> &std::ffi::CStr {
        let extension_name_pointer =
            &extension_properties.extension_name as *const std::os::raw::c_char;

        unsafe {
            std::ffi::CStr::from_ptr(extension_name_pointer)
        }
    }

    fn is_device_discrete(&self, device: vk::PhysicalDevice) -> bool {
        let properties = self.device_properties(device);

        properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
    }

    fn device_properties(&self, device: vk::PhysicalDevice) -> vk::PhysicalDeviceProperties {
        let properties = unsafe {
            self.vulkan_state
                .instance()
                .get_physical_device_properties(device)
        };

        properties
    }

    fn device_features(&self, device: vk::PhysicalDevice) -> vk::PhysicalDeviceFeatures {
        let features = unsafe {
            self.vulkan_state
                .instance()
                .get_physical_device_features(device)
        };

        features
    }

    fn create_physical_device(&mut self) {
        let requested_extensions =
            self.required_extensions.take()
            .unwrap_or(PhysicalDeviceExtensions::new());

        self.physical_device.set(PhysicalDevice {
            vulkan_state: self.vulkan_state.take(),
            vk_physical_device: self.selected_device.take(),
            queue_family_indices: self.queue_family_indices.take(),
            requested_extensions
        });
    }
}

pub struct PhysicalDeviceSurfaceProperties {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>
}

pub type QueueFamilyIndex = u32;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum QueueFamily {
    Graphics,
    Compute,
    Transfer,
    SparseBinding,
    Presentation
}

impl std::fmt::Display for QueueFamily {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self)
    }
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
            indices.update_family_support(
                physical_device,
                queue_family.queue_flags,
                i as QueueFamilyIndex,
                surface);
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

    pub fn index(&self, family: QueueFamily) -> Option<QueueFamilyIndex> {
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
        let supported_families = self.family_hash_set();
        supported_families.is_superset(required_families)
    }

    fn family_hash_set(&self) -> HashSet<QueueFamily> {
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
