#[macro_use]
mod c_string_collection;
mod double_type_buffer;
mod window;
mod renderer;
mod vulkan;
mod debugging;
mod builder;

use std::rc::Rc;
use std::cell::RefCell;
use window::{Window, WindowSize};
use renderer::Renderer;

fn main() -> Result<(), renderer::RenderError> {
    let window = Rc::new(RefCell::new(Window::builder()
        .title("Magmacraft")
        .size(WindowSize { width: 800, height: 600 })
        .build()
        .expect("failed to create game window")));

    let mut renderer = Renderer::new(Rc::clone(&window)).unwrap();

    let mut window = window.borrow_mut();

    while window.loop_condition() {
        renderer.render()?;
        window.poll_events();
    }

    Ok(())
}
