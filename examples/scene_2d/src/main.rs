//! Starts the scene test on desktop systems; see the library.

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::GraphicsBackend;

const DEFAULT_LOG_FILTER: &str = "info";

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(DEFAULT_LOG_FILTER))
        .init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let backend = match std::env::args().nth(1) {
        Some(name) => name.parse()?,
        None => GraphicsBackend::Auto,
    };
    scene_2d::run_with(backend)
}
