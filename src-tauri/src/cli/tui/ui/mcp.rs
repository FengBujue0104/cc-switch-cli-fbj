use super::*;

pub(super) fn mcp_rows_filtered<'a>(app: &App, data: &'a UiData) -> Vec<&'a McpRow> {
    let query = app.filter.query_lower();
    data.mcp
        .rows
        .iter()
        .filter(|row| match &query {
            None => true,
            Some(q) => {
                row.server.name.to_lowercase().contains(q) || row.id.to_lowercase().contains(q)
            }
        })
        .collect()
}

pub(super) fn render_mcp(
    frame: &mut Frame<'_>,
    app: &App,
    data: &UiData,
    area: Rect,
    theme: &super::theme::Theme,
) {
    let visible = mcp_rows_filtered(app, data);

    // 表头/单元格/摘要全部由同一份 harness 清单驱动。硬编码列名会让已删的
    // Gemini/OpenCode 从这张表里复活（用户最初反馈的"没删干净"正是这类）。
    let columns = crate::services::McpService::supported_mcp_apps().collect::<Vec<_>>();

    let mut header = vec![Cell::from(texts::header_name())];
    header.extend(columns.iter().map(|app| Cell::from(app.as_str())));
    let header =
        Row::new(header).style(Style::default().fg(theme.dim).add_modifier(Modifier::BOLD));

    let rows = visible.iter().map(|row| {
        let mut cells = vec![Cell::from(row.server.name.clone())];
        cells.extend(columns.iter().map(|app| {
            Cell::from(if row.server.apps.is_enabled_for(app) {
                texts::tui_marker_active()
            } else {
                texts::tui_marker_inactive()
            })
        }));
        Row::new(cells)
    });

    let keys = crate::cli::tui::keymap::mcp::key_bar_items(app, data);
    let summary = texts::tui_mcp_server_counts(
        &columns
            .iter()
            .map(|app| {
                let count = data
                    .mcp
                    .rows
                    .iter()
                    .filter(|row| row.server.apps.is_enabled_for(app))
                    .count();
                (app.display_name(), count)
            })
            .collect::<Vec<_>>(),
    );
    let body = render_page_frame(
        frame,
        area,
        theme,
        app,
        texts::menu_manage_mcp(),
        &keys,
        Some(summary),
    );

    // 第一列吃掉剩余宽度，每个 harness 一列固定宽度——列数来自 `columns`，
    // 加减 harness 时这里自动跟着变。
    let mut widths = vec![Constraint::Percentage(50)];
    widths.extend(std::iter::repeat_n(Constraint::Length(10), columns.len()));

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(selection_style(theme))
        .highlight_symbol(highlight_symbol(theme));

    if data.mcp.rows.is_empty() {
        render_empty_state(
            frame,
            body,
            theme,
            texts::tui_mcp_empty_title(),
            texts::tui_mcp_empty_subtitle(),
        );
        return;
    }

    let mut state = TableState::default();
    state.select(Some(app.mcp_idx));

    frame.render_stateful_widget(table, inset_left(body, CONTENT_INSET_LEFT), &mut state);
}
