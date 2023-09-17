#![feature(return_position_impl_trait_in_trait)]
#![feature(adt_const_params)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

pub mod accessibility;
pub mod app;
pub mod atlas;
pub mod buffer;
pub mod color;
pub mod debug;
pub mod element;
pub mod graphics;
pub mod hash;
pub mod history;
pub mod input;
pub mod num;
pub mod scene;
pub mod shape;
pub mod surface;
pub mod text;
pub mod texture;
pub mod time;
pub mod util;

pub use accesskit;
pub use cosmic_text;
pub use keyframe;
pub use lyon;
pub use palette;

pub mod math {
    pub use crate::util::math::*;
}

use app::ToposEvent;
pub use refbox;
