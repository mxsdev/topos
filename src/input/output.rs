use crate::math::Rect;

/// Information about text being edited.
///
/// Useful for IME.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IMEOutput {
    /// Where the [`crate::TextEdit`] is located on screen.
    pub rect: Rect,

    /// Where the primary cursor is.
    ///
    /// This is a very thin rectangle.
    pub cursor_rect: Rect,
}

/// The non-rendering part of what egui emits each frame.
///
/// You can access (and modify) this with [`crate::Context::output`].
///
/// The backend should use this.
#[derive(Default, Clone, PartialEq)]
pub struct PlatformOutput {
    /// Set the cursor to this icon.
    pub(super) cursor_icon: CursorIcon,

    /// If set, open this url.
    pub(super) open_url: Option<OpenUrl>,

    /// If set, put this text in the system clipboard. Ignore if empty.
    ///
    /// This is often a response to [`crate::Event::Copy`] or [`crate::Event::Cut`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// if ui.button("📋").clicked() {
    ///     ui.output_mut(|o| o.copied_text = "some_text".to_string());
    /// }
    /// # });
    /// ```
    pub(super) copied_text: String,

    /// Events that may be useful to e.g. a screen reader.
    pub(super) events: Vec<OutputEvent>,

    /// Is there a mutable [`TextEdit`](crate::TextEdit) under the cursor?
    /// Use by `eframe` web to show/hide mobile keyboard and IME agent.
    pub(super) mutable_text_under_cursor: bool,

    /// This is set if, and only if, the user is currently editing text.
    ///
    /// Useful for IME.
    pub ime: Option<IMEOutput>,

    pub(super) accesskit_update: Option<accesskit::TreeUpdate>,

    pub(super) drag_window: bool,
}

impl PlatformOutput {
    pub fn set_cursor(&mut self, cursor_icon: CursorIcon) {
        self.cursor_icon = cursor_icon
    }

    pub fn accesskit_update(&mut self) -> &mut accesskit::TreeUpdate {
        self.accesskit_update.get_or_insert_with(|| accesskit::TreeUpdate {
            tree: None,
            focus: accesskit::NodeId(0),
            nodes: Vec::new(),
        })
    }

    pub fn start_window_drag(&mut self) {
        self.drag_window = true;
    }

    /// Open the given url in a web browser.
    /// If egui is running in a browser, the same tab will be reused.
    pub fn open_url(&mut self, url: impl ToString) {
        self.open_url = Some(OpenUrl::same_tab(url));
    }

    /// This can be used by a text-to-speech system to describe the events (if any).
    pub fn events_description(&self) -> String {
        // only describe last event:
        if let Some(event) = self.events.iter().rev().next() {
            match event {
                OutputEvent::Clicked(widget_info)
                | OutputEvent::DoubleClicked(widget_info)
                | OutputEvent::TripleClicked(widget_info)
                | OutputEvent::FocusGained(widget_info)
                | OutputEvent::TextSelectionChanged(widget_info)
                | OutputEvent::ValueChanged(widget_info) => {
                    return widget_info.description();
                }
            }
        }
        Default::default()
    }

    /// Add on new output.
    pub fn append(&mut self, newer: Self) {
        let Self {
            cursor_icon,
            open_url,
            copied_text,
            mut events,
            mutable_text_under_cursor,
            accesskit_update,
            drag_window,
            ime,
        } = newer;

        self.cursor_icon = cursor_icon;
        if open_url.is_some() {
            self.open_url = open_url;
        }
        if !copied_text.is_empty() {
            self.copied_text = copied_text;
        }
        self.events.append(&mut events);
        self.mutable_text_under_cursor = mutable_text_under_cursor;
        self.ime = ime.or(self.ime);

        {
            // egui produces a complete AccessKit tree for each frame,
            // so overwrite rather than appending.
            self.accesskit_update = accesskit_update;
        }

        self.drag_window = drag_window;
    }

    /// Take everything ephemeral (everything except `cursor_icon` currently)
    pub fn take(&mut self) -> Self {
        let taken = std::mem::take(self);
        self.cursor_icon = taken.cursor_icon; // everything else is ephemeral
        taken
    }
}

/// What URL to open, and how.
#[derive(Clone, PartialEq, Eq)]
pub struct OpenUrl {
    pub url: String,

    /// If `true`, open the url in a new tab.
    /// If `false` open it in the same tab.
    /// Only matters when in a web browser.
    pub new_tab: bool,
}

impl OpenUrl {
    #[allow(clippy::needless_pass_by_value)]
    pub fn same_tab(url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            new_tab: false,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn new_tab(url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            new_tab: true,
        }
    }
}

/// Types of attention to request from a user when a native window is not in focus.
///
/// See [winit's documentation][user_attention_type] for platform-specific meaning of the attention types.
///
/// [user_attention_type]: https://docs.rs/winit/latest/winit/window/enum.UserAttentionType.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserAttentionType {
    /// Request an elevated amount of animations and flair for the window and the task bar or dock icon.
    Critical,

    /// Request a standard amount of attention-grabbing actions.
    Informational,

    /// Reset the attention request and interrupt related animations and flashes.
    Reset,
}

/// A mouse cursor icon.
///
/// egui emits a [`CursorIcon`] in [`PlatformOutput`] each frame as a request to the integration.
///
/// Loosely based on <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorIcon {
    /// Normal cursor icon, whatever that is.
    Default,

    /// Show no cursor
    None,

    // ------------------------------------
    // Links and status:
    /// A context menu is available
    ContextMenu,

    /// Question mark
    Help,

    /// Pointing hand, used for e.g. web links
    PointingHand,

    /// Shows that processing is being done, but that the program is still interactive.
    Progress,

    /// Not yet ready, try later.
    Wait,

    // ------------------------------------
    // Selection:
    /// Hover a cell in a table
    Cell,

    /// For precision work
    Crosshair,

    /// Text caret, e.g. "Click here to edit text"
    Text,

    /// Vertical text caret, e.g. "Click here to edit vertical text"
    VerticalText,

    // ------------------------------------
    // Drag-and-drop:
    /// Indicated an alias, e.g. a shortcut
    Alias,

    /// Indicate that a copy will be made
    Copy,

    /// Omnidirectional move icon (e.g. arrows in all cardinal directions)
    Move,

    /// Can't drop here
    NoDrop,

    /// Forbidden
    NotAllowed,

    /// The thing you are hovering can be grabbed
    Grab,

    /// You are grabbing the thing you are hovering
    Grabbing,

    // ------------------------------------
    /// Something can be scrolled in any direction (panned).
    AllScroll,

    // ------------------------------------
    // Resizing in two directions:
    /// Horizontal resize `-` to make something wider or more narrow (left to/from right)
    ResizeHorizontal,

    /// Diagonal resize `/` (right-up to/from left-down)
    ResizeNeSw,

    /// Diagonal resize `\` (left-up to/from right-down)
    ResizeNwSe,

    /// Vertical resize `|` (up-down or down-up)
    ResizeVertical,

    // ------------------------------------
    // Resizing in one direction:
    /// Resize something rightwards (e.g. when dragging the right-most edge of something)
    ResizeEast,

    /// Resize something down and right (e.g. when dragging the bottom-right corner of something)
    ResizeSouthEast,

    /// Resize something downwards (e.g. when dragging the bottom edge of something)
    ResizeSouth,

    /// Resize something down and left (e.g. when dragging the bottom-left corner of something)
    ResizeSouthWest,

    /// Resize something leftwards (e.g. when dragging the left edge of something)
    ResizeWest,

    /// Resize something up and left (e.g. when dragging the top-left corner of something)
    ResizeNorthWest,

    /// Resize something up (e.g. when dragging the top edge of something)
    ResizeNorth,

    /// Resize something up and right (e.g. when dragging the top-right corner of something)
    ResizeNorthEast,

    // ------------------------------------
    /// Resize a column
    ResizeColumn,

    /// Resize a row
    ResizeRow,

    // ------------------------------------
    // Zooming:
    /// Enhance!
    ZoomIn,

    /// Let's get a better overview
    ZoomOut,
}

impl CursorIcon {
    pub const ALL: [CursorIcon; 35] = [
        CursorIcon::Default,
        CursorIcon::None,
        CursorIcon::ContextMenu,
        CursorIcon::Help,
        CursorIcon::PointingHand,
        CursorIcon::Progress,
        CursorIcon::Wait,
        CursorIcon::Cell,
        CursorIcon::Crosshair,
        CursorIcon::Text,
        CursorIcon::VerticalText,
        CursorIcon::Alias,
        CursorIcon::Copy,
        CursorIcon::Move,
        CursorIcon::NoDrop,
        CursorIcon::NotAllowed,
        CursorIcon::Grab,
        CursorIcon::Grabbing,
        CursorIcon::AllScroll,
        CursorIcon::ResizeHorizontal,
        CursorIcon::ResizeNeSw,
        CursorIcon::ResizeNwSe,
        CursorIcon::ResizeVertical,
        CursorIcon::ResizeEast,
        CursorIcon::ResizeSouthEast,
        CursorIcon::ResizeSouth,
        CursorIcon::ResizeSouthWest,
        CursorIcon::ResizeWest,
        CursorIcon::ResizeNorthWest,
        CursorIcon::ResizeNorth,
        CursorIcon::ResizeNorthEast,
        CursorIcon::ResizeColumn,
        CursorIcon::ResizeRow,
        CursorIcon::ZoomIn,
        CursorIcon::ZoomOut,
    ];
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

/// Things that happened during this frame that the integration may be interested in.
///
/// In particular, these events may be useful for accessability, i.e. for screen readers.
#[derive(Clone, PartialEq)]
pub enum OutputEvent {
    /// A widget was clicked.
    Clicked(WidgetInfo),

    /// A widget was double-clicked.
    DoubleClicked(WidgetInfo),

    /// A widget was triple-clicked.
    TripleClicked(WidgetInfo),

    /// A widget gained keyboard focus (by tab key).
    FocusGained(WidgetInfo),

    /// Text selection was updated.
    TextSelectionChanged(WidgetInfo),

    /// A widget's value changed.
    ValueChanged(WidgetInfo),
}

impl OutputEvent {
    pub fn widget_info(&self) -> &WidgetInfo {
        match self {
            OutputEvent::Clicked(info)
            | OutputEvent::DoubleClicked(info)
            | OutputEvent::TripleClicked(info)
            | OutputEvent::FocusGained(info)
            | OutputEvent::TextSelectionChanged(info)
            | OutputEvent::ValueChanged(info) => info,
        }
    }
}

impl std::fmt::Debug for OutputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clicked(wi) => write!(f, "Clicked({:?})", wi),
            Self::DoubleClicked(wi) => write!(f, "DoubleClicked({:?})", wi),
            Self::TripleClicked(wi) => write!(f, "TripleClicked({:?})", wi),
            Self::FocusGained(wi) => write!(f, "FocusGained({:?})", wi),
            Self::TextSelectionChanged(wi) => write!(f, "TextSelectionChanged({:?})", wi),
            Self::ValueChanged(wi) => write!(f, "ValueChanged({:?})", wi),
        }
    }
}

/// Describes a widget such as a [`crate::Button`] or a [`crate::TextEdit`].
#[derive(Clone, PartialEq)]
pub struct WidgetInfo {
    /// The type of widget this is.
    pub typ: WidgetType,

    /// Whether the widget is enabled.
    pub enabled: bool,

    /// The text on labels, buttons, checkboxes etc.
    pub label: Option<String>,

    /// The contents of some editable text (for [`TextEdit`](crate::TextEdit) fields).
    pub current_text_value: Option<String>,

    /// The previous text value.
    pub prev_text_value: Option<String>,

    /// The current value of checkboxes and radio buttons.
    pub selected: Option<bool>,

    /// The current value of sliders etc.
    pub value: Option<f64>,

    /// Selected range of characters in [`Self::current_text_value`].
    pub text_selection: Option<std::ops::RangeInclusive<usize>>,
}

impl std::fmt::Debug for WidgetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            typ,
            enabled,
            label,
            current_text_value: text_value,
            prev_text_value,
            selected,
            value,
            text_selection,
        } = self;

        let mut s = f.debug_struct("WidgetInfo");

        s.field("typ", typ);
        s.field("enabled", enabled);

        if let Some(label) = label {
            s.field("label", label);
        }
        if let Some(text_value) = text_value {
            s.field("text_value", text_value);
        }
        if let Some(prev_text_value) = prev_text_value {
            s.field("prev_text_value", prev_text_value);
        }
        if let Some(selected) = selected {
            s.field("selected", selected);
        }
        if let Some(value) = value {
            s.field("value", value);
        }
        if let Some(text_selection) = text_selection {
            s.field("text_selection", text_selection);
        }

        s.finish()
    }
}

impl WidgetInfo {
    pub fn new(typ: WidgetType) -> Self {
        Self {
            typ,
            enabled: true,
            label: None,
            current_text_value: None,
            prev_text_value: None,
            selected: None,
            value: None,
            text_selection: None,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn labeled(typ: WidgetType, label: impl ToString) -> Self {
        Self {
            label: Some(label.to_string()),
            ..Self::new(typ)
        }
    }

    /// checkboxes, radio-buttons etc
    #[allow(clippy::needless_pass_by_value)]
    pub fn selected(typ: WidgetType, selected: bool, label: impl ToString) -> Self {
        Self {
            label: Some(label.to_string()),
            selected: Some(selected),
            ..Self::new(typ)
        }
    }

    pub fn drag_value(value: f64) -> Self {
        Self {
            value: Some(value),
            ..Self::new(WidgetType::DragValue)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn slider(value: f64, label: impl ToString) -> Self {
        let label = label.to_string();
        Self {
            label: if label.is_empty() { None } else { Some(label) },
            value: Some(value),
            ..Self::new(WidgetType::Slider)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text_edit(prev_text_value: impl ToString, text_value: impl ToString) -> Self {
        let text_value = text_value.to_string();
        let prev_text_value = prev_text_value.to_string();
        let prev_text_value = if text_value == prev_text_value {
            None
        } else {
            Some(prev_text_value)
        };
        Self {
            current_text_value: Some(text_value),
            prev_text_value,
            ..Self::new(WidgetType::TextEdit)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text_selection_changed(
        text_selection: std::ops::RangeInclusive<usize>,
        current_text_value: impl ToString,
    ) -> Self {
        Self {
            text_selection: Some(text_selection),
            current_text_value: Some(current_text_value.to_string()),
            ..Self::new(WidgetType::TextEdit)
        }
    }

    /// This can be used by a text-to-speech system to describe the widget.
    pub fn description(&self) -> String {
        let Self {
            typ,
            enabled,
            label,
            current_text_value: text_value,
            prev_text_value: _,
            selected,
            value,
            text_selection: _,
        } = self;

        // TODO(emilk): localization
        let widget_type = match typ {
            WidgetType::Link => "link",
            WidgetType::TextEdit => "text edit",
            WidgetType::Button => "button",
            WidgetType::Checkbox => "checkbox",
            WidgetType::RadioButton => "radio",
            WidgetType::SelectableLabel => "selectable",
            WidgetType::ComboBox => "combo",
            WidgetType::Slider => "slider",
            WidgetType::DragValue => "drag value",
            WidgetType::ColorButton => "color button",
            WidgetType::ImageButton => "image button",
            WidgetType::CollapsingHeader => "collapsing header",
            WidgetType::Label | WidgetType::Other => "",
        };

        let mut description = widget_type.to_owned();

        if let Some(selected) = selected {
            if *typ == WidgetType::Checkbox {
                let state = if *selected { "checked" } else { "unchecked" };
                description = format!("{} {}", state, description);
            } else {
                description += if *selected { "selected" } else { "" };
            };
        }

        if let Some(label) = label {
            description = format!("{}: {}", label, description);
        }

        if typ == &WidgetType::TextEdit {
            let text;
            if let Some(text_value) = text_value {
                if text_value.is_empty() {
                    text = "blank".into();
                } else {
                    text = text_value.to_string();
                }
            } else {
                text = "blank".into();
            }
            description = format!("{}: {}", text, description);
        }

        if let Some(value) = value {
            description += " ";
            description += &value.to_string();
        }

        if !enabled {
            description += ": disabled";
        }
        description.trim().to_owned()
    }
}

/// The different types of built-in widgets in egui
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetType {
    Label, // TODO(emilk): emit Label events

    /// e.g. a hyperlink
    Link,

    TextEdit,

    Button,

    Checkbox,

    RadioButton,

    SelectableLabel,

    ComboBox,

    Slider,

    DragValue,

    ColorButton,

    ImageButton,

    CollapsingHeader,

    /// If you cannot fit any of the above slots.
    ///
    /// If this is something you think should be added, file an issue.
    Other,
}
