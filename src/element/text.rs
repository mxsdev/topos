use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use cosmic_text::Edit;
use num_traits::ToPrimitive;

use crate::{
    accessibility::{AccessNodeBuilder, AccessRole},
    atlas::AtlasAllocation,
    color::ColorRgba,
    input::{input_state::InputState, output::CursorIcon, Key},
    math::{PhysicalSize, Pos, Vector},
    scene::layout::{AvailableSpace, FlexBox, LayoutPassResult},
    shape::{PaintFill, PaintRectangle},
    util::{
        guard::ReadLockable, layout::{LayoutStyle, TaffyNodeContext}, text::{AtlasContentType, CachedFloat, FontSystemRef, HasBuffer, TextBox, TextBoxLike, TextCacheBuffer}, DeviceUnit, LogicalUnit, PhysicalUnit
    },
};

use crate::util::text::{Attrs, Metrics};

use crate::{
    element::Element,
    math::{Rect, Size},
    scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
};

use super::{boundary::RectLikeBoundary, Response};

pub type TextBoxEditorElement = TextBoxElement<cosmic_text::Editor<'static>>;

pub struct TextBoxElement<Buffer: HasBuffer = cosmic_text::Buffer> {
    buffer: Arc<Mutex<TextCacheBuffer<Buffer>>>,
    layout_node: LayoutPassResult,
    response: Option<Response<Rect>>,
}

impl<Buffer: HasBuffer + 'static> TextBoxElement<Buffer> {
    pub fn new(
        scene_resources: &mut SceneResources,
        metrics: Metrics,
        color: impl Into<PaintFill>,
        text: String,
        attrs: Attrs<'static>,
        layout: impl Into<LayoutStyle>,
    ) -> Self {
        let buffer = {
            let mut font_system = scene_resources.font_system();

            let mut buffer = TextBox::<Buffer>::new(
                &mut font_system,
                metrics.font_size,
                metrics.line_height,
                color,
                Pos::default(),
            );

            buffer.set_text(&mut font_system, &text, &attrs);

            buffer.shape_until_scroll(&mut font_system);

            buffer
        };

        let buffer = Arc::new(Mutex::new(TextCacheBuffer::from(buffer)));

        let layout_node = scene_resources
            .layout_engine()
            .new_leaf_with_context(layout, TaffyNodeContext::Text(buffer.clone()))
            .unwrap();

        let interactive = buffer.lock().unwrap().buffer.buffer.editor_mut().is_some();

        Self {
            buffer,
            layout_node,
            response: interactive.then(|| Response::new(Rect::default()).with_clickable(true).with_focusable(true)),
        }
    }
}

impl<Buffer: HasBuffer + 'static> TextBoxElement<Buffer> {
    pub fn set_layout(&mut self, layout: impl Into<LayoutStyle>) {
        self.layout_node.set_style(layout);
    }
}

impl<Buffer: HasBuffer + 'static> Element for TextBoxElement<Buffer> {
    fn layout(&mut self, layout_pass: &mut LayoutPass) -> LayoutPassResult {
        self.layout_node.clone()
    }

    fn layout_post(&mut self, resources: &mut SceneResources, rect: Rect) {
        self.buffer.lock().unwrap().buffer.pos = rect.min;

        resources.prepare_text(&self.buffer.lock().unwrap().buffer);
    }

    fn input_with_resources(&mut self, input: &mut InputState, resources: &mut SceneResources, rect: Rect) {
        if input.is_focused() {
            input.editing_text = true;
        }
        
        if let Some(response) = &mut self.response {
            let mut buffer = self.buffer.lock().unwrap();

            response.update_rect(
                input,
                Rect::from_min_size(
                    rect.min,
                    buffer.computed_size.unwrap_or_else(|| rect.size()),
                ),
            );

            let Some(editor) = buffer.buffer.buffer.editor_mut() else {
                log::debug!("No editor");
                return;
            };

            if response.hovered() {
                if let Some(pos) = response.latest_mouse_pos() {
                    if response.primary_clicked() {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Click {
                            x: (pos.x - response.boundary.min.x) as i32,
                            y: (pos.y - response.boundary.min.y) as i32,
                        });
                    }
                }
            }

            if response.focused() {
                input.events.iter().for_each(|event| match event {
                    crate::input::Event::Text(text) => {
                        editor.insert_string(text, None)
                    }

                    crate::input::Event::Key { pressed: true, key: Key::Backspace, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Backspace);
                    }

                    crate::input::Event::Key { pressed: true, key: Key::ArrowLeft, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Motion(cosmic_text::Motion::Left));
                    }

                    crate::input::Event::Key { pressed: true, key: Key::ArrowRight, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Motion(cosmic_text::Motion::Right));
                    }

                    crate::input::Event::Key { pressed: true, key: Key::ArrowUp, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Motion(cosmic_text::Motion::Up));
                    }

                    crate::input::Event::Key { pressed: true, key: Key::ArrowDown, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Motion(cosmic_text::Motion::Down));
                    }

                    crate::input::Event::Key { pressed: true, key: Key::Home, .. } => {
                        editor.action(&mut resources.font_system(), cosmic_text::Action::Motion(cosmic_text::Motion::Home));
                    }

                    _ => {}
                    
                    // crate::input::Event::Key { pressed: true, key, physical_key, .. } => {
                    //     // editor.action(&mut resources.font_system(), cosmic_text::Action::Insert() {
                    //     key.name()

                    //     editor.action(&mut resources.font_system(), cosmic_text::Action::Insert(event.text));
                    // },
                })
            }
        }
    }

    fn ui(&mut self, ctx: &mut SceneContext, rect: Rect) {
        let mut buffer = self.buffer.lock().unwrap();

        let buffer_ref: &dyn TextBoxLike = &buffer.buffer;
        ctx.add_shape(buffer_ref);

        let Some(response) = &mut self.response else {
            return;
        };

        let Some(editor) = buffer.buffer.buffer.editor_mut() else {
            log::debug!("No editor");
            return;
        };
        
        if response.focused() {
            if let Some(cursor) = editor.cursor_position() {
                let relative_pos =  Vector::<f32, LogicalUnit>::new(cursor.0 as f32, cursor.1 as f32);
                
                ctx.add_shape(PaintRectangle::from_rect(Rect::from_min_size(
                    response.boundary.min + relative_pos,
                    Size::new(1., editor.buffer().metrics().line_height),
                )).with_fill(PaintFill::Color(ColorRgba::new(1., 0., 0., 1.))));
            }
        }

        if response.hovered() {
            ctx.set_cursor(CursorIcon::Text);
        }
    }

    fn ui_post(&mut self, ctx: &mut SceneContext, _rect: Rect) {
        // ctx.pop_clip_rect();
    }

    fn node(&self) -> AccessNodeBuilder {
        AccessNodeBuilder::new(AccessRole::TextRun)
    }
}

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// enum AvailableSpaceCache {
//     Definite(CachedFloat),
//     MinContent,
//     MaxContent,
// }

// impl From<AvailableSpace> for AvailableSpaceCache {
//     fn from(value: AvailableSpace) -> Self {
//         match value {
//             AvailableSpace::Definite(v) => Self::Definite(v.into()),
//             AvailableSpace::MinContent => Self::MinContent,
//             AvailableSpace::MaxContent => Self::MaxContent,
//         }
//     }
// }
