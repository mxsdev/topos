use core::panic;
use std::{
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use futures::channel::mpsc::SendError;
use itertools::Itertools;
use tao::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use std::sync::mpsc::{self, Receiver, Sender};

use crate::{scene::Scene, surface::RenderSurface, util::AsWinit};

pub struct App {
    event_loop: EventLoop<()>,
    window: tao::window::Window,

    render_surface: RenderSurface,
    scene: Scene,
}

impl App {
    pub fn run(mut self) {
        use std::time::*;

        let mut last_render_duration: Option<Duration> = None;
        let mut last_render_time: Option<Instant> = None;

        self.event_loop.run(move |event, _, control_flow| {
            let mut send_update_to_scene = false;

            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                    ..
                } if window_id == self.window.id() => match event {
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

                    WindowEvent::Resized(physical_size) => {
                        self.render_surface.resize(*physical_size, None)
                    }

                    WindowEvent::ScaleFactorChanged {
                        new_inner_size,
                        scale_factor,
                    } => self
                        .render_surface
                        .resize(**new_inner_size, Some(*scale_factor)),

                    _ => send_update_to_scene = true,
                },
                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                    let mut do_render = true;

                    if let (Some(last_render_duration), Some(last_render_time)) =
                        (last_render_duration, last_render_time)
                    {
                        if let Some(frame_time) = get_window_frame_time(&self.window) {
                            let elapsed_time = Instant::now().duration_since(last_render_time);

                            let buffer_duration = last_render_duration + Duration::from_micros(0);

                            if elapsed_time < (frame_time.saturating_sub(buffer_duration)) {
                                do_render = false;
                            }
                        }
                    }

                    if do_render {
                        let render_start_time = Instant::now();

                        let output = self.render_surface.surface().get_current_texture();

                        match output {
                            Ok(output) => {
                                let start = Instant::now();
                                self.scene.render(&self.render_surface, output);
                                let end = Instant::now();

                                last_render_time = Some(start);
                                last_render_duration = Some(end.duration_since(start));
                            }
                            // Reconfigure the surface if lost
                            Err(wgpu::SurfaceError::Lost) => self.render_surface.reconfigure(),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => panic!("out of memory"),
                            // All other errors (Outdated, Timeout) should be resolved by the next frame
                            Err(e) => eprintln!("{:?}", e),
                        }

                        let render_time = Instant::now().duration_since(render_start_time);
                        log::trace!("rendered; lag: {:?}", render_time);
                    }
                }
                Event::MainEventsCleared => {
                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    self.window.request_redraw()
                }
                _ => send_update_to_scene = true,
            }

            if send_update_to_scene {
                self.scene
                    .handle_window_event(event, self.window.scale_factor());
            }
        });
    }

    pub async fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        let render_surface = RenderSurface::new(&window).await;
        let rendering_context = render_surface.clone_rendering_context();

        let scene = Scene::new(rendering_context);

        Self {
            event_loop,
            window,

            render_surface,
            scene,
        }
    }

    pub fn resize(&mut self, new_size: tao::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        self.render_surface.resize(new_size, scale_factor);
    }

    pub fn get_size(&self) -> tao::dpi::PhysicalSize<u32> {
        self.render_surface.get_size()
    }
}

fn get_window_frame_time(window: &tao::window::Window) -> Option<std::time::Duration> {
    let monitor_tao = window.current_monitor()?;
    let monitor = unsafe { monitor_tao.as_winit() };

    let frame_rate = monitor.refresh_rate_millihertz()? as f64 / 1000.;

    return Some(Duration::from_secs_f64(1. / frame_rate));
}
