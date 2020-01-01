use std::rc::Rc;
use std::cell::RefCell;
use ash::{
    self,
    vk::{self, Handle}
};
use glfw;
use crate::{
    window::Window,
    vulkan::state::VulkanState
};

pub struct Surface {
    vk_surface: vk::SurfaceKHR,
    vulkan_state: Rc<VulkanState>,
    window: Rc<RefCell<Window>>
}

impl Surface {
    pub fn new(window: Rc<RefCell<Window>>, vulkan_state: Rc<VulkanState>) -> Self {
        let vk_surface = Self::create_window_surface(&window.borrow(), &vulkan_state);
        Surface {
            vk_surface,
            vulkan_state,
            window
        }
    }

    fn create_window_surface(window: &Window, vulkan_state: &VulkanState) -> vk::SurfaceKHR {
        let raw_window_handle = window.get_raw_handle();
        let raw_instance_handle = vulkan_state.get_raw_instance_handle();
        let raw_vk_surface =
            Self::create_raw_window_surface(raw_window_handle, raw_instance_handle);

        vk::SurfaceKHR::from_raw(raw_vk_surface)
    }

    fn create_raw_window_surface(
        raw_window_handle: *mut glfw::ffi::GLFWwindow,
        raw_instance_handle: u64
    ) -> u64 {
        let mut raw_vk_surface: u64 = unsafe {
            std::mem::MaybeUninit::uninit().assume_init()
        };

        unsafe {
            glfw::ffi::glfwCreateWindowSurface(
                raw_instance_handle as usize,
                raw_window_handle,
                std::ptr::null(),
                &mut raw_vk_surface as *mut u64);
        }

        raw_vk_surface
    }

    pub fn get_handle(&self) -> vk::SurfaceKHR {
        self.vk_surface
    }

    pub fn get_framebuffer_extent(&self) -> vk::Extent2D {
        let (width, height) = self.window.borrow().get_framebuffer_size();

        *vk::Extent2D::builder()
            .width(width)
            .height(height)
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
