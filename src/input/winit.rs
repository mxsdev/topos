//! [`egui`] bindings for [`winit`](https://github.com/rust-windowing/winit).
//!
//! The library translates winit events to egui, handled copy/paste,
//! updates the cursor, open links clicked in egui, etc.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::manual_range_contains)]

use std::time::Instant;

pub use accesskit_winit;
pub use winit;

// mod window_settings;

// pub use window_settings::WindowSettings;

use raw_window_handle::HasRawDisplayHandle;

pub fn native_pixels_per_point(window: &winit::window::Window) -> f32 {
    window.scale_factor() as f32
}

pub fn screen_size_in_pixels(window: &winit::window::Window) -> Vec2 {
    let size = window.inner_size();
    Vec2::new(size.width as f32, size.height as f32)
}

// ----------------------------------------------------------------------------

#[must_use]
pub struct EventResponse {
    // /// If true, egui consumed this event, i.e. wants exclusive use of this event
    // /// (e.g. a mouse click on an egui window, or entering text into a text field).
    // ///
    // /// For instance, if you use egui for a game, you should only
    // /// pass on the events to your game when [`Self::consumed`] is `false.
    // ///
    // /// Note that egui uses `tab` to move focus between elements, so this will always be `true` for tabs.
    // pub consumed: bool,
    /// Do we need an egui refresh because of this event?
    pub repaint: bool,
}

// ----------------------------------------------------------------------------

/// Handles the integration between egui and winit.
pub struct WinitState {
    start_time: Instant,
    egui_input: RawInput,
    pointer_pos_in_points: Option<Pos2>,
    any_pointer_button_down: bool,
    current_cursor_icon: Option<super::output::CursorIcon>,

    current_pixels_per_point: f32,

    /// What egui uses.
    // current_pixels_per_point: f32,
    clipboard: super::clipboard::Clipboard,

    /// If `true`, mouse inputs will be treated as touches.
    /// Useful for debugging touch support in egui.
    ///
    /// Creates duplicate touches, if real touch inputs are coming.
    simulate_touch_screen: bool,

    /// Is Some(…) when a touch is being translated to a pointer.
    ///
    /// Only one touch will be interpreted as pointer at any time.
    pointer_touch_id: Option<u64>,

    /// track ime state
    input_method_editor_started: bool,

    accesskit: accesskit_winit::Adapter,
}

impl WinitState {
    /// Construct a new instance
    ///
    /// # Safety
    ///
    /// The returned `State` must not outlive the input `display_target`.
    pub fn new(
        display_target: &winit::window::Window,
        event_loop_proxy: winit::event_loop::EventLoopProxy<
            impl From<accesskit_winit::ActionRequestEvent> + Send,
        >,
        initial_tree_update_factory: impl 'static + FnOnce() -> accesskit::TreeUpdate + Send,
    ) -> Self {
        let egui_input = RawInput {
            focused: false, // winit will tell us when we have focus
            ..Default::default()
        };

        Self {
            start_time: Instant::now(),
            egui_input,
            pointer_pos_in_points: None,
            any_pointer_button_down: false,
            current_cursor_icon: None,
            current_pixels_per_point: display_target.scale_factor() as f32,
            clipboard: super::clipboard::Clipboard::new(display_target),

            simulate_touch_screen: false,
            pointer_touch_id: None,

            input_method_editor_started: false,

            accesskit: accesskit_winit::Adapter::new(
                display_target,
                initial_tree_update_factory,
                event_loop_proxy,
            ),
        }
    }

    // fn init_accesskit<T: From<accesskit_winit::ActionRequestEvent> + Send>(
    //     window: &winit::window::Window,
    //     event_loop_proxy: winit::event_loop::EventLoopProxy<T>,
    //     initial_tree_update_factory: impl 'static + FnOnce() -> accesskit::TreeUpdate + Send,
    // ) {
    //     Some(accesskit_winit::Adapter::new(
    //         window,
    //         initial_tree_update_factory,
    //         event_loop_proxy,
    //     ));
    // }

    // /// Call this once a graphics context has been created to update the maximum texture dimensions
    // /// that egui will use.
    // pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
    //     self.egui_input.max_texture_side = Some(max_texture_side);
    // }

    // /// Call this when a new native Window is created for rendering to initialize the `pixels_per_point`
    // /// for that window.
    // ///
    // /// In particular, on Android it is necessary to call this after each `Resumed` lifecycle
    // /// event, each time a new native window is created.
    // ///
    // /// Once this has been initialized for a new window then this state will be maintained by handling
    // /// [`winit::event::WindowEvent::ScaleFactorChanged`] events.
    // pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
    //     // self.egui_input.pixels_per_point = Some(pixels_per_point);
    //     // self.current_pixels_per_point = pixels_per_point;
    // }

    /// The number of physical pixels per logical point,
    /// as configured on the current egui context (see [`Context::pixels_per_point`]).
    #[inline]
    pub fn pixels_per_point(&self) -> f32 {
        self.current_pixels_per_point
    }

    /// The current input state.
    /// This is changed by [`Self::on_event`] and cleared by [`Self::take_egui_input`].
    #[inline]
    pub fn egui_input(&self) -> &RawInput {
        &self.egui_input
    }

    /// Prepare for a new frame by extracting the accumulated input,
    /// as well as setting [the time](RawInput::time) and [screen rectangle](RawInput::screen_rect).
    pub fn take_egui_input(&mut self) -> RawInput {
        // let pixels_per_point = self.pixels_per_point();

        self.egui_input.time = Some(self.start_time.elapsed().as_secs_f64());

        // TODO: fix this for windows!
        // // On Windows, a minimized window will have 0 width and height.
        // // See: https://github.com/rust-windowing/winit/issues/208
        // // This solves an issue where egui window positions would be changed when minimizing on Windows.
        // let screen_size_in_pixels = screen_size_in_pixels(window);
        // let screen_size_in_points = screen_size_in_pixels / pixels_per_point;
        // self.egui_input.screen_rect =
        //     if screen_size_in_points.x > 0.0 && screen_size_in_points.y > 0.0 {
        //         Some(Rect::from_min_size(
        //             Pos2::zero(),
        //             screen_size_in_points.into(),
        //         ))
        //     } else {
        //         None
        //     };

        self.egui_input.take()
    }

    /// Call this when there is a new event.
    ///
    /// The result can be found in [`Self::egui_input`] and be extracted with [`Self::take_egui_input`].
    pub fn on_event(
        &mut self,
        event: &winit::event::WindowEvent<'_>,
        window: &winit::window::Window,
    ) -> EventResponse {
        // TODO: maybe use this?
        let _ = self.accesskit.on_event(window, event);

        use winit::event::WindowEvent;
        match event {
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let pixels_per_point = *scale_factor as f32;
                // self.egui_input.pixels_per_point = Some(pixels_per_point);
                self.current_pixels_per_point = pixels_per_point;
                EventResponse { repaint: true }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse_button_input(*state, *button);
                EventResponse { repaint: true }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.on_mouse_wheel(*delta);
                EventResponse { repaint: true }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.on_cursor_moved(*position);
                EventResponse { repaint: true }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_pos_in_points = None;
                self.egui_input.events.push(Event::PointerGone);
                EventResponse { repaint: true }
            }
            // WindowEvent::TouchpadPressure {device_id, pressure, stage, ..  } => {} // TODO
            WindowEvent::Touch(touch) => {
                self.on_touch(touch);
                // let consumed = match touch.phase {
                //     winit::event::TouchPhase::Started
                //     | winit::event::TouchPhase::Ended
                //     | winit::event::TouchPhase::Cancelled => egui_ctx.wants_pointer_input(),
                //     winit::event::TouchPhase::Moved => egui_ctx.is_using_pointer(),
                // };
                EventResponse {
                    repaint: true,
                    // consumed,
                }
            }
            WindowEvent::ReceivedCharacter(ch) => {
                // On Mac we get here when the user presses Cmd-C (copy), ctrl-W, etc.
                // We need to ignore these characters that are side-effects of commands.
                let is_mac_cmd = cfg!(target_os = "macos")
                    && (self.egui_input.modifiers.ctrl || self.egui_input.modifiers.mac_cmd);

                // let consumed = if is_printable_char(*ch) && !is_mac_cmd {
                //     self.egui_input
                //         .events
                //         .push(Event::Text(ch.to_string()));
                //     egui_ctx.wants_keyboard_input()
                // } else {
                //     false
                // };

                EventResponse {
                    repaint: true,
                    // consumed,
                }
            }
            WindowEvent::Ime(ime) => {
                // on Mac even Cmd-C is pressed during ime, a `c` is pushed to Preedit.
                // So no need to check is_mac_cmd.
                //
                // How winit produce `Ime::Enabled` and `Ime::Disabled` differs in MacOS
                // and Windows.
                //
                // - On Windows, before and after each Commit will produce an Enable/Disabled
                // event.
                // - On MacOS, only when user explicit enable/disable ime. No Disabled
                // after Commit.
                //
                // We use input_method_editor_started to manually insert CompositionStart
                // between Commits.
                match ime {
                    winit::event::Ime::Enabled | winit::event::Ime::Disabled => (),
                    winit::event::Ime::Commit(text) => {
                        self.input_method_editor_started = false;
                        self.egui_input
                            .events
                            .push(Event::CompositionEnd(text.clone()));
                    }
                    winit::event::Ime::Preedit(text, ..) => {
                        if !self.input_method_editor_started {
                            self.input_method_editor_started = true;
                            self.egui_input.events.push(Event::CompositionStart);
                        }
                        self.egui_input
                            .events
                            .push(Event::CompositionUpdate(text.clone()));
                    }
                };

                EventResponse {
                    repaint: true,
                    // consumed: egui_ctx.wants_keyboard_input(),
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                self.on_keyboard_input(input);
                // let consumed = egui_ctx.wants_keyboard_input()
                //     || input.virtual_keycode == Some(winit::event::VirtualKeyCode::Tab);
                EventResponse {
                    repaint: true,
                    // consumed,
                }
            }
            WindowEvent::Focused(focused) => {
                self.egui_input.focused = *focused;
                // We will not be given a KeyboardInput event when the modifiers are released while
                // the window does not have focus. Unset all modifier state to be safe.
                self.egui_input.modifiers = Modifiers::default();
                // self.egui_input
                //     .events
                //     .push(Event::WindowFocused(*focused));
                EventResponse {
                    repaint: true,
                    // consumed: false,
                }
            }
            WindowEvent::HoveredFile(path) => {
                self.egui_input.hovered_files.push(HoveredFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
                EventResponse {
                    repaint: true,
                    // consumed: false,
                }
            }
            WindowEvent::HoveredFileCancelled => {
                self.egui_input.hovered_files.clear();
                EventResponse {
                    repaint: true,
                    // consumed: false,
                }
            }
            WindowEvent::DroppedFile(path) => {
                self.egui_input.hovered_files.clear();
                self.egui_input.dropped_files.push(DroppedFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
                EventResponse {
                    repaint: true,
                    // consumed: false,
                }
            }
            WindowEvent::ModifiersChanged(state) => {
                self.egui_input.modifiers.alt = state.alt();
                self.egui_input.modifiers.ctrl = state.ctrl();
                self.egui_input.modifiers.shift = state.shift();
                self.egui_input.modifiers.mac_cmd = cfg!(target_os = "macos") && state.logo();
                self.egui_input.modifiers.command = if cfg!(target_os = "macos") {
                    state.logo()
                } else {
                    state.ctrl()
                };
                EventResponse {
                    repaint: true,
                    // consumed: false,
                }
            }

            // Things that may require repaint:
            WindowEvent::CloseRequested
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::Destroyed
            | WindowEvent::Occluded(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ThemeChanged(_)
            | WindowEvent::TouchpadPressure { .. } => EventResponse {
                repaint: true,
                // consumed: false,
            },

            // Things we completely ignore:
            WindowEvent::AxisMotion { .. }
            | WindowEvent::Moved(_)
            | WindowEvent::SmartMagnify { .. }
            | WindowEvent::TouchpadRotate { .. } => EventResponse {
                repaint: false,
                // consumed: false,
            },

            WindowEvent::TouchpadMagnify { delta, .. } => {
                // Positive delta values indicate magnification (zooming in).
                // Negative delta values indicate shrinking (zooming out).
                let zoom_factor = (*delta as f32).exp();
                self.egui_input.events.push(Event::Zoom(zoom_factor));
                EventResponse {
                    repaint: true,
                    // consumed: egui_ctx.wants_pointer_input(),
                }
            }
        }
    }

    /// Call this when there is a new [`accesskit::ActionRequest`].
    ///
    /// The result can be found in [`Self::egui_input`] and be extracted with [`Self::take_egui_input`].
    pub fn on_accesskit_action_request(&mut self, request: accesskit::ActionRequest) {
        self.egui_input
            .events
            .push(Event::AccessKitActionRequest(request));
    }

    fn on_mouse_button_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        if let Some(pos) = self.pointer_pos_in_points {
            if let Some(button) = translate_mouse_button(button) {
                let pressed = state == winit::event::ElementState::Pressed;

                self.egui_input.events.push(Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: self.egui_input.modifiers,
                });

                if self.simulate_touch_screen {
                    if pressed {
                        self.any_pointer_button_down = true;

                        self.egui_input.events.push(Event::Touch {
                            device_id: TouchDeviceId(0),
                            id: TouchId(0),
                            phase: TouchPhase::Start,
                            pos,
                            force: 0.0,
                        });
                    } else {
                        self.any_pointer_button_down = false;

                        self.egui_input.events.push(Event::PointerGone);

                        self.egui_input.events.push(Event::Touch {
                            device_id: TouchDeviceId(0),
                            id: TouchId(0),
                            phase: TouchPhase::End,
                            pos,
                            force: 0.0,
                        });
                    };
                }
            }
        }
    }

    fn on_cursor_moved(&mut self, pos_in_pixels: winit::dpi::PhysicalPosition<f64>) {
        let pos_in_points = Pos2::new(
            pos_in_pixels.x as f32 / self.pixels_per_point(),
            pos_in_pixels.y as f32 / self.pixels_per_point(),
        );
        self.pointer_pos_in_points = Some(pos_in_points);

        if self.simulate_touch_screen {
            if self.any_pointer_button_down {
                self.egui_input
                    .events
                    .push(Event::PointerMoved(pos_in_points));

                self.egui_input.events.push(Event::Touch {
                    device_id: TouchDeviceId(0),
                    id: TouchId(0),
                    phase: TouchPhase::Move,
                    pos: pos_in_points,
                    force: 0.0,
                });
            }
        } else {
            self.egui_input
                .events
                .push(Event::PointerMoved(pos_in_points));
        }
    }

    fn on_touch(&mut self, touch: &winit::event::Touch) {
        // Emit touch event
        self.egui_input.events.push(Event::Touch {
            device_id: TouchDeviceId(touch.device_id.hash_u64()),
            id: TouchId::from(touch.id),
            phase: match touch.phase {
                winit::event::TouchPhase::Started => TouchPhase::Start,
                winit::event::TouchPhase::Moved => TouchPhase::Move,
                winit::event::TouchPhase::Ended => TouchPhase::End,
                winit::event::TouchPhase::Cancelled => TouchPhase::Cancel,
            },
            pos: Pos2::new(
                touch.location.x as f32 / self.pixels_per_point(),
                touch.location.y as f32 / self.pixels_per_point(),
            ),
            force: match touch.force {
                Some(winit::event::Force::Normalized(force)) => force as f32,
                Some(winit::event::Force::Calibrated {
                    force,
                    max_possible_force,
                    ..
                }) => (force / max_possible_force) as f32,
                None => 0_f32,
            },
        });
        // If we're not yet translating a touch or we're translating this very
        // touch …
        if self.pointer_touch_id.is_none() || self.pointer_touch_id.unwrap() == touch.id {
            // … emit PointerButton resp. PointerMoved events to emulate mouse
            match touch.phase {
                winit::event::TouchPhase::Started => {
                    self.pointer_touch_id = Some(touch.id);
                    // First move the pointer to the right location
                    self.on_cursor_moved(touch.location);
                    self.on_mouse_button_input(
                        winit::event::ElementState::Pressed,
                        winit::event::MouseButton::Left,
                    );
                }
                winit::event::TouchPhase::Moved => {
                    self.on_cursor_moved(touch.location);
                }
                winit::event::TouchPhase::Ended => {
                    self.pointer_touch_id = None;
                    self.on_mouse_button_input(
                        winit::event::ElementState::Released,
                        winit::event::MouseButton::Left,
                    );
                    // The pointer should vanish completely to not get any
                    // hover effects
                    self.pointer_pos_in_points = None;
                    self.egui_input.events.push(Event::PointerGone);
                }
                winit::event::TouchPhase::Cancelled => {
                    self.pointer_touch_id = None;
                    self.pointer_pos_in_points = None;
                    self.egui_input.events.push(Event::PointerGone);
                }
            }
        }
    }

    fn on_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        {
            let (unit, delta) = match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    (MouseWheelUnit::Line, Vec2::new(x, y))
                }
                winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {
                    x,
                    y,
                }) => (
                    MouseWheelUnit::Point,
                    Vec2::new(x as f32, y as f32) / self.pixels_per_point(),
                ),
            };
            let modifiers = self.egui_input.modifiers;
            self.egui_input.events.push(Event::MouseWheel {
                unit,
                delta,
                modifiers,
            });
        }
        let delta = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                let points_per_scroll_line = 50.0; // Scroll speed decided by consensus: https://github.com/emilk/egui/issues/461
                Vec2::new(x, y) * points_per_scroll_line
            }
            winit::event::MouseScrollDelta::PixelDelta(delta) => {
                Vec2::new(delta.x as f32, delta.y as f32) / self.pixels_per_point()
            }
        };

        if self.egui_input.modifiers.ctrl || self.egui_input.modifiers.command {
            // Treat as zoom instead:
            let factor = (delta.y / 200.0).exp();
            self.egui_input.events.push(Event::Zoom(factor));
        } else if self.egui_input.modifiers.shift {
            // Treat as horizontal scrolling.
            // Note: one Mac we already get horizontal scroll events when shift is down.
            self.egui_input
                .events
                .push(Event::Scroll(Vec2::new(delta.x + delta.y, 0.0)));
        } else {
            self.egui_input.events.push(Event::Scroll(delta));
        }
    }

    fn on_keyboard_input(&mut self, input: &winit::event::KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            let pressed = input.state == winit::event::ElementState::Pressed;

            if pressed {
                // VirtualKeyCode::Paste etc in winit are broken/untrustworthy,
                // so we detect these things manually:
                if is_cut_command(self.egui_input.modifiers, keycode) {
                    self.egui_input.events.push(Event::Cut);
                } else if is_copy_command(self.egui_input.modifiers, keycode) {
                    self.egui_input.events.push(Event::Copy);
                } else if is_paste_command(self.egui_input.modifiers, keycode) {
                    if let Some(contents) = self.clipboard.get() {
                        let contents = contents.replace("\r\n", "\n");
                        if !contents.is_empty() {
                            self.egui_input.events.push(Event::Paste(contents));
                        }
                    }
                }
            }

            if let Some(key) = translate_virtual_key_code(keycode) {
                self.egui_input.events.push(Event::Key {
                    key,
                    pressed,
                    repeat: false, // egui will fill this in for us!
                    modifiers: self.egui_input.modifiers,
                });
            }
        }
    }

    /// Call with the output given by `egui`.
    ///
    /// This will, if needed:
    /// * update the cursor
    /// * copy text to the clipboard
    /// * open any clicked urls
    /// * update the IME
    /// *
    pub fn handle_platform_output(
        &mut self,
        window: &winit::window::Window,
        platform_output: PlatformOutput,
    ) {
        let PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _,                    // handled above
            mutable_text_under_cursor: _, // only used in eframe web
            text_cursor_pos,
            accesskit_update,
            drag_window,
        } = platform_output;

        // self.current_pixels_per_point = egui_ctx.pixels_per_point(); // someone can have changed it to scale the UI

        self.set_cursor_icon(window, cursor_icon);

        if let Some(open_url) = open_url {
            open_url_in_browser(&open_url.url);
        }

        if !copied_text.is_empty() {
            self.clipboard.set(copied_text);
        }

        if let Some(Pos2 { x, y, .. }) = text_cursor_pos {
            window.set_ime_position(winit::dpi::LogicalPosition { x, y });
        }

        if let Some(update) = accesskit_update {
            self.accesskit.update_if_active(|| update);
        }

        if drag_window {
            match window.drag_window() {
                Ok(()) => {}
                Err(e) => log::error!("{e}"),
            }
        }
    }

    fn set_cursor_icon(
        &mut self,
        window: &winit::window::Window,
        cursor_icon: super::output::CursorIcon,
    ) {
        if self.current_cursor_icon == Some(cursor_icon) {
            // Prevent flickering near frame boundary when Windows OS tries to control cursor icon for window resizing.
            // On other platforms: just early-out to save CPU.
            return;
        }

        let is_pointer_in_window = self.pointer_pos_in_points.is_some();
        if is_pointer_in_window {
            self.current_cursor_icon = Some(cursor_icon);

            if let Some(winit_cursor_icon) = translate_cursor(cursor_icon) {
                window.set_cursor_visible(true);
                window.set_cursor_icon(winit_cursor_icon);
            } else {
                window.set_cursor_visible(false);
            }
        } else {
            // Remember to set the cursor again once the cursor returns to the screen:
            self.current_cursor_icon = None;
        }
    }
}

fn open_url_in_browser(_url: &str) {
    if let Err(err) = webbrowser::open(_url) {
        log::warn!("Failed to open url: {}", err);
    }
}

/// Winit sends special keys (backspace, delete, F1, …) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}

fn is_cut_command(modifiers: Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::X)
        || (cfg!(target_os = "windows")
            && modifiers.shift
            && keycode == winit::event::VirtualKeyCode::Delete)
}

fn is_copy_command(modifiers: Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::C)
        || (cfg!(target_os = "windows")
            && modifiers.ctrl
            && keycode == winit::event::VirtualKeyCode::Insert)
}

fn is_paste_command(modifiers: Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::V)
        || (cfg!(target_os = "windows")
            && modifiers.shift
            && keycode == winit::event::VirtualKeyCode::Insert)
}

fn translate_mouse_button(button: winit::event::MouseButton) -> Option<PointerButton> {
    match button {
        winit::event::MouseButton::Left => Some(PointerButton::Primary),
        winit::event::MouseButton::Right => Some(PointerButton::Secondary),
        winit::event::MouseButton::Middle => Some(PointerButton::Middle),
        winit::event::MouseButton::Other(n) => Some(PointerButton::Other(n)),
    }
}

fn translate_virtual_key_code(key: winit::event::VirtualKeyCode) -> Option<Key> {
    use winit::event::VirtualKeyCode;
    use Key;

    Some(match key {
        VirtualKeyCode::Down => Key::ArrowDown,
        VirtualKeyCode::Left => Key::ArrowLeft,
        VirtualKeyCode::Right => Key::ArrowRight,
        VirtualKeyCode::Up => Key::ArrowUp,

        VirtualKeyCode::Escape => Key::Escape,
        VirtualKeyCode::Tab => Key::Tab,
        VirtualKeyCode::Back => Key::Backspace,
        VirtualKeyCode::Return => Key::Enter,
        VirtualKeyCode::Space => Key::Space,

        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::PageDown => Key::PageDown,

        VirtualKeyCode::Minus => Key::Minus,
        // Using Mac the key with the Plus sign on it is reported as the Equals key
        // (with both English and Swedish keyboard).
        VirtualKeyCode::Equals => Key::PlusEquals,

        VirtualKeyCode::Key0 | VirtualKeyCode::Numpad0 => Key::Num0,
        VirtualKeyCode::Key1 | VirtualKeyCode::Numpad1 => Key::Num1,
        VirtualKeyCode::Key2 | VirtualKeyCode::Numpad2 => Key::Num2,
        VirtualKeyCode::Key3 | VirtualKeyCode::Numpad3 => Key::Num3,
        VirtualKeyCode::Key4 | VirtualKeyCode::Numpad4 => Key::Num4,
        VirtualKeyCode::Key5 | VirtualKeyCode::Numpad5 => Key::Num5,
        VirtualKeyCode::Key6 | VirtualKeyCode::Numpad6 => Key::Num6,
        VirtualKeyCode::Key7 | VirtualKeyCode::Numpad7 => Key::Num7,
        VirtualKeyCode::Key8 | VirtualKeyCode::Numpad8 => Key::Num8,
        VirtualKeyCode::Key9 | VirtualKeyCode::Numpad9 => Key::Num9,

        VirtualKeyCode::A => Key::A,
        VirtualKeyCode::B => Key::B,
        VirtualKeyCode::C => Key::C,
        VirtualKeyCode::D => Key::D,
        VirtualKeyCode::E => Key::E,
        VirtualKeyCode::F => Key::F,
        VirtualKeyCode::G => Key::G,
        VirtualKeyCode::H => Key::H,
        VirtualKeyCode::I => Key::I,
        VirtualKeyCode::J => Key::J,
        VirtualKeyCode::K => Key::K,
        VirtualKeyCode::L => Key::L,
        VirtualKeyCode::M => Key::M,
        VirtualKeyCode::N => Key::N,
        VirtualKeyCode::O => Key::O,
        VirtualKeyCode::P => Key::P,
        VirtualKeyCode::Q => Key::Q,
        VirtualKeyCode::R => Key::R,
        VirtualKeyCode::S => Key::S,
        VirtualKeyCode::T => Key::T,
        VirtualKeyCode::U => Key::U,
        VirtualKeyCode::V => Key::V,
        VirtualKeyCode::W => Key::W,
        VirtualKeyCode::X => Key::X,
        VirtualKeyCode::Y => Key::Y,
        VirtualKeyCode::Z => Key::Z,

        VirtualKeyCode::F1 => Key::F1,
        VirtualKeyCode::F2 => Key::F2,
        VirtualKeyCode::F3 => Key::F3,
        VirtualKeyCode::F4 => Key::F4,
        VirtualKeyCode::F5 => Key::F5,
        VirtualKeyCode::F6 => Key::F6,
        VirtualKeyCode::F7 => Key::F7,
        VirtualKeyCode::F8 => Key::F8,
        VirtualKeyCode::F9 => Key::F9,
        VirtualKeyCode::F10 => Key::F10,
        VirtualKeyCode::F11 => Key::F11,
        VirtualKeyCode::F12 => Key::F12,
        VirtualKeyCode::F13 => Key::F13,
        VirtualKeyCode::F14 => Key::F14,
        VirtualKeyCode::F15 => Key::F15,
        VirtualKeyCode::F16 => Key::F16,
        VirtualKeyCode::F17 => Key::F17,
        VirtualKeyCode::F18 => Key::F18,
        VirtualKeyCode::F19 => Key::F19,
        VirtualKeyCode::F20 => Key::F20,

        _ => {
            return None;
        }
    })
}

fn translate_cursor(cursor_icon: super::output::CursorIcon) -> Option<winit::window::CursorIcon> {
    match cursor_icon {
        super::output::CursorIcon::None => None,

        super::output::CursorIcon::Alias => Some(winit::window::CursorIcon::Alias),
        super::output::CursorIcon::AllScroll => Some(winit::window::CursorIcon::AllScroll),
        super::output::CursorIcon::Cell => Some(winit::window::CursorIcon::Cell),
        super::output::CursorIcon::ContextMenu => Some(winit::window::CursorIcon::ContextMenu),
        super::output::CursorIcon::Copy => Some(winit::window::CursorIcon::Copy),
        super::output::CursorIcon::Crosshair => Some(winit::window::CursorIcon::Crosshair),
        super::output::CursorIcon::Default => Some(winit::window::CursorIcon::Default),
        super::output::CursorIcon::Grab => Some(winit::window::CursorIcon::Grab),
        super::output::CursorIcon::Grabbing => Some(winit::window::CursorIcon::Grabbing),
        super::output::CursorIcon::Help => Some(winit::window::CursorIcon::Help),
        super::output::CursorIcon::Move => Some(winit::window::CursorIcon::Move),
        super::output::CursorIcon::NoDrop => Some(winit::window::CursorIcon::NoDrop),
        super::output::CursorIcon::NotAllowed => Some(winit::window::CursorIcon::NotAllowed),
        super::output::CursorIcon::PointingHand => Some(winit::window::CursorIcon::Hand),
        super::output::CursorIcon::Progress => Some(winit::window::CursorIcon::Progress),

        super::output::CursorIcon::ResizeHorizontal => Some(winit::window::CursorIcon::EwResize),
        super::output::CursorIcon::ResizeNeSw => Some(winit::window::CursorIcon::NeswResize),
        super::output::CursorIcon::ResizeNwSe => Some(winit::window::CursorIcon::NwseResize),
        super::output::CursorIcon::ResizeVertical => Some(winit::window::CursorIcon::NsResize),

        super::output::CursorIcon::ResizeEast => Some(winit::window::CursorIcon::EResize),
        super::output::CursorIcon::ResizeSouthEast => Some(winit::window::CursorIcon::SeResize),
        super::output::CursorIcon::ResizeSouth => Some(winit::window::CursorIcon::SResize),
        super::output::CursorIcon::ResizeSouthWest => Some(winit::window::CursorIcon::SwResize),
        super::output::CursorIcon::ResizeWest => Some(winit::window::CursorIcon::WResize),
        super::output::CursorIcon::ResizeNorthWest => Some(winit::window::CursorIcon::NwResize),
        super::output::CursorIcon::ResizeNorth => Some(winit::window::CursorIcon::NResize),
        super::output::CursorIcon::ResizeNorthEast => Some(winit::window::CursorIcon::NeResize),
        super::output::CursorIcon::ResizeColumn => Some(winit::window::CursorIcon::ColResize),
        super::output::CursorIcon::ResizeRow => Some(winit::window::CursorIcon::RowResize),

        super::output::CursorIcon::Text => Some(winit::window::CursorIcon::Text),
        super::output::CursorIcon::VerticalText => Some(winit::window::CursorIcon::VerticalText),
        super::output::CursorIcon::Wait => Some(winit::window::CursorIcon::Wait),
        super::output::CursorIcon::ZoomIn => Some(winit::window::CursorIcon::ZoomIn),
        super::output::CursorIcon::ZoomOut => Some(winit::window::CursorIcon::ZoomOut),
    }
}

// ---------------------------------------------------------------------------

/// Profiling macro for feature "puffin"
#[allow(unused_macros)]
macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_function!($($arg)*);
    };
}

#[allow(unused_imports)]
pub(crate) use profile_function;

/// Profiling macro for feature "puffin"
#[allow(unused_macros)]
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        puffin::profile_scope!($($arg)*);
    };
}

#[allow(unused_imports)]
pub(crate) use profile_scope;
use winit::window::CursorIcon;

use crate::{
    debug::HashU64,
    input::{DroppedFile, Event, HoveredFile, Modifiers},
    util::{FromMinSize, Pos2, Rect, Vec2},
};

use super::{
    output::PlatformOutput, Key, MouseWheelUnit, PointerButton, RawInput, TouchDeviceId, TouchId,
    TouchPhase,
};
