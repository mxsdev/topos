#![feature(return_position_impl_trait_in_trait)]

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
mod mesh;
mod num;
mod scene;
mod shape;
mod surface;
mod test;
mod text;
mod time;
mod util;

use app::ToposEvent;
pub use refbox;

use pollster::FutureExt;
use test::{TestRect, TestRoot};
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub async fn run() {
    let event_loop = EventLoopBuilder::<ToposEvent>::with_user_event().build();
    app::App::<TestRoot>::new(&event_loop).await.run(event_loop);
}

fn main() {
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

    run().block_on();
}
