use super::*;

pub(super) fn skills_installed_filtered<'a>(
    app: &App,
    data: &'a UiData,
) -> Vec<&'a crate::services::skill::InstalledSkill> {
    let query = app.filter.query_lower();
    data.skills
        .installed
        .iter()
        .filter(|skill| match &query {
            None => true,
            Some(q) => {
                skill.name.to_lowercase().contains(q)
                    || skill.directory.to_lowercase().contains(q)
                    || skill.id.to_lowercase().contains(q)
            }
        })
        .collect()
}

pub(super) fn skill_display_name<'a>(name: &'a str, directory: &'a str) -> &'a str {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        directory
    } else {
        trimmed
    }
}

/// 概要行"已启用: ..."里的 harness 名。
///
/// 从 `supported_skill_apps()` 派生，不在这个函数里手写清单：手写的那份就是
/// 已删 harness 复活的位置（旧数据里的勾选可以留着，但不该被拼进这行字）。
pub(super) fn enabled_skill_apps_text(apps: &crate::app_config::SkillApps) -> String {
    let enabled = crate::services::SkillService::supported_skill_apps()
        .filter(|app| apps.is_enabled_for(app))
        .map(|app| app.display_name())
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        texts::none().to_string()
    } else {
        enabled.join(", ")
    }
}
