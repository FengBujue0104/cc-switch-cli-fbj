use super::*;

pub(super) fn render_skills_installed(
    frame: &mut Frame<'_>,
    app: &App,
    data: &UiData,
    area: Rect,
    theme: &super::theme::Theme,
) {
    let keys = crate::cli::tui::keymap::skills_installed::key_bar_items(app, data);
    let body = render_page_frame(
        frame,
        area,
        theme,
        app,
        texts::menu_manage_skills(),
        &keys,
        Some(installed_summary(app, data)),
    );

    let visible = skills_installed_filtered(app, data);

    // 表头/单元格/摘要全部由同一份 harness 清单驱动（`supported_skill_apps()`）。
    // 硬编码列名会让已删的 Gemini/OpenCode 从这张表里复活。
    let columns = crate::services::SkillService::supported_skill_apps().collect::<Vec<_>>();

    let mut header = vec![Cell::from(texts::header_name())];
    header.extend(columns.iter().map(|app| Cell::from(app.as_str())));
    let header =
        Row::new(header).style(Style::default().fg(theme.dim).add_modifier(Modifier::BOLD));

    let rows = visible.iter().map(|skill| {
        let display_name = skill_display_name(&skill.name, &skill.directory);
        let display_name = if app.skill_updates.contains_key(&skill.id) {
            format!("{display_name} {}", texts::tui_skills_update_marker())
        } else {
            display_name.to_string()
        };
        let mut cells = vec![Cell::from(display_name)];
        cells.extend(
            columns
                .iter()
                .map(|app| Cell::from(skill_marker(skill.apps.is_enabled_for(app)))),
        );
        Row::new(cells)
    });

    // 第一列吃掉剩余宽度，每个 harness 一列固定宽度。
    let mut widths = vec![Constraint::Percentage(50)];
    widths.extend(columns.iter().map(|app| {
        if app.as_str().len() <= 5 {
            Constraint::Length(5)
        } else {
            Constraint::Length(8)
        }
    }));

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(selection_style(theme))
        .highlight_symbol(highlight_symbol(theme));

    if data.skills.installed.is_empty() {
        render_empty_state(
            frame,
            body,
            theme,
            texts::tui_skills_empty_title(),
            texts::tui_skills_empty_subtitle(),
        );
        return;
    }

    let mut state = TableState::default();
    state.select(Some(app.skills_idx));
    frame.render_stateful_widget(table, inset_left(body, CONTENT_INSET_LEFT), &mut state);
}

fn installed_summary(app: &App, data: &UiData) -> String {
    // 摘要与表头共用 `supported_skill_apps()`，加减 harness 时不会脱节。
    let counts = crate::services::SkillService::supported_skill_apps()
        .map(|app_type| {
            let count = data
                .skills
                .installed
                .iter()
                .filter(|s| s.apps.is_enabled_for(&app_type))
                .count();
            (app_type.display_name(), count)
        })
        .collect::<Vec<_>>();

    let counts = texts::tui_skills_installed_counts(&counts);
    if app.skill_updates.is_empty() {
        counts
    } else {
        format!(
            "{counts} · {}",
            texts::tui_skills_updates_available(app.skill_updates.len())
        )
    }
}

fn skill_marker(enabled: bool) -> &'static str {
    if enabled {
        texts::tui_marker_active()
    } else {
        texts::tui_marker_inactive()
    }
}
