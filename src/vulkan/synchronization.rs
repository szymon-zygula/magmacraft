use std::{
    convert::TryInto,
    rc::Rc
};
use ash::{
    version::DeviceV1_0,
    vk
};
use crate::vulkan::{
    VulkanError,
    VulkanResult,
    logical_device::LogicalDevice
};

pub struct Semaphore {
    vk_semaphore: vk::Semaphore,
    logical_device: Rc<LogicalDevice>
}

impl Semaphore {
    pub fn new(logical_device: Rc<LogicalDevice>) -> VulkanResult<Self> {
        let create_info = vk::SemaphoreCreateInfo::builder();

        let vk_semaphore = unsafe {
            logical_device.create_semaphore(&create_info, None)
        }.map_err(|result| VulkanError::SemaphoreCreateError {result})?;

        Ok(Self {
            vk_semaphore,
            logical_device
        })
    }

    pub fn handle(&self) -> vk::Semaphore {
        self.vk_semaphore
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_semaphore(self.vk_semaphore, None);
        }
    }
}

pub struct Fence {
    vk_fence: vk::Fence,
    logical_device: Rc<LogicalDevice>
}

impl Fence {
    pub fn new(logical_device: Rc<LogicalDevice>, status: FenceStatus) -> VulkanResult<Self> {
        let flags = Self::create_flags(status);
        let create_info = vk::FenceCreateInfo::builder()
            .flags(flags);

        let vk_fence = unsafe {
            logical_device.create_fence(&create_info, None)
        }.map_err(|result| VulkanError::FenceCreateError {result})?;

        Ok(Self {
            vk_fence,
            logical_device
        })
    }

    fn create_flags(status: FenceStatus) -> vk::FenceCreateFlags {
        if status == FenceStatus::Ready {
            vk::FenceCreateFlags::SIGNALED
        }
        else {
            vk::FenceCreateFlags::empty()
        }
    }

    pub fn handle(&self) -> vk::Fence {
        self.vk_fence
    }

    pub fn status(&self) -> VulkanResult<FenceStatus> {
        let status = unsafe {
            self.logical_device
                .get_fence_status(self.vk_fence)
        };

        match status {
            Ok(_) => Ok(FenceStatus::Ready),
            Err(vk::Result::NOT_READY) => Ok(FenceStatus::NotReady),
            Err(result) => Err(VulkanError::FenceGetStatusError {result})
        }
    }

    pub fn reset(&self) -> VulkanResult<()> {
        let fences = [self.vk_fence];
        unsafe {
            self.logical_device.reset_fences(&fences)
        }.map_err(|result| VulkanError::FenceResetError {result})?;

        Ok(())
    }

    pub fn wait(&self, timeout: std::time::Duration) -> VulkanResult<()> {
        let fences = [self.vk_fence];
        let timeout = timeout.as_nanos().try_into()
            .map_err(|_| VulkanError::FenceTimeoutTooLargeError)?;

        unsafe {
            self.logical_device.wait_for_fences(&fences, true, timeout)
        }.map_err(|result| VulkanError::FenceWaitError {result})?;

        Ok(())
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_fence(self.vk_fence, None);
        }
    }
}

#[derive(PartialEq)]
pub enum FenceStatus {
    Ready,
    NotReady
}
