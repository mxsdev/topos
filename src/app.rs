use crate::time::Duration;
use core::panic;

use raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    element::{ElementRef, RootConstructor},
    input::{input_state::InputState, winit::WinitState},
    scene::{framepacer::Framepacer, scene::Scene},
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

    framepacer: Framepacer,
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
        use crate::time::*;

        let main_proxy = event_loop.create_proxy();

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
                    self.try_create_new_output();

                    let output = match self.swap_chain.take() {
                        Some(output) => output,
                        None => return,
                    };

                    let (should_render, render_start_time) = self.framepacer.should_render();

                    // if !should_render {
                    //     self.swap_chain = Some(output);
                    //     return;
                    // }

                    let raw_input = self.winit_state.take_egui_input();

                    let input_state =
                        std::mem::take(&mut self.input_state).begin_frame(raw_input, true);

                    let (mut result_input, result_output) =
                        self.scene.render(&self.render_surface, output, input_state);

                    let render_finish_time = crate::time::Instant::now();
                    let render_time = render_finish_time.duration_since(render_start_time);

                    if self.framepacer.check_missed_deadline(render_finish_time) {
                        // log::debug!("  missed deadline render time: {:?}", render_time);
                    }

                    result_input.end_frame();

                    // self.try_create_new_output();

                    self.framepacer.push_frametime(render_time);

                    self.input_state = result_input;

                    self.winit_state
                        .handle_platform_output(&self.window, result_output);
                }

                Event::MainEventsCleared => {
                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    self.window.request_redraw()
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

    fn try_create_new_output(&mut self) {
        if self.swap_chain.is_some() {
            return;
        }

        if let Some((new_size, scale_fac)) = self.queued_resize.take() {
            self.resize(new_size, scale_fac);
        }

        let output_start_time = crate::time::Instant::now();

        match self.render_surface.get_output() {
            Ok(output) => {
                self.framepacer.start_window(
                    crate::time::Instant::now(),
                    get_window_frame_time(&self.window),
                );

                self.swap_chain = output.into();
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
        };
    }

    pub async fn new(event_loop: &ToposEventLoop) -> Self {
        let mut builder = WindowBuilder::new();

        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowBuilderExtMacOS;

            builder = builder
                .with_title_hidden(true)
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true);
        }

        let window = builder.build(event_loop).unwrap();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;

            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas();

            let window = web_sys::window().unwrap();

            window
                .document()
                .unwrap()
                .body()
                .unwrap()
                .append_child(&canvas);

            // window.set_onresize(Some(js_sys::Function::new_with_args(args, body)))

            canvas.set_attribute("oncontextmenu", "return false;");
        }

        // TODO: move this to separate file
        let rwh = window.raw_window_handle();
        match rwh {
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::AppKit(handle) => unsafe {
                use cocoa::appkit::{NSColor, NSWindow};

                use cocoa::base::id;
                use objc::{sel, sel_impl};

                let ns_window: id = std::mem::transmute(handle.ns_window);

                ns_window.setMovable_(false);

                let fs_button = ns_window
                    .standardWindowButton_(cocoa::appkit::NSWindowButton::NSWindowZoomButton);
                let _: () = objc::msg_send![fs_button, setHidden:true];

                let min_button = ns_window.standardWindowButton_(
                    cocoa::appkit::NSWindowButton::NSWindowMiniaturizeButton,
                );
                let _: () = objc::msg_send![min_button, setHidden:true];

                let bg_col = id::colorWithRed_green_blue_alpha_(ns_window, 0., 0., 0., 1.);
                ns_window.setBackgroundColor_(bg_col);
            },
            _ => {}
        }

        let render_surface = RenderSurface::new(&window).await;
        let rendering_context = render_surface.clone_rendering_context();

        let mut scene = Scene::new(rendering_context, window.scale_factor());

        let winit_state_proxy = event_loop.create_proxy();

        let root_id = scene.root_id().as_access_id();
        let root_node = scene.root_access_node();

        let winit_state = WinitState::new(
            &window,
            winit_state_proxy,
            // TODO: use featuere
            #[cfg(not(target_arch = "wasm32"))]
            move || accesskit::TreeUpdate {
                tree: Some(accesskit::Tree::new(root_id)),
                nodes: vec![(root_id, root_node)],
                ..Default::default()
            },
        );

        let input_state = InputState::default().into();

        Self {
            window,

            render_surface,
            scene,

            winit_state,
            input_state,

            swap_chain: None,
            queued_resize: None,

            framepacer: Default::default(),
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

fn get_window_frame_time(window: &winit::window::Window) -> Option<f64> {
    let monitor = window.current_monitor()?;

    let frame_rate = monitor.refresh_rate_millihertz()? as f64 / 1000.;

    return Some(1. / frame_rate);

    // return Some(Duration::from_secs_f64());
}
