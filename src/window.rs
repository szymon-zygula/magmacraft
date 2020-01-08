use custom_error::custom_error;
use glfw::{
    self,
    Context
};
use crate::{
    vulkan,
    builder::*
};

custom_error!{pub WindowError
    GlfwInitializationError { source: glfw::InitError } = "failed to initialize GLFW",
    CreateError = "failed to create GLFW window"
}

type WindowResult<T> = Result<T, WindowError>;

pub struct Window {
    glfw_instance: glfw::Glfw,
    glfw_window: glfw::Window,
    event_receiver: std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>
}

impl Window {
    pub fn builder() -> WindowBuilder {
        WindowBuilder {
            ..Default::default()
        }
    }

    pub fn loop_condition(&mut self) -> bool {
        !self.glfw_window.should_close()
    }

    pub fn poll_events(&mut self) {
        self.glfw_instance.poll_events();
        for (_, event) in glfw::flush_messages(&self.event_receiver) {
            Self::match_event(&event, &mut self.glfw_window);
        }
    }

    fn match_event(event: &glfw::WindowEvent, glfw_window: &mut glfw::Window) {
        match event {
            glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
                glfw_window.set_should_close(true);
            },
            _ => {}
        }
    }

    pub fn required_vulkan_extensions(&self) -> vulkan::instance::InstanceExtensions {
        let a = self.glfw_instance
            .get_required_instance_extensions()
            .unwrap_or(Vec::new());

        vulkan::instance::InstanceExtensions::from_vec(a)
    }

    pub fn raw_handle(&self) -> *mut glfw::ffi::GLFWwindow {
        self.glfw_window.window_ptr()
    }

    pub fn framebuffer_size(&self) -> (u32, u32) {
        let (width, height) = self.glfw_window.get_framebuffer_size();
        (width as u32, height as u32)
    }
}

#[derive(Default)]
pub struct WindowBuilder {
    size: BuilderRequirement<WindowSize>,
    title: BuilderRequirement<String>,

    glfw_instance: BuilderInternal<glfw::Glfw>,
    glfw_window: BuilderInternal<glfw::Window>,
    event_receiver: BuilderInternal<std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>>,

    window: BuilderProduct<Window>
}

impl WindowBuilder {
    pub fn size(mut self, size: WindowSize) -> Self {
        self.size.set(size);
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title.set(String::from(title));
        self
    }

    pub fn build(mut self) -> WindowResult<Window> {
        self.ready_for_creation()?;
        self.create_window();

        Ok(self.window.unwrap())
    }

    fn ready_for_creation(&mut self) -> WindowResult<()> {
        self.init_glfw_instance()?;
        self.set_window_hints();
        self.init_glfw_window_and_receiver()?;
        self.set_window_options();
        Ok(())
    }

    fn init_glfw_instance(&mut self) -> WindowResult<()> {
        self.glfw_instance.set(glfw::init(glfw::FAIL_ON_ERRORS)?);
        Ok(())
    }

    fn set_window_hints(&mut self) {
        self.glfw_instance.window_hint(
            glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
        self.glfw_instance.window_hint(
            glfw::WindowHint::Resizable(false));
    }

    fn init_glfw_window_and_receiver(&mut self) -> WindowResult<()> {
        let width = self.size.width;
        let height = self.size.height;

        let window_creation = self.glfw_instance
            .create_window(width, height, &self.title, glfw::WindowMode::Windowed);

        let (glfw_window, event_receiver) = match window_creation {
            Some(window_and_receiver) => window_and_receiver,
            None => return Err(WindowError::CreateError)
        };

        self.glfw_window.set(glfw_window);
        self.event_receiver.set(event_receiver);

        Ok(())
    }

    fn set_window_options(&mut self) {
        self.glfw_window.as_mut().set_key_polling(true);
    }

    fn create_window(&mut self) {
        self.window.set(Window {
            glfw_window: self.glfw_window.take(),
            glfw_instance: self.glfw_instance.take(),
            event_receiver: self.event_receiver.take()
        })
    }
}

pub struct WindowSize {
    pub width: u32,
    pub height: u32
}
