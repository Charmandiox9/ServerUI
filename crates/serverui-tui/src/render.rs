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
mod infrastructure;
mod packages;

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
        .title(Line::from(title).style(Style::default().add_modifier(Modifier::BOLD)))
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
            } else if app.section == Section::Services {
                "Servicios: acciones con confirmacion"
            } else if app.section == Section::Packages {
                "APT: cambios con confirmacion"
            } else if app.section == Section::Containers {
                "Contenedores: cambios con confirmacion"
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
            match (app.mode, app.section) {
                (ProviderMode::Linux, Section::Services) => "LINUX / SERVICIOS",
                (ProviderMode::Linux, Section::Packages) => "LINUX / APT",
                (ProviderMode::Linux, Section::Containers) => "LINUX / CONTENEDORES",
                _ => app.mode.label(),
            }
        ))
        .style(accent.add_modifier(Modifier::BOLD)),
        sections[0],
    );
    let nav: Vec<Span<'_>> = Section::ALL
        .into_iter()
        .map(|section| {
            let label = if area.width < 56 {
                match section {
                    Section::Dashboard => "D",
                    Section::Processes => "P",
                    Section::Services => "S",
                    Section::Settings => "A",
                    Section::Packages => "T",
                    Section::Containers => "C",
                    Section::Inventory => "R",
                }
            } else if area.width < 100 {
                match section {
                    Section::Dashboard => "Dash",
                    Section::Processes => "Proc",
                    Section::Services => "Serv",
                    Section::Settings => "Ajust",
                    Section::Packages => "APT",
                    Section::Containers => "Cont",
                    Section::Inventory => "Red",
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
    let state_style = if options.no_color {
        Style::default()
    } else if matches!(app.status, DataStatus::Error(_)) {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(status(app, options)).style(state_style),
        sections[2],
    );
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
            Paragraph::new(if app.section == Section::Services {
                vec![
                    Line::from("Tab/Shift+Tab: seccion | /: filtro"),
                    Line::from("j/k: fila/log | Enter: logs | Esc: volver"),
                    Line::from("a: iniciar | d: parar | R: reiniciar"),
                    Line::from("e: habilitar al arranque (no inicia)"),
                    Line::from("y: confirmar | n/Esc: cancelar"),
                    Line::from("h: historial | r: refrescar | q: salir"),
                ]
            } else if app.section == Section::Packages {
                vec![
                    Line::from("/: buscar en cache | r: repetir"),
                    Line::from("j/k: seleccionar o desplazar salida"),
                    Line::from("i: instalar | u: actualizar | d: quitar"),
                    Line::from("y: aplicar plan | n/Esc: cancelar"),
                    Line::from("APT requiere autorizacion puntual"),
                    Line::from("Tab: seccion | q: salir al terminar"),
                ]
            } else if app.section == Section::Containers {
                vec![
                    Line::from("e: Docker/Podman | r: consultar"),
                    Line::from("j/k: elegir | Enter: logs | Esc: volver"),
                    Line::from("a: iniciar | d: parar | R: reiniciar"),
                    Line::from("y: confirmar | n/Esc: cancelar"),
                    Line::from("La operacion usa permisos existentes"),
                ]
            } else if app.section == Section::Inventory {
                vec![
                    Line::from("n: red | b: bloques | m: montajes"),
                    Line::from("j/k: desplazar | r: actualizar"),
                    Line::from("Inventario de solo lectura"),
                ]
            } else {
                vec![
                    Line::from("Tab/Shift+Tab/flechas: seccion; 1-7"),
                    Line::from("Arriba/abajo: fila/disco; Enter: detalle"),
                    Line::from("/: filtro | s: orden | S: invertir"),
                    Line::from("r: refrescar | Esc: volver/cerrar"),
                    Line::from("?: ayuda | q/Ctrl+C: salir"),
                    Line::from("Filtro: Enter/Esc fin; Ctrl+U limpia"),
                ]
            }),
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
    } else if app.section == Section::Services {
        services(frame, inner, app, options, accent);
    } else if app.section == Section::Packages {
        packages::draw(frame, inner, &app.packages, options, accent);
    } else if app.section == Section::Containers {
        infrastructure::containers(frame, inner, &app.infrastructure, options, accent);
    } else if app.section == Section::Inventory {
        infrastructure::inventory(frame, inner, app, options, accent);
    } else if app.section == Section::Dashboard {
        dashboard(frame, inner, app, options, accent);
    } else if app.section == Section::Processes {
        processes(frame, inner, app, options, accent);
    } else {
        frame.render_widget(
            Paragraph::new(content(app, options)).wrap(Wrap { trim: false }),
            inner,
        );
    }
    let footer = if app.section == Section::Packages {
        packages::footer(&app.packages)
    } else if app.section == Section::Containers {
        infrastructure::container_footer(&app.infrastructure)
    } else if app.section == Section::Inventory {
        "n:red b:bloques m:montajes r:actualizar"
    } else if app.section == Section::Services && app.services.confirmation.is_some() {
        "y: confirmar | n/Esc: cancelar | q: salir"
    } else if app.section == Section::Services && app.services.editing {
        "Filtro | Enter/Esc: fin | Ctrl+U: limpiar"
    } else if app.section == Section::Services
        && (app.services.logs.is_some() || app.services.show_history)
    {
        "j/k: logs | r: actualizar | Esc: volver"
    } else if app.section == Section::Services {
        "a:iniciar d:parar R:reiniciar e:habilitar"
    } else if app.editing_filter {
        "Escribe filtro | Enter/Esc: fin | ^C:salir"
    } else if app.inspecting.is_some() {
        "Esc: cerrar detalle | q/Ctrl+C: salir"
    } else if app.section == Section::Processes {
        "/: filtro | s: orden | Enter: detalle | q"
    } else {
        "j/k: discos | ?: ayuda | q: salir"
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().add_modifier(Modifier::BOLD)),
        sections[4],
    );
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
        .row_highlight_style(accent.add_modifier(Modifier::BOLD | Modifier::REVERSED))
        .highlight_symbol("> ");
    let mut state = TableState::default().with_selected(Some(selected - start));
    frame.render_stateful_widget(table, parts[1], &mut state);
}

fn dashboard(frame: &mut Frame, area: Rect, app: &App, options: DisplayOptions, accent: Style) {
    if area.width < 76 || area.height < 12 || app.snapshot.is_none() {
        frame.render_widget(
            Paragraph::new(content(app, options)).wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let Some(snapshot) = &app.snapshot else {
        return;
    };
    let parts = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);
    let metrics = Layout::horizontal([Constraint::Length(38), Constraint::Min(1)]).split(parts[0]);
    let texts = vec![
        Line::from(format!(
            "CPU: {} ({} CPU)",
            percent(snapshot.cpu_usage),
            snapshot.cpu_count
        )),
        Line::from(format!(
            "RAM: {} / {} MiB",
            mib(snapshot.memory_used),
            mib(snapshot.memory_total)
        )),
        Line::from(format!(
            "Swap: {} / {} MiB",
            mib(snapshot.swap_used),
            mib(snapshot.swap_total)
        )),
        Line::from(format!("Uptime: {} s", snapshot.uptime_seconds)),
        Line::from(format!("Procesos visibles: {}", snapshot.process_count)),
    ];
    frame.render_widget(Paragraph::new(texts), metrics[0]);
    let ratio = |used: u64, total: u64| {
        (total > 0).then(|| ((u128::from(used) * 10000 / u128::from(total)).min(10000)) as u32)
    };
    let bars = [
        snapshot.cpu_usage,
        ratio(snapshot.memory_used, snapshot.memory_total),
        ratio(snapshot.swap_used, snapshot.swap_total),
    ]
    .into_iter()
    .map(|value| {
        let width = usize::from(metrics[1].width.saturating_sub(9)).min(32);
        let filled = value.map_or(0, |v| (v.min(10000) as usize * width / 10000).min(width));
        let label = if value.is_none() {
            "--".into()
        } else {
            percent(value)
        };
        Line::from(vec![
            Span::raw("["),
            Span::styled("#".repeat(filled), accent),
            Span::raw("-".repeat(width - filled)),
            Span::raw(format!("] {label}")),
        ])
    })
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(bars), metrics[1]);
    frame.render_widget(
        Paragraph::new(format!(
            "MONTAJES  {}  |  j/k: desplazar",
            snapshot.disks.len()
        ))
        .style(accent.add_modifier(Modifier::BOLD)),
        parts[1],
    );
    let rows = snapshot
        .disks
        .iter()
        .skip(app.dashboard_scroll)
        .take(usize::from(parts[2].height.saturating_sub(2)))
        .map(|disk| {
            let (free, total) = disk.space.map_or(("--".into(), "--".into()), |space| {
                (mib(space.available_bytes), mib(space.total_bytes))
            });
            Row::new(vec![safe_text(&disk.mount, options.ascii), free, total])
        });
    let warning = snapshot
        .disk_warning
        .as_deref()
        .map(|w| safe_text(w, options.ascii))
        .unwrap_or_else(|| {
            if snapshot.disks.is_empty() {
                "Sin montajes visibles".into()
            } else {
                "Capacidad por montaje; no se suman volumenes".into()
            }
        });
    let table_area = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(parts[2]);
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Min(18),
                Constraint::Length(18),
                Constraint::Length(18),
            ],
        )
        .column_spacing(2)
        .header(
            Row::new(["Montaje", "Disponible MiB", "Total MiB"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        ),
        table_area[0],
    );
    frame.render_widget(Paragraph::new(warning), table_area[1]);
}

fn services(frame: &mut Frame, area: Rect, app: &App, options: DisplayOptions, accent: Style) {
    let service = &app.services;
    if let Some((unit, operation)) = &service.confirmation {
        let parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(format!("{}?", operation.label()))
                .style(accent.add_modifier(Modifier::BOLD)),
            parts[0],
        );
        let width = usize::from(parts[1].width.max(1));
        let name = safe_text(unit.as_str(), options.ascii);
        let max_scroll = name
            .len()
            .div_ceil(width)
            .saturating_sub(usize::from(parts[1].height));
        let scroll = service.log_scroll.min(max_scroll).min(u16::MAX as usize) as u16;
        frame.render_widget(
            Paragraph::new(name)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            parts[1],
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(if app.mode == ProviderMode::Demo {
                    "DEMO: accion simulada"
                } else {
                    "Puede cortar servicios/SSH; sin rollback"
                }),
                Line::from("j/k: revisar unidad | y: confirmar"),
            ]),
            parts[2],
        );
        return;
    }
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    if service.show_history {
        frame.render_widget(
            Paragraph::new("HISTORIAL DE ESTA SESION").style(accent.add_modifier(Modifier::BOLD)),
            parts[0],
        );
        let lines = service
            .history
            .iter()
            .rev()
            .skip(service.log_scroll)
            .take(usize::from(parts[1].height))
            .map(|line| Line::from(safe_text(line, options.ascii)))
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(if lines.is_empty() {
                vec![Line::from("Sin acciones ejecutadas")]
            } else {
                lines
            }),
            parts[1],
        );
    } else if let Some((unit, lines)) = &service.logs {
        frame.render_widget(
            Paragraph::new(safe_text(unit.as_str(), options.ascii))
                .style(accent.add_modifier(Modifier::BOLD)),
            parts[0],
        );
        let rows = lines
            .iter()
            .skip(service.log_scroll)
            .take(usize::from(parts[1].height))
            .map(|line| Line::from(safe_text(line, options.ascii)))
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(if rows.is_empty() {
                vec![Line::from("Sin entradas visibles; revisar permisos")]
            } else {
                rows
            }),
            parts[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!(
                "/{}{} | Enter: logs",
                safe_text(&service.filter, options.ascii),
                if service.editing { "_" } else { "" }
            )),
            parts[0],
        );
        let rows = service.visible();
        if rows.is_empty() {
            frame.render_widget(
                Paragraph::new(if service.busy {
                    "Consultando servicios..."
                } else if service.loaded {
                    "Sin servicios que coincidan"
                } else {
                    "r: consultar servicios systemd"
                }),
                parts[1],
            );
        } else {
            let selected = rows
                .iter()
                .position(|s| Some(&s.unit) == service.selected.as_ref())
                .unwrap_or(0);
            let capacity = usize::from(parts[1].height.saturating_sub(1).max(1));
            let start = selected / capacity * capacity;
            let wide = area.width >= 70;
            let data = rows.iter().skip(start).take(capacity).map(|s| {
                let mut cells = vec![
                    safe_text(s.unit.as_str(), options.ascii),
                    safe_text(&s.active, options.ascii),
                ];
                if wide {
                    cells.push(safe_text(&s.sub, options.ascii));
                    cells.push(safe_text(&s.description, options.ascii));
                }
                Row::new(cells)
            });
            let widths = if wide {
                vec![
                    Constraint::Percentage(36),
                    Constraint::Length(12),
                    Constraint::Length(12),
                    Constraint::Min(8),
                ]
            } else {
                vec![Constraint::Min(20), Constraint::Length(12)]
            };
            let header = if wide {
                vec!["Servicio", "Estado", "Subestado", "Descripcion"]
            } else {
                vec!["Servicio", "Estado"]
            };
            let table = Table::new(data, widths)
                .header(Row::new(header).style(Style::default().add_modifier(Modifier::BOLD)))
                .column_spacing(1)
                .highlight_symbol("> ")
                .row_highlight_style(accent.add_modifier(Modifier::REVERSED | Modifier::BOLD));
            frame.render_stateful_widget(
                table,
                parts[1],
                &mut TableState::default().with_selected(Some(selected - start)),
            );
        }
    }
    let message = if service.busy {
        "Consultando...; q: salir sin esperar".into()
    } else if let Some(message) = &service.message {
        safe_text(message, options.ascii)
    } else if service.limited {
        "Lista limitada; algunas unidades omitidas".into()
    } else {
        format!(
            "{} cargados | /:filtro h:historial ?:ayuda",
            service.units.len()
        )
    };
    frame.render_widget(Paragraph::new(message), parts[2]);
}

fn content(app: &App, options: DisplayOptions) -> Vec<Line<'static>> {
    match app.section {
        Section::Services => Vec::new(),
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
        Section::Processes | Section::Packages | Section::Containers | Section::Inventory => {
            Vec::new()
        }
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
    #[test]
    fn service_table_logs_history_and_confirmation_fit_and_sanitize() -> Result<(), Infallible> {
        use serverui_core::services::*;
        let unit = UnitName::parse("fixture.service").unwrap_or_else(|e| panic!("{e}"));
        let mut app = App {
            section: Section::Services,
            ..App::default()
        };
        app.services.receive(
            ServiceRequest::List,
            Ok(ServiceData::List {
                units: vec![ServiceInfo {
                    unit: unit.clone(),
                    load: "loaded".into(),
                    active: "failed".into(),
                    sub: "failed".into(),
                    description: "bad\x1b\u{202e}".into(),
                }],
                limited: false,
            }),
        );
        for (w, h) in [(40, 12), (80, 24), (120, 30)] {
            let output = screen(&app, w, h)?;
            assert!(output.contains("fixture.service"));
            assert!(output.contains("failed"));
            assert!(!output.contains('\x1b'));
            app.services.propose(ServiceOperation::Restart);
            let output = screen(&app, w, h)?;
            for expected in ["Reiniciar?", "fixture.service", "DEMO", "y: confirmar"] {
                assert!(output.contains(expected), "{expected}");
            }
            app.services.confirmation = None;
        }
        app.services.receive(
            ServiceRequest::Logs(unit),
            Ok(ServiceData::Logs(vec!["bad\x1b[31m\u{202e}".into(); 80])),
        );
        assert!(!screen(&app, 40, 12)?.contains('\x1b'));
        app.services.show_history = true;
        assert!(screen(&app, 40, 12)?.contains("Sin acciones ejecutadas"));
        Ok(())
    }
    #[test]
    fn compact_container_confirmation_shows_engine_and_full_id() -> Result<(), Infallible> {
        use serverui_core::infrastructure::{
            Container, ContainerChange, ContainerId, ContainerOperation, Engine,
        };
        let mut app = App {
            section: Section::Containers,
            ..App::default()
        };
        let id = ContainerId::parse(&"a".repeat(64)).unwrap_or_else(|| panic!("fixture"));
        app.infrastructure.confirmation = Some(ContainerChange {
            engine: Engine::Podman,
            operation: ContainerOperation::Stop,
            container: Container {
                id,
                name: "fixture\x1b".into(),
                state: "running".into(),
            },
        });
        let output = screen(&app, 40, 12)?;
        assert!(output.contains("Detener (Podman)"));
        assert!(output.contains("y: confirmar"));
        assert!(!output.contains('\x1b'));
        assert_eq!(
            output
                .split(|c| c != 'a')
                .filter(|part| part.len() >= 8)
                .map(str::len)
                .sum::<usize>(),
            64
        );
        Ok(())
    }

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
