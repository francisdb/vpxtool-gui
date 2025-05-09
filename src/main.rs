use std::io;
use std::process::ExitCode;
use vpxtool::config;
use vpxtool_gui::guifrontend;

fn main() -> ExitCode {
    run().unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        ExitCode::FAILURE
    })
}

fn run() -> io::Result<ExitCode> {
    if let Some((_, resolved_config)) = config::load_config()? {
        // TODO we want to run the indexer once the frontend has started and report progress in the frontend
        guifrontend::guifrontend(resolved_config.clone())
    } else {
        let warning = "No config file found. Run vpxtool to create one.";
        eprintln!("{}", warning);
        Ok(ExitCode::FAILURE)
    }
}
