#[macro_use]
mod c_string_collection;
mod window;
mod renderer;
mod vulkan;
mod debugging;
mod builder;

use window::{Window, WindowSize};
use renderer::Renderer;

fn main() -> Result<(), renderer::RendererError> {
    let mut window = Window::builder()
        .title("Magmacraft")
        .size(WindowSize { width: 800, height: 600 })
        .build()
        .expect("failed to create game window");

    let _renderer = Renderer::new(&window)?;

    while window.loop_condition() {
        window.poll_events();
    }

    Ok(())
}
