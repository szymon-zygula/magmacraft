use std::rc::Rc;

use ash::{self, vk};

use crate::{
    vulkan::{
        self,
        VulkanError
    }
};

pub struct DebugMessenger {
    debug_utils_loader: Rc<ash::extensions::ext::DebugUtils>,
    // used to keep instance alive so that it isnt' dropped before `DebugMessenger`
    _instance: Rc<vulkan::instance::Instance>,
    debug_messenger: vk::DebugUtilsMessengerEXT
}

impl DebugMessenger {
    pub fn new(debug_utils_loader: Rc<ash::extensions::ext::DebugUtils>, instance: Rc<vulkan::instance::Instance>) -> Result<Self, VulkanError> {
        let debug_messenger_create_info = Self::get_create_info();

        let debug_messenger = unsafe { debug_utils_loader
            .create_debug_utils_messenger(&debug_messenger_create_info, None)
            .map_err(VulkanError::operation_failed_mapping("create debug messenger"))?
        };

        Ok(DebugMessenger {
            debug_utils_loader,
            _instance: instance,
            debug_messenger
        })
    }

    pub fn get_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        let message_severity =
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;

        let message_type =
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL |
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION |
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;

        let debug_messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(message_severity)
            .message_type(message_type)
            .pfn_user_callback(Some(debug_callback));

        *debug_messenger_create_info
    }
}

impl Drop for DebugMessenger {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
        }
    }
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void) -> vk::Bool32 {
    let message_severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "info",
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "verbose",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "warning",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "error",
        _ => "Unknown severity"
    };

    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "general",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "validation",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "performance",
        _ => "Unknown type"
    };

    let message = std::ffi::CStr::from_ptr((*callback_data).p_message)
        .to_str().unwrap();

    eprintln!("VL {} ({}): {}", message_severity, message_type, message);

    vk::FALSE
}

create_c_string_collection_type!(ValidationLayers);
