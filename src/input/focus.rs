use std::collections::HashSet;

use crate::element::ElementId;

/// Keeps tracks of what widget has keyboard focus
#[derive(Clone, Debug, Default)]
pub(crate) struct FocusState {
    /// The widget with keyboard focus (i.e. a text input field).
    pub(crate) id: Option<ElementId>,

    /// What had keyboard focus previous frame?
    id_previous_frame: Option<ElementId>,

    /// Give focus to this widget next frame
    id_next_frame: Option<ElementId>,

    id_requested_by_accesskit: Option<accesskit::NodeId>,

    /// If set, the next widget that is interested in focus will automatically get it.
    /// Probably because the user pressed Tab.
    give_to_next: bool,

    /// The last widget interested in focus.
    last_interested: Option<ElementId>,

    /// If `true`, pressing tab will NOT move focus away from the current widget.
    is_focus_locked: bool,

    /// Set at the beginning of the frame, set to `false` when "used".
    pressed_tab: bool,

    /// Set at the beginning of the frame, set to `false` when "used".
    pressed_shift_tab: bool,

    focusable_elements_this_frame: HashSet<ElementId>,
}

impl FocusState {
    /// Which widget currently has keyboard focus?
    pub fn focused(&self) -> Option<ElementId> {
        self.id
    }

    pub fn just_focused(&self, id: ElementId) -> bool {
        self.focused() == Some(id) && !self.had_focus_last_frame(id)
    }

    pub fn has_focus(&self, id: ElementId) -> bool {
        self.focused() == Some(id)
    }

    pub(super) fn request_focus(&mut self, id: ElementId) {
        self.id = Some(id);
    }

    pub(super) fn surrender_focus(&mut self, id: ElementId) {
        if self.id == Some(id) {
            self.id = None
        }
    }

    pub(super) fn lock_focus(&mut self, id: ElementId, lock_focus: bool) {
        if self.id == Some(id) {
            self.is_focus_locked = lock_focus;
        }
    }

    pub(super) fn has_lock_focus(&mut self, id: ElementId) -> bool {
        if self.had_focus_last_frame(id) && self.has_focus(id) {
            self.is_focus_locked
        } else {
            false
        }
    }

    pub(super) fn begin_frame(&mut self, new_input: &crate::input::RawInput) {
        self.id_previous_frame = self.id;
        if let Some(id) = self.id_next_frame.take() {
            self.id = Some(id);
        }

        self.id_requested_by_accesskit = None;

        self.pressed_tab = false;
        self.pressed_shift_tab = false;
        for event in &new_input.events {
            if matches!(
                event,
                crate::input::Event::Key {
                    key: crate::input::Key::Escape,
                    pressed: true,
                    modifiers: _,
                    ..
                }
            ) {
                self.id = None;
                self.is_focus_locked = false;
                break;
            }

            if let crate::input::Event::Key {
                key: crate::input::Key::Tab,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                if !self.is_focus_locked {
                    // these are reversed from egui because the input pass goes
                    // in the reverse order of elements
                    if modifiers.shift {
                        self.pressed_tab = true;
                    } else {
                        self.pressed_shift_tab = true;
                    }
                }
            }

            {
                if let crate::input::Event::AccessKitActionRequest(accesskit::ActionRequest {
                    action: accesskit::Action::Focus,
                    target,
                    data: None,
                }) = event
                {
                    self.id_requested_by_accesskit = Some(*target);
                }
            }
        }

        self.focusable_elements_this_frame.clear();
    }

    pub(crate) fn end_frame(&mut self) {
        if let Some(id) = self.id {
            // Allow calling `request_focus` one frame and not using it until next frame
            let recently_gained_focus = self.id_previous_frame != Some(id);

            if !recently_gained_focus && !self.focusable_elements_this_frame.contains(&id) {
                // Dead-mans-switch: the widget with focus has disappeared!
                self.id = None;
            }
        }
    }

    pub(crate) fn had_focus_last_frame(&self, id: ElementId) -> bool {
        self.id_previous_frame == Some(id)
    }

    pub(super) fn interested_in_focus(&mut self, id: ElementId) {
        if self.id_requested_by_accesskit == Some(id.as_access_id()) {
            self.id = Some(id);
            self.id_requested_by_accesskit = None;
            self.give_to_next = false;
            self.pressed_tab = false;
            self.pressed_shift_tab = false;
        }

        if self.give_to_next && !self.had_focus_last_frame(id) {
            self.id = Some(id);
            self.give_to_next = false;
        } else if self.id == Some(id) {
            if self.pressed_tab && !self.is_focus_locked {
                self.id = None;
                self.give_to_next = true;
                self.pressed_tab = false;
            } else if self.pressed_shift_tab && !self.is_focus_locked {
                self.id_next_frame = self.last_interested; // frame-delay so gained_focus works
                self.pressed_shift_tab = false;
            }
        // } else if self.pressed_tab && self.id.is_none() && !self.give_to_next {
        } else if self.id.is_none() {
            if self.pressed_tab && !self.give_to_next {
                // nothing has focus and the user pressed tab - give focus to the first widgets that wants it:
                self.id = Some(id);
                self.pressed_tab = false;
            } else if self.pressed_shift_tab && self.id_next_frame.is_none() {
                self.id = Some(id);
                self.pressed_shift_tab = false;
            }
        }

        self.focusable_elements_this_frame.insert(id);

        self.last_interested = Some(id);
    }
}
