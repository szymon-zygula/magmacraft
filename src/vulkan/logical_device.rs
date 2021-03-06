use std::{
    rc::Rc,
    collections::{
        HashSet,
        HashMap
    },
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
    builder::{
        BuilderRequirement,
        BuilderInternal,
        BuilderProduct
    },
    vulkan::{
        VulkanError,
        VulkanResult,
        state::VulkanState,
        physical_device::{
            PhysicalDevice,
            QueueFamilyIndex,
            QueueFamily,
            PhysicalDeviceExtensions
        }
    }
};

pub struct LogicalDevice {
    vk_logical_device: ash::Device,
    swapchain_loader: Rc<ash::extensions::khr::Swapchain>,
    device_queues: HashMap<QueueFamily, vk::Queue>,
    // lifetime extenders
    _physical_device: Rc<PhysicalDevice>
}

impl LogicalDevice {
    pub fn builder() -> LogicalDeviceBuilder {
        LogicalDeviceBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self) -> &ash::Device {
        &self.vk_logical_device
    }

    pub fn swapchain_loader(&self) -> Rc<ash::extensions::khr::Swapchain> {
        Rc::clone(&self.swapchain_loader)
    }

    pub fn device_queue(&self, queue_family: QueueFamily) -> VulkanResult<vk::Queue> {
        let device_queue = *self.device_queues.get(&queue_family)
            .ok_or(VulkanError::LogicalDeviceGetDeviceQueueError)?;

        Ok(device_queue)
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
        let wait_result = unsafe {
            self.device_wait_idle()
        };

        wait_result
            .map_err(|result| VulkanError::LogicalDeviceWaitIdleError {result})
            .unwrap();

        unsafe {
            self.destroy_device(None);
        }
    }
}

#[derive(Default)]
pub struct LogicalDeviceBuilder {
    vulkan_state: BuilderRequirement<Rc<VulkanState>>,
    physical_device: BuilderRequirement<Rc<PhysicalDevice>>,
    queue_families: BuilderRequirement<Vec<QueueFamily>>,

    unique_queue_family_indices: BuilderInternal<Vec<QueueFamilyIndex>>,
    queue_create_infos: BuilderInternal<Vec<vk::DeviceQueueCreateInfo>>,
    device_extensions: BuilderInternal<PhysicalDeviceExtensions>,
    logical_device_create_info: BuilderInternal<vk::DeviceCreateInfo>,
    vk_logical_device: BuilderInternal<ash::Device>,
    swapchain_loader: BuilderInternal<ash::extensions::khr::Swapchain>,
    device_queues: BuilderInternal<HashMap<QueueFamily, vk::Queue>>,

    logical_device: BuilderProduct<LogicalDevice>
}

impl LogicalDeviceBuilder {
    const DEFAULT_QUEUE_PRIORITIES: [f32; 1] = [1.0];

    pub fn vulkan_state(mut self, vulkan_state: Rc<VulkanState>) -> Self {
        self.vulkan_state.set(vulkan_state);
        self
    }

    pub fn physical_device(mut self, physical_device: Rc<PhysicalDevice>) -> Self {
        self.physical_device.set(physical_device);
        self
    }

    pub fn queue_families(mut self, queue_families: &[QueueFamily]) -> Self {
        let mut queue_families_vec = Vec::with_capacity(queue_families.len());
        unsafe {
            queue_families_vec.set_len(queue_families.len());
        }

        queue_families_vec.copy_from_slice(queue_families);
        self.queue_families.set(queue_families_vec);
        self
    }

    pub fn build(mut self) -> VulkanResult<LogicalDevice> {
        self.get_ready_for_creation()?;
        self.create_logical_device();

        Ok(self.logical_device.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_unique_queue_family_indices()?;
        self.init_queue_create_infos();
        self.init_device_extensions();
        self.init_logical_device_create_info();
        self.init_vk_logical_device()?;
        self.init_swapchain_loader();
        self.init_device_queues()?;

        Ok(())
    }

    fn init_unique_queue_family_indices(&mut self) -> VulkanResult<()> {
        let mut unique_queue_family_indices = HashSet::new();
        for queue_family in &*self.queue_families {
            self.insert_queue_family_index_into_hashset(
                *queue_family, &mut unique_queue_family_indices
            )?;
        }

        self.unique_queue_family_indices.set(
            Vec::from_iter(unique_queue_family_indices.into_iter())
        );

        Ok(())
    }

    fn insert_queue_family_index_into_hashset(
        &self, queue_family: QueueFamily, hashset: &mut HashSet<QueueFamilyIndex>
    ) -> VulkanResult<()> {
        let index = self.physical_device.queue_family_index(queue_family)?;
        hashset.insert(index);

        Ok(())
    }

    fn init_queue_create_infos(&mut self) {
        let mut queue_create_infos = Vec::with_capacity(self.unique_queue_family_indices.len());

        for queue_family_index in &*self.unique_queue_family_indices {
            let builder = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*queue_family_index)
                .queue_priorities(&Self::DEFAULT_QUEUE_PRIORITIES);

            queue_create_infos.push(*builder);
        }

        self.queue_create_infos.set(queue_create_infos);
    }

    fn init_device_extensions(&mut self) {
        let device_extensions = self.physical_device.requested_extensions();
        self.device_extensions.set(device_extensions.clone());
    }

    fn init_logical_device_create_info(&mut self) {
        let builder = vk::DeviceCreateInfo::builder()
            .queue_create_infos(self.queue_create_infos.as_slice())
            .enabled_extension_names(self.device_extensions.pointers());

        self.logical_device_create_info.set(*builder);
    }

    fn init_vk_logical_device(&mut self) -> VulkanResult<()> {
        let vk_instance = self.vulkan_state.instance();
        let vk_logical_device = unsafe {
            vk_instance.create_device(
                self.physical_device.handle(),
                &self.logical_device_create_info,
                None
            ).map_err(|result| VulkanError::LogicalDeviceCreateError {result})?
        };

        self.vk_logical_device.set(vk_logical_device);

        Ok(())
    }

    fn init_swapchain_loader(&mut self) {
        let vk_instance = self.vulkan_state.instance();
        let swapchain_loader =
            ash::extensions::khr::Swapchain::new(
                vk_instance.handle(), self.vk_logical_device.as_ref());

        self.swapchain_loader.set(swapchain_loader);
    }

    fn init_device_queues(&mut self) -> VulkanResult<()> {
        let mut device_queues = HashMap::new();
        for queue_family in self.queue_families.as_slice() {
            self.insert_device_queue_into_hashmap(*queue_family, &mut device_queues)?;
        }

        self.device_queues.set(device_queues);

        Ok(())
    }

    fn insert_device_queue_into_hashmap(
        &self,
        queue_family: QueueFamily,
        device_queues: &mut HashMap<QueueFamily, vk::Queue>
    ) -> VulkanResult<()> {
        let queue_family_index =
            self.physical_device.queue_family_index(queue_family)?;

        let device_queue = unsafe {
            self.vk_logical_device.get_device_queue(queue_family_index, 0)
        };

        device_queues.insert(queue_family, device_queue);

        Ok(())
    }

    fn create_logical_device(&mut self) {
        self.logical_device.set(LogicalDevice {
            vk_logical_device: self.vk_logical_device.take(),
            swapchain_loader: Rc::new(self.swapchain_loader.take()),
            device_queues: self.device_queues.take(),
            _physical_device: self.physical_device.take()
        });
    }
}
