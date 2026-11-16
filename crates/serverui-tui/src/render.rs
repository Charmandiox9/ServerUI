use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState, Wrap},
};
use serverui_core::{
    application::{App, DataStatus, Section},
    domain::ProviderMode,
};

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

fn safe_text(text: &str, ascii: bool) -> String {
    text.chars().take(256).map(|c| {
        if c.is_control() || matches!(c,'\u{061c}'|'\u{200b}'..='\u{200f}'|'\u{2028}'..='\u{202f}'|'\u{2060}'..='\u{206f}') || (ascii && !c.is_ascii()) {'?'} else {c}
    }).collect()
}
fn percent(value: Option<u32>) -> String {
    value.map_or_else(
        || "--".into(),
        |v| format!("{}.{:01}%", v / 100, (v % 100) / 10),
    )
}
fn mib(value: u64) -> String {
    format!("{:.1}", value as f64 / (1024.0 * 1024.0))
}
fn border<'a>(title: &'a str, options: DisplayOptions) -> Block<'a> {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(if options.ascii {
            BorderType::Plain
        } else {
            BorderType::Rounded
        });
    if options.ascii {
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
    }
}
fn status(app: &App, options: DisplayOptions) -> String {
    match &app.status {
        DataStatus::Loading => "Cargando... q/Ctrl+C: salir".into(),
        DataStatus::Empty => "Sin datos. r: reintentar".into(),
        DataStatus::Error(error) => format!(
            "Error; datos antiguos; r: {}",
            safe_text(&error.to_string(), options.ascii)
        ),
        DataStatus::Ready => format!(
            "{}{}",
            if app.mode == ProviderMode::Demo {
                "Datos simulados"
            } else {
                "Datos Linux / solo lectura"
            },
            if app.refreshing {
                " | actualizando"
            } else {
                ""
            }
        ),
    }
}

pub fn draw(frame: &mut Frame, app: &App, options: DisplayOptions) {
    let area = frame.area();
    if area.width < 40 || area.height < 12 {
        frame.render_widget(
            Paragraph::new(format!("{}: minimo 40x12. q: salir", app.mode.label())),
            area,
        );
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
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!(
            "ServerUI / v{} / {}",
            env!("CARGO_PKG_VERSION"),
            app.mode.label()
        ))
        .style(accent.add_modifier(Modifier::BOLD)),
        sections[0],
    );
    let nav: Vec<Span<'_>> = Section::ALL
        .into_iter()
        .map(|section| {
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
            Span::styled(
                if selected {
                    format!("[{} {label}]", section.index() + 1)
                } else {
                    format!(" {} {label} ", section.index() + 1)
                },
                if selected {
                    accent.add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(nav)), sections[1]);
    frame.render_widget(Paragraph::new(status(app, options)), sections[2]);
    let title = if app.help {
        " Ayuda "
    } else if app.inspecting.is_some() {
        " Detalle de proceso "
    } else {
        app.section.label()
    };
    let block = border(title, options);
    let inner = block.inner(sections[3]);
    frame.render_widget(block, sections[3]);
    if app.help {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Tab/Shift+Tab/flechas: seccion; 1-4"),
                Line::from("Arriba/abajo: fila/disco; Enter: detalle"),
                Line::from("/: filtro | s: orden | S: invertir"),
                Line::from("r: refrescar | Esc: volver/cerrar"),
                Line::from("?: ayuda | q/Ctrl+C: salir"),
                Line::from("Filtro: Enter/Esc fin; Ctrl+U limpia"),
            ]),
            inner,
        );
    } else if let Some(process) = app.inspected_process() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(
                    "Nombre: {}",
                    safe_text(&process.name, options.ascii)
                )),
                Line::from(format!(
                    "PID: {} | Padre: {}",
                    process.id.pid,
                    process
                        .parent_pid
                        .map_or_else(|| "--".into(), |p| p.to_string())
                )),
                Line::from(format!(
                    "Estado: {}",
                    safe_text(&process.status, options.ascii)
                )),
                Line::from(format!(
                    "CPU: {} | RSS: {} MiB",
                    percent(process.cpu_usage),
                    mib(process.memory_bytes)
                )),
                Line::from(format!(
                    "Virtual: {} MiB (no es RAM)",
                    mib(process.virtual_memory_bytes)
                )),
                Line::from(format!(
                    "Inicio: {} | Activo: {} s",
                    process.id.start_time, process.run_time_seconds
                )),
            ]),
            inner,
        );
    } else if app.section == Section::Processes {
        processes(frame, inner, app, options, accent);
    } else {
        frame.render_widget(
            Paragraph::new(content(app, options)).wrap(Wrap { trim: false }),
            inner,
        );
    }
    let footer = if app.editing_filter {
        "Escribe filtro | Enter/Esc: fin | ^C:salir"
    } else if app.inspecting.is_some() {
        "Esc: cerrar detalle | q/Ctrl+C: salir"
    } else if app.section == Section::Processes {
        "/: filtro | s: orden | Enter: detalle | q"
    } else {
        "Arriba/abajo: discos | ?: ayuda | q:salir"
    };
    frame.render_widget(Paragraph::new(footer), sections[4]);
}

fn processes(frame: &mut Frame, area: Rect, app: &App, options: DisplayOptions, accent: Style) {
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let filter = safe_text(&app.filter, options.ascii);
    frame.render_widget(
        Paragraph::new(format!(
            "/{}{} | s:{} {}",
            filter,
            if app.editing_filter { "_" } else { "" },
            app.sort.label(),
            if app.descending { "v" } else { "^" }
        )),
        parts[0],
    );
    let rows = app.visible_processes();
    let total = app.snapshot.as_ref().map_or(0, |s| s.process_count);
    let captured = app.snapshot.as_ref().map_or(0, |s| s.processes.len());
    let footer = app.notice.clone().unwrap_or_else(|| {
        format!(
            "{} / {} visibles{}; CPU 100%=1 nucleo",
            rows.len(),
            total,
            if total > captured { " (limitado)" } else { "" }
        )
    });
    frame.render_widget(Paragraph::new(footer), parts[2]);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(if app.snapshot.is_some() {
                "Sin procesos que coincidan"
            } else {
                "Procesos aun no disponibles"
            }),
            parts[1],
        );
        return;
    }
    let selected = rows
        .iter()
        .position(|p| Some(p.id) == app.selected)
        .unwrap_or(0);
    let capacity = parts[1].height.saturating_sub(1).max(1) as usize;
    let start = selected / capacity * capacity;
    let wide = area.width >= 70;
    let data = rows.iter().skip(start).take(capacity).map(|p| {
        let mut cells = vec![
            p.id.pid.to_string(),
            percent(p.cpu_usage),
            mib(p.memory_bytes),
        ];
        if wide {
            cells.push(safe_text(&p.status, options.ascii));
        }
        cells.push(safe_text(&p.name, options.ascii));
        Row::new(cells)
    });
    let (widths, header) = if wide {
        (
            vec![
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Min(10),
            ],
            vec!["PID", "CPU", "RAM MiB", "Estado", "Nombre"],
        )
    } else {
        (
            vec![
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(6),
            ],
            vec!["PID", "CPU", "RAM MiB", "Nombre"],
        )
    };
    let table = Table::new(data, widths)
        .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
        .column_spacing(1)
        .row_highlight_style(accent.add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    let mut state = TableState::default().with_selected(Some(selected - start));
    frame.render_stateful_widget(table, parts[1], &mut state);
}

fn content(app: &App, options: DisplayOptions) -> Vec<Line<'static>> {
    match app.section {
        Section::Services => vec![
            Line::from("Servicios: disponible en Stage 2."),
            Line::from("Sin acceso a systemd ni privilegios."),
        ],
        Section::Settings => vec![
            Line::from(format!("Proveedor: {}", app.mode.label())),
            Line::from(format!("Refresco: {} ms", options.refresh_ms)),
            Line::from(format!(
                "ASCII: {} | Sin color: {}",
                options.ascii, options.no_color
            )),
            Line::from("Ajustes persistentes: pendiente."),
        ],
        Section::Dashboard => {
            let Some(s) = app.snapshot.as_ref() else {
                return vec![
                    Line::from(match &app.status {
                        DataStatus::Error(e) => safe_text(&e.to_string(), options.ascii),
                        _ => "Esperando una muestra del proveedor".into(),
                    }),
                    Line::from("r: reintentar | q: salir"),
                ];
            };
            let mut lines = vec![
                Line::from(format!(
                    "CPU: {} ({} CPU)",
                    percent(s.cpu_usage),
                    s.cpu_count
                )),
                Line::from(format!(
                    "RAM: {} / {} MiB",
                    mib(s.memory_used),
                    mib(s.memory_total)
                )),
                Line::from(format!(
                    "Swap: {} / {} MiB",
                    mib(s.swap_used),
                    mib(s.swap_total)
                )),
                Line::from(format!("Uptime: {} s", s.uptime_seconds)),
                Line::from(format!("Discos: {} (arriba/abajo)", s.disks.len())),
            ];
            if let Some(warning) = &s.disk_warning {
                lines.push(Line::from(safe_text(warning, options.ascii)));
            }
            if s.disks.is_empty() {
                lines.push(Line::from("Sin montajes visibles"));
            }
            for disk in s.disks.iter().skip(app.dashboard_scroll) {
                let mount = safe_text(&disk.mount, options.ascii);
                lines.push(Line::from(match disk.space {
                    Some(space) => format!(
                        "{}: {}/{} MiB libres",
                        mount,
                        mib(space.available_bytes),
                        mib(space.total_bytes)
                    ),
                    None => format!("{mount}: espacio no disponible"),
                }));
            }
            lines
        }
        Section::Processes => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use serverui_core::{
        application::Action,
        domain::{DiskInfo, DiskSpace, ProcessId, ProcessInfo, ProviderError, Snapshot},
    };
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
        for cell in &terminal.backend().buffer().content {
            assert_eq!(cell.fg, Color::Reset);
            assert_eq!(cell.bg, Color::Reset);
        }
        Ok(terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect())
    }
    fn sample() -> Snapshot {
        let processes = (1..=20)
            .map(|pid| ProcessInfo {
                id: ProcessId { pid, start_time: 1 },
                parent_pid: Some(0),
                name: format!("proc-{pid}"),
                status: "Running".into(),
                cpu_usage: Some(2500),
                memory_bytes: 1024 * 1024,
                virtual_memory_bytes: 2 * 1024 * 1024,
                run_time_seconds: 1,
            })
            .collect();
        Snapshot {
            cpu_usage: Some(2500),
            cpu_count: 4,
            memory_used: 1024 * 1024,
            memory_total: 4 * 1024 * 1024,
            uptime_seconds: 123,
            disks: vec![DiskInfo {
                mount: "/".into(),
                space: Some(DiskSpace {
                    total_bytes: 8 * 1024 * 1024,
                    available_bytes: 6 * 1024 * 1024,
                }),
            }],
            process_count: 20,
            processes,
            ..Snapshot::default()
        }
    }
    #[test]
    fn all_modes_sections_states_and_sizes_are_safe() -> Result<(), Infallible> {
        for mode in [ProviderMode::Demo, ProviderMode::Linux] {
            for (w, h) in [(80, 24), (40, 12), (20, 5), (1, 1), (0, 0)] {
                for section in Section::ALL {
                    for status in [
                        DataStatus::Loading,
                        DataStatus::Ready,
                        DataStatus::Empty,
                        DataStatus::Error(ProviderError::Unavailable("bad\x1b".repeat(500))),
                    ] {
                        let mut a = App::new(mode);
                        a.section = section;
                        a.status = status;
                        for help in [false, true] {
                            a.help = help;
                            let output = screen(&a, w, h)?;
                            assert!(output.is_ascii());
                            if w >= 40 && h >= 12 {
                                assert!(output.contains(if mode == ProviderMode::Demo {
                                    "DEMO"
                                } else {
                                    "LINUX"
                                }));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[test]
    fn help_fits_compact() -> Result<(), Infallible> {
        let a = App {
            help: true,
            ..App::default()
        };
        let output = screen(&a, 40, 12)?;
        for text in ["Shift+Tab", "Enter", "s: orden", "Ctrl+C", "Ctrl+U"] {
            assert!(output.contains(text), "{text}");
        }
        Ok(())
    }
    #[test]
    fn process_table_paginates_filters_and_inspects() -> Result<(), Infallible> {
        let mut a = App::default();
        a.receive(Ok(Some(sample())));
        a.update(Action::Select(Section::Processes));
        a.update(Action::LastRow);
        let output = screen(&a, 40, 12)?;
        assert!(output.contains("proc-20"));
        assert!(output.contains("RAM MiB"));
        a.update(Action::Inspect);
        assert!(screen(&a, 40, 12)?.contains("PID: 20"));
        a.update(Action::Close);
        a.update(Action::StartFilter);
        a.update(Action::FilterInput('!'));
        assert!(screen(&a, 40, 12)?.contains("Sin procesos que coincidan"));
        Ok(())
    }
    #[test]
    fn dashboard_units_warmup_and_stale_status() -> Result<(), Infallible> {
        let mut a = App::new(ProviderMode::Linux);
        a.receive(Ok(Some(sample())));
        for (w, h) in [(80, 24), (40, 12)] {
            let output = screen(&a, w, h)?;
            for text in ["CPU: 25.0%", "RAM: 1.0 / 4.0 MiB", "Uptime: 123 s"] {
                assert!(output.contains(text), "{text}");
            }
        }
        a.receive(Err(ProviderError::Unavailable("fallo".into())));
        assert!(screen(&a, 80, 24)?.contains("datos antiguos"));
        let mut s = sample();
        s.cpu_usage = None;
        a.receive(Ok(Some(s)));
        assert!(screen(&a, 80, 24)?.contains("CPU: --"));
        Ok(())
    }
    #[test]
    fn untrusted_process_mount_and_error_text_are_neutralized() -> Result<(), Infallible> {
        let mut s = sample();
        s.processes[0].name = "evil\x1b[31m\n\u{202e}界".into();
        s.disks[0].mount = "/\x1b]0;bad\x07".into();
        let mut a = App::default();
        a.receive(Ok(Some(s)));
        assert!(!screen(&a, 80, 24)?.contains('\x1b'));
        a.update(Action::Select(Section::Processes));
        a.update(Action::Inspect);
        let output = screen(&a, 80, 24)?;
        assert!(output.is_ascii());
        assert!(output.contains("evil?"));
        assert_eq!(safe_text(&"x".repeat(500), true).len(), 256);
        Ok(())
    }
}
