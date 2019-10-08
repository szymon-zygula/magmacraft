use std::{
    rc::Rc,
    collections::HashSet,
    iter::FromIterator
};
use ash::{
    self,
    vk,
    version::{
        InstanceV1_0,
        DeviceV1_0
    }
};
use crate::{
    builder::*,
    vulkan::{
        self,
        VulkanError,
        state::VulkanState,
        physical_device::QueueFamily
    }
};

pub struct LogicalDevice {
    vk_logical_device: ash::Device
}

impl LogicalDevice {
    pub fn new(vulkan_state: Rc<VulkanState>, physical_device: &vulkan::physical_device::PhysicalDevice, queue_families: Vec<QueueFamily>) -> Result<Self, VulkanError> {
        let extension_names = physical_device.get_raw_extension_names();

        let mut unique_queue_family_indices = HashSet::new();
        for queue_family in queue_families {
            let index = physical_device.get_queue_family_index(queue_family)?;
            unique_queue_family_indices.insert(index);
        }

        let queue_family_indices = Vec::from_iter(unique_queue_family_indices.into_iter());

        let mut queue_create_infos = Vec::with_capacity(queue_family_indices.len());
        for queue_family_index in queue_family_indices {
            let create_info = *vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&[1.0]);

            queue_create_infos.push(create_info);
        }


        let builder = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(extension_names)
            .queue_create_infos(queue_create_infos.as_slice());

        let vk_logical_device = unsafe { vulkan_state.get_instance().create_device(
            **physical_device,
            &*builder,
            None
            ).map_err(|e| {
                VulkanError::OperationFailed {
                    source: e,
                    operation: String::from("create logical device")
                }
            })?
        };

        Ok(LogicalDevice {
            vk_logical_device
        })
    }
}

impl std::ops::Deref for LogicalDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.vk_logical_device
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe {
            self.destroy_device(None);
        }
    }
}

struct LogicalDeviceBuilder {
    vulkan_state: BuilderRequirement<Rc<VulkanState>>,
    physical_device: BuilderRequirement<Rc<vulkan::physical_device::PhysicalDevice>>,
    queue_families: BuilderRequirement<Vec<QueueFamily>>,

    logical_device: BuilderProduct<LogicalDevice>
}

impl LogicalDeviceBuilder {
    pub fn vulkan_state(mut self, vulkan_state: Rc<VulkanState>) -> Self {
        self.vulkan_state.set(vulkan_state);
        self
    }

    pub fn physical_device(mut self, physical_device: Rc<vulkan::physical_device::PhysicalDevice>) -> Self {
        self.physical_device.set(physical_device);
        self
    }

    pub fn queue_families(mut self, queue_families: Vec<QueueFamily>) -> Self {
        self.queue_families.set(queue_families);
        self
    }

    pub fn build(mut self) -> LogicalDevice {
        self.logical_device.unwrap()
    }
}
