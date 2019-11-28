use std::rc::Rc;
use std::cell::RefCell;
use ash::{
    self,
    vk::{self, Handle}
};
use glfw;
use crate::{
    window::Window,
    vulkan
};

pub struct Surface {
    vk_surface: vk::SurfaceKHR,
    vulkan_state: Rc<vulkan::state::VulkanState>,
    // lifetime extenders
    _window: Rc<RefCell<Window>>
}

impl Surface {
    pub fn new(window: Rc<RefCell<Window>>, vulkan_state: Rc<vulkan::state::VulkanState>) -> Self {
        let raw_window_handle = window.borrow().get_raw_handle();
        let raw_instance_handle = vulkan_state.get_raw_instance_handle();
        let mut raw_vk_surface: u64 = unsafe { std::mem::MaybeUninit::uninit().assume_init() };

        unsafe {
            glfw::ffi::glfwCreateWindowSurface(
                raw_instance_handle as usize,
                raw_window_handle,
                std::ptr::null(),
                &mut raw_vk_surface as *mut u64
            );
        }

        Surface {
            vk_surface: vk::SurfaceKHR::from_raw(raw_vk_surface),
            vulkan_state: Rc::clone(&vulkan_state),
            _window: Rc::clone(&window)
        }
    }

    pub fn get_handle(&self) -> vk::SurfaceKHR {
        self.vk_surface
    }

    pub unsafe fn is_supported_by_vk_device(
        &self, physical_device: vk::PhysicalDevice, queue_family_index: u32
        ) -> bool {
        self.vulkan_state.get_surface_loader().get_physical_device_surface_support(
            physical_device,
            queue_family_index,
            self.vk_surface
        )
    }
}

impl std::ops::Deref for Surface {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.vk_surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_state.get_surface_loader().destroy_surface(self.vk_surface, None);
        }
    }
}
