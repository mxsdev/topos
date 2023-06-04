use core::panic;
use std::time::Duration;

use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    element::TestElement,
    input::{self, input_state::InputState, winit::WinitState},
    scene::scene::Scene,
    surface::RenderSurface,
};

type RootElement = TestElement;

pub struct App {
    event_loop: EventLoop<()>,
    window: winit::window::Window,

    render_surface: RenderSurface,
    scene: Scene<RootElement>,

    winit_state: WinitState,
    input_state: InputState,
}

impl App {
    pub fn run(mut self) {
        use std::time::*;

        let mut last_render_duration: Option<Duration> = None;
        let mut last_render_time: Option<Instant> = None;

        self.event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                    ..
                } if window_id == self.window.id() => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
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

                    e => {
                        let _ = self.winit_state.on_event(e);
                    }
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

                        let raw_input = self.winit_state.take_egui_input();

                        let output = self.render_surface.surface().get_current_texture();

                        let input_state =
                            std::mem::take(&mut self.input_state).begin_frame(raw_input, true);

                        match output {
                            Ok(output) => {
                                let start = Instant::now();

                                self.scene.render(&self.render_surface, output, input_state);
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
                        // log::trace!("rendered; lag: {:?}", render_time);
                    }
                }
                Event::MainEventsCleared => {
                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    self.window.request_redraw()
                }

                // Event::UserEvent(UserEvent::AccessKitActionRequest(
                //     accesskit_winit::ActionRequestEvent { request, .. },
                // )) => {
                //     self.winit_state
                //         .on_accesskit_action_request(request.clone());
                // }
                _ => {}
            }
        });
    }

    pub async fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        let render_surface = RenderSurface::new(&window).await;
        let rendering_context = render_surface.clone_rendering_context();

        let root = TestElement::new();

        let scene = Scene::new(rendering_context, root);

        let winit_state = WinitState::new(&window);
        let input_state = InputState::default().into();

        Self {
            event_loop,
            window,

            render_surface,
            scene,

            winit_state,
            input_state,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        self.render_surface.resize(new_size, scale_factor);
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.render_surface.get_size()
    }
}

fn get_window_frame_time(window: &winit::window::Window) -> Option<std::time::Duration> {
    let monitor = window.current_monitor()?;
    // let monitor = unsafe { monitor_tao.as_winit() };

    let frame_rate = monitor.refresh_rate_millihertz()? as f64 / 1000.;

    return Some(Duration::from_secs_f64(1. / frame_rate));
}
