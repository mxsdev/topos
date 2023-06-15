use core::panic;
use std::time::Duration;

use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    element::RootConstructor,
    input::{input_state::InputState, winit::WinitState},
    scene::scene::Scene,
    surface::RenderSurface,
};

pub struct App<Root: RootConstructor + 'static> {
    window: winit::window::Window,

    render_surface: RenderSurface,
    scene: Scene<Root>,

    winit_state: WinitState,
    input_state: InputState,
}

impl<Root: RootConstructor + 'static> App<Root> {
    pub fn run(mut self, event_loop: EventLoop<()>) {
        use std::time::*;

        let mut last_render_duration: Option<Duration> = None;
        let mut last_render_time: Option<Instant> = None;

        event_loop.run(move |event, _, control_flow| {
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

                    WindowEvent::Resized(physical_size) => self.resize(*physical_size, None),

                    WindowEvent::ScaleFactorChanged {
                        new_inner_size,
                        scale_factor,
                    } => self.resize(**new_inner_size, Some(*scale_factor)),

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
                            let elapsed_time = last_render_time.elapsed();

                            let buffer_duration = last_render_duration + Duration::from_micros(0);

                            if elapsed_time < (frame_time.saturating_sub(buffer_duration)) {
                                do_render = false;
                            }
                        }
                    }

                    if do_render {
                        let render_start_time = Instant::now();

                        let texture_block_start = Instant::now();
                        // let output = self.render_surface.surface().get_current_texture();
                        let output = self.render_surface.get_output();
                        let texture_block_time = texture_block_start.elapsed();
                        // log::trace!("texture block time: {:?}", texture_block_time);

                        match output {
                            Ok(output) => {
                                let start = Instant::now();

                                let raw_input = self.winit_state.take_egui_input();

                                let input_state = std::mem::take(&mut self.input_state)
                                    .begin_frame(raw_input, true);

                                let (result_input, result_output) =
                                    self.scene.render(&self.render_surface, output, input_state);

                                self.input_state = result_input;

                                self.winit_state
                                    .handle_platform_output(&self.window, result_output);

                                last_render_time = Some(start);
                                last_render_duration = Some(start.elapsed());
                            }
                            // Reconfigure the surface if lost
                            Err(wgpu::SurfaceError::Lost) => self
                                .render_surface
                                .reconfigure(self.scene.get_dependents_mut()),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => panic!("out of memory"),
                            // All other errors (Outdated, Timeout) should be resolved by the next frame
                            Err(e) => eprintln!("{:?}", e),
                        }

                        let render_time = render_start_time.elapsed();
                        // log::trace!("render_time: {:?}", render_time);
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

    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new().build(event_loop).unwrap();

        let render_surface = RenderSurface::new(&window).await;
        let rendering_context = render_surface.clone_rendering_context();

        let scene = Scene::new(rendering_context, window.scale_factor());

        let winit_state = WinitState::new(&window);
        let input_state = InputState::default().into();

        Self {
            window,

            render_surface,
            scene,

            winit_state,
            input_state,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        self.render_surface
            .resize(new_size, scale_factor, self.scene.get_dependents_mut());
    }
}

fn get_window_frame_time(window: &winit::window::Window) -> Option<std::time::Duration> {
    let monitor = window.current_monitor()?;

    let frame_rate = monitor.refresh_rate_millihertz()? as f64 / 1000.;

    return Some(Duration::from_secs_f64(1. / frame_rate));
}
