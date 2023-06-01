#![feature(return_position_impl_trait_in_trait)]
// #![feature(new_uninit)]
// #![feature(maybe_uninit_write_slice)]

mod app;
mod atlas;
mod buffer;
mod debug;
mod element;
mod graphics;
mod hash;
mod num;
mod paint;
mod scene;
mod shape;
mod surface;
mod text;
mod time;
mod util;

use pollster::FutureExt;

pub async fn run() {
    app::App::new().await.run();
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
