// use crate::emath::*;

use crate::math::{Pos, Vector};

/// What the integrations provides to egui at the start of each frame.
///
/// Set the values that make sense, leave the rest at their `Default::default()`.
///
/// You can check if `egui` is using the inputs using
/// [`crate::Context::wants_pointer_input`] and [`crate::Context::wants_keyboard_input`].
///
/// All coordinates are in points (logical pixels) with origin (0, 0) in the top left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct RawInput {
    // /// Position and size of the area that egui should use, in points.
    // /// Usually you would set this to
    // ///
    // /// `Some(Rect::from_min_size(Default::default(), screen_size_in_points))`.
    // ///
    // /// but you could also constrain egui to some smaller portion of your window if you like.
    // ///
    // /// `None` will be treated as "same as last frame", with the default being a very big area.
    // pub screen_rect: Option<Rect>,

    // /// Also known as device pixel ratio, > 1 for high resolution screens.
    // /// If text looks blurry you probably forgot to set this.
    // /// Set this the first frame, whenever it changes, or just on every frame.
    // pub pixels_per_point: Option<f32>,
    // /// Maximum size of one side of the font texture.
    // ///
    // /// Ask your graphics drivers about this. This corresponds to `GL_MAX_TEXTURE_SIZE`.
    // ///
    // /// The default is a very small (but very portable) 2048.
    // pub max_texture_side: Option<usize>,
    /// Monotonically increasing time, in seconds. Relative to whatever. Used for animations.
    /// If `None` is provided, egui will assume a time delta of `predicted_dt` (default 1/60 seconds).
    pub time: Option<f64>,

    /// Should be set to the expected time between frames when painting at vsync speeds.
    /// The default for this is 1/60.
    /// Can safely be left at its default value.
    pub predicted_dt: f32,

    /// Which modifier keys are down at the start of the frame?
    pub modifiers: Modifiers,

    /// In-order events received this frame.
    ///
    /// There is currently no way to know if egui handles a particular event,
    /// but you can check if egui is using the keyboard with [`crate::Context::wants_keyboard_input`]
    /// and/or the pointer (mouse/touch) with [`crate::Context::is_using_pointer`].
    pub events: Vec<Event>,

    /// Dragged files hovering over egui.
    pub hovered_files: Vec<HoveredFile>,

    /// Dragged files dropped into egui.
    ///
    /// Note: when using `eframe` on Windows you need to enable
    /// drag-and-drop support using `eframe::NativeOptions`.
    pub dropped_files: Vec<DroppedFile>,

    /// The native window has the keyboard focus (i.e. is receiving key presses).
    ///
    /// False when the user alt-tab away from the application, for instance.
    pub focused: bool,
}

impl Default for RawInput {
    fn default() -> Self {
        Self {
            // screen_rect: None,
            // pixels_per_point: None,
            // max_texture_side: None,
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: Modifiers::default(),
            events: vec![],
            hovered_files: Default::default(),
            dropped_files: Default::default(),
            focused: true, // integrations opt into global focus tracking
        }
    }
}

impl RawInput {
    /// Helper: move volatile (deltas and events), clone the rest.
    ///
    /// * [`Self::hovered_files`] is cloned.
    /// * [`Self::dropped_files`] is moved.
    pub fn take(&mut self) -> RawInput {
        RawInput {
            // screen_rect: self.screen_rect.take(),
            // pixels_per_point: self.pixels_per_point.take(),
            // max_texture_side: self.max_texture_side.take(),
            time: self.time.take(),
            predicted_dt: self.predicted_dt,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            hovered_files: self.hovered_files.clone(),
            dropped_files: std::mem::take(&mut self.dropped_files),
            focused: self.focused,
        }
    }

    /// Add on new input.
    pub fn append(&mut self, newer: Self) {
        let Self {
            // screen_rect,
            // pixels_per_point,
            // max_texture_side,
            time,
            predicted_dt,
            modifiers,
            mut events,
            mut hovered_files,
            mut dropped_files,
            focused,
        } = newer;

        // self.screen_rect = screen_rect.or(self.screen_rect);
        // self.pixels_per_point = pixels_per_point.or(self.pixels_per_point);
        // self.max_texture_side = max_texture_side.or(self.max_texture_side);
        self.time = time; // use latest time
        self.predicted_dt = predicted_dt; // use latest dt
        self.modifiers = modifiers; // use latest
        self.events.append(&mut events);
        self.hovered_files.append(&mut hovered_files);
        self.dropped_files.append(&mut dropped_files);
        self.focused = focused;
    }
}

/// A file about to be dropped into egui.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HoveredFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,

    /// With the `eframe` web backend, this is set to the mime-type of the file (if available).
    pub mime: String,
}

/// A file dropped into egui.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DroppedFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,

    /// Name of the file. Set by the `eframe` web backend.
    pub name: String,

    /// Set by the `eframe` web backend.
    pub last_modified: Option<crate::time::SystemTime>,

    /// Set by the `eframe` web backend.
    pub bytes: Option<std::sync::Arc<[u8]>>,
}

/// IME event.
///
/// See <https://docs.rs/winit/latest/winit/event/enum.Ime.html>
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImeEvent {
    /// Notifies when the IME was enabled.
    Enabled,

    /// A new IME candidate is being suggested.
    Preedit(String),

    /// IME composition ended with this final result.
    Commit(String),

    /// Notifies when the IME was disabled.
    Disabled,
}

/// An input event generated by the integration.
///
/// This only covers events that egui cares about.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The integration detected a "copy" event (e.g. Cmd+C).
    Copy,

    /// The integration detected a "cut" event (e.g. Cmd+X).
    Cut,

    /// The integration detected a "paste" event (e.g. Cmd+V).
    Paste(String),

    /// Text input, e.g. via keyboard.
    ///
    /// When the user presses enter/return, do not send a [`Text`](Event::Text) (just [`Key::Enter`]).
    Text(String),

    /// A key was pressed or released.
    Key {
        key: Key,

        /// The physical key, corresponding to the actual position on the keyboard.
        ///
        /// This ignores keymaps, so it is not recommended to use this.
        /// The only thing it makes sense for is things like games,
        /// where e.g. the physical location of WSAD on QWERTY should always map to movement,
        /// even if the user is using Dvorak or AZERTY.
        ///
        /// `eframe` does not (yet) implement this on web.
        physical_key: Option<Key>,

        /// Was it pressed or released?
        pressed: bool,

        /// If this is a `pressed` event, is it a key-repeat?
        ///
        /// On many platforms, holding down a key produces many repeated "pressed" events for it, so called key-repeats.
        /// Sometimes you will want to ignore such events, and this lets you do that.
        ///
        /// egui will automatically detect such repeat events and mark them as such here.
        /// Therefore, if you are writing an egui integration, you do not need to set this (just set it to `false`).
        repeat: bool,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The mouse or touch moved to a new place.
    PointerMoved(Pos),

    /// The mouse moved, the units are unspecified.
    /// Represents the actual movement of the mouse, without acceleration or clamped by screen edges.
    /// `PointerMoved` and `MouseMoved` can be sent at the same time.
    /// This event is optional. If the integration can not determine unfiltered motion it should not send this event.
    MouseMoved(Vector),

    /// A mouse button was pressed or released (or a touch started or stopped).
    PointerButton {
        /// Where is the pointer?
        pos: Pos,

        /// What mouse button? For touches, use [`PointerButton::Primary`].
        button: PointerButton,

        /// Was it the button/touch pressed this frame, or released?
        pressed: bool,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The mouse left the screen, or the last/primary touch input disappeared.
    ///
    /// This means there is no longer a cursor on the screen for hovering etc.
    ///
    /// On touch-up first send `PointerButton{pressed: false, …}` followed by `PointerLeft`.
    PointerGone,

    // /// How many points (logical pixels) the user scrolled.
    // ///
    // /// The direction of the vector indicates how to move the _content_ that is being viewed.
    // /// So if you get positive values, the content being viewed should move to the right and down,
    // /// revealing new things to the left and up.
    // ///
    // /// A positive X-value indicates the content is being moved right,
    // /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    // ///
    // /// A positive Y-value indicates the content is being moved down,
    // /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    // ///
    // /// Shift-scroll should result in horizontal scrolling (it is up to the integrations to do this).
    // Scroll(Vector),

    /// Zoom scale factor this frame (e.g. from ctrl-scroll or pinch gesture).
    /// * `zoom = 1`: no change.
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    Zoom(f32),

    /// Rotation delta this frame (e.g. from pinch gesture).
    Rotate(f32),

    /// IME Event
    Ime(ImeEvent),

    // /// IME composition start.
    // CompositionStart,

    // /// A new IME candidate is being suggested.
    // CompositionUpdate(String),

    // /// IME composition ended with this final result.
    // CompositionEnd(String),

    /// On touch screens, report this *in addition to*
    /// [`Self::PointerMoved`], [`Self::PointerButton`], [`Self::PointerGone`]
    Touch {
        /// Hashed device identifier (if available; may be zero).
        /// Can be used to separate touches from different devices.
        device_id: TouchDeviceId,

        /// Unique identifier of a finger/pen. Value is stable from touch down
        /// to lift-up
        id: TouchId,

        /// One of: start move end cancel.
        phase: TouchPhase,

        /// Position of the touch (or where the touch was last detected)
        pos: Pos,

        /// Describes how hard the touch device was pressed. May always be `None` if the platform does
        /// not support pressure sensitivity.
        /// The value is in the range from 0.0 (no pressure) to 1.0 (maximum pressure).
        force: Option<f32>,
    },

    /// A raw mouse wheel event as sent by the backend (minus the z coordinate),
    /// for implementing alternative custom controls.
    /// Note that the same event can also trigger [`Self::Zoom`] and [`Self::Scroll`],
    /// so you probably want to handle only one of them.
    MouseWheel {
        /// The unit of scrolling: points, lines, or pages.
        unit: MouseWheelUnit,

        /// The amount scrolled horizontally and vertically. The amount and direction corresponding
        /// to one step of the wheel depends on the platform.
        delta: Vector,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The native window gained or lost focused (e.g. the user clicked alt-tab).
    WindowFocused(bool),

    /// An assistive technology (e.g. screen reader) requested an action.
    AccessKitActionRequest(accesskit::ActionRequest),

    TouchPad,

    // TODO: screenshot request lol
}

/// Mouse button (or similar for touch input)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PointerButton {
    /// The primary mouse button is usually the left one.
    Primary,

    /// The secondary mouse button is usually the right one,
    /// and most often used for context menus or other optional things.
    Secondary,

    /// The tertiary mouse button is usually the middle mouse button (e.g. clicking the scroll wheel).
    Middle,

    // Other
    Other(u16),
}

impl PointerButton {
    pub fn as_u16(&self) -> u16 {
        match self {
            PointerButton::Primary => 0,
            PointerButton::Secondary => 1,
            PointerButton::Middle => 2,
            PointerButton::Other(n) => *n,
        }
    }

    pub fn from_u16(val: u16) -> Self {
        match val {
            0 => PointerButton::Primary,
            1 => PointerButton::Secondary,
            2 => PointerButton::Middle,
            x => PointerButton::Other(x),
        }
    }
}

// /// Number of pointer buttons supported by egui, i.e. the number of possible states of [`PointerButton`].
// pub const NUM_POINTER_BUTTONS: usize = 5;

/// State of the modifier keys. These must be fed to egui.
///
/// The best way to compare [`Modifiers`] is by using [`Modifiers::matches`].
///
/// NOTE: For cross-platform uses, ALT+SHIFT is a bad combination of modifiers
/// as on mac that is how you type special characters,
/// so those key presses are usually not reported to egui.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Modifiers {
    /// Either of the alt keys are down (option ⌥ on Mac).
    pub alt: bool,

    /// Either of the control keys are down.
    /// When checking for keyboard shortcuts, consider using [`Self::command`] instead.
    pub ctrl: bool,

    /// Either of the shift keys are down.
    pub shift: bool,

    /// The Mac ⌘ Command key. Should always be set to `false` on other platforms.
    pub mac_cmd: bool,

    /// On Windows and Linux, set this to the same value as `ctrl`.
    /// On Mac, this should be set whenever one of the ⌘ Command keys are down (same as `mac_cmd`).
    /// This is so that egui can, for instance, select all text by checking for `command + A`
    /// and it will work on both Mac and Windows.
    pub command: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };

    pub const ALT: Self = Self {
        alt: true,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const CTRL: Self = Self {
        alt: false,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const SHIFT: Self = Self {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: false,
        command: false,
    };

    #[deprecated = "Use `Modifiers::ALT | Modifiers::SHIFT` instead"]
    pub const ALT_SHIFT: Self = Self {
        alt: true,
        ctrl: false,
        shift: true,
        mac_cmd: false,
        command: false,
    };

    /// The Mac ⌘ Command key
    pub const MAC_CMD: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: true,
        command: false,
    };

    /// On Mac: ⌘ Command key, elsewhere: Ctrl key
    pub const COMMAND: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: true,
    };

    /// ```
    /// # use egui::Modifiers;
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers { ctrl: true, alt: true, ..Default::default() }
    /// );
    /// assert_eq!(
    ///     Modifiers::ALT.plus(Modifiers::CTRL),
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// ```
    #[inline]
    pub const fn plus(self, rhs: Self) -> Self {
        Self {
            alt: self.alt | rhs.alt,
            ctrl: self.ctrl | rhs.ctrl,
            shift: self.shift | rhs.shift,
            mac_cmd: self.mac_cmd | rhs.mac_cmd,
            command: self.command | rhs.command,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self == &Self::default()
    }

    #[inline]
    pub fn any(&self) -> bool {
        !self.is_none()
    }

    /// Is shift the only pressed button?
    #[inline]
    pub fn shift_only(&self) -> bool {
        self.shift && !(self.alt || self.command)
    }

    /// true if only [`Self::ctrl`] or only [`Self::mac_cmd`] is pressed.
    #[inline]
    pub fn command_only(&self) -> bool {
        !self.alt && !self.shift && self.command
    }

    /// Check for equality but with proper handling of [`Self::command`].
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let current_modifiers = Modifiers::default();
    /// if current_modifiers.matches(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, and nothing else
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!(!(Modifiers::CTRL | Modifiers::SHIFT).matches(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches(Modifiers::MAC_CMD));
    /// ```
    pub fn matches(&self, pattern: Modifiers) -> bool {
        // alt and shift must always match the pattern:
        if pattern.alt != self.alt || pattern.shift != self.shift {
            return false;
        }

        if pattern.mac_cmd {
            // Mac-specific match:
            if !self.mac_cmd {
                return false;
            }
            if pattern.ctrl != self.ctrl {
                return false;
            }
            return true;
        }

        if !pattern.ctrl && !pattern.command {
            // the pattern explicitly doesn't want any ctrl/command:
            return !self.ctrl && !self.command;
        }

        // if the pattern is looking for command, then `ctrl` may or may not be set depending on platform.
        // if the pattern is looking for `ctrl`, then `command` may or may not be set depending on platform.

        if pattern.ctrl && !self.ctrl {
            return false;
        }
        if pattern.command && !self.command {
            return false;
        }

        true
    }

    /// Whether another set of modifiers is contained in this set of modifiers with proper handling of [`Self::command`].
    ///
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::default().contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::CTRL));
    /// assert!(Modifiers::CTRL.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::MAC_CMD.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::MAC_CMD));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::CTRL));
    /// assert!(!(Modifiers::ALT | Modifiers::CTRL).contains(Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).contains(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.contains(Modifiers::CTRL | Modifiers::SHIFT));
    /// ```
    pub fn contains(&self, query: Modifiers) -> bool {
        if query == Modifiers::default() {
            return true;
        }

        let Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = *self;

        if alt && query.alt {
            return self.contains(Modifiers {
                alt: false,
                ..query
            });
        }
        if shift && query.shift {
            return self.contains(Modifiers {
                shift: false,
                ..query
            });
        }

        if (ctrl || command) && (query.ctrl || query.command) {
            return self.contains(Modifiers {
                command: false,
                ctrl: false,
                ..query
            });
        }
        if (mac_cmd || command) && (query.mac_cmd || query.command) {
            return self.contains(Modifiers {
                mac_cmd: false,
                command: false,
                ..query
            });
        }

        false
    }

        /// Checks that the `ctrl/cmd` matches, and that the `shift/alt` of the argument is a subset
    /// of the pressed key (`self`).
    ///
    /// This means that if the pattern has not set `shift`, then `self` can have `shift` set or not.
    ///
    /// The reason is that many logical keys require `shift` or `alt` on some keyboard layouts.
    /// For instance, in order to press `+` on an English keyboard, you need to press `shift` and `=`,
    /// but a Swedish keyboard has dedicated `+` key.
    /// So if you want to make a [`KeyboardShortcut`] looking for `Cmd` + `+`, it makes sense
    /// to ignore the shift key.
    /// Similarly, the `Alt` key is sometimes used to type special characters.
    ///
    /// However, if the pattern (the argument) explicitly requires the `shift` or `alt` keys
    /// to be pressed, then they must be pressed.
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let pressed_modifiers = Modifiers::default();
    /// if pressed_modifiers.matches_logically(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, but not ctrl/command
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches_logically(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches_logically(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches_logically(Modifiers::MAC_CMD));
    /// ```
    pub fn matches_logically(&self, pattern: Self) -> bool {
        if pattern.alt && !self.alt {
            return false;
        }
        if pattern.shift && !self.shift {
            return false;
        }

        self.cmd_ctrl_matches(pattern)
    }

        /// Checks only cmd/ctrl, not alt/shift.
    ///
    /// `self` here are the currently pressed modifiers,
    /// and the argument the pattern we are testing for.
    ///
    /// This takes care to properly handle the difference between
    /// [`Self::ctrl`], [`Self::command`] and [`Self::mac_cmd`].
    pub fn cmd_ctrl_matches(&self, pattern: Self) -> bool {
        if pattern.mac_cmd {
            // Mac-specific match:
            if !self.mac_cmd {
                return false;
            }
            if pattern.ctrl != self.ctrl {
                return false;
            }
            return true;
        }

        if !pattern.ctrl && !pattern.command {
            // the pattern explicitly doesn't want any ctrl/command:
            return !self.ctrl && !self.command;
        }

        // if the pattern is looking for command, then `ctrl` may or may not be set depending on platform.
        // if the pattern is looking for `ctrl`, then `command` may or may not be set depending on platform.

        if pattern.ctrl && !self.ctrl {
            return false;
        }
        if pattern.command && !self.command {
            return false;
        }

        true
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.plus(rhs)
    }
}

// ----------------------------------------------------------------------------

/// Names of different modifier keys.
///
/// Used to name modifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModifierNames<'a> {
    pub is_short: bool,

    pub alt: &'a str,
    pub ctrl: &'a str,
    pub shift: &'a str,
    pub mac_cmd: &'a str,
    pub mac_alt: &'a str,

    /// What goes between the names
    pub concat: &'a str,
}

impl ModifierNames<'static> {
    /// ⌥ ^ ⇧ ⌘ - NOTE: not supported by the default egui font.
    pub const SYMBOLS: Self = Self {
        is_short: true,
        alt: "⌥",
        ctrl: "^",
        shift: "⇧",
        mac_cmd: "⌘",
        mac_alt: "⌥",
        concat: "",
    };

    /// Alt, Ctrl, Shift, Cmd
    pub const NAMES: Self = Self {
        is_short: false,
        alt: "Alt",
        ctrl: "Ctrl",
        shift: "Shift",
        mac_cmd: "Cmd",
        mac_alt: "Option",
        concat: "+",
    };
}

impl<'a> ModifierNames<'a> {
    pub fn format(&self, modifiers: &Modifiers, is_mac: bool) -> String {
        let mut s = String::new();

        let mut append_if = |modifier_is_active, modifier_name| {
            if modifier_is_active {
                if !s.is_empty() {
                    s += self.concat;
                }
                s += modifier_name;
            }
        };

        if is_mac {
            append_if(modifiers.ctrl, self.ctrl);
            append_if(modifiers.shift, self.shift);
            append_if(modifiers.alt, self.mac_alt);
            append_if(modifiers.mac_cmd || modifiers.command, self.mac_cmd);
        } else {
            append_if(modifiers.ctrl || modifiers.command, self.ctrl);
            append_if(modifiers.alt, self.alt);
            append_if(modifiers.shift, self.shift);
        }

        s
    }
}

// ----------------------------------------------------------------------------

/// Keyboard keys.
///
/// egui usually uses logical keys, i.e. after applying any user keymap.\
// See comment at the end of `Key { … }` on how to add new keys.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Key {
    // ----------------------------------------------
    // Commands:
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,

    Escape,
    Tab,
    Backspace,
    Enter,
    Space,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    Copy,
    Cut,
    Paste,

    // ----------------------------------------------
    // Punctuation:
    /// `:`
    Colon,

    /// `,`
    Comma,

    /// `\`
    Backslash,

    /// `/`
    Slash,

    /// `|`, a vertical bar
    Pipe,

    /// `?`
    Questionmark,

    // '!'
    Exclamationmark,

    // `[`
    OpenBracket,

    // `]`
    CloseBracket,

    // `{`
    OpenCurlyBracket,

    // `}`
    CloseCurlyBracket,

    /// Also known as "backquote" or "grave"
    Backtick,

    /// `-`
    Minus,

    /// `.`
    Period,

    /// `+`
    Plus,

    /// `=`
    Equals,

    /// `;`
    Semicolon,

    /// `'`
    Quote,

    // ----------------------------------------------
    // Digits:
    /// `0` (from main row or numpad)
    Num0,

    /// `1` (from main row or numpad)
    Num1,

    /// `2` (from main row or numpad)
    Num2,

    /// `3` (from main row or numpad)
    Num3,

    /// `4` (from main row or numpad)
    Num4,

    /// `5` (from main row or numpad)
    Num5,

    /// `6` (from main row or numpad)
    Num6,

    /// `7` (from main row or numpad)
    Num7,

    /// `8` (from main row or numpad)
    Num8,

    /// `9` (from main row or numpad)
    Num9,

    // ----------------------------------------------
    // Letters:
    A, // Used for cmd+A (select All)
    B,
    C, // |CMD COPY|
    D, // |CMD BOOKMARK|
    E, // |CMD SEARCH|
    F, // |CMD FIND firefox & chrome|
    G, // |CMD FIND chrome|
    H, // |CMD History|
    I, // italics
    J, // |CMD SEARCH firefox/DOWNLOAD chrome|
    K, // Used for ctrl+K (delete text after cursor)
    L,
    M,
    N,
    O, // |CMD OPEN|
    P, // |CMD PRINT|
    Q,
    R, // |CMD REFRESH|
    S, // |CMD SAVE|
    T, // |CMD TAB|
    U, // Used for ctrl+U (delete text before cursor)
    V, // |CMD PASTE|
    W, // Used for ctrl+W (delete previous word)
    X, // |CMD CUT|
    Y,
    Z, // |CMD UNDO|

    // ----------------------------------------------
    // Function keys:
    F1,
    F2,
    F3,
    F4,
    F5, // |CMD REFRESH|
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    // When adding keys, remember to also update:
    // * crates/egui-winit/src/lib.rs
    // * Key::ALL
    // * Key::from_name
    // You should test that it works using the "Input Event History" window in the egui demo app.
    // Make sure to test both natively and on web!
    // Also: don't add keys last; add them to the group they best belong to.
}

impl Key {
    /// All egui keys
    pub const ALL: &'static [Self] = &[
        // Commands:
        Self::ArrowDown,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::ArrowUp,
        Self::Escape,
        Self::Tab,
        Self::Backspace,
        Self::Enter,
        Self::Insert,
        Self::Delete,
        Self::Home,
        Self::End,
        Self::PageUp,
        Self::PageDown,
        Self::Copy,
        Self::Cut,
        Self::Paste,
        // Punctuation:
        Self::Space,
        Self::Colon,
        Self::Comma,
        Self::Minus,
        Self::Period,
        Self::Plus,
        Self::Equals,
        Self::Semicolon,
        Self::OpenBracket,
        Self::CloseBracket,
        Self::OpenCurlyBracket,
        Self::CloseCurlyBracket,
        Self::Backtick,
        Self::Backslash,
        Self::Slash,
        Self::Pipe,
        Self::Questionmark,
        Self::Exclamationmark,
        Self::Quote,
        // Digits:
        Self::Num0,
        Self::Num1,
        Self::Num2,
        Self::Num3,
        Self::Num4,
        Self::Num5,
        Self::Num6,
        Self::Num7,
        Self::Num8,
        Self::Num9,
        // Letters:
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
        Self::I,
        Self::J,
        Self::K,
        Self::L,
        Self::M,
        Self::N,
        Self::O,
        Self::P,
        Self::Q,
        Self::R,
        Self::S,
        Self::T,
        Self::U,
        Self::V,
        Self::W,
        Self::X,
        Self::Y,
        Self::Z,
        // Function keys:
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::F13,
        Self::F14,
        Self::F15,
        Self::F16,
        Self::F17,
        Self::F18,
        Self::F19,
        Self::F20,
        Self::F21,
        Self::F22,
        Self::F23,
        Self::F24,
        Self::F25,
        Self::F26,
        Self::F27,
        Self::F28,
        Self::F29,
        Self::F30,
        Self::F31,
        Self::F32,
        Self::F33,
        Self::F34,
        Self::F35,
    ];

    /// Converts `"A"` to `Key::A`, `Space` to `Key::Space`, etc.
    ///
    /// Makes sense for logical keys.
    ///
    /// This will parse the output of both [`Self::name`] and [`Self::symbol_or_name`],
    /// but will also parse single characters, so that both `"-"` and `"Minus"` will return `Key::Minus`.
    ///
    /// This should support both the names generated in a web browser,
    /// and by winit. Please test on both with `eframe`.
    pub fn from_name(key: &str) -> Option<Self> {
        Some(match key {
            "⏷" | "ArrowDown" | "Down" => Self::ArrowDown,
            "⏴" | "ArrowLeft" | "Left" => Self::ArrowLeft,
            "⏵" | "ArrowRight" | "Right" => Self::ArrowRight,
            "⏶" | "ArrowUp" | "Up" => Self::ArrowUp,

            "Escape" | "Esc" => Self::Escape,
            "Tab" => Self::Tab,
            "Backspace" => Self::Backspace,
            "Enter" | "Return" => Self::Enter,

            "Help" | "Insert" => Self::Insert,
            "Delete" => Self::Delete,
            "Home" => Self::Home,
            "End" => Self::End,
            "PageUp" => Self::PageUp,
            "PageDown" => Self::PageDown,

            "Copy" => Self::Copy,
            "Cut" => Self::Cut,
            "Paste" => Self::Paste,

            " " | "Space" => Self::Space,
            ":" | "Colon" => Self::Colon,
            "," | "Comma" => Self::Comma,
            "-" | "−" | "Minus" => Self::Minus,
            "." | "Period" => Self::Period,
            "+" | "Plus" => Self::Plus,
            "=" | "Equal" | "Equals" | "NumpadEqual" => Self::Equals,
            ";" | "Semicolon" => Self::Semicolon,
            "\\" | "Backslash" => Self::Backslash,
            "/" | "Slash" => Self::Slash,
            "|" | "Pipe" => Self::Pipe,
            "?" | "Questionmark" => Self::Questionmark,
            "!" | "Exclamationmark" => Self::Exclamationmark,
            "[" | "OpenBracket" => Self::OpenBracket,
            "]" | "CloseBracket" => Self::CloseBracket,
            "{" | "OpenCurlyBracket" => Self::OpenCurlyBracket,
            "}" | "CloseCurlyBracket" => Self::CloseCurlyBracket,
            "`" | "Backtick" | "Backquote" | "Grave" => Self::Backtick,
            "'" | "Quote" => Self::Quote,

            "0" | "Digit0" | "Numpad0" => Self::Num0,
            "1" | "Digit1" | "Numpad1" => Self::Num1,
            "2" | "Digit2" | "Numpad2" => Self::Num2,
            "3" | "Digit3" | "Numpad3" => Self::Num3,
            "4" | "Digit4" | "Numpad4" => Self::Num4,
            "5" | "Digit5" | "Numpad5" => Self::Num5,
            "6" | "Digit6" | "Numpad6" => Self::Num6,
            "7" | "Digit7" | "Numpad7" => Self::Num7,
            "8" | "Digit8" | "Numpad8" => Self::Num8,
            "9" | "Digit9" | "Numpad9" => Self::Num9,

            "a" | "A" => Self::A,
            "b" | "B" => Self::B,
            "c" | "C" => Self::C,
            "d" | "D" => Self::D,
            "e" | "E" => Self::E,
            "f" | "F" => Self::F,
            "g" | "G" => Self::G,
            "h" | "H" => Self::H,
            "i" | "I" => Self::I,
            "j" | "J" => Self::J,
            "k" | "K" => Self::K,
            "l" | "L" => Self::L,
            "m" | "M" => Self::M,
            "n" | "N" => Self::N,
            "o" | "O" => Self::O,
            "p" | "P" => Self::P,
            "q" | "Q" => Self::Q,
            "r" | "R" => Self::R,
            "s" | "S" => Self::S,
            "t" | "T" => Self::T,
            "u" | "U" => Self::U,
            "v" | "V" => Self::V,
            "w" | "W" => Self::W,
            "x" | "X" => Self::X,
            "y" | "Y" => Self::Y,
            "z" | "Z" => Self::Z,

            "F1" => Self::F1,
            "F2" => Self::F2,
            "F3" => Self::F3,
            "F4" => Self::F4,
            "F5" => Self::F5,
            "F6" => Self::F6,
            "F7" => Self::F7,
            "F8" => Self::F8,
            "F9" => Self::F9,
            "F10" => Self::F10,
            "F11" => Self::F11,
            "F12" => Self::F12,
            "F13" => Self::F13,
            "F14" => Self::F14,
            "F15" => Self::F15,
            "F16" => Self::F16,
            "F17" => Self::F17,
            "F18" => Self::F18,
            "F19" => Self::F19,
            "F20" => Self::F20,
            "F21" => Self::F21,
            "F22" => Self::F22,
            "F23" => Self::F23,
            "F24" => Self::F24,
            "F25" => Self::F25,
            "F26" => Self::F26,
            "F27" => Self::F27,
            "F28" => Self::F28,
            "F29" => Self::F29,
            "F30" => Self::F30,
            "F31" => Self::F31,
            "F32" => Self::F32,
            "F33" => Self::F33,
            "F34" => Self::F34,
            "F35" => Self::F35,

            _ => return None,
        })
    }

    /// Emoji or name representing the key
    pub fn symbol_or_name(self) -> &'static str {
        // TODO(emilk): add support for more unicode symbols (see for instance https://wincent.com/wiki/Unicode_representations_of_modifier_keys).
        // Before we do we must first make sure they are supported in `Fonts` though,
        // so perhaps this functions needs to take a `supports_character: impl Fn(char) -> bool` or something.
        match self {
            Self::ArrowDown => "⏷",
            Self::ArrowLeft => "⏴",
            Self::ArrowRight => "⏵",
            Self::ArrowUp => "⏶",

            Self::Colon => ":",
            Self::Comma => ",",
            Self::Minus => "−",
            Self::Period => ".",
            Self::Plus => "+",
            Self::Equals => "=",
            Self::Semicolon => ";",
            Self::Backslash => "\\",
            Self::Slash => "/",
            Self::Pipe => "|",
            Self::Questionmark => "?",
            Self::Exclamationmark => "!",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::OpenCurlyBracket => "{",
            Self::CloseCurlyBracket => "}",
            Self::Backtick => "`",

            _ => self.name(),
        }
    }

    /// Human-readable English name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ArrowDown => "Down",
            Self::ArrowLeft => "Left",
            Self::ArrowRight => "Right",
            Self::ArrowUp => "Up",

            Self::Escape => "Escape",
            Self::Tab => "Tab",
            Self::Backspace => "Backspace",
            Self::Enter => "Enter",

            Self::Insert => "Insert",
            Self::Delete => "Delete",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",

            Self::Copy => "Copy",
            Self::Cut => "Cut",
            Self::Paste => "Paste",

            Self::Space => "Space",
            Self::Colon => "Colon",
            Self::Comma => "Comma",
            Self::Minus => "Minus",
            Self::Period => "Period",
            Self::Plus => "Plus",
            Self::Equals => "Equals",
            Self::Semicolon => "Semicolon",
            Self::Backslash => "Backslash",
            Self::Slash => "Slash",
            Self::Pipe => "Pipe",
            Self::Questionmark => "Questionmark",
            Self::Exclamationmark => "Exclamationmark",
            Self::OpenBracket => "OpenBracket",
            Self::CloseBracket => "CloseBracket",
            Self::OpenCurlyBracket => "OpenCurlyBracket",
            Self::CloseCurlyBracket => "CloseCurlyBracket",
            Self::Backtick => "Backtick",
            Self::Quote => "Quote",

            Self::Num0 => "0",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",

            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
            Self::F20 => "F20",
            Self::F21 => "F21",
            Self::F22 => "F22",
            Self::F23 => "F23",
            Self::F24 => "F24",
            Self::F25 => "F25",
            Self::F26 => "F26",
            Self::F27 => "F27",
            Self::F28 => "F28",
            Self::F29 => "F29",
            Self::F30 => "F30",
            Self::F31 => "F31",
            Self::F32 => "F32",
            Self::F33 => "F33",
            Self::F34 => "F34",
            Self::F35 => "F35",
        }
    }
}


// ----------------------------------------------------------------------------

/// A keyboard shortcut, e.g. `Ctrl+Alt+W`.
///
/// Can be used with [`crate::InputState::consume_shortcut`]
/// and [`crate::Context::format_shortcut`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct KeyboardShortcut {
    pub modifiers: Modifiers,
    pub logical_key: Key,
}

impl KeyboardShortcut {
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, logical_key: key }
    }

    pub fn format(&self, names: &ModifierNames<'_>, is_mac: bool) -> String {
        let mut s = names.format(&self.modifiers, is_mac);
        if !s.is_empty() {
            s += names.concat;
        }
        if names.is_short {
            s += self.logical_key.symbol_or_name();
        } else {
            s += self.logical_key.name();
        }
        s
    }
}

#[test]
fn format_kb_shortcut() {
    let cmd_shift_f = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, false),
        "Ctrl+Shift+F"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, true),
        "Shift+Cmd+F"
    );
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, false), "^⇧F");
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, true), "⇧⌘F");
}

// ----------------------------------------------------------------------------

// impl RawInput {
//     pub fn ui(&self, ui: &mut crate::Ui) {
//         let Self {
//             screen_rect,
//             pixels_per_point,
//             max_texture_side,
//             time,
//             predicted_dt,
//             modifiers,
//             events,
//             hovered_files,
//             dropped_files,
//             focused,
//         } = self;

//         ui.label(format!("screen_rect: {:?} points", screen_rect));
//         ui.label(format!("pixels_per_point: {:?}", pixels_per_point))
//             .on_hover_text(
//                 "Also called HDPI factor.\nNumber of physical pixels per each logical pixel.",
//             );
//         ui.label(format!("max_texture_side: {:?}", max_texture_side));
//         if let Some(time) = time {
//             ui.label(format!("time: {:.3} s", time));
//         } else {
//             ui.label("time: None");
//         }
//         ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
//         ui.label(format!("modifiers: {:#?}", modifiers));
//         ui.label(format!("hovered_files: {}", hovered_files.len()));
//         ui.label(format!("dropped_files: {}", dropped_files.len()));
//         ui.label(format!("focused: {}", focused));
//         ui.scope(|ui| {
//             ui.set_min_height(150.0);
//             ui.label(format!("events: {:#?}", events))
//                 .on_hover_text("key presses etc");
//         });
//     }
// }

/// this is a `u64` as values of this kind can always be obtained by hashing
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TouchDeviceId(pub u64);

/// Unique identification of a touch occurrence (finger or pen or …).
/// A Touch ID is valid until the finger is lifted.
/// A new ID is used for the next touch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TouchId(pub u64);

/// In what phase a touch event is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchPhase {
    /// User just placed a touch point on the touch surface
    Start,

    /// User moves a touch point along the surface. This event is also sent when
    /// any attributes (position, force, …) of the touch point change.
    Move,

    /// User lifted the finger or pen from the surface, or slid off the edge of
    /// the surface
    End,

    /// Touch operation has been disrupted by something (various reasons are possible,
    /// maybe a pop-up alert or any other kind of interruption which may not have
    /// been intended by the user)
    Cancel,
}

/// The unit associated with the numeric value of a mouse wheel event
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseWheelUnit {
    /// Number of ui points (logical pixels)
    Point,

    /// Number of lines
    Line,

    /// Number of pages
    Page,
}

impl From<u64> for TouchId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<i32> for TouchId {
    fn from(id: i32) -> Self {
        Self(id as u64)
    }
}

impl From<u32> for TouchId {
    fn from(id: u32) -> Self {
        Self(id as u64)
    }
}


// TODO(emilk): generalize this to a proper event filter.
/// Controls which events that a focused widget will have exclusive access to.
///
/// Currently this only controls a few special keyboard events,
/// but in the future this `struct` should be extended into a full callback thing.
///
/// Any events not covered by the filter are given to the widget, but are not exclusive.
#[derive(Clone, Copy, Debug)]
pub struct EventFilter {
    /// If `true`, pressing tab will act on the widget,
    /// and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub tab: bool,

    /// If `true`, pressing horizontal arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub horizontal_arrows: bool,

    /// If `true`, pressing vertical arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub vertical_arrows: bool,

    /// If `true`, pressing escape will act on the widget,
    /// and NOT surrender focus from the focused widget.
    ///
    /// Default: `false`
    pub escape: bool,
}

#[expect(clippy::derivable_impls)] // let's be explicit
impl Default for EventFilter {
    fn default() -> Self {
        Self {
            tab: false,
            horizontal_arrows: false,
            vertical_arrows: false,
            escape: false,
        }
    }
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Event::Key { key, .. } = event {
            match key {
                Key::Tab => self.tab,
                Key::ArrowUp | Key::ArrowDown => self.vertical_arrows,
                Key::ArrowRight | Key::ArrowLeft => self.horizontal_arrows,
                Key::Escape => self.escape,
                _ => true,
            }
        } else {
            true
        }
    }
}
