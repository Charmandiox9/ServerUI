//! Subprocess fixture for PTY tests; never called from the ServerUI binary.
use serverui_tui::terminal;
use std::{io, process::ExitCode};

fn main() -> ExitCode {
    let mode = std::env::args().nth(1).unwrap_or_default();
    terminal::install_panic_hook();
    let result = terminal::run(|terminal| {
        terminal.draw(|frame| frame.render_widget("terminal probe", frame.area()))?;
        match mode.as_str() {
            "background-panic" => {
                let caught = std::thread::spawn(|| {
                    std::panic::catch_unwind(|| panic!("injected worker panic")).is_err()
                })
                .join()
                .map_err(|_| io::Error::other("worker fixture failed"))?;
                if !caught {
                    return Err(io::Error::other("worker panic not caught"));
                }
                terminal.draw(|frame| frame.render_widget("WORKER_OK", frame.area()))?;
                Ok(())
            }
            "panic" => panic!("injected PTY panic"),
            "error" => Err(io::Error::other("injected PTY error")),
            _ => Ok(()),
        }
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
