use crate::cli::tui::data;

use super::*;

fn opencode_configured_provider_count(data: &UiData) -> usize {
    data.providers
        .rows
        .iter()
        .filter(|row| row.is_in_config)
        .count()
}

fn main_provider_status(app: &App, data: &UiData) -> String {
    if matches!(app.app_type, AppType::OpenCode) {
        return texts::tui_provider_config_count(
            opencode_configured_provider_count(data),
            data.providers.rows.len(),
        );
    }

    data.providers
        .rows
        .iter()
        .find(|p| p.is_current)
        .map(|row| data::provider_display_name(&app.app_type, row))
        .unwrap_or_else(|| texts::none().to_string())
}

fn main_api_url(app: &App, data: &UiData) -> String {
    let api_url = if matches!(app.app_type, AppType::OpenCode) {
        data.providers
            .rows
            .iter()
            .find(|p| p.is_in_config)
            .and_then(|p| p.api_url.as_deref())
    } else {
        data.providers
            .rows
            .iter()
            .find(|p| p.is_current)
            .and_then(|p| p.api_url.as_deref())
    };

    api_url.unwrap_or(texts::tui_na()).to_string()
}

pub(super) fn render_main(
    frame: &mut Frame<'_>,
    app: &App,
    data: &UiData,
    area: Rect,
    theme: &super::theme::Theme,
) {
    let current_provider = main_provider_status(app, data);

    let api_url = main_api_url(app, data);

    let label_width = 14;
    let value_style = Style::default().fg(theme.cyan);
    let provider_name_style = if theme.no_color {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.fg_strong)
            .add_modifier(Modifier::BOLD)
    };

    let proxy_running = data.proxy.running;
    let current_app_routed = data
        .proxy
        .routes_current_app_through_proxy(&app.app_type)
        .unwrap_or(false);
    let uptime_text = if proxy_running {
        format_uptime_compact(data.proxy.uptime_seconds)
    } else {
        texts::tui_proxy_dashboard_uptime_stopped().to_string()
    };
    let proxy_last_error_text = data
        .proxy
        .last_error
        .clone()
        .unwrap_or_else(|| texts::none().to_string());
    let auto_failover_queue_len = data
        .providers
        .rows
        .iter()
        .filter(|row| row.provider.in_failover_queue)
        .count();
    let mut connection_lines = vec![
        kv_line(
            theme,
            texts::provider_label(),
            label_width,
            vec![Span::styled(current_provider.clone(), provider_name_style)],
        ),
        kv_line(
            theme,
            texts::tui_label_api_url(),
            label_width,
            vec![Span::styled(api_url, value_style)],
        ),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(pane_border_style(app, Focus::Content, theme))
        .title(format!(" {} ", icons::strip_icon(texts::welcome_title())));
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);
    let content = inset_left(inner, CONTENT_INSET_LEFT);
    // The ASCII logo hero is gone: without the proxy dashboard the whole elastic
    // region below the env-check card stays empty.
    let bottom_hero_height = if current_app_routed { 10 } else { 0 };
    // The card does not wrap. Ratatui word-wraps, so a wrap estimate built from
    // character counts under-counts on narrow terminals and clips the last line
    // (the WebDAV one) out of the card. Clipping each line to the card's content
    // width instead makes the row count exact: one line, one row.
    let card_text_width = content.width.saturating_sub(2);
    for line in &mut connection_lines {
        let spans = std::mem::take(&mut line.spans);
        line.spans = truncate_spans_to_width(spans, card_text_width);
    }
    // usize until the final clamp: an absurdly long provider name or URL must
    // not wrap the arithmetic into a plausible-looking height.
    let connection_card_height =
        u16::try_from(connection_lines.len().saturating_add(2).max(4)).unwrap_or(u16::MAX);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(bottom_hero_height)])
        .split(content);

    let top_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(connection_card_height),
            Constraint::Length(ENV_CHECK_CARD_HEIGHT),
        ])
        .split(chunks[0]);

    let card_border = Style::default().fg(theme.dim);
    render_connection_card(frame, top_chunks[0], theme, &connection_lines, card_border);
    render_local_env_check_card(frame, app, top_chunks[1], theme, card_border);

    if current_app_routed {
        render_proxy_activity_dashboard(
            frame,
            chunks[1],
            theme,
            &app.proxy_input_activity_samples,
            &app.proxy_output_activity_samples,
            &uptime_text,
            &proxy_last_error_text,
            data.proxy.last_error.is_some(),
            &format!("{}:{}", data.proxy.listen_address, data.proxy.listen_port),
            data.proxy.auto_failover_enabled,
            auto_failover_queue_len,
            data.proxy.estimated_input_tokens_total,
            data.proxy.estimated_output_tokens_total,
        );
    }
}

/// The env-check card lays the supported CLIs out as a two-column grid; with the
/// four supported harnesses that is two rows of two lines plus the card borders.
const ENV_CHECK_CARD_HEIGHT: u16 = 6;

#[expect(
    clippy::too_many_arguments,
    reason = "dashboard renderer receives precomputed proxy display metrics"
)]
fn render_proxy_activity_dashboard(
    frame: &mut Frame<'_>,
    area: Rect,
    theme: &super::theme::Theme,
    input_activity_samples: &[u64],
    output_activity_samples: &[u64],
    uptime_text: &str,
    proxy_last_error_text: &str,
    has_proxy_error: bool,
    listen_text: &str,
    auto_failover_enabled: bool,
    auto_failover_queue_len: usize,
    input_tokens_total: u64,
    output_tokens_total: u64,
) -> Rect {
    let has_token_traffic = input_tokens_total > 0 || output_tokens_total > 0;
    let title_output_style = if has_token_traffic {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.surface)
    };
    let title_input_style = if has_token_traffic {
        Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.surface)
    };
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(vec![
            Span::raw(format!(" {}   ", texts::tui_home_section_proxy())),
            Span::styled(
                format!("▲ {}", format_estimated_token_compact(output_tokens_total)),
                title_output_style,
            ),
            Span::styled(" / ", Style::default().fg(theme.comment)),
            Span::styled(
                format!("▼ {}", format_estimated_token_compact(input_tokens_total)),
                title_input_style,
            ),
            Span::raw(" "),
        ]));
    frame.render_widget(outer.clone(), area);

    let inner = outer.inner(area);
    let label_style = Style::default()
        .fg(theme.comment)
        .add_modifier(Modifier::BOLD);
    let mut meta_spans = Vec::new();
    let mut meta_plain = String::new();
    let mut push_segment = |label: &'static str, value: &str, style: Style| {
        if !meta_spans.is_empty() {
            meta_spans.push(Span::raw("  "));
            meta_plain.push_str("  ");
        }
        meta_spans.push(Span::styled(format!("{label}: "), label_style));
        meta_spans.push(Span::styled(value.to_string(), style));
        meta_plain.push_str(label);
        meta_plain.push_str(": ");
        meta_plain.push_str(value);
    };

    push_segment(
        texts::tui_label_listen(),
        listen_text,
        Style::default().fg(theme.cyan),
    );
    push_segment(
        texts::tui_label_uptime(),
        uptime_text,
        Style::default().fg(theme.cyan),
    );
    if auto_failover_enabled {
        let auto_failover_value = if auto_failover_queue_len > 0 {
            format!(
                "{} · {} {}",
                crate::t!("enabled", "开启"),
                crate::t!("Queue", "队列"),
                auto_failover_queue_len
            )
        } else {
            crate::t!("enabled", "开启").to_string()
        };
        push_segment(
            crate::t!("Automatic failover", "自动故障转移"),
            auto_failover_value.as_str(),
            Style::default().fg(theme.ok),
        );
    }
    if has_proxy_error {
        push_segment(
            texts::tui_label_last_proxy_error(),
            proxy_last_error_text,
            Style::default().fg(theme.warn),
        );
    }

    let max_text_height = inner.height.saturating_sub(2).clamp(1, 4);
    let text_height = wrapped_display_line_count(&meta_plain, inner.width).min(max_text_height);
    let graph_height = inner.height.saturating_sub(text_height).max(2);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(text_height),
            Constraint::Length(graph_height),
            Constraint::Min(0),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(meta_spans)).wrap(Wrap { trim: false }),
        sections[0],
    );

    let upper_height = (graph_height / 2).max(1);
    let lower_height = graph_height.saturating_sub(upper_height).max(1);
    let wave_width = sections[1].width.saturating_sub(1);
    let mut graph_lines = Vec::new();
    let upper_style = Style::default().fg(theme.accent);
    let lower_style = if theme.no_color {
        Style::default()
    } else {
        Style::default().fg(theme.cyan)
    };

    graph_lines.extend(
        proxy_wave_lines(
            wave_width,
            upper_height,
            true,
            output_activity_samples,
            &DOTS,
            false,
        )
        .into_iter()
        .map(|row| Line::from(vec![Span::raw(" "), Span::styled(row, upper_style)])),
    );
    graph_lines.extend(
        proxy_wave_lines(
            wave_width,
            lower_height,
            true,
            input_activity_samples,
            &REV_DOTS,
            true,
        )
        .into_iter()
        .map(|row| Line::from(vec![Span::raw(" "), Span::styled(row, lower_style)])),
    );

    frame.render_widget(
        Paragraph::new(graph_lines).wrap(Wrap { trim: false }),
        sections[1],
    );

    inner
}

fn wrapped_display_line_count(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }

    // Clamped in `usize`: the cast is the last step, never the first.
    UnicodeWidthStr::width(text)
        .max(1)
        .div_ceil(width as usize)
        .min(u16::MAX as usize) as u16
}

/// The connection card draws exactly the lines it is given, one row each — the
/// caller has already clipped them to the card's content width, and the card's
/// height was derived from that same count.
fn render_connection_card(
    frame: &mut Frame<'_>,
    area: Rect,
    _theme: &super::theme::Theme,
    connection_lines: &[Line<'_>],
    card_border: Style,
) {
    frame.render_widget(
        Paragraph::new(connection_lines.to_vec()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(card_border)
                .title(format!(" {} ", texts::tui_home_section_connection())),
        ),
        area,
    );
}

fn render_local_env_check_card(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    theme: &super::theme::Theme,
    card_border: Style,
) {
    use crate::services::local_env_check::LocalTool;

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(card_border)
        .title(format!(" {} ", texts::tui_home_section_local_env_check()));
    frame.render_widget(outer.clone(), area);
    let inner = outer.inner(area);

    let tools = LocalTool::all();
    // Two columns, two lines per cell: one row of the grid holds two tools.
    let grid_rows = tools.len().div_ceil(2);
    let row_constraints = vec![Constraint::Length(2); grid_rows];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    let cell_areas = rows
        .iter()
        .flat_map(|row| {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*row)
                .to_vec()
        })
        .collect::<Vec<_>>();

    for (tool, cell_area) in tools.iter().zip(cell_areas) {
        render_local_env_tool_cell(frame, app, theme, *tool, tool.display_name(), cell_area);
    }
}

fn render_local_env_tool_cell(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &super::theme::Theme,
    tool: crate::services::local_env_check::LocalTool,
    display_name: &str,
    cell_area: Rect,
) {
    use crate::services::local_env_check::ToolCheckStatus;

    let pending = app.is_local_env_pending(tool);
    let status = app
        .local_env_results
        .iter()
        .find(|result| result.tool == tool)
        .map(|result| &result.status);

    let (icon, icon_style) = if pending {
        (
            spinner_frame(app.tick),
            if theme.no_color {
                Style::default()
            } else {
                Style::default().fg(theme.cyan)
            },
        )
    } else {
        match status {
            Some(ToolCheckStatus::Ok { .. }) => (
                "✓",
                if theme.no_color {
                    Style::default()
                } else {
                    Style::default().fg(theme.ok)
                },
            ),
            Some(ToolCheckStatus::NotInstalledOrNotExecutable) => (
                "!",
                if theme.no_color {
                    Style::default()
                } else {
                    Style::default().fg(theme.warn)
                },
            ),
            Some(ToolCheckStatus::VersionUnavailable { .. }) | None => {
                ("•", Style::default().fg(theme.surface))
            }
        }
    };

    let name_style = if theme.no_color {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.fg_strong)
            .add_modifier(Modifier::BOLD)
    };

    let detail_style = if theme.no_color {
        Style::default()
    } else {
        Style::default().fg(theme.surface)
    };

    let value_style = Style::default().fg(theme.cyan);
    let (detail_text, detail_line_style) = if pending {
        (texts::tui_local_env_checking().to_string(), detail_style)
    } else {
        match status {
            Some(ToolCheckStatus::Ok { version }) => (version.clone(), value_style),
            Some(ToolCheckStatus::NotInstalledOrNotExecutable) => (
                texts::tui_local_env_not_installed().to_string(),
                detail_style,
            ),
            Some(ToolCheckStatus::VersionUnavailable { .. }) => (
                texts::tui_local_env_version_unavailable().to_string(),
                detail_style,
            ),
            None => (
                texts::tui_local_env_check_unavailable().to_string(),
                detail_style,
            ),
        }
    };

    let detail_width = cell_area.width.saturating_sub(1);
    let detail_text = truncate_to_display_width(&detail_text, detail_width);

    let lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(">_ ", Style::default().fg(theme.surface)),
            Span::styled(display_name.to_string(), name_style),
            Span::raw(" "),
            Span::styled(icon.to_string(), icon_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled(detail_text, detail_line_style),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), cell_area);
}

#[cfg(test)]
pub(super) fn proxy_activity_wave(width: u16, current_app_routed: bool, samples: &[u64]) -> String {
    proxy_wave_lines(width, 1, current_app_routed, samples, &DOTS, false)
        .into_iter()
        .next()
        .unwrap_or_default()
}
