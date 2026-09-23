//! 官方供应商种子数据
//!
//! 启动时调用 `Database::init_default_official_providers` 把这些条目
//! 写入 `providers` 表，让所有用户都能看到一个"一键切回官方"的入口。
//!
//! 只覆盖 `AppType::all()` 里的 harness。给已删 harness 留一条种子，等于让
//! 全新安装也自动长出一行 Gemini 供应商，然后顺着 `config export` / WebDAV /
//! S3 备份一直传下去——`store::initialize_common_config_snippets` 早就写明
//! 不再给这些 harness 播种，两处必须保持一致。旧库里已有的那行保持原样读取，
//! 不删不覆写。

use crate::app_config::AppType;

pub(crate) struct OfficialProviderSeed {
    pub id: &'static str,
    pub app_type: AppType,
    pub name: &'static str,
    pub website_url: &'static str,
    pub icon: &'static str,
    pub icon_color: &'static str,
    pub settings_config_json: &'static str,
}

pub(crate) const OFFICIAL_SEEDS: &[OfficialProviderSeed] = &[
    OfficialProviderSeed {
        id: "claude-official",
        app_type: AppType::Claude,
        name: "Claude Official",
        website_url: "https://www.anthropic.com/claude-code",
        icon: "anthropic",
        icon_color: "#D4915D",
        settings_config_json: r#"{"env":{}}"#,
    },
    OfficialProviderSeed {
        id: "codex-official",
        app_type: AppType::Codex,
        name: "OpenAI Official",
        website_url: "https://chatgpt.com/codex",
        icon: "openai",
        icon_color: "#00A67E",
        settings_config_json: r#"{"auth":{},"config":""}"#,
    },
];

pub(crate) fn is_official_seed_id(id: &str) -> bool {
    OFFICIAL_SEEDS.iter().any(|seed| seed.id == id)
}
