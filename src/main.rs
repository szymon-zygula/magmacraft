#[macro_use]
mod c_string_collection;
mod double_type_buffer;
mod window;
mod rendering;
mod vulkan;
mod debugging;
mod builder;

use std::rc::Rc;
use std::cell::RefCell;
use window::{
    Window,
    WindowSize
};
use rendering::{
    RenderingResult,
    renderer::Renderer,
    render_state::{
        RenderState,
        PushConstants,
        VertexShader,
        FragmentShader
    }
};

#[repr(C)]
struct Positions {
    number: [f32; 4]
}

impl PushConstants for Positions {}

fn main() -> RenderingResult<()> {
    let window = Rc::new(RefCell::new(Window::builder()
        .title("Magmacraft")
        .size(WindowSize { width: 800, height: 600 })
        .build()
        .expect("failed to create game window")));

    let mut renderer = Renderer::new(Rc::clone(&window)).unwrap();
    let vertex_shader = VertexShader::from_file(Rc::clone(renderer.logical_device()), &std::path::Path::new("shaders/triangle.vert.spv")).unwrap();
    let fragment_shader = FragmentShader::from_file(Rc::clone(renderer.logical_device()), &std::path::Path::new("shaders/triangle.frag.spv")).unwrap();
    let mut render_state = RenderState::<(), Positions, ()>::builder()
        .renderer(&renderer)
        .vertex_shader(&vertex_shader)
        .fragment_shader(&fragment_shader)
        .build()?;
    let mut render_state2 = RenderState::<(), Positions, ()>::builder()
        .renderer(&renderer)
        .vertex_shader(&vertex_shader)
        .fragment_shader(&fragment_shader)
        .build()?;

    let mut window = window.borrow_mut();
    let mut x: f32 = 0.0;

    while window.loop_condition() {
        x += 0.001;
        let number = Positions { number: [x.tan() * x.tan(), x.cos(), x.cos().cos().sin(), 0.0] };
        let number2 = Positions { number: [x.cos() * x.sin(), x.tan().sin(), x.sin(), x.sin() / 2.0] };
        render_state.push_vertex_constants(number);
        render_state2.push_vertex_constants(number2);
        renderer.render(&[&render_state, &render_state2])?;
        window.poll_events();
    }

    Ok(())
}
