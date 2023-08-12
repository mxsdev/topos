#![feature(return_position_impl_trait_in_trait)]
#![feature(adt_const_params)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

mod accessibility;
mod app;
mod atlas;
mod buffer;
mod color;
mod debug;
mod element;
mod graphics;
mod hash;
mod history;
mod input;
mod lib;
mod math;
mod num;
mod scene;
mod shape;
mod surface;
mod test;
mod text;
mod texture;
mod time;
mod util;

use app::ToposEvent;
pub use refbox;

use test::{TestRect, TestRoot};
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub async fn run() {
    let event_loop = EventLoopBuilder::<ToposEvent>::with_user_event().build();

    let app = app::App::<TestRoot>::new(&event_loop).await;
    app.run(event_loop)
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::formatted_timed_builder()
        .parse_env(env_logger::Env::default().filter_or(
            env_logger::DEFAULT_FILTER_ENV,
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "warn"
            },
        ))
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("cosmic_text", log::LevelFilter::Warn)
        .filter_module("tao", log::LevelFilter::Warn)
        .filter_module("winit", log::LevelFilter::Warn)
        .init();

    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::default());

    #[cfg(not(target_arch = "wasm32"))]
    pollster::FutureExt::block_on(run());

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run());
}
