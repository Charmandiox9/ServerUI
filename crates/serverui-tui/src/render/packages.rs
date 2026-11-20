use super::{DisplayOptions, safe_text};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
};
use serverui_core::packages::{PackageRequest, Packages};

pub(super) fn footer(packages: &Packages) -> &'static str {
    if packages.confirmation.is_some() {
        "y: aplicar | n/Esc: cancelar | j/k"
    } else if packages.editing {
        "Enter: buscar | Esc: volver | ^U: limpia"
    } else if matches!(packages.busy, Some(PackageRequest::Apply(_))) {
        "APT en curso | Tab: cambiar de seccion"
    } else if packages
        .report
        .as_ref()
        .is_some_and(|report| report.undo_install.is_some())
    {
        "b: deshacer | j/k: salida | Esc: volver"
    } else if packages.report.is_some() {
        "j/k: salida | Esc: volver | /: buscar"
    } else {
        "/: buscar i: instalar u: actual. d: quitar"
    }
}

pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    packages: &Packages,
    options: DisplayOptions,
    accent: Style,
) {
    let lines = if let Some(plan) = &packages.confirmation {
        let mut lines = vec![
            Line::styled(
                format!("{} {}", plan.operation.label(), plan.name.as_str()),
                accent,
            ),
            Line::from("Simulacion APT. Revisa los cambios."),
            Line::from(format!(
                "Cambios {}/{} (j/k)",
                packages.scroll + 1,
                plan.changes.len()
            )),
        ];
        lines.extend(
            plan.changes
                .iter()
                .skip(packages.scroll)
                .take(usize::from(area.height))
                .map(|line| Line::from(safe_text(line, options.ascii))),
        );
        lines
    } else if let Some(request) = &packages.busy {
        let mut lines = match request {
            PackageRequest::Recommendations => vec![Line::from("Consultando recomendados APT...")],
            PackageRequest::Search(name) => {
                vec![Line::from(format!("Buscando {}...", name.as_str()))]
            }
            PackageRequest::Preview(name, operation) => {
                vec![Line::from(format!(
                    "Simulando {} {}...",
                    operation.label(),
                    name.as_str()
                ))]
            }
            PackageRequest::Apply(plan) => vec![
                Line::styled(
                    format!("{} {} (APT)", plan.operation.label(), plan.name.as_str()),
                    accent,
                ),
                Line::from("Espera a que APT termine para salir."),
            ],
        };
        if let Some(progress) = &packages.progress {
            if let Some(value) = progress.percent {
                lines.push(Line::styled(format!("Progreso APT: {value}%"), accent));
            }
            lines.push(Line::from(safe_text(&progress.line, options.ascii)));
        }
        lines
    } else if let Some(report) = &packages.report {
        let mut lines = vec![Line::styled(
            safe_text(&report.status, options.ascii),
            accent,
        )];
        lines.extend(
            report
                .lines
                .iter()
                .skip(packages.scroll)
                .take(usize::from(area.height))
                .map(|line| Line::from(safe_text(line, options.ascii))),
        );
        lines
    } else {
        let mut lines = vec![Line::from(if packages.editing {
            format!("Buscar: {}_", packages.query)
        } else if let Some(query) = &packages.active_query {
            format!("Cache APT: {}", query.as_str())
        } else {
            String::from("Recomendados APT")
        })];
        if let Some(message) = &packages.message {
            lines.push(Line::from(safe_text(message, options.ascii)));
        }
        if packages.rows.is_empty() {
            if packages.searched && packages.active_query.is_none() {
                lines.push(Line::from("Sin recomendados disponibles. /: buscar"));
            } else if packages.searched {
                lines.push(Line::from("Sin coincidencias"));
            } else if !packages.editing {
                lines.push(Line::from("Pulsa / y escribe un nombre de paquete."));
            }
        } else {
            if packages.limited {
                lines.push(Line::from("Resultados limitados. Afina la busqueda."));
            }
            let visible = usize::from(area.height).saturating_sub(lines.len()).max(1);
            let first = (packages.selected + 1).saturating_sub(visible);
            for (index, row) in packages.rows.iter().enumerate().skip(first).take(visible) {
                let selected = packages.selected == index;
                let text = format!(
                    "{} {}  {}",
                    if selected { ">" } else { " " },
                    row.name.as_str(),
                    safe_text(&row.description, options.ascii),
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
