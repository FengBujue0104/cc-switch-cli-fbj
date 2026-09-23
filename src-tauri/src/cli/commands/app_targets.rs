use std::str::FromStr;

use crate::app_config::AppType;
use crate::error::AppError;

/// `--app` 候选清单，也是所有报错文案里 "Supported apps: ..." 的来源。
///
/// 必须从 `AppType::all()` 派生：这份字符串既是帮助/报错里唯一对外的 harness
/// 清单，又是用户判断"这个程序支持什么"的依据。写死一份就是已删 harness
/// 最常见的复活位置（gemini/opencode 曾经就一直挂在这里）。
pub(crate) fn supported_app_target_labels() -> String {
    AppType::all()
        .map(|app| app.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// MCP 不支持 Pi（`cli/commands/mcp.rs` 会明确拒绝），所以 MCP 的候选清单
/// 比全局清单少一个。除此之外与全局清单一致。
fn supported_app_target_labels_for(feature: &str) -> String {
    let apps = if feature.eq_ignore_ascii_case("MCP") {
        AppType::all()
            .filter(|app| *app != AppType::Pi)
            .map(|app| app.as_str())
            .collect::<Vec<_>>()
    } else {
        AppType::all().map(|app| app.as_str()).collect::<Vec<_>>()
    };
    apps.join(", ")
}

pub(crate) fn app_targets_or_default(
    raw_targets: &[String],
    fallback: AppType,
    feature: &str,
) -> Result<Vec<AppType>, AppError> {
    if raw_targets.is_empty() {
        return parse_app_targets(&[fallback.as_str().to_string()], feature);
    }

    parse_app_targets(raw_targets, feature)
}

pub(crate) fn parse_app_targets(
    raw_targets: &[String],
    feature: &str,
) -> Result<Vec<AppType>, AppError> {
    let mut targets = Vec::new();

    for raw in raw_targets {
        for value in raw.split(',') {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            let app = parse_app_target(value, feature)?;
            if !targets.contains(&app) {
                targets.push(app);
            }
        }
    }

    if targets.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Please provide at least one app. Supported apps: {}",
            supported_app_target_labels_for(feature)
        )));
    }

    Ok(targets)
}

fn parse_app_target(value: &str, feature: &str) -> Result<AppType, AppError> {
    let normalized = value.trim().to_lowercase().replace('-', "");
    let app = AppType::from_str(&normalized).map_err(|_| {
        AppError::InvalidInput(format!(
            "Unsupported app id: '{value}'. Supported apps: {}",
            supported_app_target_labels_for(feature)
        ))
    })?;

    // `AppType::from_str` 故意仍然认识已删的 id（旧数据要能读回来），所以这里
    // 必须自己拦住它们：否则 `--app gemini` 会解析成功，然后在各 feature 里
    // 变成一次静默的空操作——用户看到 "✓ Set skill 'x' apps to gemini" 却什么
    // 都没发生，正是"没删干净"最难看的一种形态。OpenClaw 也走这条（它以前有
    // 一条更具体的分支，但 "does not support openclaw yet" 的 "yet" 现在是个
    // 假承诺：它不是还没做，是不会再做）。
    if !AppType::all().any(|kept| kept == app) {
        return Err(AppError::InvalidInput(format!(
            "Unsupported app id: '{value}'. Supported apps: {}",
            supported_app_target_labels_for(feature)
        )));
    }

    if matches!(app, AppType::Pi) && feature.eq_ignore_ascii_case("MCP") {
        return Err(AppError::InvalidInput(format!(
            "{feature} does not support pi. Supported apps: {}",
            supported_app_target_labels_for(feature)
        )));
    }

    Ok(app)
}

pub(crate) fn app_target_names(apps: &[AppType]) -> String {
    apps.iter()
        .map(AppType::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_app_targets_accepts_backend_ids_and_aliases() {
        let apps = parse_app_targets(&["claude,codex".to_string(), "her-mes".to_string()], "MCP")
            .expect("apps should parse");

        assert_eq!(apps, vec![AppType::Claude, AppType::Codex, AppType::Hermes]);
    }

    #[test]
    fn parse_app_targets_deduplicates_in_order() {
        let apps = parse_app_targets(&["codex".to_string(), "claude,codex".to_string()], "Skills")
            .expect("apps should parse");

        assert_eq!(apps, vec![AppType::Codex, AppType::Claude]);
    }

    /// OpenClaw 现在和其它已删 harness 一样走统一的拒绝路径（它以前有一条
    /// "does not support openclaw yet" 的分支，见 `parse_app_target` 的注释）。
    #[test]
    fn parse_app_targets_rejects_openclaw() {
        let err = parse_app_targets(&["openclaw".to_string()], "MCP")
            .expect_err("openclaw should be rejected");

        assert!(
            err.to_string().contains("Unsupported app id"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_app_targets_accepts_pi_for_skills_but_not_mcp() {
        assert_eq!(
            parse_app_targets(&["pi".to_string()], "Skills").expect("Pi skills target"),
            vec![AppType::Pi]
        );
        let error = parse_app_targets(&["pi".to_string()], "MCP")
            .expect_err("Pi must not be an MCP target");
        assert!(error.to_string().contains("does not support pi"));
    }

    /// `--app` 的候选清单必须等于 `AppType::all()`：这是唯一一处会同时出现在
    /// 帮助文案和报错文案里的 harness 清单，漏改就等于还在对外宣传已删的 harness。
    #[test]
    fn supported_app_target_labels_match_app_type_all() {
        let labels = supported_app_target_labels();
        assert_eq!(labels, "claude, codex, hermes, pi");
        assert_eq!(
            labels,
            app_target_names(&AppType::all().collect::<Vec<_>>())
        );
        assert!(
            !labels.contains("gemini")
                && !labels.contains("opencode")
                && !labels.contains("openclaw"),
            "removed harnesses must not be advertised as --app targets: {labels}"
        );
    }

    /// MCP 清单比全局清单少 Pi。其余 harness 一律不许多。
    #[test]
    fn supported_app_target_labels_for_mcp_is_all_without_pi() {
        assert_eq!(
            supported_app_target_labels_for("MCP"),
            "claude, codex, hermes"
        );
        assert_eq!(
            supported_app_target_labels_for("Skills"),
            supported_app_target_labels()
        );
    }

    /// 已删的 id 能通过 `from_str`（旧数据兼容），但不能再通过 `--app` 的解析。
    #[test]
    fn parse_app_target_rejects_removed_harnesses() {
        for id in ["gemini", "opencode", "openclaw"] {
            assert!(
                AppType::from_str(id).is_ok(),
                "{id} must still parse for legacy data"
            );
            for feature in ["MCP", "Skills"] {
                let err = parse_app_targets(&[id.to_string()], feature)
                    .expect_err(&format!("{id} must not be a {feature} target"));
                let rendered = err.to_string();
                assert!(
                    rendered.contains("Unsupported app id"),
                    "unexpected error for {id}/{feature}: {rendered}"
                );
                // 报错里会回显用户输入的那个 id，所以只检查 "Supported apps:"
                // 之后的部分——那才是本构建对外的 harness 清单。
                let advertised = rendered
                    .split_once("Supported apps: ")
                    .map(|(_, rest)| rest.to_string())
                    .unwrap_or_default();
                assert_eq!(advertised, supported_app_target_labels_for(feature));
                assert!(
                    !advertised.contains("gemini")
                        && !advertised.contains("opencode")
                        && !advertised.contains("openclaw"),
                    "rejection message must not advertise removed harnesses: {rendered}"
                );
            }
        }
    }
}
