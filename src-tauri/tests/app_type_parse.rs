use std::str::FromStr;

use cc_switch_lib::AppType;

#[test]
fn parse_known_apps_case_insensitive_and_trim() {
    assert!(matches!(AppType::from_str("claude"), Ok(AppType::Claude)));
    assert!(matches!(AppType::from_str("codex"), Ok(AppType::Codex)));
    assert!(matches!(AppType::from_str("hermes"), Ok(AppType::Hermes)));
    assert!(matches!(
        AppType::from_str("openclaw"),
        Ok(AppType::OpenClaw)
    ));
    assert!(matches!(
        AppType::from_str(" ClAuDe \n"),
        Ok(AppType::Claude)
    ));
    assert!(matches!(AppType::from_str("\tcoDeX\t"), Ok(AppType::Codex)));
    assert!(matches!(
        AppType::from_str(" HeRmEs\t"),
        Ok(AppType::Hermes)
    ));
    assert!(matches!(
        AppType::from_str("\nOpenClaw\t"),
        Ok(AppType::OpenClaw)
    ));
    assert!(matches!(AppType::from_str("Pi"), Ok(AppType::Pi)));
    assert!(matches!(AppType::from_str("gemini"), Ok(AppType::Gemini)));
    assert!(matches!(
        AppType::from_str("opencode"),
        Ok(AppType::OpenCode)
    ));
}

/// 已删 harness 必须还能从旧数据里解析出来（否则库里已有的 `--app` 字符串会让
/// 应用起不来），但不能再被 `all()` 列出来，也不能再走追加式 live 语义。
///
/// 这一条就是"自动检测把删掉的 harness 又显示出来"的回归测试：`all()` 是
/// 标签循环、MCP 同步、skill 投影、启动期导入的唯一入口。
#[test]
fn removed_harnesses_parse_but_are_not_listed() {
    for removed in [AppType::Gemini, AppType::OpenCode, AppType::OpenClaw] {
        let id = removed.as_str();
        assert!(
            !AppType::all().any(|app| app == removed),
            "{id} must not be part of AppType::all()"
        );
        assert!(
            !removed.is_additive_mode(),
            "{id} must not use additive live-config semantics"
        );

        // 旧数据里的字符串仍要能原样解析回来。
        assert_eq!(AppType::from_str(id).unwrap(), removed);
    }

    assert_eq!(
        AppType::all().map(|app| app.as_str()).collect::<Vec<_>>(),
        ["claude", "codex", "hermes", "pi"]
    );
}

#[test]
fn hermes_is_listed_and_uses_additive_mode() {
    assert!(AppType::all().any(|app| app == AppType::Hermes));
    assert!(AppType::Hermes.is_additive_mode());
}

#[test]
fn pi_is_listed_and_uses_additive_mode() {
    assert!(AppType::all().any(|app| app == AppType::Pi));
    assert!(AppType::Pi.is_additive_mode());
}

#[test]
fn parse_unknown_app_returns_localized_error_message() {
    let err = AppType::from_str("unknown").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("此构建支持") || msg.contains("Supported by this build"));
    assert!(msg.contains("unknown"));
}
