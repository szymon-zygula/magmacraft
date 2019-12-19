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
        physical_device::{
            PhysicalDevice,
            QueueFamily
        },
        logical_device::LogicalDevice,
        command_buffer::CommandBuffer
    }
};

pub struct CommandPool {
    vk_command_pool: vk::CommandPool,
    logical_device: Rc<LogicalDevice>
}

impl CommandPool {
    pub fn builder() -> CommandPoolBuilder {
        CommandPoolBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self) -> vk::CommandPool {
        self.vk_command_pool
    }

    pub fn allocate_command_buffers(&self, count: usize) -> VulkanResult<Vec<CommandBuffer>> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.handle())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count as u32);

        let command_buffers = unsafe {
            self.logical_device.allocate_command_buffers(&allocate_info)
        }.map_err(|result| VulkanError::CommandBufferAllocateError {result})?;

        Ok(command_buffers.into_iter().map(
                |vk_command_buffer| CommandBuffer { vk_command_buffer }).collect())
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_command_pool(self.vk_command_pool, None);
        }
    }
}

#[derive(Default)]
pub struct CommandPoolBuilder {
    physical_device: BuilderRequirement<Rc<PhysicalDevice>>,
    logical_device: BuilderRequirement<Rc<LogicalDevice>>,
    often_rerecorded: Option<bool>,
    queue_family: BuilderRequirement<QueueFamily>,

    create_flags: BuilderInternal<vk::CommandPoolCreateFlags>,
    vk_command_pool: BuilderInternal<vk::CommandPool>,

    command_pool: BuilderProduct<CommandPool>
}

impl CommandPoolBuilder {
    pub fn physical_device(mut self, physical_device: Rc<PhysicalDevice>) -> Self {
        self.physical_device.set(physical_device);
        self
    }

    pub fn logical_device(mut self, logical_device: Rc<LogicalDevice>) -> Self {
        self.logical_device.set(logical_device);
        self
    }

    pub fn queue_family(mut self, queue_family: QueueFamily) -> Self {
        self.queue_family.set(queue_family);
        self
    }

    pub fn often_rerecorded(mut self, often_rerecorded: bool) -> Self {
        self.often_rerecorded = Some(often_rerecorded);
        self
    }

    pub fn build(mut self) -> VulkanResult<CommandPool> {
        self.init_create_flags();
        self.init_vk_command_pool()?;
        self.create_command_pool();

        Ok(self.command_pool.unwrap())
    }

    fn init_create_flags(&mut self) {
        let mut flags = vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER;
        if self.often_rerecorded.unwrap_or(false) {
            flags |= vk::CommandPoolCreateFlags::TRANSIENT;
        }

        self.create_flags.set(flags);
    }

    fn init_vk_command_pool(&mut self) -> VulkanResult<()> {
        let queue_family_index =
            self.physical_device.get_queue_family_index(*self.queue_family)?;

        let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(*self.create_flags);

        let vk_command_pool = unsafe {
            self.logical_device.create_command_pool(&command_pool_create_info, None)
        }.map_err(|result| VulkanError::CommandPoolCreateError {result})?;

        self.vk_command_pool.set(vk_command_pool);
        Ok(())
    }

    fn create_command_pool(&mut self) {
        let command_pool = CommandPool {
            vk_command_pool: self.vk_command_pool.take(),
            logical_device: self.logical_device.take()
        };

        self.command_pool.set(command_pool);
    }
}
