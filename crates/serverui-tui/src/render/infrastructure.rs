use super::{DisplayOptions, mib, safe_text};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
};
use serverui_core::{
    application::App,
    infrastructure::{Infrastructure, InfrastructureRequest, InventoryView},
};

pub(super) fn container_footer(state: &Infrastructure) -> &'static str {
    if state.confirmation.is_some() {
        "y: confirmar | n/Esc: cancelar"
    } else if state.logs.is_some() {
        "j/k: logs | r: actualizar | Esc: volver"
    } else {
        "e:motor a:iniciar d:parar R:reiniciar"
    }
}

pub(super) fn containers(
    frame: &mut Frame,
    area: Rect,
    state: &Infrastructure,
    options: DisplayOptions,
    accent: Style,
) {
    let lines = if let Some(change) = &state.confirmation {
        let name = safe_text(&change.container.name, options.ascii);
        vec![
            Line::styled(
                format!("{} ({})", change.operation.label(), change.engine.label()),
                accent,
            ),
            Line::from(
                name.chars()
                    .take(usize::from(area.width))
                    .collect::<String>(),
            ),
            Line::from(format!("ID {}", change.container.id.as_str())),
            Line::from("Puede interrumpir el servicio."),
        ]
    } else {
        let mut lines = vec![Line::styled(
            format!("{} local", state.engine.label()),
            accent,
        )];
        if let Some(request) = &state.busy {
            lines.push(Line::from(match request {
                InfrastructureRequest::List(_) => "Consultando contenedores...",
                InfrastructureRequest::Logs(..) => "Leyendo logs...",
                InfrastructureRequest::Change(_) => "Aplicando cambio...",
                InfrastructureRequest::Inventory => "Esperando al inventario...",
            }));
        }
        if let Some(message) = &state.container_message {
            lines.push(Line::from(safe_text(message, options.ascii)));
        }
        if let Some((id, logs)) = &state.logs {
            lines.push(Line::from(format!("Logs [{}]", &id.as_str()[..12])));
            if logs.is_empty() {
                lines.push(Line::from("Sin logs visibles"));
            }
            lines.extend(
                logs.iter()
                    .skip(state.container_scroll)
                    .take(usize::from(area.height))
                    .map(|line| Line::from(safe_text(line, options.ascii))),
            );
        } else if state.rows.is_empty()
            && state.listed
            && !state.limited
            && state.container_message.is_none()
        {
            lines.push(Line::from("Sin contenedores"));
        } else {
            if state.limited {
                lines.push(Line::from("Listado parcial"));
            }
            let visible = usize::from(area.height).saturating_sub(lines.len()).max(1);
            let first = (state.selected + 1).saturating_sub(visible);
            for (index, row) in state.rows.iter().enumerate().skip(first).take(visible) {
                let selected = index == state.selected;
                let text = format!(
                    "{} {}  {} [{}]",
                    if selected { ">" } else { " " },
                    safe_text(&row.name, options.ascii),
                    safe_text(&row.state, options.ascii),
                    &row.id.as_str()[..12],
                );
                lines.push(Line::styled(
                    text.chars()
                        .take(usize::from(area.width))
                        .collect::<String>(),
                    if selected {
                        accent.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ));
            }
        }
        lines
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

pub(super) fn inventory(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    options: DisplayOptions,
    accent: Style,
) {
    let state = &app.infrastructure;
    let title = match state.view {
        InventoryView::Network => "Interfaces y direcciones IP",
        InventoryView::Devices => "Dispositivos de bloque",
        InventoryView::Mounts => "Montajes y capacidad",
    };
    let mut lines = vec![Line::styled(title, accent)];
    if state.view == InventoryView::Mounts {
        if let Some(snapshot) = &app.snapshot {
            if let Some(warning) = &snapshot.disk_warning {
                lines.push(Line::from(safe_text(warning, options.ascii)));
            }
            if snapshot.disks.is_empty() {
                lines.push(Line::from("Sin montajes visibles"));
            }
            lines.extend(
                snapshot
                    .disks
                    .iter()
                    .skip(
                        state
                            .inventory_scroll
                            .min(snapshot.disks.len().saturating_sub(1)),
                    )
                    .take(usize::from(area.height))
                    .map(|disk| {
                        let name = safe_text(&disk.mount, options.ascii);
                        Line::from(match disk.space {
                            Some(space) => format!(
                                "{name}  {} / {} MiB disponibles",
                                mib(space.available_bytes),
                                mib(space.total_bytes)
                            ),
                            None => format!("{name}  capacidad no disponible"),
                        })
                    }),
            );
        } else {
            lines.push(Line::from("Esperando las metricas del sistema"));
        }
    } else if let Some(error) = &state.inventory_error {
        lines.push(Line::from(safe_text(error, options.ascii)));
    } else if let Some(inventory) = &state.inventory {
        let rows: Result<Vec<String>, _> = match state.view {
            InventoryView::Network => inventory.interfaces.as_ref().map(|rows| {
                rows.iter()
                    .map(|row| {
                        let addresses = match row.addresses.as_deref() {
                            None => "IP no disponible".into(),
                            Some([]) => "sin IP".into(),
                            Some(addresses) => addresses.join(" "),
                        };
                        format!(
                            "{}  {}  {}",
                            safe_text(&row.name, options.ascii),
                            safe_text(&row.state, options.ascii),
                            addresses
                        )
                    })
                    .collect()
            }),
            InventoryView::Devices => inventory.devices.as_ref().map(|rows| {
                rows.iter()
                    .map(|row| {
                        format!(
                            "{}  {}  {} MiB  {}",
                            safe_text(&row.name, options.ascii),
                            safe_text(&row.kind, options.ascii),
                            mib(row.bytes),
                            if row.read_only {
                                "solo lectura"
                            } else {
                                "escritura"
                            }
                        )
                    })
                    .collect()
            }),
            InventoryView::Mounts => Ok(Vec::new()),
        };
        match rows {
            Ok(rows) if rows.is_empty() => lines.push(Line::from("Sin datos visibles")),
            Ok(rows) => {
                let offset = state.inventory_scroll.min(rows.len().saturating_sub(1));
                lines.extend(
                    rows.into_iter()
                        .skip(offset)
                        .take(usize::from(area.height))
                        .map(Line::from),
                );
            }
            Err(error) => lines.push(Line::from(safe_text(&error.to_string(), options.ascii))),
        }
    } else {
        lines.push(Line::from("Consultando inventario..."));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
