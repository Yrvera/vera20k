//! RA2 Engine — entry point.
//!
//! Creates the winit event loop and delegates everything to App.
//! This file should stay minimal (~50 lines). All logic lives in app.rs and modules.
//!
//! Module declarations live in lib.rs so integration tests can import them.

use anyhow::Result;
use winit::event_loop::EventLoop;

fn main() -> Result<()> {
    let log_path = match vera20k::util::logging::init_file_logger("ra2") {
        Ok(path) => {
            eprintln!("Logging to {}", path.display());
            Some(path)
        }
        Err(err) => {
            eprintln!("Failed to initialize file logger: {err}");
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .init();
            None
        }
    };

    if let Some(path) = &log_path {
        vera20k::util::logging::install_panic_hook(path);
    }

    log::info!("RA2 Engine starting");
    if let Some(path) = &log_path {
        log::info!("Log file: {}", path.display());
    }

    let launch_mode = vera20k::app_launch::parse_launch_args(std::env::args_os().skip(1))?;

    // Create the OS event loop. This drives the entire application:
    // window events, input, redraws, lifecycle events.
    let event_loop: EventLoop<()> = EventLoop::builder().build()?;

    // Create the app and hand control to the event loop.
    // This blocks until the window is closed.
    let mut app: vera20k::app::App = match launch_mode {
        vera20k::app_launch::AppLaunchMode::Interactive => vera20k::app::App::new(),
        vera20k::app_launch::AppLaunchMode::ShellCapture(request) => {
            request.validate_runtime_environment()?;
            vera20k::app::App::new_shell_capture(request)
        }
        vera20k::app_launch::AppLaunchMode::TacticalCapture(request) => {
            request.validate_runtime_environment()?;
            vera20k::app::App::new_tactical_capture(request)
        }
    };
    event_loop.run_app(&mut app)?;
    app.finish_capture()?;

    log::info!("RA2 Engine shut down cleanly");
    Ok(())
}
