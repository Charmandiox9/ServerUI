use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use serverui_core::application::{App, DataStatus, Section};

#[derive(Debug, Clone, Copy)]
pub struct DisplayOptions {
    pub ascii: bool,
    pub no_color: bool,
    pub refresh_ms: u64,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            ascii: false,
            no_color: false,
            refresh_ms: 1000,
        }
    }
}

/// Neutralize terminal controls and invisible formatting. Bound output size
/// independently of provider input; widgets do the final cell-aware clipping.
fn safe_text(text: &str) -> String {
    text.chars().take(256).map(|c| {
        if c.is_control() || matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}') { '?' } else { c }
    }).collect()
}

pub fn draw(frame: &mut Frame, app: &App, options: DisplayOptions) {
    let area = frame.area();
    if area.width < 40 || area.height < 12 {
        frame.render_widget(Paragraph::new("DEMO: minimo 40x12. q: salir"), area);
        return;
    }
    let compact = area.width < 80 || area.height < 24;
    let accent = if options.no_color {
        Style::default()
    } else {
        Style::default().fg(Color::Cyan)
    };
    let sections = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new("ServerUI / v0.1.0 / DEMO").style(accent.add_modifier(Modifier::BOLD)),
        sections[0],
    );
    let navigation: Vec<Span<'_>> = Section::ALL
        .into_iter()
        .flat_map(|section| {
            let label = if compact {
                match section {
                    Section::Dashboard => "Dash",
                    Section::Processes => "Proc",
                    Section::Services => "Serv",
                    Section::Settings => "Ajust",
                }
            } else {
                section.label()
            };
            let selected = section == app.section;
            let text = if selected {
                format!("[{} {label}]", section.index() + 1)
            } else {
                format!(" {} {label} ", section.index() + 1)
            };
            [Span::styled(
                text,
                if selected {
                    accent.add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )]
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(navigation)), sections[1]);

    let title = if app.help {
        " Ayuda "
    } else {
        app.section.label()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(if options.ascii {
            BorderType::Plain
        } else {
            BorderType::Rounded
        });
    // Ratatui border sets are Unicode by default, including BorderType::Plain.
    let block = if options.ascii {
        block.border_set(ratatui::symbols::border::Set {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        })
    } else {
        block
    };
    let lines = if app.help {
        vec![
            Line::from("Tab / flechas: cambiar seccion"),
            Line::from("Shift+Tab: seccion anterior"),
            Line::from("1-4: ir a seccion"),
            Line::from("r: actualizar / reintentar"),
            Line::from("?: ayuda | Esc: cerrar ayuda"),
            Line::from("q / Ctrl+C: salir"),
            Line::from("DEMO: no administra el host"),
        ]
    } else {
        content(app, options)
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        sections[2],
    );
    frame.render_widget(
        Paragraph::new("Tab: navegar | ?: ayuda | q: salir"),
        sections[3],
    );
}

fn content(app: &App, options: DisplayOptions) -> Vec<Line<'static>> {
    match app.section {
        Section::Processes => vec![
            Line::from("Procesos: disponible en Stage 1."),
            Line::from("Sin consultas ni acciones reales."),
        ],
        Section::Services => vec![
            Line::from("Servicios: disponible en Stage 2."),
            Line::from("Sin acceso a systemd ni privilegios."),
        ],
        Section::Settings => vec![
            Line::from("Proveedor: MockProvider (DEMO)"),
            Line::from(format!("Refresco: {} ms", options.refresh_ms)),
            Line::from(format!(
                "ASCII: {} | Sin color: {}",
                options.ascii, options.no_color
            )),
            Line::from("Ajustes persistentes: pendiente."),
        ],
        Section::Dashboard => {
            let status = match &app.status {
                DataStatus::Loading => "Cargando datos simulados...".into(),
                DataStatus::Ready => "Datos simulados / host sin cambios".into(),
                DataStatus::Empty => "Sin datos simulados. r: reintentar".into(),
                DataStatus::Error(error) => format!("Error: {}", safe_text(&error.to_string())),
            };
            let mut lines = vec![Line::from(status)];
            if let Some(sample) = &app.snapshot {
                if matches!(app.status, DataStatus::Error(_)) {
                    lines.push(Line::from("Ultima muestra valida (sin actualizar)"));
                }
                const MIB: u64 = 1024 * 1024;
                lines.extend([
                    Line::from(format!("CPU: {} %", sample.cpu_percent)),
                    Line::from(format!(
                        "RAM: {} / {} MiB",
                        sample.memory_used / MIB,
                        sample.memory_total / MIB
                    )),
                    Line::from(format!(
                        "Swap: {} / {} MiB",
                        sample.swap_used / MIB,
                        sample.swap_total / MIB
                    )),
                    Line::from(format!(
                        "Disco: {} / {} MiB",
                        sample.disk_used / MIB,
                        sample.disk_total / MIB
                    )),
                    Line::from(format!("Uptime: {} s (simulado)", sample.uptime_seconds)),
                ]);
            }
            lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use serverui_core::domain::ProviderError;
    use std::convert::Infallible;

    fn screen(app: &App, width: u16, height: u16) -> Result<String, Infallible> {
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|f| {
            draw(
                f,
                app,
                DisplayOptions {
                    ascii: true,
                    no_color: true,
                    ..Default::default()
                },
            )
        })?;
        let buffer = terminal.backend().buffer();
        for cell in &buffer.content {
            assert_eq!(cell.fg, Color::Reset);
            assert_eq!(cell.bg, Color::Reset);
        }
        Ok(buffer.content.iter().map(|cell| cell.symbol()).collect())
    }

    #[test]
    fn all_states_and_sections_render_at_boundary_sizes() -> Result<(), Infallible> {
        let statuses = [
            DataStatus::Loading,
            DataStatus::Ready,
            DataStatus::Empty,
            DataStatus::Error(ProviderError::Unavailable("fallo".repeat(500))),
        ];
        for (width, height) in [(80, 24), (40, 12), (20, 5), (1, 1), (0, 0)] {
            for section in Section::ALL {
                for status in &statuses {
                    for help in [false, true] {
                        let app = App {
                            section,
                            status: status.clone(),
                            help,
                            ..App::default()
                        };
                        let output = screen(&app, width, height)?;
                        assert!(output.is_ascii());
                        if width >= 40 && height >= 12 {
                            assert!(output.contains("DEMO"));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn compact_help_keeps_exit_and_all_shortcuts_visible() -> Result<(), Infallible> {
        let app = App {
            help: true,
            ..App::default()
        };
        let output = screen(&app, 40, 12)?;
        for text in ["Tab", "Shift+Tab", "1-4", "reintentar", "Esc", "Ctrl+C"] {
            assert!(output.contains(text));
        }
        Ok(())
    }

    #[test]
    fn placeholders_and_empty_state_are_explicit() -> Result<(), Infallible> {
        let mut app = App {
            status: DataStatus::Empty,
            ..App::default()
        };
        assert!(screen(&app, 80, 24)?.contains("Sin datos simulados"));
        app.section = Section::Processes;
        assert!(screen(&app, 80, 24)?.contains("Stage 1"));
        app.section = Section::Services;
        assert!(screen(&app, 80, 24)?.contains("Stage 2"));
        Ok(())
    }

    #[test]
    fn dashboard_displays_units_and_marks_retained_data_stale() -> Result<(), Infallible> {
        use serverui_core::domain::Snapshot;
        let mut app = App::default();
        app.receive(Ok(Some(Snapshot {
            cpu_percent: 25,
            memory_used: 1024 * 1024,
            memory_total: 4 * 1024 * 1024,
            swap_used: 0,
            swap_total: 0,
            disk_used: 2 * 1024 * 1024,
            disk_total: 8 * 1024 * 1024,
            uptime_seconds: 123,
        })));
        for (width, height) in [(80, 24), (40, 12)] {
            let output = screen(&app, width, height)?;
            for text in [
                "CPU: 25 %",
                "RAM: 1 / 4 MiB",
                "Swap: 0 / 0 MiB",
                "Disco: 2 / 8 MiB",
                "Uptime: 123 s",
            ] {
                assert!(output.contains(text), "missing {text}");
            }
        }
        app.receive(Err(ProviderError::Unavailable("sin conexion".into())));
        let output = screen(&app, 80, 24)?;
        assert!(output.contains("Error: sin conexion"));
        assert!(output.contains("Ultima muestra valida"));
        assert!(output.contains("CPU: 25 %"));
        Ok(())
    }

    #[test]
    fn provider_text_cannot_emit_terminal_controls() {
        assert_eq!(safe_text("\x1b[31m\n\r\t\u{202e}text"), "?[31m????text");
        assert_eq!(safe_text(&"x".repeat(1000)).len(), 256);
    }
}
