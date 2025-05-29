mod element;

pub async fn run() {
    topos::app::App::<element::TestRoot>::run();
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
        .filter_module("handlebars", log::LevelFilter::Warn)
        .init();

    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::default());

    #[cfg(not(target_arch = "wasm32"))]
    pollster::FutureExt::block_on(run());

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run());
}
