//! Starts the example on desktop systems; see the library.

use std::error::Error;
use std::process::ExitCode;

const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn,symphonia=warn";

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
    sounds_2d::run()
}
