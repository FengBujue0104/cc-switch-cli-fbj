use std::collections::HashMap;

use crate::app_config::{AppType, McpApps, McpServer, MultiAppConfig};
use crate::error::AppError;
use crate::mcp;
use crate::store::AppState;

/// MCP 相关业务逻辑（v3.7.0 统一结构）
pub struct McpService;

impl McpService {
    /// 本构建里真正能承接 MCP 投影的 harness。
    ///
    /// 不能直接等于 `AppType::all()`：Pi 明确不支持 MCP 管理
    /// （`cli/commands/mcp.rs` 会以 "Pi does not support MCP management" 拒绝），
    /// `McpApps` 里也没有 `pi` 位，`is_enabled_for(Pi)` 恒为 false。把它算进来的
    /// 后果只有一个：`sync_all_enabled` 白白多跑一轮针对 Pi 的空投影。
    ///
    /// Gemini/OpenCode 已从本构建移除，它们的 live 投影路径（
    /// `sync_single_server_to_gemini` / `..._to_opencode`）虽然还编译着，但这里
    /// 不能再出现它们，否则 `sync_all_enabled` 又会往已删 harness 的 live 目录
    /// 里写文件（`project_servers_to_app` 只会读 `should_sync_live` 启发式，
    /// 没有其它门槛）。
    pub fn supported_mcp_apps() -> impl Iterator<Item = AppType> {
        AppType::all()
            .filter(|app| matches!(app, AppType::Claude | AppType::Codex | AppType::Hermes))
    }

    /// 获取所有 MCP 服务器（统一结构）
    pub fn get_all_servers(state: &AppState) -> Result<HashMap<String, McpServer>, AppError> {
        let cfg = state.config.read()?;

        // 如果是新结构，直接返回
        if let Some(servers) = &cfg.mcp.servers {
            return Ok(servers.clone());
        }

        // 理论上不应该走到这里，因为 load 时会自动迁移
        Err(AppError::localized(
            "mcp.old_structure",
            "检测到旧版 MCP 结构，请重启应用完成迁移",
            "Old MCP structure detected, please restart app to complete migration",
        ))
    }

    /// 添加或更新 MCP 服务器
    pub fn upsert_server(state: &AppState, server: McpServer) -> Result<(), AppError> {
        let (server_id, apps_to_remove) = {
            let mut cfg = state.config.write()?;

            let servers = cfg.mcp.servers.get_or_insert_with(HashMap::new);
            let server_id = server.id.clone();

            let apps_to_remove = servers
                .get(&server_id)
                .map(|existing| {
                    existing
                        .apps
                        .enabled_apps()
                        .into_iter()
                        .filter(|app| !server.apps.is_enabled_for(app))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            // 插入或更新
            servers.insert(server_id.clone(), server.clone());

            (server_id, apps_to_remove)
        };

        state.save()?;

        // 如果是更新：对“由启用变为禁用”的应用，清理对应 live 配置
        for app in apps_to_remove {
            Self::remove_server_from_app(state, &server_id, &app)?;
        }

        // 同步到各个启用的应用
        Self::sync_server_to_apps(state, &server)?;

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn delete_server(state: &AppState, id: &str) -> Result<bool, AppError> {
        let server = {
            let mut cfg = state.config.write()?;

            if let Some(servers) = &mut cfg.mcp.servers {
                servers.remove(id)
            } else {
                None
            }
        };

        if let Some(server) = server {
            state.save()?;

            // 从所有应用的 live 配置中移除
            Self::remove_server_from_all_apps(state, id, &server)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 切换指定应用的启用状态
    pub fn toggle_app(
        state: &AppState,
        server_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        let server = {
            let mut cfg = state.config.write()?;

            if let Some(servers) = &mut cfg.mcp.servers {
                if let Some(server) = servers.get_mut(server_id) {
                    server.apps.set_enabled_for(&app, enabled);
                    Some(server.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(server) = server {
            state.save()?;

            // 同步到对应应用
            if enabled {
                Self::sync_server_to_app(state, &server, &app)?;
            } else {
                Self::remove_server_from_app(state, server_id, &app)?;
            }
        }

        Ok(())
    }

    /// Replace the full supported-app matrix for one MCP server.
    pub fn set_apps(state: &AppState, server_id: &str, apps: McpApps) -> Result<bool, AppError> {
        let (server, changes) = {
            let mut cfg = state.config.write()?;

            let Some(servers) = &mut cfg.mcp.servers else {
                return Ok(false);
            };
            let Some(server) = servers.get_mut(server_id) else {
                return Ok(false);
            };

            let before = server.apps.clone();
            server.apps = apps;
            let server = server.clone();
            let changes = Self::supported_mcp_apps()
                .filter_map(|app| {
                    let before_enabled = before.is_enabled_for(&app);
                    let after_enabled = server.apps.is_enabled_for(&app);
                    (before_enabled != after_enabled).then_some((app, after_enabled))
                })
                .collect::<Vec<_>>();

            (server, changes)
        };

        state.save()?;

        for (app, enabled) in changes {
            if enabled {
                Self::sync_server_to_app(state, &server, &app)?;
            } else {
                Self::remove_server_from_app(state, server_id, &app)?;
            }
        }

        Ok(true)
    }

    /// 将 MCP 服务器同步到所有启用的应用
    fn sync_server_to_apps(state: &AppState, server: &McpServer) -> Result<(), AppError> {
        let cfg = state.config.read()?;

        for app in server.apps.enabled_apps() {
            Self::sync_server_to_app_internal(&cfg, server, &app)?;
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到指定应用
    fn sync_server_to_app(
        state: &AppState,
        server: &McpServer,
        app: &AppType,
    ) -> Result<(), AppError> {
        let cfg = state.config.read()?;
        Self::sync_server_to_app_internal(&cfg, server, app)
    }

    fn sync_server_to_app_internal(
        cfg: &MultiAppConfig,
        server: &McpServer,
        app: &AppType,
    ) -> Result<(), AppError> {
        match app {
            AppType::Claude => {
                mcp::sync_single_server_to_claude(cfg, &server.id, &server.server)?;
            }
            AppType::Codex => {
                mcp::sync_single_server_to_codex(cfg, &server.id, &server.server)?;
            }
            AppType::Gemini => {
                mcp::sync_single_server_to_gemini(cfg, &server.id, &server.server)?;
            }
            AppType::OpenCode => {
                mcp::sync_single_server_to_opencode(cfg, &server.id, &server.server)?;
            }
            AppType::Hermes => {
                mcp::sync_single_server_to_hermes(cfg, &server.id, &server.server)?;
            }
            AppType::OpenClaw => {}
            AppType::Pi => {}
        }
        Ok(())
    }

    /// 从所有曾启用过该服务器的应用中移除
    fn remove_server_from_all_apps(
        state: &AppState,
        id: &str,
        server: &McpServer,
    ) -> Result<(), AppError> {
        // 从所有曾启用的应用中移除
        for app in server.apps.enabled_apps() {
            Self::remove_server_from_app(state, id, &app)?;
        }
        Ok(())
    }

    fn remove_server_from_app(_state: &AppState, id: &str, app: &AppType) -> Result<(), AppError> {
        match app {
            AppType::Claude => mcp::remove_server_from_claude(id)?,
            AppType::Codex => mcp::remove_server_from_codex(id)?,
            AppType::Gemini => mcp::remove_server_from_gemini(id)?,
            AppType::OpenCode => mcp::remove_server_from_opencode(id)?,
            AppType::Hermes => mcp::remove_server_from_hermes(id)?,
            AppType::OpenClaw => {}
            AppType::Pi => {}
        }
        Ok(())
    }

    /// 手动同步所有启用的 MCP 服务器到对应的应用。
    ///
    /// Best-effort：单个应用投影失败不阻断其余应用。各应用的 live 文件互相独立，
    /// 一处损坏没有理由让其它应用的 MCP 状态保持陈旧。全部执行完后聚合错误，
    /// 保留调用方对部分失败的可见性。
    pub fn sync_all_enabled(state: &AppState) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        let mut failures = Vec::new();
        for app in Self::supported_mcp_apps() {
            if let Err(err) = Self::project_servers_to_app(state, &servers, &app) {
                log::warn!("同步 MCP 到 {app:?} 失败: {err}");
                failures.push(format!("{}: {err}", app.as_str()));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Message(format!(
                "部分应用 MCP 同步失败: {}",
                failures.join("; ")
            )))
        }
    }

    /// 只把启用状态投影到单个应用。某个应用的 live 被整体重写后用它做
    /// 定向重投影，避免把无关应用的失败面牵连进目标应用的关键路径。
    pub fn sync_enabled_for_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;
        Self::project_servers_to_app(state, &servers, app)
    }

    fn project_servers_to_app(
        state: &AppState,
        servers: &HashMap<String, McpServer>,
        app: &AppType,
    ) -> Result<(), AppError> {
        for server in servers.values() {
            if server.apps.is_enabled_for(app) {
                Self::sync_server_to_app(state, server, app)?;
            } else {
                Self::remove_server_from_app(state, &server.id, app)?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // 兼容层：支持旧的 v3.6.x 命令（已废弃，将在 v4.0 移除）
    // ========================================================================

    /// [已废弃] 获取指定应用的 MCP 服务器（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use get_all_servers instead")]
    pub fn get_servers(
        state: &AppState,
        app: AppType,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let all_servers = Self::get_all_servers(state)?;
        let mut result = HashMap::new();

        for (id, server) in all_servers {
            if server.apps.is_enabled_for(&app) {
                result.insert(id, server.server);
            }
        }

        Ok(result)
    }

    /// [已废弃] 设置 MCP 服务器在指定应用的启用状态（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use toggle_app instead")]
    pub fn set_enabled(
        state: &AppState,
        app: AppType,
        id: &str,
        enabled: bool,
    ) -> Result<bool, AppError> {
        Self::toggle_app(state, id, app, enabled)?;
        Ok(true)
    }

    /// [已废弃] 同步启用的 MCP 到指定应用（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use sync_all_enabled instead")]
    pub fn sync_enabled(state: &AppState, app: AppType) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        for server in servers.values() {
            if server.apps.is_enabled_for(&app) {
                Self::sync_server_to_app(state, server, &app)?;
            }
        }

        Ok(())
    }

    /// 从 Claude 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_claude(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let count = mcp::import_from_claude(&mut cfg)?;
        drop(cfg);
        state.save()?;
        Ok(count)
    }

    /// 从 Codex 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_codex(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let count = mcp::import_from_codex(&mut cfg)?;
        drop(cfg);
        state.save()?;
        Ok(count)
    }

    /// 从 Gemini 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_gemini(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let count = mcp::import_from_gemini(&mut cfg)?;
        drop(cfg);
        state.save()?;
        Ok(count)
    }

    /// 从 OpenCode 导入 MCP
    pub fn import_from_opencode(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let count = mcp::import_from_opencode(&mut cfg)?;
        drop(cfg);
        state.save()?;
        Ok(count)
    }

    /// 从 Hermes 导入 MCP
    pub fn import_from_hermes(state: &AppState) -> Result<usize, AppError> {
        let mut cfg = state.config.write()?;
        let count = mcp::import_from_hermes(&mut cfg)?;
        drop(cfg);
        state.save()?;
        Ok(count)
    }

    /// 只从本构建保留的 harness 导入。
    ///
    /// 按 `supported_mcp_apps()` 派生，不手抄 app 列表：已删的 Gemini/OpenCode
    /// 的 live 目录里可能还留着配置，但从那里"导入"本身就是已删 harness 复活的
    /// 一条路径——导入会把它们的服务器并回统一配置，用户随后在 UI 里看到的来源
    /// 就成了谜。（导入本身是读，不会往那些目录写；真正会写的是紧随其后的同步，
    /// 而那一步由 `enabled_apps()` / `supported_mcp_apps()` 兜住。）
    pub fn import_from_supported_apps(state: &AppState) -> Result<usize, AppError> {
        let mut total = 0;
        for app in Self::supported_mcp_apps() {
            let count = match app {
                AppType::Claude => Self::import_from_claude(state)?,
                AppType::Codex => Self::import_from_codex(state)?,
                AppType::Hermes => Self::import_from_hermes(state)?,
                other => unreachable!(
                    "supported_mcp_apps() must stay within Claude/Codex/Hermes, got {other:?}"
                ),
            };
            total += count;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `sync_all_enabled` 按这个集合 `mkdir -p` 并写每个 app 的 live MCP 配置，
    /// 所以这里必须是 `AppType::all()` 的子集（Pi 不支持 MCP），且绝不能混入
    /// 已删的 Gemini/OpenCode。
    #[test]
    fn supported_mcp_apps_stays_within_app_type_all() {
        let apps = McpService::supported_mcp_apps()
            .map(|app| app.as_str())
            .collect::<Vec<_>>();
        assert_eq!(apps, ["claude", "codex", "hermes"]);
        assert!(
            apps.iter()
                .all(|app| AppType::all().any(|kept| kept.as_str() == *app)),
            "MCP apps must be a subset of AppType::all(): {apps:?}"
        );
    }

    /// `mcp import` 只能从 `supported_mcp_apps()` 里的 harness 导入。
    ///
    /// 上游版本会把 `~/.gemini`、`~/.config/opencode` 里的 mcpServers 一起吸进
    /// 统一配置。那两个 harness 已从本构建删除，导入它们是"删不干净"最隐蔽的
    /// 一种形态：导入本身只读那些目录，但随后 `sync_all_enabled` 会按统一配置
    /// 做投影，用户看到的就是来源不明的条目——能不能不往已删目录写文件，全指望
    /// 别的门槛，而不是这里。
    #[test]
    #[serial_test::serial(home_settings)]
    fn import_from_supported_apps_ignores_removed_harness_live_configs() {
        use serde_json::json;

        use crate::test_support::TestEnvGuard;

        let temp_home = tempfile::TempDir::new().expect("create temp home");
        let _env = TestEnvGuard::isolated(temp_home.path());

        let gemini_dir = temp_home.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        std::fs::write(
            gemini_dir.join("settings.json"),
            json!({"mcpServers": {"gemini_only": {"command": "echo"}}}).to_string(),
        )
        .expect("seed gemini mcp config");

        let opencode_dir = temp_home.path().join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).expect("create opencode dir");
        std::fs::write(
            opencode_dir.join("opencode.json"),
            json!({"mcp": {"opencode_only": {"type": "local", "command": ["echo"]}}}).to_string(),
        )
        .expect("seed opencode mcp config");

        let state = crate::store::AppState::try_new().expect("create app state");

        let imported =
            McpService::import_from_supported_apps(&state).expect("import should succeed");

        assert_eq!(
            imported, 0,
            "removed harnesses must not contribute imported servers"
        );
        let servers = McpService::get_all_servers(&state).expect("load unified servers");
        assert!(
            !servers.contains_key("gemini_only") && !servers.contains_key("opencode_only"),
            "removed harness MCP servers must not enter the unified config: {:?}",
            servers.keys().collect::<Vec<_>>()
        );
    }
}
