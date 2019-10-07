use std::rc::Rc;
use ash::{
    self,
    vk::{self, Handle},
    version::InstanceV1_0
};
use glfw;
use crate::{
    window::Window,
    vulkan
};

pub struct Surface {
    vk_surface: vk::SurfaceKHR,
    surface_loader: Rc<ash::extensions::khr::Surface>
}

impl Surface {
    pub fn new(window: &Window, vulkan_state: &vulkan::state::VulkanState) -> Self {
        let raw_window_handle = window.get_raw_handle();
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
            surface_loader: vulkan_state.get_surface_loader()
        }
    }

    pub fn get_handle(&self) -> vk::SurfaceKHR {
        self.vk_surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.vk_surface, None);
        }
    }
}
