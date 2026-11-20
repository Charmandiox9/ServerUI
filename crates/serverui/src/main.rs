mod action_worker;
mod runner;
mod sampler;

use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use serverui_core::domain::{ProviderError, ProviderMode, SystemProvider};
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
    about = "ServerUI: metricas, procesos, systemd, APT, contenedores, red y discos. --demo: simulacion."
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
    /// Intervalo entre muestras de metricas (100..60000 ms)
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
    if !args.demo && !cfg!(target_os = "linux") {
        return Err(io::Error::other(
            "el proveedor real requiere Linux; ejecuta cargo run -- --demo",
        ));
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "se requiere una terminal interactiva (TTY) en stdin y stdout",
        ));
    }
    let scenario = match args.demo_scenario {
        DemoScenario::Normal => Scenario::Normal,
        DemoScenario::Empty => Scenario::Empty,
        DemoScenario::Error => Scenario::RecoverableError,
    };
    let mode = if args.demo {
        ProviderMode::Demo
    } else {
        ProviderMode::Linux
    };
    let options = DisplayOptions {
        ascii: args.ascii,
        no_color: args.no_color
            || std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
        refresh_ms: args.refresh_ms,
    };
    terminal::install_panic_hook();
    let mut source = sampler::Sampler::spawn(move || make_provider(args.demo, scenario))?;
    source.service_factory = Some(Box::new(move || make_services(args.demo, scenario)));
    source.package_factory = Some(Box::new(move || make_packages(args.demo, scenario)));
    source.infrastructure_factory =
        Some(Box::new(move || make_infrastructure(args.demo, scenario)));
    terminal::run(|terminal| {
        runner::run(
            &mut source,
            &mut runner::TerminalEvents::new()?,
            Duration::from_millis(args.refresh_ms),
            mode,
            |app| {
                terminal
                    .draw(|frame| serverui_tui::render::draw(frame, app, options))
                    .map(|_| ())
            },
        )
        .map(|_| ())
    })
}

fn make_infrastructure(
    demo: bool,
    scenario: Scenario,
) -> Box<dyn serverui_core::infrastructure::InfrastructureProvider> {
    if demo {
        return Box::new(serverui_providers::MockInfrastructure::new(scenario));
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(serverui_providers::LinuxInfrastructure)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(serverui_providers::MockInfrastructure::new(Scenario::Empty))
    }
}

fn make_packages(
    demo: bool,
    scenario: Scenario,
) -> Box<dyn serverui_core::packages::PackageProvider> {
    if demo {
        return Box::new(serverui_providers::MockPackages::new(scenario));
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(serverui_providers::LinuxPackages)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(serverui_providers::MockPackages::new(Scenario::Empty))
    }
}

fn make_provider(
    demo: bool,
    scenario: Scenario,
) -> Result<Box<dyn SystemProvider + Send>, ProviderError> {
    if demo {
        return Ok(Box::new(MockProvider::new(scenario)));
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(serverui_providers::LinuxProvider::new()?))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(ProviderError::Unavailable(
            "El proveedor real requiere Linux".into(),
        ))
    }
}

fn make_services(
    demo: bool,
    scenario: Scenario,
) -> Box<dyn serverui_core::services::ServiceProvider> {
    if demo {
        return Box::new(serverui_providers::MockServices::new(scenario));
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(serverui_providers::LinuxServices)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(serverui_providers::MockServices::new(Scenario::Empty))
    }
}
