use core::panic;
use std::{sync::Once, time::Duration};

use muda::{icon::Icon, AboutMetadata, MenuItem, PredefinedMenuItem, Submenu};
use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    element::{ElementRef, RootConstructor},
    input::{input_state::InputState, winit::WinitState},
    scene::scene::Scene,
    surface::{RenderAttachment, RenderSurface},
};

pub struct App<Root: RootConstructor + 'static> {
    swap_chain: Option<RenderAttachment>,

    render_surface: RenderSurface,
    scene: Scene<Root>,

    winit_state: WinitState,
    input_state: InputState,

    queued_resize: Option<(winit::dpi::PhysicalSize<u32>, Option<f64>)>,

    window: winit::window::Window,
}

#[derive(Debug)]
pub enum ToposEvent {
    Exit(i32),
    AccessKitActionRequest(accesskit_winit::ActionRequestEvent),
}

impl From<accesskit_winit::ActionRequestEvent> for ToposEvent {
    fn from(value: accesskit_winit::ActionRequestEvent) -> Self {
        Self::AccessKitActionRequest(value)
    }
}

pub type ToposEventLoop = EventLoop<ToposEvent>;

impl<Root: RootConstructor + 'static> App<Root> {
    pub fn run(mut self, event_loop: ToposEventLoop) {
        use std::time::*;

        let mut last_render_duration: Option<Duration> = None;
        let mut last_render_time: Option<Instant> = None;

        let main_proxy = event_loop.create_proxy();

        let about = PredefinedMenuItem::about(
            Some("About"),
            None,
            // Some(AboutMetadata {
            //     name: Some("Test".into()),
            //     version: Some("Version 1.0".into()),
            //     short_version: Some("1.0".into()),
            //     copyright: Some("copyright beans industries".into()),
            //     credits: Some("shoutouts to my fwiends".into()),
            //     icon: Some(Icon::from_rgba(vec![255; 4 * 512 * 512], 512, 512).unwrap()),
            //     ..Default::default()
            // }),
        );

        let menu_item2 = MenuItem::new("Menu item #2", false, None);

        let submenu_help = Submenu::new("Help", true);

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                    ..
                } if window_id == self.window.id() => {
                    let _ = self.winit_state.on_event(event, &self.window);

                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            main_proxy.send_event(ToposEvent::Exit(0)).unwrap();
                        }

                        WindowEvent::Resized(physical_size) => self.resize(*physical_size, None),

                        WindowEvent::ScaleFactorChanged {
                            new_inner_size,
                            scale_factor,
                        } => self.resize(**new_inner_size, Some(*scale_factor)),

                        _ => {}
                    }
                }

                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                    let output = match self.swap_chain.take() {
                        None => {
                            match self.render_surface.get_output() {
                                Ok(output) => {
                                    self.swap_chain = output.into();
                                    last_render_time = Instant::now().into();
                                }
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => self
                                    .render_surface
                                    .reconfigure(self.scene.get_dependents_mut()),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => panic!("out of memory"),
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => {
                                    eprintln!("{:?}", e);
                                }
                            }

                            return;
                        }

                        Some(output) => output,
                    };

                    if let (Some(last_render_duration), Some(last_render_time)) =
                        (last_render_duration, last_render_time)
                    {
                        if let Some(frame_time) = get_window_frame_time(&self.window) {
                            let elapsed_time = last_render_time.elapsed();

                            let buffer_duration = last_render_duration + Duration::from_micros(200);

                            if elapsed_time < (frame_time.saturating_sub(buffer_duration)) {
                                self.swap_chain = Some(output);
                                return;
                            }
                        }
                    }

                    let render_start_time = Instant::now();

                    let start = Instant::now();

                    let raw_input = self.winit_state.take_egui_input();

                    let input_state =
                        std::mem::take(&mut self.input_state).begin_frame(raw_input, true);

                    let (result_input, result_output) =
                        self.scene.render(&self.render_surface, output, input_state);

                    self.input_state = result_input;

                    self.winit_state
                        .handle_platform_output(&self.window, result_output);

                    last_render_duration = Some(start.elapsed());

                    let render_time = render_start_time.elapsed();
                    // log::trace!("render_time: {:?}", render_time);

                    if let Some((new_size, scale_fac)) = self.queued_resize.take() {
                        self.resize(new_size, scale_fac);
                    }
                }

                Event::RedrawEventsCleared => {}

                Event::MainEventsCleared => {
                    static START: Once = Once::new();

                    START.call_once(|| {
                        use muda::{accelerator::*, *};

                        let submenu = Submenu::with_items("Root", true, &[&menu_item2, &about]);

                        let menu_item3 = MenuItem::new("Menu item #3", false, None);
                        let submenu2 = Submenu::with_items("Test", true, &[&menu_item2]);

                        submenu_help.append(&menu_item3);

                        let menu = Menu::with_items(&[&submenu, &submenu2, &submenu_help]);

                        #[cfg(target_os = "macos")]
                        menu.set_help_menu_for_nsapp(Some(&submenu_help));

                        // let submenu = Submenu::with_items(
                        //     "Submenu Outer",
                        //     true,
                        //     &[
                        //         &MenuItem::new(
                        //             "Menu item #1",
                        //             true,
                        //             Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyD)),
                        //         ),
                        //         &PredefinedMenuItem::separator(),
                        //         &menu_item2,
                        //         &MenuItem::new("Menu item #3", true, None),
                        //         &PredefinedMenuItem::separator(),
                        //         &Submenu::with_items(
                        //             "Submenu Inner",
                        //             true,
                        //             &[
                        //                 &MenuItem::new("Submenu item #1", true, None),
                        //                 &PredefinedMenuItem::separator(),
                        //                 &menu_item2,
                        //             ],
                        //         ),
                        //     ],
                        // );

                        menu.init_for_nsapp();
                    });

                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    self.window.request_redraw();
                }

                Event::UserEvent(ToposEvent::Exit(code)) => {
                    // should do any de-init logic here

                    *control_flow = ControlFlow::ExitWithCode(code);
                }

                Event::UserEvent(ToposEvent::AccessKitActionRequest(
                    accesskit_winit::ActionRequestEvent { request, .. },
                )) => {
                    self.winit_state
                        .on_accesskit_action_request(request.clone());
                }

                _ => {}
            }
        });
    }

    pub async fn new(event_loop: &ToposEventLoop) -> Self {
        let window = WindowBuilder::new().build(event_loop).unwrap();

        let render_surface = RenderSurface::new(&window).await;
        let rendering_context = render_surface.clone_rendering_context();

        let mut scene = Scene::new(rendering_context, window.scale_factor());

        let winit_state_proxy = event_loop.create_proxy();

        let root_id = scene.root_id().as_access_id();
        let root_node = scene.root_access_node();

        let winit_state =
            WinitState::new(&window, winit_state_proxy, move || accesskit::TreeUpdate {
                tree: Some(accesskit::Tree::new(root_id)),
                nodes: vec![(root_id, root_node)],
                ..Default::default()
            });

        let input_state = InputState::default().into();

        Self {
            window,

            render_surface,
            scene,

            winit_state,
            input_state,

            swap_chain: None,
            queued_resize: None,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: Option<f64>) {
        if self.swap_chain.is_some() {
            self.queued_resize = Some((
                new_size,
                scale_factor.or(self.queued_resize.map(|(_, sf)| sf).flatten()),
            ));
            return;
        }

        self.render_surface
            .resize(new_size, scale_factor, self.scene.get_dependents_mut());
    }
}

fn get_window_frame_time(window: &winit::window::Window) -> Option<std::time::Duration> {
    let monitor = window.current_monitor()?;

    let frame_rate = monitor.refresh_rate_millihertz()? as f64 / 1000.;

    return Some(Duration::from_secs_f64(1. / frame_rate));
}
