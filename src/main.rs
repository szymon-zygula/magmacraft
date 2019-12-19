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

fn main() -> Result<(), renderer::RendererError> {
    let window = Rc::new(RefCell::new(Window::builder()
        .title("Magmacraft")
        .size(WindowSize { width: 800, height: 600 })
        .build()
        .expect("failed to create game window")));

    let renderer = Renderer::new(Rc::clone(&window));
    if let Err(err) = renderer {
        eprintln!("{}", err);
        return Err(err);
    }

    let mut window = window.borrow_mut();

    while window.loop_condition() {
        window.poll_events();
    }

    Ok(())
}
