mod runner;

use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use serverui_providers::{MockProvider, Scenario};
use serverui_tui::{render::DisplayOptions, terminal};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum DemoScenario {
    #[default]
    Normal,
    Empty,
    Error,
}

#[derive(Debug, Parser)]
#[command(
    version,
    about = "ServerUI: administracion visual de servidor. Stage 0: solo DEMO."
)]
struct Args {
    /// Usar datos simulados sin consultar ni modificar el host
    #[arg(long)]
    demo: bool,
    /// Bordes ASCII para terminales limitados
    #[arg(long)]
    ascii: bool,
    /// Usar los colores predeterminados del terminal
    #[arg(long)]
    no_color: bool,
    /// Intervalo entre muestras simuladas (100..60000 ms)
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(100..=60000))]
    refresh_ms: u64,
    /// Escenario reproducible para revisar estados de la demo
    #[arg(long, value_enum, default_value_t = DemoScenario::Normal, requires = "demo")]
    demo_scenario: DemoScenario,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match start(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ServerUI: {error}");
            ExitCode::FAILURE
        }
    }
}

fn start(args: Args) -> io::Result<()> {
    if !args.demo {
        return Err(io::Error::other(
            "el proveedor real esta pendiente; ejecuta cargo run -- --demo",
        ));
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "--demo requiere una terminal interactiva (TTY) en stdin y stdout",
        ));
    }
    let scenario = match args.demo_scenario {
        DemoScenario::Normal => Scenario::Normal,
        DemoScenario::Empty => Scenario::Empty,
        DemoScenario::Error => Scenario::RecoverableError,
    };
    let mut provider = MockProvider::new(scenario);
    let options = DisplayOptions {
        ascii: args.ascii,
        no_color: args.no_color
            || std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
        refresh_ms: args.refresh_ms,
    };
    terminal::install_panic_hook();
    terminal::run(|terminal| {
        runner::run(
            &mut provider,
            &mut runner::TerminalEvents::default(),
            Duration::from_millis(args.refresh_ms),
            |app| {
                terminal
                    .draw(|frame| serverui_tui::render::draw(frame, app, options))
                    .map(|_| ())
            },
        )
        .map(|_| ())
    })
}
