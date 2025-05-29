use crate::{
    element::boundary::{Boundary, RectLikeBoundary},
    input::{input_state::InputState, Key, PointerButton},
    math::{Rect, RoundedRect},
};
use num_traits::Float;
use paste::paste;

use super::ElementId;

pub struct Response<B: Boundary = RoundedRect, const NUM_POINTER_BUTTONS: usize = 3> {
    pub boundary: B,

    // user config
    focusable: bool,
    focus_locked: bool,
    focus_on_click: bool,
    focus_on_mouse_down: bool,
    blur_on_click_outside: bool,
    hoverable: bool,
    clickable: bool,
    consume_hover: bool,

    // state
    hovered: bool,

    clicked: [bool; NUM_POINTER_BUTTONS],
    double_clicked: [bool; NUM_POINTER_BUTTONS],
    triple_clicked: [bool; NUM_POINTER_BUTTONS],
    pointer_button_down_on: [bool; NUM_POINTER_BUTTONS],
    pointer_button_released: [bool; NUM_POINTER_BUTTONS],

    focused: bool,
    focused_within: bool,
    just_focused: bool,
    just_blurred: bool,
}

impl<B: Boundary + Default, const NUM_POINTER_BUTTONS: usize> Default
    for Response<B, NUM_POINTER_BUTTONS>
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

macro_rules! input_boundary_config_boilerplate {
    ($field: ident, $ty: ty) => {
        paste! {
            pub fn [<with_ $field>](mut self, $field: $ty) -> Self {
                self.[<set_ $field>]($field);
                self
            }

            pub fn [<is_ $field>](&self) -> $ty {
                self.$field
            }
        }
    };
}

macro_rules! input_boundary_config_setter {
    ($field: ident, $ty: ty) => {
        paste! {
            // inline always?
            pub fn [<set_ $field>](&mut self, $field: $ty) {
                self.$field = $field
            }
        }
    };
}

macro_rules! input_boundary_config {
    ($field: ident, $ty: ty) => {
        input_boundary_config_boilerplate!($field, $ty);
        input_boundary_config_setter!($field, $ty);
    };
}

macro_rules! input_boundary_state {
    ($field: ident, $ty: ty) => {
        paste! {
            pub fn [<$field>](&self) -> $ty {
                self.$field
            }
        }
    };
}

impl<B: Boundary, const NUM_POINTER_BUTTONS: usize> Response<B, NUM_POINTER_BUTTONS> {
    pub fn new(boundary: B) -> Self {
        Self {
            boundary,

            focusable: false,
            focus_locked: false,
            focus_on_click: true,
            focus_on_mouse_down: false,
            blur_on_click_outside: true,
            hoverable: true,
            consume_hover: true,
            clickable: true,

            clicked: [Default::default(); NUM_POINTER_BUTTONS],
            double_clicked: [Default::default(); NUM_POINTER_BUTTONS],
            triple_clicked: [Default::default(); NUM_POINTER_BUTTONS],
            pointer_button_down_on: [Default::default(); NUM_POINTER_BUTTONS],
            pointer_button_released: [Default::default(); NUM_POINTER_BUTTONS],

            focused: Default::default(),
            focused_within: Default::default(),
            hovered: Default::default(),
            just_focused: Default::default(),
            just_blurred: Default::default(),
        }
    }

    input_boundary_config!(focus_locked, bool);
    input_boundary_config!(consume_hover, bool);
    input_boundary_config!(focus_on_click, bool);

    input_boundary_config_boilerplate!(clickable, bool);

    pub fn set_clickable(&mut self, clickable: bool) {
        self.clickable = clickable;

        if !clickable {
            self.pointer_button_down_on = [false; NUM_POINTER_BUTTONS];
        }
    }

    input_boundary_config_boilerplate!(focusable, bool);
    input_boundary_config!(focus_on_mouse_down, bool);

    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;

        if !focusable {
            self.focused = false;
        }
    }

    input_boundary_config_boilerplate!(hoverable, bool);

    pub fn set_hoverable(&mut self, hoverable: bool) {
        self.hoverable = hoverable;

        if !hoverable {
            self.hovered = false;
        }
    }

    input_boundary_state!(focused, bool);
    input_boundary_state!(just_focused, bool);
    input_boundary_state!(just_blurred, bool);
    input_boundary_state!(hovered, bool);

    pub fn primary_clicked(&self) -> bool {
        self.clicked_by(PointerButton::Primary)
    }

    pub fn secondary_clicked(&self) -> bool {
        self.clicked_by(PointerButton::Secondary)
    }

    pub fn middle_clicked(&self) -> bool {
        self.clicked_by(PointerButton::Middle)
    }

    pub fn clicked_by(&self, button: PointerButton) -> bool {
        self.clicked
            .get(button.as_u16() as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn double_clicked_by(&self, button: PointerButton) -> bool {
        self.double_clicked
            .get(button.as_u16() as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn triple_clicked_by(&self, button: PointerButton) -> bool {
        self.triple_clicked
            .get(button.as_u16() as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn pointer_button_down_on(&self, button: PointerButton) -> bool {
        self.pointer_button_down_on
            .get(button.as_u16() as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn primary_button_down_on(&self) -> bool {
        self.pointer_button_down_on(PointerButton::Primary)
    }

    pub fn secondary_button_down_on(&self) -> bool {
        self.pointer_button_down_on(PointerButton::Secondary)
    }

    pub fn middle_button_down_on(&self) -> bool {
        self.pointer_button_down_on(PointerButton::Middle)
    }

    pub fn hovered_or_pointer_down_on(&self, button: PointerButton) -> bool {
        self.hovered() || self.pointer_button_down_on(button)
    }

    pub fn hovered_or_primary_down_on(&self) -> bool {
        self.hovered() || self.pointer_button_down_on(PointerButton::Primary)
    }

    pub fn hovered_or_secondary_down_on(&self) -> bool {
        self.hovered() || self.pointer_button_down_on(PointerButton::Secondary)
    }

    pub fn hovered_or_middle_down_on(&self) -> bool {
        self.hovered() || self.pointer_button_down_on(PointerButton::Middle)
    }

    pub fn pointer_button_released(&self, button: PointerButton) -> bool {
        self.pointer_button_released
            .get(button.as_u16() as usize)
            .copied()
            .unwrap_or(false)
    }

    #[inline(always)]
    fn supported_pointer_buttons(&self) -> impl Iterator<Item = PointerButton> {
        (0..NUM_POINTER_BUTTONS as u16).map(PointerButton::from_u16)
    }

    pub fn request_focus(&self, input: &mut InputState) {
        if self.focusable {
            input.request_focus();
        }
    }

    pub fn request_blur(&self, input: &mut InputState) {
        input.surrender_focus();
    }

    pub fn update(&mut self, input: &mut InputState) {
        if self.focusable {
            input.interested_in_focus();
        }

        self.focused_within = input.focused_within();

        {
            let new_focus = input.is_focused();

            self.just_focused = !self.focused && new_focus;
            self.just_blurred = self.focused && !new_focus;

            self.focused = new_focus;
        }

        if self.hoverable {
            self.hovered = input
                .pointer
                .hover_pos()
                .map_or(false, |hover| self.boundary.is_inside(&hover));
        }

        self.clicked = [Default::default(); NUM_POINTER_BUTTONS];
        self.double_clicked = [Default::default(); NUM_POINTER_BUTTONS];
        self.triple_clicked = [Default::default(); NUM_POINTER_BUTTONS];

        if self.hovered && self.consume_hover {
            input.pointer.consume_hover();
        }

        if self.clickable {
            for button in self.supported_pointer_buttons() {
                let idx = button.as_u16() as usize;

                // TODO: faster to iterate through `pointer.pointer_events`

                if self.hovered {
                    self.clicked[idx] = input.pointer.button_clicked(button);
                    self.double_clicked[idx] = input.pointer.button_clicked(button);
                    self.triple_clicked[idx] = input.pointer.button_clicked(button);

                    if input.pointer.button_pressed(button) {
                        self.pointer_button_down_on[idx] = true;
                    }
                }

                if input.pointer.button_released(button) {
                    self.pointer_button_down_on[idx] = false;
                }
            }
        }

        // enter/space are primary click
        if self.clickable
            && self.focused
            && (input.key_pressed(Key::Space) || input.key_pressed(Key::Enter))
        {
            self.clicked[PointerButton::Primary.as_u16() as usize] = true;
        }

        if self.clickable && input.current_element.map_or(false, |id| input.has_accesskit_action_request(id, accesskit::Action::Click)) {
            self.clicked[PointerButton::Primary.as_u16() as usize] = true;
        }

        if self.focus_on_click && self.primary_clicked() {
            input.request_focus();
        }

        if self.focus_on_mouse_down && self.primary_button_down_on() {
            input.request_focus();
        }

        if self.blur_on_click_outside && input.pointer.primary_clicked() && !self.hovered {
            input.surrender_focus();
        }
    }

    pub fn update_rect<F: Float, U>(&mut self, input: &mut InputState, rect: Rect<F, U>)
    where
        B: RectLikeBoundary<F, U>,
    {
        self.boundary.set_rect(rect);
        self.update(input)
    }

    pub fn update_boundary<U>(&mut self, input: &mut InputState, boundary: B) {
        self.boundary = boundary;
        self.update(input)
    }
}
