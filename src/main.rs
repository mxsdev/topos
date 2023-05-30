#![feature(return_position_impl_trait_in_trait)]
// #![feature(new_uninit)]
// #![feature(maybe_uninit_write_slice)]

mod atlas;
mod buffer;
mod debug;
mod element;
mod graphics;
mod hash;
mod num;
mod paint;
mod shape;
mod surface;
mod text;
mod time;
mod util;

use pollster::FutureExt;
use tao::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = surface::State::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        // asd
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
                ..
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: tao::keyboard::KeyCode::Escape,
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => state.resize(*physical_size, None),
                WindowEvent::ScaleFactorChanged {
                    new_inner_size,
                    scale_factor,
                } => state.resize(**new_inner_size, Some(*scale_factor)),
                e => state.input(e),
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();

                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.get_size(), None),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw()
            }
            _ => {}
        }
    });
}

fn main() {
    pretty_env_logger::formatted_builder()
        .parse_env(env_logger::Env::default().filter_or(
            env_logger::DEFAULT_FILTER_ENV,
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "warn"
            },
        ))
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("cosmic_text", log::LevelFilter::Warn)
        .filter_module("tao", log::LevelFilter::Warn)
        .init();

    run().block_on();
}
