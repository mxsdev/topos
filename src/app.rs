use crate::{
    math::{Pos, Rect},
    scene::{
        framepacer::{
            self, CADisplayLinkFramepacer, Framepacer, FramepacerInstant, InstantLike,
            NoopFramepacer,
        },
        layout::{self, ElementTree},
    },
    surface::{RenderTarget, RenderingContext},
    texture::TextureManagerRef,
    util::{min, PhysicalUnit},
};
use core::panic;

use raw_window_handle::{AppKitWindowHandle, HasRawDisplayHandle, HasRawWindowHandle};
use winit::{
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    element::RootConstructor,
    input::{input_state::InputState, winit::WinitState},
    scene::{framepacer::ManagedFramepacer, scene::Scene},
    surface::{RenderAttachment, RenderSurface},
};

pub struct App<Root: RootConstructor + 'static> {
    swap_chain: Option<(RenderAttachment, ElementTree)>,

    render_surface: RenderSurface,
    scene: Scene<Root>,

    winit_state: WinitState,
    input_state: InputState,

    queued_resize: Option<(winit::dpi::PhysicalSize<u32>, Option<f64>)>,

    window: winit::window::Window,

    texture_manager: TextureManagerRef,

    last_presentation_time: Option<wgpu::PresentationTimestamp>,

    framepacer: ManagedFramepacer,
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
        let main_proxy = event_loop.create_proxy();
        // let mut framepacer = ManagedFramepacer::<wgpu::PresentationTimestamp>::default();
        let mut framepacer = CADisplayLinkFramepacer::new();

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
                    self.draw(&mut framepacer)
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

    fn draw<I: InstantLike + Copy + Debug>(&mut self, framepacer: &mut impl Framepacer<I>) {
        self.try_create_new_output(None, framepacer);

        // let framepacer = || {
        //     (match external_framepacer {
        //         Some(framepacer) => framepacer,
        //         None => &mut self.framepacer,
        //     }) as &mut dyn Framepacer
        // };

        let time_context = I::context_from(self.render_surface.rendering_context());

        let (output, element_tree) = match self.swap_chain.take() {
            Some(output) => output,
            None => return,
        };

        let (should_render, render_start_time) =
            framepacer.should_render(InstantLike::now(time_context));

        if !should_render {
            self.swap_chain = Some((output, element_tree));
            return;
        }

        let raw_input = self.winit_state.take_egui_input();

        let input_state = std::mem::take(&mut self.input_state).begin_frame(raw_input, true);

        let (mut result_input, result_output, render_time, approx_present_time) =
            self.scene.render(
                &self.render_surface,
                output,
                element_tree,
                input_state,
                render_start_time.into(),
                framepacer,
                time_context,
            );

        // if self.framepacer.check_missed_deadline(render_finish_time) {
        //     log::debug!("  missed deadline render time: {:?}", render_time);
        // }

        result_input.end_frame();

        self.try_create_new_output::<I>(approx_present_time.into(), framepacer);

        framepacer.push_frametime(render_time);

        self.input_state = result_input;

        self.winit_state
            .handle_platform_output(&self.window, result_output);
    }

    fn try_create_new_output<I: InstantLike + Copy>(
        &mut self,
        approx_presentation_start: Option<I>,
        framepacer: &mut impl Framepacer<I>,
    ) {
        if self.swap_chain.is_some() {
            return;
        }

        if let Some((new_size, scale_fac)) = self.queued_resize.take() {
            self.resize(new_size, scale_fac);
        }

        let _output_start_time = crate::time::Instant::now();

        match self.render_surface.get_output() {
            Ok(output) => {
                let render_ctx = self.render_surface.rendering_context();

                // let presentation_start = match self.last_presentation_time {
                //     Some(_) => {
                //         let presentation_stats = loop {
                //             let presentation_stats = self
                //                 .render_surface
                //                 .surface()
                //                 .query_presentation_statistics();

                //             if presentation_stats.len() == 0 {
                //                 std::thread::sleep(std::time::Duration::from_micros(1));

                //                 continue;
                //             }

                //             break presentation_stats;
                //         };

                //         if presentation_stats.len() != 1 {
                //             log::warn!(
                //                 "unexpected number of frames drawn: {:?}",
                //                 presentation_stats.len()
                //             )
                //         }

                //         presentation_stats.last().unwrap().presentation_start
                //     }

                //     None => {
                //         log::warn!("unable to retrieve presentation stats");

                //         self.render_surface
                //             .rendering_context()
                //             .adapter
                //             .get_presentation_timestamp()
                //     }
                // };

                if let Some(ns_screen) = get_ns_screen(&self.window) {
                    let fps = unsafe { ns_screen.maximumFramesPerSecond() as f32 };
                    framepacer.sync_to_fps(fps);
                }

                let last_presentation_time = I::query_presentation_statistics(
                    self.render_surface.surface(),
                    &self.window,
                    approx_presentation_start.unwrap_or(I::now(I::context_from(render_ctx))),
                );

                framepacer.start_window(
                    last_presentation_time,
                    get_window_frame_time_nanos(&self.window),
                );

                let layout_result = self.scene.do_layout(&self.render_surface);

                self.swap_chain = (output, layout_result).into();
            }
            // Reconfigure the surface if lost
            Err(wgpu::SurfaceError::Lost) => {
                log::warn!("render surface lost");

                self.render_surface.reconfigure()
            }
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
                // .with_title_hidden(true)
                .with_title("topos")
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_transparent(true);
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

        let rwh_target = match rwh {
            // #[cfg(target_os = "macos")]
            // raw_window_handle::RawWindowHandle::AppKit(handle) => unsafe {
            //     use icrate::AppKit::{
            //         NSColor, NSView, NSViewHeightSizable, NSViewWidthSizable,
            //         NSVisualEffectBlendingModeBehindWindow,
            //         NSVisualEffectMaterialUnderWindowBackground, NSVisualEffectStateActive,
            //         NSVisualEffectView, NSWindow, NSWindowBelow, NSWindowMiniaturizeButton,
            //         NSWindowZoomButton,
            //     };

            //     use objc2::ClassType;

            //     let ns_window: &mut NSWindow =
            //         (handle.ns_window as *mut NSWindow).as_mut().unwrap();

            //     let ns_view: &NSView = (handle.ns_view as *mut NSView).as_mut().unwrap();

            //     ns_window.setMovable(false);

            //     ns_window
            //         .standardWindowButton(NSWindowMiniaturizeButton)
            //         .map(|b| b.setHidden(true));

            //     ns_window
            //         .standardWindowButton(NSWindowZoomButton)
            //         .map(|b| b.setHidden(true));

            //     let metal_view = NSView::initWithFrame(NSView::alloc(), ns_view.bounds());

            //     metal_view.setAutoresizingMask(NSViewWidthSizable | NSViewHeightSizable);
            //     metal_view.setTranslatesAutoresizingMaskIntoConstraints(true);
            //     metal_view.setFrame(ns_view.bounds());

            //     ns_view.addSubview(&metal_view);

            //     let view = NSVisualEffectView::initWithFrame(
            //         NSVisualEffectView::alloc(),
            //         ns_view.bounds(),
            //     );

            //     view.setAutoresizingMask(NSViewWidthSizable | NSViewHeightSizable);
            //     view.setTranslatesAutoresizingMaskIntoConstraints(true);
            //     view.setFrame(ns_view.bounds());
            //     view.setWantsLayer(true);

            //     view.setMaterial(NSVisualEffectMaterialUnderWindowBackground);
            //     view.setState(NSVisualEffectStateActive);
            //     view.setBlendingMode(NSVisualEffectBlendingModeBehindWindow);

            //     ns_view.addSubview_positioned_relativeTo(&view, NSWindowBelow, None);

            //     ns_window.setBackgroundColor(NSColor::windowBackgroundColor().as_ref().into());

            //     let mut appkit_wh = AppKitWindowHandle::empty();

            //     appkit_wh.ns_view = metal_view.as_ref() as *const _ as *mut _;
            //     appkit_wh.ns_window = handle.ns_window;

            //     raw_window_handle::RawWindowHandle::AppKit(appkit_wh)
            // },
            _ => rwh,
        };

        let render_target = RenderTarget {
            raw_window_handle: rwh_target,
            raw_display_handle: window.raw_display_handle(),
        };

        let render_surface = RenderSurface::new(&window, &render_target).await;
        let rendering_context = render_surface.clone_rendering_context();

        let wgpu::Limits {
            max_sampled_textures_per_shader_stage,
            max_bindings_per_bind_group,
            ..
        } = rendering_context.device.limits();

        let max_textures = min(
            max_sampled_textures_per_shader_stage,
            max_bindings_per_bind_group,
        );

        let texture_manager = TextureManagerRef::new(max_textures, &rendering_context);

        let mut scene = Scene::new(
            rendering_context,
            &render_surface,
            &texture_manager,
            window.scale_factor(),
        );

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

            last_presentation_time: Default::default(),

            texture_manager,
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

        self.render_surface.resize(new_size, scale_factor);
    }
}

fn get_window_frame_time_nanos(window: &winit::window::Window) -> Option<std::time::Duration> {
    let monitor = window.current_monitor()?;

    return std::time::Duration::from_secs_f64(1000. / monitor.refresh_rate_millihertz()? as f64)
        .into();
}

pub fn get_ns_screen(window: &winit::window::Window) -> Option<Id<icrate::AppKit::NSScreen>> {
    match window.raw_window_handle() {
        raw_window_handle::RawWindowHandle::AppKit(handle) => unsafe {
            use icrate::AppKit::NSWindow;
            let ns_window: &mut NSWindow = (handle.ns_window as *mut NSWindow).as_mut().unwrap();
            ns_window.screen()
        },
        _ => None,
    }
}

pub fn get_window_last_screen_draw_time(
    window: &winit::window::Window,
) -> Option<std::time::Duration> {
    match window.raw_window_handle() {
        raw_window_handle::RawWindowHandle::AppKit(handle) => unsafe {
            use icrate::AppKit::{
                NSColor, NSView, NSViewHeightSizable, NSViewWidthSizable,
                NSVisualEffectBlendingModeBehindWindow,
                NSVisualEffectMaterialUnderWindowBackground, NSVisualEffectStateActive,
                NSVisualEffectView, NSWindow, NSWindowBelow, NSWindowMiniaturizeButton,
                NSWindowZoomButton,
            };

            use objc2::ClassType;

            let ns_window: &mut NSWindow = (handle.ns_window as *mut NSWindow).as_mut().unwrap();

            let ns_screen = ns_window.screen().unwrap();

            std::time::Duration::from_secs_f64(ns_screen.lastDisplayUpdateTimestamp()).into()
        },
        // raw_window_handle::RawWindowHandle::UiKit(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Orbital(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Xlib(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Xcb(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Wayland(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Drm(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Gbm(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Win32(_) => todo!(),
        // raw_window_handle::RawWindowHandle::WinRt(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Web(_) => todo!(),
        // raw_window_handle::RawWindowHandle::AndroidNdk(_) => todo!(),
        // raw_window_handle::RawWindowHandle::Haiku(_) => todo!(),
        _ => None,
    }
}

use std::{fmt::Debug, os::raw::c_int};

use icrate::Foundation::{NSCopying, NSObject, NSObjectProtocol, NSZone};
use objc2::declare::{Ivar, IvarBool, IvarDrop, IvarEncode};
use objc2::rc::Id;
use objc2::{
    declare_class, extern_protocol, msg_send, msg_send_id, mutability, ClassType, ProtocolType,
};

declare_class!(
    struct CustomAppDelegate {
        pub should_render: IvarBool<"_should_render">,
    }

    mod ivars;

    unsafe impl ClassType for CustomAppDelegate {
        type Super = NSObject;
        type Mutability = mutability::Mutable;
        const NAME: &'static str = "CustomAppDelegate";
    }

    unsafe impl CustomAppDelegate {
        #[method(init:)]
        fn init(this: &mut Self) -> Option<&mut Self> {
            let this: Option<&mut Self> = unsafe { msg_send![super(this), init] };

            this.map(|this| {
                *this.should_render = false;

                this
            })
        }
    }
);
