//! Deep link import functionality for CC Switch (CLI edition).
//!
//! Implements the `ccswitch://v1/import?...` protocol for importing resources.
//! Supports importing providers, MCP servers, prompts, and skill repositories.

mod mcp;
mod parser;
mod prompt;
mod provider;
mod skill;
mod utils;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::services::McpService;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

pub use mcp::{import_mcp_from_deeplink, McpImportResult};
pub use parser::parse_deeplink_url;
pub use prompt::import_prompt_from_deeplink;
pub use provider::import_provider_from_deeplink;
pub use skill::import_skill_from_deeplink;

/// 深链协议里合法的 harness id 清单（逗号分隔），所有对外文案的单一来源。
///
/// 不能手抄：深链 URL 是第三方嵌到网页里分发的，文案里列出的 id 就是用户会去
/// 试的东西。上游把 gemini/opencode/openclaw 写死在 `parser.rs` 的三处
/// `matches!` 里，链接照样能解析、照样能走完导入——已删 harness 的复活位置
/// 不止 UI 一处。
pub(crate) fn supported_deeplink_app_ids() -> String {
    supported_app_ids(AppType::all())
}

/// 能通过深链导入 **provider** 的 harness 清单。
///
/// 比全局清单少一个 Pi：Pi 的供应商必须从 Pi 供应商页添加
/// （`provider::build_provider_from_request` 对 Pi 有专门的报错），上游的 parser
/// 也一直不放 `app=pi` 的 provider 链接过去。定义仍然是派生式的——
/// `AppType::all()` 一收窄，这个集合跟着收窄，不会留下手抄的已删 id。
pub(crate) fn supported_deeplink_provider_app_ids() -> String {
    supported_app_ids(deeplink_provider_apps())
}

fn deeplink_provider_apps() -> impl Iterator<Item = AppType> {
    AppType::all().filter(|app| *app != AppType::Pi)
}

/// MCP 的 `apps=` 比全局少一个 Pi：`McpApps` 里没有 pi 位，
/// `is_enabled_for(Pi)` 恒为 false。清单取自 `McpService`，不另抄一份。
pub(crate) fn supported_deeplink_mcp_app_ids() -> String {
    supported_app_ids(McpService::supported_mcp_apps())
}

fn supported_app_ids(apps: impl Iterator<Item = AppType>) -> String {
    apps.map(|app| app.as_str()).collect::<Vec<_>>().join(", ")
}

/// `app` 是否属于本构建保留的 harness 集合。
///
/// 大小写敏感，和 `AppType::from_str` 一致：URL 协议里用的就是小写 id，两层的
/// 宽松度不一致会让 parser 放行、导入再拒，只会更好看不会更好用。
pub(crate) fn is_supported_deeplink_app(app: &str) -> bool {
    AppType::all().any(|kept| kept.as_str() == app)
}

/// `app` 是否属于可通过深链导入 provider 的 harness 集合。
pub(crate) fn is_supported_deeplink_provider_app(app: &str) -> bool {
    deeplink_provider_apps().any(|kept| kept.as_str() == app)
}

/// `apps` 参数里的单个 id 是否属于可承接 MCP 投影的 harness 集合。
pub(crate) fn is_supported_deeplink_mcp_app(app: &str) -> bool {
    McpService::supported_mcp_apps().any(|kept| kept.as_str() == app)
}

/// 深链入口的 harness 闸门。
///
/// 要过两道关，缺一不可：`AppType::from_str` 必须仍然认识已删的 id（旧 URL 和
/// 旧请求体要能给出准确的错误），但本构建只放 `AppType::all()` 里的 harness 过去。
/// 少了第二道，`app=openclaw` 的链接会一路走到 `ProviderService::add`，而它在新
/// 供应商成为"当前"时直接把 live 配置写进 `~/.openclaw/openclaw.json`
/// （OpenClaw 的写入没有 `should_sync_live` 门槛）；`app=gemini` 的 prompt 链接
/// 则会让 `PromptService::enable_prompt` 写 `~/.gemini/GEMINI.md`。
pub(crate) fn parse_deeplink_app(app_str: &str, resource: &str) -> Result<AppType, AppError> {
    let app = AppType::from_str(app_str)
        .map_err(|_| AppError::InvalidInput(format!("Invalid app type: {app_str}")))?;
    if !AppType::all().any(|kept| kept == app) {
        return Err(AppError::InvalidInput(format!(
            "Unsupported app id: '{app_str}' for {resource}. Supported apps: {}",
            supported_deeplink_app_ids()
        )));
    }
    Ok(app)
}

/// Deep link import request model.
///
/// This mirrors the upstream request model to keep URL parsing compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkImportRequest {
    pub version: String,
    pub resource: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub haiku_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sonnet_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opus_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_auto_interval: Option<u64>,

    #[serde(skip)]
    pub(crate) openclaw_config: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 深链闸门的两道关，两个方向都要钉住。
    ///
    /// `AppType::from_str` 仍然认识已删的 id（旧 URL 要得到"已删除"，而不是
    /// "不认识"），但闸门只放 `AppType::all()` 里的过去。二者的区别就是
    /// "一条 `app=openclaw` 的链接能不能写到 `~/.openclaw`"。
    #[test]
    fn parse_deeplink_app_keeps_only_shipped_harnesses() {
        for id in AppType::all().map(|app| app.as_str()) {
            let parsed = parse_deeplink_app(id, "provider");
            assert!(parsed.is_ok(), "{id} must stay importable, got {parsed:?}");
        }
        for id in ["gemini", "opencode", "openclaw"] {
            let err = parse_deeplink_app(id, "provider")
                .expect_err(&format!("{id} must be rejected by the deeplink gate"));
            let rendered = err.to_string();
            assert!(
                rendered.contains(&format!("Unsupported app id: '{id}' for provider")),
                "unexpected error for {id}: {rendered}"
            );
            // 只断言 "Supported apps: " 之后的那一段：前面回显的是用户输入的那个
            // id，不能拿它当"本构建支持什么"的证据。
            let advertised = rendered
                .split_once("Supported apps: ")
                .map(|(_, rest)| rest.to_string())
                .unwrap_or_default();
            assert_eq!(advertised, "claude, codex, hermes, pi");
            for removed in ["gemini", "opencode", "openclaw"] {
                assert!(
                    !advertised.contains(removed),
                    "rejection for {id} still advertises {removed}: {rendered}"
                );
            }
        }

        // 完全不认识的 id 仍然走"不认识"，不能和"已删除"混成一个错。
        let err = parse_deeplink_app("nope", "provider").expect_err("unknown id must be rejected");
        assert!(
            err.to_string().contains("Invalid app type: nope"),
            "unexpected error: {err}"
        );
    }

    /// 两份对外清单：全局按 `AppType::all()` 派生，MCP 少一个 Pi。
    ///
    /// 文案里列出的 id 就是用户会去试的，所以也不能多。
    #[test]
    fn supported_deeplink_app_id_lists_derive_from_the_kept_set() {
        assert_eq!(supported_deeplink_app_ids(), "claude, codex, hermes, pi");
        assert_eq!(
            supported_deeplink_provider_app_ids(),
            "claude, codex, hermes"
        );
        assert_eq!(supported_deeplink_mcp_app_ids(), "claude, codex, hermes");

        for id in ["claude", "codex", "hermes", "pi"] {
            assert!(is_supported_deeplink_app(id), "{id} must be a valid app id");
        }
        for id in ["gemini", "opencode", "openclaw", "nope", ""] {
            assert!(
                !is_supported_deeplink_app(id),
                "{id} must not be a valid app id"
            );
        }

        // provider 侧：Pi 的供应商来自 Pi 供应商页，不走深链。
        for id in ["claude", "codex", "hermes"] {
            assert!(
                is_supported_deeplink_provider_app(id),
                "{id} must be a valid provider app id"
            );
        }
        for id in ["gemini", "opencode", "openclaw", "pi", "nope", ""] {
            assert!(
                !is_supported_deeplink_provider_app(id),
                "{id} must not be a valid provider app id"
            );
        }

        // MCP 侧：保留的 harness 里只有 Pi 不该出现在 `apps=` 里。
        for id in ["claude", "codex", "hermes"] {
            assert!(
                is_supported_deeplink_mcp_app(id),
                "{id} must be a valid MCP app"
            );
        }
        for id in ["gemini", "opencode", "openclaw", "pi", "nope", ""] {
            assert!(
                !is_supported_deeplink_mcp_app(id),
                "{id} must not be a valid MCP app"
            );
        }
    }
}
