use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
pub(crate) struct PiProviderPreset {
    pub(crate) label: &'static str,
    pub(crate) provider_key: &'static str,
    pub(crate) website_url: &'static str,
    pub(crate) category: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) icon_color: Option<&'static str>,
    pub(crate) partner_promotion_key: Option<&'static str>,
    pub(crate) sponsor_id: Option<&'static str>,
    settings: fn() -> Value,
}

impl PiProviderPreset {
    pub(crate) fn settings_config(&self) -> Value {
        (self.settings)()
    }
}

pub(crate) static PI_BUILTIN_PROVIDER_PRESETS: [PiProviderPreset; 5] = [
    PiProviderPreset {
        label: "DeepSeek",
        provider_key: "cc-switch-deep-seek",
        website_url: "https://platform.deepseek.com",
        category: "cn_official",
        icon: "deepseek",
        icon_color: Some("#1E88E5"),
        partner_promotion_key: None,
        sponsor_id: None,
        settings: deepseek_settings,
    },
    PiProviderPreset {
        label: "Zhipu GLM",
        provider_key: "cc-switch-zhipu-glm",
        website_url: "https://open.bigmodel.cn",
        category: "cn_official",
        icon: "zhipu",
        icon_color: Some("#0F62FE"),
        partner_promotion_key: None,
        sponsor_id: None,
        settings: zhipu_glm_settings,
    },
    PiProviderPreset {
        label: "MiniMax",
        provider_key: "cc-switch-mini-max",
        website_url: "https://platform.minimaxi.com",
        category: "cn_official",
        icon: "minimax",
        icon_color: Some("#FF6B6B"),
        partner_promotion_key: Some("minimax_cn"),
        sponsor_id: None,
        settings: minimax_settings,
    },
    PiProviderPreset {
        label: "Xiaomi MiMo",
        provider_key: "cc-switch-xiaomi-mi-mo",
        website_url: "https://platform.xiaomimimo.com",
        category: "cn_official",
        icon: "xiaomimimo",
        icon_color: Some("#000000"),
        partner_promotion_key: None,
        sponsor_id: None,
        settings: xiaomi_mimo_settings,
    },
    PiProviderPreset {
        label: "OpenRouter",
        provider_key: "cc-switch-open-router",
        website_url: "https://openrouter.ai",
        category: "aggregator",
        icon: "openrouter",
        icon_color: Some("#6566F1"),
        partner_promotion_key: None,
        sponsor_id: None,
        settings: openrouter_settings,
    },
];

pub(crate) static PI_SPONSOR_PROVIDER_PRESETS: [PiProviderPreset; 0] = [];

fn claude_sonnet(id: &str) -> Value {
    json!({
        "name": "Claude Sonnet 5",
        "reasoning": true,
        "input": ["text", "image"],
        "contextWindow": 1_000_000,
        "maxTokens": 128_000,
        "id": id,
        "thinkingLevelMap": { "xhigh": "xhigh", "max": "max" },
        "compat": { "forceAdaptiveThinking": true },
    })
}

fn claude_opus(id: &str) -> Value {
    json!({
        "name": "Claude Opus 5",
        "reasoning": true,
        "input": ["text", "image"],
        "contextWindow": 1_000_000,
        "maxTokens": 128_000,
        "id": id,
        "thinkingLevelMap": { "xhigh": "xhigh", "max": "max" },
        "compat": { "forceAdaptiveThinking": true },
    })
}

fn deepseek_model(name: &str, id: &str) -> Value {
    json!({
        "name": name,
        "reasoning": true,
        "input": ["text"],
        "contextWindow": 1_000_000,
        "maxTokens": 384_000,
        "id": id,
        "thinkingLevelMap": {
            "minimal": null,
            "low": null,
            "medium": null,
            "high": "high",
            "max": "max",
        },
    })
}

fn deepseek_settings() -> Value {
    json!({
        "name": "DeepSeek",
        "baseUrl": "https://api.deepseek.com/v1",
        "api": "openai-completions",
        "apiKey": "",
        "models": [
            deepseek_model("DeepSeek V4 Pro", "deepseek-v4-pro"),
            deepseek_model("DeepSeek V4 Flash", "deepseek-v4-flash"),
        ],
    })
}

fn glm_5_1() -> Value {
    json!({
        "name": "GLM-5.1",
        "reasoning": true,
        "input": ["text"],
        "contextWindow": 200_000,
        "maxTokens": 131_072,
        "id": "glm-5.1",
        "thinkingLevelMap": {},
    })
}

fn zhipu_glm_settings() -> Value {
    json!({
        "name": "Zhipu GLM",
        "baseUrl": "https://open.bigmodel.cn/api/coding/paas/v4",
        "api": "openai-completions",
        "apiKey": "",
        "models": [glm_5_1()],
    })
}

fn minimax_settings() -> Value {
    json!({
        "name": "MiniMax",
        "baseUrl": "https://api.minimaxi.com/v1",
        "api": "openai-completions",
        "apiKey": "",
        "models": [{
            "name": "MiniMax-M2.7",
            "reasoning": true,
            "input": ["text"],
            "contextWindow": 204_800,
            "maxTokens": 131_072,
            "id": "MiniMax-M2.7",
            "thinkingLevelMap": {},
        }],
    })
}

fn xiaomi_model(name: &str, id: &str, input: &[&str], compat: bool) -> Value {
    let mut model = json!({
        "name": name,
        "reasoning": true,
        "input": input,
        "contextWindow": 1_048_576,
        "maxTokens": 131_072,
        "id": id,
        "thinkingLevelMap": {},
    });
    if compat {
        model["compat"] = json!({
            "requiresReasoningContentOnAssistantMessages": true,
            "thinkingFormat": "deepseek",
        });
    }
    model
}

fn xiaomi_mimo_settings() -> Value {
    json!({
        "name": "Xiaomi MiMo",
        "baseUrl": "https://api.xiaomimimo.com/v1",
        "api": "openai-completions",
        "apiKey": "",
        "models": [
            xiaomi_model("MiMo-V2.5-Pro", "mimo-v2.5-pro", &["text"], true),
            xiaomi_model("MiMo-V2.5", "mimo-v2.5", &["text", "image"], true),
        ],
    })
}

fn openrouter_settings() -> Value {
    json!({
        "name": "OpenRouter",
        "baseUrl": "https://openrouter.ai/api",
        "api": "anthropic-messages",
        "apiKey": "",
        "models": [
            claude_sonnet("anthropic/claude-sonnet-5"),
            claude_opus("anthropic/claude-opus-5"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn pi_preset_scope_is_the_selected_upstream_subset() {
        assert_eq!(
            PI_BUILTIN_PROVIDER_PRESETS
                .iter()
                .map(|preset| preset.label)
                .collect::<Vec<_>>(),
            [
                "DeepSeek",
                "Zhipu GLM",
                "MiniMax",
                "Xiaomi MiMo",
                "OpenRouter",
            ]
        );
        assert!(PI_SPONSOR_PROVIDER_PRESETS.is_empty());
    }

    #[test]
    fn pi_preset_keys_are_unique_and_native_settings_are_complete() {
        let presets = PI_BUILTIN_PROVIDER_PRESETS
            .iter()
            .chain(PI_SPONSOR_PROVIDER_PRESETS.iter());
        let mut keys = HashSet::new();
        for preset in presets {
            assert!(keys.insert(preset.provider_key), "duplicate provider key");
            let settings = preset.settings_config();
            assert_eq!(settings["name"], preset.label);
            assert!(settings["baseUrl"].is_string());
            assert!(settings["api"].is_string());
            assert_eq!(settings["apiKey"], "");
            assert!(settings["models"]
                .as_array()
                .is_some_and(|items| !items.is_empty()));
        }
    }
}
