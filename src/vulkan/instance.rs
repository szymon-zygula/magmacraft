use std::rc::Rc;
use ash::{
    self,
    vk_make_version,
    vk::{
        self,
        Handle
    },
    version::{
        InstanceV1_0,
        EntryV1_0
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
        debug_utils::ValidationLayers
    }
};


pub struct Instance {
    vk_instance: ash::Instance
}

impl Instance {
    pub fn builder() -> InstanceBuilder {
        InstanceBuilder {
            ..Default::default()
        }
    }

    pub fn handle(&self) -> &ash::Instance {
        &self.vk_instance
    }

    pub fn raw_handle(&self) -> u64 {
        self.vk_instance.handle().as_raw()
    }
}

impl AsRef<ash::Instance> for Instance {
    fn as_ref(&self) -> &ash::Instance {
        &self.vk_instance
    }
}

impl std::ops::Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.vk_instance
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.vk_instance.destroy_instance(None);
        }
    }
}

#[derive(Default)]
pub struct InstanceBuilder {
    entry: BuilderRequirement<Rc<ash::Entry>>,
    version: BuilderRequirement<u32>,
    name: BuilderRequirement<String>,
    extensions: InstanceExtensions,
    validation_layers: ValidationLayers,

    debug_mode: BuilderInternal<bool>,
    c_name: BuilderInternal<std::ffi::CString>,
    app_info: BuilderInternal<vk::ApplicationInfo>,
    instance_create_info: BuilderInternal<vk::InstanceCreateInfo>,
    debug_messenger_create_info: BuilderInternal<vk::DebugUtilsMessengerCreateInfoEXT>,

    instance: BuilderProduct<Instance>
}

impl InstanceBuilder {
    pub fn entry(mut self, entry: Rc<ash::Entry>) -> Self {
        self.entry.set(entry);
        self
    }

    pub fn version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.version.set(vk_make_version!(major, minor, patch));
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name.set(String::from(name));
        self
    }

    pub fn extensions(mut self, extensions: InstanceExtensions) -> Self {
        self.extensions = extensions;
        self
    }

    pub fn validation_layers(mut self, validation_layers: ValidationLayers) -> Self {
        self.validation_layers = validation_layers;
        self
    }

    pub fn build(mut self) -> VulkanResult<Instance> {
        self.get_ready_for_creation()?;
        self.create_instance()?;
        Ok(self.instance.unwrap())
    }

    fn get_ready_for_creation(&mut self) -> VulkanResult<()> {
        self.init_debug_information()?;
        self.init_app_info()?;
        self.init_instance_create_info()?;
        Ok(())
    }

    fn init_debug_information(&mut self) -> VulkanResult<()> {
        let is_debugging = self.validation_layers.len() != 0;
        self.debug_mode.set(is_debugging);

        if is_debugging {
            self.check_if_validation_layers_are_available()?;
            self.init_debug_messenger_create_info();
        }

        Ok(())
    }

    fn check_if_validation_layers_are_available(&self) -> VulkanResult<()> {
        let properties = self.validation_layer_properties()?;

        for layer in self.validation_layers.strings() {
            if !Self::is_validation_layer_in_properties(&layer, &properties) {
                return Err(VulkanError::ValidationLayersNotAvailable);
            }
        }

        Ok(())
    }

    fn validation_layer_properties(&self) -> VulkanResult<Vec<vk::LayerProperties>> {
        let properties = self.entry
            .enumerate_instance_layer_properties()
            .map_err(|result| VulkanError::ValidationLayersError {result})?;

        Ok(properties)
    }

    fn is_validation_layer_in_properties(layer_name: &std::ffi::CStr, properties: &Vec<vk::LayerProperties>) -> bool {
        for property in properties {
            let layer_name_from_properties = unsafe {
                std::ffi::CStr::from_ptr(&property.layer_name as *const std::os::raw::c_char)
            };

            if layer_name_from_properties == layer_name {
                return true;
            }
        }

        false
    }

    fn init_debug_messenger_create_info(&mut self) {
        self.debug_messenger_create_info.set(
            vulkan::debug_utils::DebugMessenger::create_info()
        );
    }

    fn init_app_info(&mut self) -> VulkanResult<()> {
        let c_name = std::ffi::CString::new(self.name.as_bytes())?;
        self.c_name.set(c_name);

        self.app_info.set(*vk::ApplicationInfo::builder()
            .api_version(vk_make_version!(1, 0, 0))
            .application_name(&self.c_name)
            .application_version(*self.version)
            .engine_name(&self.c_name)
            .engine_version(*self.version));

        Ok(())
    }

    fn init_instance_create_info(&mut self) -> VulkanResult<()> {
        let mut instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&self.app_info)
            .enabled_extension_names(self.extensions.pointers())
            .enabled_layer_names(self.validation_layers.pointers())
            .flags(vk::InstanceCreateFlags::empty());

        if *self.debug_mode {
            instance_create_info = instance_create_info
                .push_next(self.debug_messenger_create_info.as_mut());
        }

        self.instance_create_info.set(*instance_create_info);

        Ok(())
    }

    fn create_instance(&mut self) -> VulkanResult<()> {
        let vk_instance = unsafe {
            self.entry.create_instance(
                &self.instance_create_info,
                None
            )?
        };

        self.instance.set(Instance {
            vk_instance
        });

        Ok(())
    }
}

create_c_string_collection_type!(InstanceExtensions);
