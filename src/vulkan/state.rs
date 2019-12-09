use std::rc::Rc;
use ash;
use crate::{
    builder::*,
    vulkan::{
        self,
        VulkanResult,
        instance::InstanceExtensions,
        debug_utils::ValidationLayers,
    }
};

pub struct VulkanState {
    entry: Rc<ash::Entry>,
    instance: Rc<vulkan::instance::Instance>,
    debug_messenger: Option<vulkan::debug_utils::DebugMessenger>,
    debug_utils_loader: Rc<ash::extensions::ext::DebugUtils>,
    surface_loader: Rc<ash::extensions::khr::Surface>
}

impl VulkanState {
    pub fn builder() -> VulkanStateBuilder {
        VulkanStateBuilder {
            ..Default::default()
        }
    }

    pub fn get_entry(&self) -> &ash::Entry {
        &self.entry
    }

    pub fn get_instance(&self) -> Rc<vulkan::instance::Instance> {
        Rc::clone(&self.instance)
    }

    pub fn get_raw_instance_handle(&self) -> u64 {
        self.instance.get_raw_handle()
    }

    pub fn get_debug_utils_loader(&self) -> Rc<ash::extensions::ext::DebugUtils> {
        Rc::clone(&self.debug_utils_loader)
    }

    pub fn get_surface_loader(&self) -> Rc<ash::extensions::khr::Surface> {
        Rc::clone(&self.surface_loader)
    }
}

#[derive(Default)]
pub struct VulkanStateBuilder {
    debug_mode: bool,
    instance_extensions: BuilderRequirement<InstanceExtensions>,

    entry: BuilderInternal<Rc<ash::Entry>>,
    instance: BuilderInternal<Rc<vulkan::instance::Instance>>,
    debug_messenger: BuilderInternal<Option<vulkan::debug_utils::DebugMessenger>>,
    validation_layers: BuilderInternal<ValidationLayers>,
    debug_utils_loader: BuilderInternal<Rc<ash::extensions::ext::DebugUtils>>,
    surface_loader: BuilderInternal<Rc<ash::extensions::khr::Surface>>,

    vulkan_state: BuilderProduct<VulkanState>
}

impl VulkanStateBuilder {
    pub fn debug_mode(mut self, debug_mode: bool) -> Self {
        self.debug_mode = debug_mode;
        self
    }

    pub fn instance_extensions(mut self, extensions: InstanceExtensions) -> Self {
        self.instance_extensions.set(extensions);
        self
    }

    pub fn build(mut self) -> VulkanResult<VulkanState> {
        self.get_ready_for_state_creation()?;
        self.create_state();

        Ok(self.vulkan_state.unwrap())
    }

    fn get_ready_for_state_creation(&mut self) -> VulkanResult<()> {
        self.init_entry()?;
        self.add_instance_debug_extension();
        self.init_instance()?;
        self.init_extension_loaders();
        self.init_debug_messenger()?;

        Ok(())
    }

    fn init_entry(&mut self) -> VulkanResult<()> {
        self.entry.set(Rc::new(ash::Entry::new()?));
        Ok(())
    }

    fn init_instance(&mut self) -> VulkanResult<()> {
        let mut instance_builder = vulkan::instance::Instance::builder()
            .entry(Rc::clone(&self.entry))
            .version(0, 0, 0)
            .name("Magmacraft")
            .extensions(self.instance_extensions.take());

        if self.debug_mode {
            let mut validation_layers = ValidationLayers::with_capacity(1);
            validation_layers.push("VK_LAYER_KHRONOS_validation");
            instance_builder = instance_builder.validation_layers(validation_layers);
        }

        self.instance.set(
            Rc::new(instance_builder.build()?));

        Ok(())
    }

    fn init_extension_loaders(&mut self) {
        let instance_handle = self.instance.get_handle();
        // Builder -> &Rc -> &ash::Entry
        let entry = self.entry.as_ref().as_ref();

        self.debug_utils_loader.set(
            Rc::new(ash::extensions::ext::DebugUtils::new(
                entry, instance_handle
            )
        ));

        self.surface_loader.set(
            Rc::new(ash::extensions::khr::Surface::new(
                entry, instance_handle
            )
        ));
    }

    fn add_instance_debug_extension(&mut self) {
        if self.debug_mode {
            let extension_name = ash::extensions::ext::DebugUtils::name()
                .to_str().unwrap();

            self.instance_extensions.push(extension_name);
        }
    }

    fn init_debug_messenger(&mut self) -> VulkanResult<()> {
        let debug_messenger = if self.debug_mode {
            Some(vulkan::debug_utils::DebugMessenger::new(
                Rc::clone(&self.debug_utils_loader),
                Rc::clone(&self.instance))?)
        }
        else {
            None
        };

        self.debug_messenger.set(debug_messenger);

        Ok(())
    }

    fn create_state(&mut self) {
        self.vulkan_state.set(VulkanState {
            entry: self.entry.take(),
            instance: self.instance.take(),
            debug_utils_loader: self.debug_utils_loader.take(),
            surface_loader: self.surface_loader.take(),
            debug_messenger: self.debug_messenger.take()
        })
    }
}
