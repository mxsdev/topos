// use std::{
//     ops::{Deref, DerefMut},
//     sync::{Arc, Mutex},
// };

// use crate::{
//     accessibility::{AccessNodeBuilder, AccessRole},
//     atlas::AtlasAllocation,
//     color::ColorRgba,
//     math::{PhysicalSize, Pos},
//     scene::layout::{AvailableSpace, FlexBox, LayoutPassResult},
//     shape::PaintFill,
//     util::{
//         guard::ReadLockable, layout::{LayoutStyle, TaffyNodeContext}, text::{AtlasContentType, CachedFloat, FontSystemRef, TextBox, TextCacheBuffer}
//     },
// };

// use crate::util::text::{Attrs, Metrics};

// use crate::{
//     element::Element,
//     math::{Rect, Size},
//     scene::{ctx::SceneContext, layout::LayoutPass, scene::SceneResources},
// };

// pub struct TextEditElement {
//     text_box: TextBoxElement,
// }

// impl TextEditElement {
//     pub fn new(
//         scene_resources: &mut SceneResources,
//         metrics: Metrics,
//         color: impl Into<PaintFill>,
//         text: String,
//         attrs: Attrs<'static>,
//         layout: impl Into<LayoutStyle>,
//     ) -> Self {
//         Self {
//         }
//     }
// }