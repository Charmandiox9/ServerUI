//! Deterministic preview of the actual rendered buffer; no terminal or host I/O.
//! cargo run -p serverui-tui --example render_preview -- [dashboard|services|confirm|logs|compact]
//! Output is SVG; redirect it to a development artifact, never a production path.
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use serverui_core::{
    application::{App, Section},
    domain::{DiskInfo, DiskSpace, Snapshot},
    services::*,
};
use serverui_tui::render::{DisplayOptions, draw};
use std::error::Error;

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn main() -> Result<(), Box<dyn Error>> {
    let view = std::env::args().nth(1).unwrap_or_default();
    let (width, height) = if view == "compact" {
        (40, 12)
    } else {
        (80, 24)
    };
    let mut app = App::default();
    const GIB: u64 = 1024 * 1024 * 1024;
    app.receive(Ok(Some(Snapshot {
        cpu_usage: Some(2450),
        cpu_count: 4,
        memory_used: 3 * GIB,
        memory_total: 8 * GIB,
        swap_total: 2 * GIB,
        uptime_seconds: 86400,
        process_count: 72,
        disks: vec![DiskInfo {
            mount: "/ (demo)".into(),
            space: Some(DiskSpace {
                total_bytes: 64 * GIB,
                available_bytes: 46 * GIB,
            }),
        }],
        ..Snapshot::default()
    })));
    if ["services", "confirm", "logs"].contains(&view.as_str()) {
        app.section = Section::Services;
        let units = ["cron.service", "nginx.service", "ssh.service"]
            .into_iter()
            .map(|name| {
                Ok(ServiceInfo {
                    unit: UnitName::parse(name)?,
                    load: "loaded".into(),
                    active: "active".into(),
                    sub: "running".into(),
                    description: "Servicio simulado".into(),
                })
            })
            .collect::<Result<Vec<_>, serverui_core::domain::ProviderError>>()?;
        app.services.receive(
            ServiceRequest::List,
            Ok(ServiceData::List {
                units,
                limited: false,
            }),
        );
        if view == "confirm" {
            app.services.propose(ServiceOperation::Restart);
        }
        if view == "logs" {
            app.services.receive(
                ServiceRequest::Logs(UnitName::parse("cron.service")?),
                Ok(ServiceData::Logs(vec![
                    "DEMO cron.service: inicio simulado".into(),
                    "DEMO Sin acceso al journal del host".into(),
                ])),
            );
        }
    }
    let mut terminal = Terminal::new(TestBackend::new(width, height))?;
    terminal.draw(|frame| {
        draw(
            frame,
            &app,
            DisplayOptions {
                ascii: true,
                ..Default::default()
            },
        )
    })?;
    println!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"#11151c\"/>",
        width * 12 + 32,
        height * 24 + 32,
        width * 12 + 32,
        height * 24 + 32
    );
    for y in 0..height {
        for x in 0..width {
            let cell = &terminal.backend().buffer()[(x, y)];
            let mut fg = match cell.fg {
                Color::Cyan => "#67d7e5",
                Color::Yellow => "#f2ce75",
                _ => "#e8e8eb",
            };
            let mut bg = "#11151c";
            if cell.modifier.contains(Modifier::REVERSED) {
                std::mem::swap(&mut fg, &mut bg);
            }
            let weight = if cell.modifier.contains(Modifier::BOLD) {
                "bold"
            } else {
                "normal"
            };
            println!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"24\" fill=\"{bg}\"/><text x=\"{}\" y=\"{}\" fill=\"{fg}\" font-family=\"Consolas,monospace\" font-size=\"20\" font-weight=\"{weight}\">{}</text>",
                16 + x * 12,
                16 + y * 24,
                16 + x * 12,
                35 + y * 24,
                xml(cell.symbol())
            );
        }
    }
    println!("</svg>");
    Ok(())
}
