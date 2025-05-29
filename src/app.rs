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

use wgpu::rwh::{AppKitWindowHandle, HasDisplayHandle, HasWindowHandle};
use winit::{
    application::ApplicationHandler, error::EventLoopError, event::{ElementState, Event, KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

use crate::{
    element::RootConstructor,
    input::{input_state::InputState, winit::WinitState},
    scene::{framepacer::ManagedFramepacer, scene::Scene},
    surface::{RenderAttachment, WindowSurface},
};

struct AppInner<Root: RootConstructor + 'static> {
    swap_chain: Option<(RenderAttachment, ElementTree)>,

    window_surface: WindowSurface,

    scene: Scene<Root>,

    winit_state: WinitState,
    input_state: InputState,

    queued_resize: Option<(winit::dpi::PhysicalSize<u32>, Option<f64>)>,

    texture_manager: TextureManagerRef,

    last_presentation_time: Option<wgpu::PresentationTimestamp>,

    framepacer: ManagedFramepacer,
}

pub enum ToposEvent<Root: RootConstructor + 'static> {
    Exit(i32),
    AccessKitActionRequest(accesskit_winit::WindowEvent),
    AppInnerCreated(PhantomData<AppInner<Root>>),
}

unsafe impl<Root: RootConstructor + 'static> Send for ToposEvent<Root> {}

impl<Root: RootConstructor + 'static> From<accesskit_winit::WindowEvent> for ToposEvent<Root> {
    fn from(value: accesskit_winit::WindowEvent) -> Self {
        Self::AccessKitActionRequest(value)
    }
}

pub struct App<Root: RootConstructor + 'static> {
    // event_loop: ToposEventLoop<Root>,
    event_loop_proxy: EventLoopProxy<ToposEvent<Root>>,
    app_inner: Option<AppInner<Root>>,
}

pub type ToposEventLoop<Root: RootConstructor + 'static> = EventLoop<ToposEvent<Root>>;

impl<Root: RootConstructor + 'static> App<Root> {
    pub fn run() {
        let event_loop = EventLoop::with_user_event().build().expect("Failed to create event loop");

          // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = Self {
            event_loop_proxy: event_loop.create_proxy(),
            app_inner: None,
        };

        event_loop.run_app(&mut app).unwrap();
    }
}

impl<Root: RootConstructor + 'static> ApplicationHandler<ToposEvent<Root>> for App<Root> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app_inner.is_some() {
            return;
        }

        let winit_event_loop_proxy = self.event_loop_proxy.clone();
        // let thread_event_loop_proxy = self.event_loop_proxy.clone();

        log::info!("creating app inner");

        // TODO: maybe do this on another thread...
        // std::thread::spawn(move || {
            let app_inner = pollster::block_on(AppInner::<Root>::new(event_loop, winit_event_loop_proxy));

            self.app_inner = Some(app_inner);

            // self.event_loop_proxy.send_event(ToposEvent::AppInnerCreated(app_inner)).unwrap_or_else(|_| panic!("Failed to send app inner created event"));
        // });

        log::info!("app inner created, presumably starting now");
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(app_inner) = self.app_inner.as_mut() else {
            log::warn!("app inner not created yet");
            return;
        };

        let window = app_inner.window_surface.window();

        if window.id() != window_id {
            return;
        };
        
        let main_proxy = &self.event_loop_proxy;

        // TODO: use this information to determine whether to repaint, i guess
        let _ = app_inner.winit_state.on_window_event(window, &event);

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            } => {
                main_proxy.send_event(ToposEvent::Exit(0)).unwrap_or_else(|_| panic!("Failed to send exit event"));
            }

            WindowEvent::Resized(physical_size) => app_inner.resize(physical_size, None),

            WindowEvent::ScaleFactorChanged {
                scale_factor,
                ..
            } => {
                app_inner.resize(None, Some(scale_factor))
            },

            WindowEvent::RedrawRequested => {
                app_inner.draw()
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app_inner) = self.app_inner.as_mut() else {
            log::warn!("app inner not created yet");
            return;
        };

        app_inner.window_surface.window().request_redraw();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ToposEvent<Root>) {
        match event {
            // ToposEvent::AppInnerCreated(app_inner) => {
            //     self.app_inner = Some(app_inner);
            // }

            // TODO: handle exit code?
            ToposEvent::Exit(_) => {
                event_loop.exit();
            }

            ToposEvent::AccessKitActionRequest(accesskit_winit::WindowEvent::ActionRequested(request)) => {
                if let Some(app_inner) = &mut self.app_inner {
                    app_inner.winit_state.on_accesskit_action_request(request);
                }
            }

            _ => {}
        }
    }
}

impl<Root: RootConstructor + 'static> AppInner<Root> {
    fn draw(&mut self) {
        type I = wgpu::PresentationTimestamp;

        let dpi = self.window_surface.window().scale_factor();
        
        self.try_create_new_output(None);

        // let framepacer = || {
        //     (match external_framepacer {
        //         Some(framepacer) => framepacer,
        //         None => &mut self.framepacer,
        //     }) as &mut dyn Framepacer
        // };

        let time_context = I::context_from(self.window_surface.surface().rendering_context());

        let (output, element_tree) = match self.swap_chain.take() {
            Some(output) => output,
            None => return,
        };

        let (should_render, render_start_time) =
            self.framepacer.should_render(I::now(time_context));

        if !should_render {
            self.swap_chain = Some((output, element_tree));
            return;
        }

        let raw_input = self.winit_state.take_egui_input(self.window_surface.window());

        let input_state = std::mem::take(&mut self.input_state).begin_pass(raw_input, true, dpi as f32, &Default::default());

        let (mut result_input, result_output, render_time, approx_present_time) =
            self.scene.render(
                self.window_surface.surface(),
                output,
                element_tree,
                input_state,
                render_start_time.into(),
                &mut self.framepacer,
                time_context,
            );

        // if self.framepacer.check_missed_deadline(render_finish_time) {
        //     log::debug!("  missed deadline render time: {:?}", render_time);
        // }

        result_input.end_frame();

        self.try_create_new_output(approx_present_time.into());

        self.framepacer.push_frametime(render_time);

        self.input_state = result_input;

        self.winit_state
            .handle_platform_output(self.window_surface.window(), result_output, &self.input_state);
    }

    fn try_create_new_output(
        &mut self,
        approx_presentation_start: Option<wgpu::PresentationTimestamp>,
    ) {
        type I = wgpu::PresentationTimestamp;
        
        if self.swap_chain.is_some() {
            return;
        }

        if let Some((new_size, scale_fac)) = self.queued_resize.take() {
            self.resize(new_size, scale_fac);
        }

        let _output_start_time = crate::time::Instant::now();

        match self.window_surface.surface().get_output() {
            Ok(output) => {
                let render_ctx = self.window_surface.surface().rendering_context();

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

                // if let Some(ns_screen) = get_ns_screen(&self.window) {
                //     let fps = unsafe { ns_screen.maximumFramesPerSecond() as f32 };
                //     framepacer.sync_to_fps(fps);
                // }

                let last_presentation_time = I::query_presentation_statistics(
                    self.window_surface.surface().surface(),
                    &self.window_surface.window(),
                    approx_presentation_start.unwrap_or(I::now(I::context_from(render_ctx))),
                );

                self.framepacer.start_window(
                    last_presentation_time,
                    get_window_frame_time_nanos(self.window_surface.window()),
                );

                let layout_result = self.scene.do_layout(&self.window_surface.surface());

                self.swap_chain = (output, layout_result).into();
            }
            // Reconfigure the surface if lost
            Err(wgpu::SurfaceError::Lost) => {
                log::warn!("render surface lost");

                self.window_surface.surface_mut().reconfigure()
            }
            // The system is out of memory, we should probably quit
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("out of memory"),
            // All other errors (Outdated, Timeout) should be resolved by the next frame
            Err(e) => {
                eprintln!("{:?}", e);
            }
        };
    }

    pub async fn new(event_loop: &ActiveEventLoop, winit_state_proxy: EventLoopProxy<ToposEvent<Root>>) -> Self {
        let mut builder = Window::default_attributes();

        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowAttributesExtMacOS;
            
            builder = builder
                // .with_title_hidden(true)
                .with_title("topos")
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_transparent(true);
        }

        let window = event_loop.create_window(builder).expect("Failed to create window");
        let scale_factor = window.scale_factor();

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
        let rwh = window.window_handle().expect("Window should have handle");

        let rwh_target = match rwh.as_raw() {
            #[cfg(target_os = "macos")]
            wgpu::rwh::RawWindowHandle::AppKit(handle) => unsafe {
                use icrate::AppKit::{
                    NSColor, NSView, NSViewHeightSizable, NSViewWidthSizable,
                    NSVisualEffectBlendingModeBehindWindow,
                    NSVisualEffectMaterialUnderWindowBackground, NSVisualEffectStateActive,
                    NSVisualEffectView, NSWindow, NSWindowBelow, NSWindowMiniaturizeButton,
                    NSWindowZoomButton,
                };

                use objc2::ClassType;

                let ns_view: Id<NSView> = unsafe { Id::retain(handle.ns_view.as_ptr().cast()) }.unwrap();
                let ns_window = ns_view.window().expect("view was not installed in a window");


                ns_window.setMovable(false);

                ns_window
                    .standardWindowButton(NSWindowMiniaturizeButton)
                    .map(|b| b.setHidden(true));

                ns_window
                    .standardWindowButton(NSWindowZoomButton)
                    .map(|b| b.setHidden(true));

                let metal_view = NSView::initWithFrame(NSView::alloc(), ns_view.bounds());

                metal_view.setAutoresizingMask(NSViewWidthSizable | NSViewHeightSizable);
                metal_view.setTranslatesAutoresizingMaskIntoConstraints(true);
                metal_view.setFrame(ns_view.bounds());

                ns_view.addSubview(&metal_view);

                let view = NSVisualEffectView::initWithFrame(
                    NSVisualEffectView::alloc(),
                    ns_view.bounds(),
                );

                view.setAutoresizingMask(NSViewWidthSizable | NSViewHeightSizable);
                view.setTranslatesAutoresizingMaskIntoConstraints(true);
                view.setFrame(ns_view.bounds());
                view.setWantsLayer(true);

                view.setMaterial(NSVisualEffectMaterialUnderWindowBackground);
                view.setState(NSVisualEffectStateActive);
                view.setBlendingMode(NSVisualEffectBlendingModeBehindWindow);

                ns_view.addSubview_positioned_relativeTo(&view, NSWindowBelow, None);

                ns_window.setBackgroundColor(NSColor::windowBackgroundColor().as_ref().into());

                let appkit_wh = AppKitWindowHandle::new(NonNull::new(metal_view.as_ref() as *const _ as *mut _).unwrap());

                wgpu::rwh::RawWindowHandle::AppKit(appkit_wh)
            },
            rwh => rwh,
        };

        let render_target = RenderTarget {
            raw_window_handle: rwh_target,
            raw_display_handle: window.display_handle().expect("Window should have display handle").as_raw(),
        };

        let window_surface = WindowSurface::new(window, render_target).await;
        let rendering_context = window_surface.surface().clone_rendering_context();

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
            &window_surface.surface(),
            &texture_manager,
            scale_factor,
        );

        let root_id = scene.root_id().as_access_id();
        let root_node = scene.root_access_node();

        let winit_state = WinitState::new(
            window_surface.window(),
            None
            // winit_state_proxy,
            // TODO: use featuere
            // #[cfg(not(target_arch = "wasm32"))]
            // move || accesskit::TreeUpdate {
            //     tree: Some(accesskit::Tree::new(root_id)),
            //     nodes: vec![(root_id, root_node)],
            //     focus: root_id,
            // },
        );

        let input_state = InputState::default().into();

        Self {
            window_surface,

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

    pub fn resize(&mut self, new_size: impl Into<Option<winit::dpi::PhysicalSize<u32>>>, scale_factor: Option<f64>) {
        let new_size = new_size.into().unwrap_or_else(|| self.window_surface.window().inner_size());

        if self.swap_chain.is_some() {
            self.queued_resize = Some((
                new_size,
                scale_factor.or(self.queued_resize.map(|(_, sf)| sf).flatten()),
            ));
            return;
        }

        self.window_surface.surface_mut().resize(new_size, scale_factor);
    }
}

fn get_window_frame_time_nanos(window: &winit::window::Window) -> Option<std::time::Duration> {
    let monitor = window.current_monitor()?;

    return std::time::Duration::from_secs_f64(1000. / monitor.refresh_rate_millihertz()? as f64)
        .into();
}

// pub fn get_ns_screen(window: &winit::window::Window) -> Option<Id<icrate::AppKit::NSScreen>> {
//     match window.window_handle().expect("Window should have handle").as_raw() {
//         wgpu::rwh::RawWindowHandle::AppKit(handle) => unsafe {
//             use icrate::AppKit::NSWindow;
//             let ns_window: &mut NSWindow = (handle.ns_view as *mut NSWindow).as_mut().unwrap();
//             ns_window.screen()
//         },
//         _ => None,
//     }
// }

pub fn get_window_last_screen_draw_time(
    window: &winit::window::Window,
) -> Option<std::time::Duration> {
    None

    // match window.raw_window_handle() {
    //     raw_window_handle::RawWindowHandle::AppKit(handle) => unsafe {
    //         use icrate::AppKit::{
    //             NSColor, NSView, NSViewHeightSizable, NSViewWidthSizable,
    //             NSVisualEffectBlendingModeBehindWindow,
    //             NSVisualEffectMaterialUnderWindowBackground, NSVisualEffectStateActive,
    //             NSVisualEffectView, NSWindow, NSWindowBelow, NSWindowMiniaturizeButton,
    //             NSWindowZoomButton,
    //         };

    //         use objc2::ClassType;

    //         let ns_window: &mut NSWindow = (handle.ns_window as *mut NSWindow).as_mut().unwrap();

    //         let ns_screen = ns_window.screen().unwrap();

    //         std::time::Duration::from_secs_f64(ns_screen.lastDisplayUpdateTimestamp()).into()
    //     },
    //     // raw_window_handle::RawWindowHandle::UiKit(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Orbital(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Xlib(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Xcb(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Wayland(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Drm(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Gbm(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Win32(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::WinRt(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Web(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::AndroidNdk(_) => todo!(),
    //     // raw_window_handle::RawWindowHandle::Haiku(_) => todo!(),
    //     _ => None,
    // }
}

use std::{fmt::Debug, marker::PhantomData, os::raw::c_int, ptr::NonNull};

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
