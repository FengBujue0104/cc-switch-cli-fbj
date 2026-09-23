use crate::app_config::AppType;

/// Whether we should write/delete "live" config files for a given app.
///
/// Policy: **auto** (safe default)
/// - If the target app looks uninitialized (its config dir / key live file is missing),
///   skip live writes/deletes and do **not** create any directories/files.
/// - That auto-detect only applies to the harnesses this build ships. The removed
///   ones answer `false` unconditionally, however intact their directory looks.
///
/// 第二条就是"自动检测可用 harness"里最该关掉的那一半：Gemini/OpenCode/OpenClaw 的
/// live 目录检测对已删 harness 只有坏处。删掉客户端的用户机器上 `~/.gemini` 往往还在
/// （或者根本没删干净），于是"这里看起来初始化过"就把门打开了。各调用方本来已经各自
/// 收窄过（clap 的 `value(skip)`、`McpApps::enabled_apps()`、
/// `SkillService::supported_skill_apps()` 都从 `AppType::all()` 派生），但传进来的
/// `AppType` 完全可能是旧数据库行 `AppType::from_str` 解析出来的——那道防线只能靠
/// 每个调用点自觉，这里补一道不依赖调用方的闸。
pub(crate) fn should_sync_live(app_type: &AppType) -> bool {
    match app_type {
        // Claude is considered initialized if either:
        // - ~/.claude (settings dir) exists, or
        // - ~/.claude.json (MCP file) exists
        AppType::Claude => {
            crate::config::get_claude_config_dir().exists()
                || crate::config::get_claude_mcp_path().exists()
        }
        // Codex is considered initialized if ~/.codex (or override dir) exists.
        AppType::Codex => crate::codex_config::get_codex_config_dir().exists(),
        // Hermes is considered initialized if ~/.hermes (or override dir) exists.
        AppType::Hermes => crate::hermes_config::get_hermes_dir().exists(),
        // Pi live provider writes are owned by the revision-aware native service.
        AppType::Pi => false,
        // Gemini / OpenCode / OpenClaw are not part of this build, so their live
        // directories are never probed and never written to — see the note above.
        AppType::Gemini | AppType::OpenCode | AppType::OpenClaw => false,
    }
}

#[cfg(test)]
mod tests {
    use super::should_sync_live;
    use crate::app_config::AppType;
    use crate::test_support::TestEnvGuard;

    /// 已删 harness 的 live 目录就算摆在那儿，也不再是"看起来初始化过"。
    ///
    /// 这是"自动检测可用 harness"在本构建里唯一还生效的一半：Gemini/OpenCode/OpenClaw
    /// 的目录检测以前会把 live 写入门打开，删掉客户端、留下 `~/.gemini` 的用户机器上
    /// 只要碰一次旧的 MCP/供应商行，配置就被写回那个目录。这里把三者连同留下的目录
    /// 一起钉死。
    #[test]
    #[serial_test::serial(home_settings)]
    fn removed_harnesses_never_sync_live_even_when_their_directory_exists() {
        let temp_home = tempfile::TempDir::new().expect("create temp home");
        let _env = TestEnvGuard::isolated(temp_home.path());

        for dir in [
            temp_home.path().join(".gemini"),
            temp_home.path().join(".config").join("opencode"),
            temp_home.path().join(".openclaw"),
        ] {
            std::fs::create_dir_all(&dir).expect("create removed harness live dir");
        }

        for removed in [AppType::Gemini, AppType::OpenCode, AppType::OpenClaw] {
            assert!(
                !should_sync_live(&removed),
                "{removed:?} must never sync live, its directory exists on this host"
            );
        }
    }

    /// 留下的 harness 仍按目录存在性判断：这条自动检测本来就是给它们用的。
    #[test]
    #[serial_test::serial(home_settings)]
    fn kept_harnesses_still_auto_detect_from_their_live_directory() {
        let temp_home = tempfile::TempDir::new().expect("create temp home");
        let _env = TestEnvGuard::isolated(temp_home.path());

        std::fs::create_dir_all(temp_home.path().join(".claude")).expect("create ~/.claude");
        std::fs::create_dir_all(temp_home.path().join(".codex")).expect("create ~/.codex");

        assert!(should_sync_live(&AppType::Claude));
        assert!(should_sync_live(&AppType::Codex));
        assert!(
            !should_sync_live(&AppType::Hermes),
            "~/.hermes does not exist in this sandbox"
        );
        assert!(
            !should_sync_live(&AppType::Pi),
            "Pi live writes are owned by the revision-aware native service"
        );
    }
}
