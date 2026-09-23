use base64::prelude::*;
use cc_switch_lib::{
    import_mcp_from_deeplink, import_prompt_from_deeplink, import_provider_from_deeplink,
    import_skill_from_deeplink, parse_deeplink_url, AppType, MultiAppConfig,
};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, lock_test_mutex, reset_test_fs, state_from_config};

/// 报错文案里"合法取值"那一段。
///
/// 文案末尾会回显用户输入的那个 id（这是应该的），所以断言只能落在列举合法值的
/// 部分——那才是本构建对外的 harness 清单。形式固定为
/// `... must be one of <list>, got '<input>'`。
fn advertised_app_ids(message: &str) -> &str {
    let (_, rest) = message
        .split_once("must be one of ")
        .unwrap_or_else(|| panic!("rejection should list the supported app ids: {message}"));
    rest.split(", got ").next().unwrap_or(rest)
}

#[test]
fn deeplink_import_claude_provider_persists_to_config() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let url = "ccswitch://v1/import?resource=provider&app=claude&name=DeepLink%20Claude&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com%2Fv1&apiKey=sk-test-claude-key&model=claude-sonnet-4";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);

    let state = state_from_config(config);

    let provider_id = import_provider_from_deeplink(&state, request.clone())
        .expect("import provider from deeplink");

    // 验证内存状态
    let guard = state.config.read().expect("read config");
    let manager = guard
        .get_manager(&AppType::Claude)
        .expect("claude manager should exist");
    let provider = manager
        .providers
        .get(&provider_id)
        .expect("provider created via deeplink");
    assert_eq!(
        provider.name,
        request.name.clone().expect("request name"),
        "provider name should match deeplink"
    );
    assert_eq!(provider.website_url.as_deref(), request.homepage.as_deref());
    let auth_token = provider
        .settings_config
        .pointer("/env/ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str());
    let base_url = provider
        .settings_config
        .pointer("/env/ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str());
    assert_eq!(auth_token, request.api_key.as_deref());
    assert_eq!(base_url, request.endpoint.as_deref());
    drop(guard);

    // 验证配置已持久化
    let persisted = state
        .db
        .get_provider_by_id(&provider_id, AppType::Claude.as_str())
        .expect("read provider from db");
    assert!(persisted.is_some(), "provider should be persisted to db");
}

#[test]
fn deeplink_import_codex_provider_builds_auth_and_config() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let url = "ccswitch://v1/import?resource=provider&app=codex&name=DeepLink%20Codex&homepage=https%3A%2F%2Fopenai.example&endpoint=https%3A%2F%2Fapi.openai.example%2Fv1&apiKey=sk-test-codex-key&model=gpt-4o";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Codex);

    let state = state_from_config(config);

    let provider_id = import_provider_from_deeplink(&state, request.clone())
        .expect("import provider from deeplink");

    let guard = state.config.read().expect("read config");
    let manager = guard
        .get_manager(&AppType::Codex)
        .expect("codex manager should exist");
    let provider = manager
        .providers
        .get(&provider_id)
        .expect("provider created via deeplink");
    assert_eq!(
        provider.name,
        request.name.clone().expect("request name"),
        "provider name should match deeplink"
    );
    assert_eq!(provider.website_url.as_deref(), request.homepage.as_deref());
    let auth_value = provider
        .settings_config
        .pointer("/auth/OPENAI_API_KEY")
        .and_then(|v| v.as_str());
    let config_text = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(auth_value, request.api_key.as_deref());
    assert!(
        request
            .endpoint
            .as_deref()
            .is_some_and(|endpoint| config_text.contains(endpoint)),
        "config.toml content should contain endpoint"
    );
    assert!(
        config_text.contains("model = \"gpt-4o\""),
        "config.toml content should contain model setting"
    );
    assert!(
        config_text.contains("model_provider = "),
        "config.toml should use upstream model_provider format"
    );
    assert!(
        config_text.contains("[model_providers."),
        "config.toml should have [model_providers.xxx] section"
    );
    drop(guard);

    let persisted = state
        .db
        .get_provider_by_id(&provider_id, AppType::Codex.as_str())
        .expect("read provider from db");
    assert!(persisted.is_some(), "provider should be persisted to db");
}

/// Regression for issue #333: a deeplink-imported Codex provider must carry a
/// non-empty `name` in its `[model_providers.custom]` table, otherwise Codex
/// refuses to load config.toml ("provider name must not be empty"). Mirrors the
/// upstream `build_codex_settings_uses_custom_key_and_preserves_display_name`.
#[test]
fn deeplink_import_codex_provider_preserves_display_name_in_config() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    // Name carries a quote to exercise TOML escaping; model is omitted to hit
    // the default. `%22` decodes to `"`.
    let url = "ccswitch://v1/import?resource=provider&app=codex&name=My%20%22Relay%22&endpoint=https%3A%2F%2Fapi.example.com%2Fv1%2F&apiKey=sk-test";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Codex);
    let state = state_from_config(config);

    let provider_id =
        import_provider_from_deeplink(&state, request).expect("import provider from deeplink");

    let guard = state.config.read().expect("read config");
    let provider = guard
        .get_manager(&AppType::Codex)
        .expect("codex manager should exist")
        .providers
        .get(&provider_id)
        .expect("provider created via deeplink");
    let config_text = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .expect("config text");

    // Must parse as valid Codex config (this is what Codex itself does).
    let parsed: toml::Value = toml::from_str(config_text).expect("valid Codex config.toml");
    assert_eq!(
        parsed.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "deeplink Codex import should use the shared `custom` model_provider id"
    );
    let custom = parsed
        .get("model_providers")
        .and_then(|v| v.get("custom"))
        .expect("[model_providers.custom] table");
    assert_eq!(
        custom.get("name").and_then(|v| v.as_str()),
        Some("My \"Relay\""),
        "provider display name must be preserved (issue #333)"
    );
    assert_eq!(
        custom.get("base_url").and_then(|v| v.as_str()),
        Some("https://api.example.com/v1"),
        "trailing slash should be trimmed from the endpoint"
    );
    assert_eq!(
        custom.get("requires_openai_auth").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        parsed.get("model").and_then(|v| v.as_str()),
        Some("gpt-5-codex"),
        "omitted model should fall back to the default"
    );
}

/// OpenClaw 已从本构建移除，深链 URL 必须在这一关就被拒掉。
///
/// 上游这三条 openclaw provider 测试断言的是导入成功后的字段形状。那条路径没了：
/// `import_provider_from_deeplink` 拿到 openclaw 会走到 `ProviderService::add`，
/// 而它在新供应商成为"当前"时把 live 配置写进 `~/.openclaw/openclaw.json`
/// （OpenClaw 的写入没有 `should_sync_live` 门槛）——正是清理要堵住的行为。
/// 更深的 `canonicalize_openclaw_config` 校验改由 `deeplink::provider` 的单元
/// 测试直接覆盖，因为它已经不可能从 URL 到达了。
#[test]
fn deeplink_import_rejects_removed_harness_provider_links() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();

    for app in ["gemini", "opencode", "openclaw"] {
        let url = format!(
            "ccswitch://v1/import?resource=provider&app={app}&name=Gone&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com%2Fv1&apiKey=sk-test-key"
        );

        let err = parse_deeplink_url(&url)
            .expect_err(&format!("{app} provider link must not parse in this build"));
        let rendered = err.to_string();
        assert!(
            rendered.contains("Invalid app type"),
            "unexpected parser error for {app}: {rendered}"
        );
        // 报错文案不能再把已删 harness 当合法值列出来——URL 是第三方分发的，
        // 这里列什么，用户就会去试什么。（文案末尾会回显用户输入的那个 id，
        // 所以只断言列举合法值的那一段。）
        let advertised = advertised_app_ids(&rendered);
        assert_eq!(
            advertised, "claude, codex, hermes",
            "advertised provider app ids changed for {app}"
        );
        for removed in ["gemini", "opencode", "openclaw"] {
            assert!(
                !advertised.contains(removed),
                "rejection for {app} still advertises {removed}: {rendered}"
            );
        }
    }

    for relative in [
        ".openclaw/openclaw.json",
        ".openclaw/AGENTS.md",
        ".gemini/settings.json",
        ".config/opencode/opencode.json",
    ] {
        assert!(
            !home.join(relative).exists(),
            "{relative} must not exist after a rejected deep link"
        );
    }
}

#[test]
fn deeplink_import_rejects_non_http_endpoints_from_config() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    ensure_test_home();

    let config_json =
        r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-test","ANTHROPIC_BASE_URL":"ftp://example.com/v1"}}"#;
    let config_b64 = BASE64_URL_SAFE_NO_PAD.encode(config_json.as_bytes());

    let url = format!(
        "ccswitch://v1/import?resource=provider&app=claude&name=BadEndpoint&config={config_b64}&configFormat=json"
    );
    let request = parse_deeplink_url(&url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);

    let state = state_from_config(config);

    let err = import_provider_from_deeplink(&state, request)
        .expect_err("non-http endpoints should be rejected");
    assert!(
        err.to_string().contains("Invalid URL scheme"),
        "expected scheme validation error, got {err:?}"
    );
}

#[test]
fn deeplink_import_mcp_server_persists_with_app_flags() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let config_json = r#"{"mcpServers":{"fetch":{"command":"uvx","args":["mcp-server-fetch"]}}}"#;
    let config_b64 = BASE64_URL_SAFE_NO_PAD.encode(config_json.as_bytes());
    let url = format!(
        "ccswitch://v1/import?resource=mcp&apps=claude,codex&config={config_b64}&enabled=true"
    );
    let request = parse_deeplink_url(&url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);
    let state = state_from_config(config);

    let result = import_mcp_from_deeplink(&state, request).expect("import mcp from deeplink");
    assert_eq!(result.imported_count, 1);
    assert!(result.failed.is_empty(), "no imports should fail");
    assert_eq!(result.imported_ids, vec!["fetch".to_string()]);

    let servers = state.db.get_all_mcp_servers().expect("read mcp servers");
    let server = servers.get("fetch").expect("mcp server persisted");
    assert!(server.apps.claude, "claude flag should be set");
    assert!(server.apps.codex, "codex flag should be set");
    assert!(!server.apps.gemini, "gemini flag should remain unset");
    assert_eq!(
        server.server.pointer("/command").and_then(|v| v.as_str()),
        Some("uvx")
    );
}

/// `apps=` 只接受本构建能真正投影 MCP 的 harness。
///
/// 上游这里断言的是"openclaw-only 会晚一步报 'At least one app'"，因为 parser
/// 那时还认 openclaw、`parse_mcp_apps` 再静默丢掉它。现在两关都拒：parser
/// 先看到不认识的 id，`parse_mcp_apps` 作为库侧兜底也不放它过去（`McpApps`
/// 保留 `gemini`/`opencode` 位只是为了读旧数据库，不是为了在深链里用）。
#[test]
fn deeplink_import_mcp_rejects_removed_or_unsupported_app_ids() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let config_json = r#"{"mcpServers":{"test-server":{"command":"echo","args":["hi"]}}}"#;
    let config_b64 = BASE64_URL_SAFE_NO_PAD.encode(config_json.as_bytes());

    for app in ["openclaw", "gemini", "opencode", "pi"] {
        let url = format!("ccswitch://v1/import?resource=mcp&apps={app}&config={config_b64}");
        let err = parse_deeplink_url(&url)
            .expect_err(&format!("apps={app} must be rejected by the parser"));
        let rendered = err.to_string();
        assert!(
            rendered.contains("Invalid app in 'apps'"),
            "unexpected parser error for apps={app}: {rendered}"
        );
        let advertised = advertised_app_ids(&rendered);
        assert_eq!(
            advertised, "claude, codex, hermes",
            "advertised MCP app ids changed for apps={app}"
        );
        for removed in ["gemini", "opencode", "openclaw"] {
            assert!(
                !advertised.contains(removed),
                "rejection for apps={app} still advertises {removed}: {rendered}"
            );
        }
    }
}

#[test]
fn deeplink_import_mcp_requires_at_least_one_app() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let config_json = r#"{"mcpServers":{"test-server":{"command":"echo","args":["hi"]}}}"#;
    let config_b64 = BASE64_URL_SAFE_NO_PAD.encode(config_json.as_bytes());
    let url = format!("ccswitch://v1/import?resource=mcp&apps=&config={config_b64}");

    // 空白 id 走的是"一个 app 都没给"这条分支，不是"不认识的 id"。
    let request = parse_deeplink_url(&url).expect("whitespace apps pass parser validation");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);
    let state = state_from_config(config);

    let err = match import_mcp_from_deeplink(&state, request) {
        Err(e) => e,
        Ok(_) => panic!("blank apps should have failed"),
    };
    assert!(
        err.to_string()
            .contains("At least one app must be specified"),
        "expected late error about no apps, got: {err}"
    );
}

#[test]
fn deeplink_import_prompt_persists_and_enables() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let content = "You are a helpful assistant.";
    let content_b64 = BASE64_URL_SAFE_NO_PAD.encode(content.as_bytes());
    let url = format!(
        "ccswitch://v1/import?resource=prompt&app=claude&name=Helper&content={content_b64}&description=desc&enabled=true"
    );
    let request = parse_deeplink_url(&url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);
    let state = state_from_config(config);

    let prompt_id = import_prompt_from_deeplink(&state, request).expect("import prompt");

    let prompts = state.db.get_prompts("claude").expect("read prompts");
    let prompt = prompts.get(&prompt_id).expect("prompt persisted");
    assert_eq!(prompt.content, content);
    assert_eq!(prompt.name, "Helper");
    assert_eq!(prompt.description.as_deref(), Some("desc"));
    assert!(prompt.enabled, "enabled=true should activate the prompt");
}

#[test]
fn deeplink_import_prompt_without_enabled_stays_disabled() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let content_b64 = BASE64_URL_SAFE_NO_PAD.encode(b"hello");
    let url =
        format!("ccswitch://v1/import?resource=prompt&app=claude&name=Idle&content={content_b64}");
    let request = parse_deeplink_url(&url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);
    let state = state_from_config(config);

    let prompt_id = import_prompt_from_deeplink(&state, request).expect("import prompt");

    let prompts = state.db.get_prompts("claude").expect("read prompts");
    let prompt = prompts.get(&prompt_id).expect("prompt persisted");
    assert!(!prompt.enabled, "prompt should default to disabled");
}

#[test]
fn deeplink_import_prompt_for_non_claude_app() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let content_b64 = BASE64_URL_SAFE_NO_PAD.encode(b"codex prompt content");
    let url = format!(
        "ccswitch://v1/import?resource=prompt&app=codex&name=CodexPrompt&content={content_b64}&description=codex-desc"
    );
    let request = parse_deeplink_url(&url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Codex);
    let state = state_from_config(config);

    let prompt_id = import_prompt_from_deeplink(&state, request).expect("import prompt for codex");

    let prompts = state.db.get_prompts("codex").expect("read codex prompts");
    let prompt = prompts.get(&prompt_id).expect("prompt persisted for codex");
    assert_eq!(prompt.name, "CodexPrompt");
    assert_eq!(prompt.content, "codex prompt content");
    assert_eq!(prompt.description.as_deref(), Some("codex-desc"));
    assert!(!prompt.enabled, "should default to disabled");
}

#[test]
fn deeplink_parser_accepts_pi_prompts_but_not_pi_providers() {
    let content_b64 = BASE64_URL_SAFE_NO_PAD.encode(b"pi prompt content");
    let prompt_url =
        format!("ccswitch://v1/import?resource=prompt&app=pi&name=PiPrompt&content={content_b64}");
    let prompt = parse_deeplink_url(&prompt_url).expect("parse Pi prompt deeplink");
    assert_eq!(prompt.app.as_deref(), Some("pi"));

    let provider =
        parse_deeplink_url("ccswitch://v1/import?resource=provider&app=pi&name=PiProvider");
    assert!(
        provider.is_err(),
        "Pi provider deeplinks remain unsupported"
    );
}

#[test]
fn deeplink_import_skill_repo_persists() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    let _home = ensure_test_home();

    let url = "ccswitch://v1/import?resource=skill&repo=octocat/example-skills&branch=dev";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let mut config = MultiAppConfig::default();
    config.ensure_app(&AppType::Claude);
    let state = state_from_config(config);

    let repo_id = import_skill_from_deeplink(&state, request).expect("import skill repo");
    assert_eq!(repo_id, "octocat/example-skills");

    let repos = state.db.get_skill_repos().expect("read skill repos");
    let repo = repos
        .iter()
        .find(|r| r.owner == "octocat" && r.name == "example-skills")
        .expect("skill repo persisted");
    assert_eq!(repo.branch, "dev");
    assert!(repo.enabled, "repo should default to enabled");
}

#[test]
fn deeplink_import_skill_rejects_malformed_repo() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    ensure_test_home();

    let err = parse_deeplink_url("ccswitch://v1/import?resource=skill&repo=not-a-repo")
        .expect_err("malformed repo should be rejected");
    assert!(
        err.to_string().contains("Invalid repo format"),
        "expected repo format error, got {err:?}"
    );
}

#[test]
fn deeplink_command_execute_dispatches_by_resource_type() {
    let _guard = lock_test_mutex();

    let mcp_content = r#"{"mcpServers":{"dispatch-mcp":{"command":"echo","args":["hello"]}}}"#;
    let mcp_b64 = BASE64_URL_SAFE_NO_PAD.encode(mcp_content.as_bytes());
    let prompt_b64 = BASE64_URL_SAFE_NO_PAD.encode(b"dispatch prompt content");

    let resource_urls: &[(&str, &str)] = &[
        (
            "provider",
            "ccswitch://v1/import?resource=provider&app=claude&name=DispatchProvider&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com%2Fv1&apiKey=sk-dispatch",
        ),
        (
            "mcp",
            &format!("ccswitch://v1/import?resource=mcp&apps=claude,codex&config={mcp_b64}"),
        ),
        (
            "prompt",
            &format!("ccswitch://v1/import?resource=prompt&app=claude&name=DispatchPrompt&content={prompt_b64}"),
        ),
        (
            "skill",
            "ccswitch://v1/import?resource=skill&repo=dispatch-org/dispatch-skills",
        ),
    ];

    for (resource, url) in resource_urls {
        reset_test_fs();
        let _home = ensure_test_home();

        let cmd = cc_switch_lib::cli::commands::deeplink::DeeplinkCommand {
            url: url.to_string(),
        };
        cc_switch_lib::cli::commands::deeplink::execute(cmd, None)
            .unwrap_or_else(|e| panic!("[{resource}] execute() should succeed, got: {e}"));
    }
}

#[test]
fn deeplink_parse_rejects_unknown_resource_type() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    ensure_test_home();

    let err = parse_deeplink_url("ccswitch://v1/import?resource=unknown&app=claude&name=test")
        .expect_err("unknown resource type should be rejected at parser level");
    assert!(
        err.to_string().contains("Unsupported resource type"),
        "expected 'Unsupported resource type', got: {err}"
    );
}

#[test]
fn deeplink_parse_rejects_missing_required_params() {
    let _guard = lock_test_mutex();

    let cases: &[(&str, &str, &str)] = &[
        // (case label, URL missing a required param, expected error fragment)
        (
            "mcp without config",
            "ccswitch://v1/import?resource=mcp&apps=claude",
            "config",
        ),
        (
            "prompt without content",
            "ccswitch://v1/import?resource=prompt&app=claude&name=NoContent",
            "content",
        ),
        (
            "provider without app",
            "ccswitch://v1/import?resource=provider&name=NoApp",
            "app",
        ),
    ];

    for (label, url, expected_fragment) in cases {
        reset_test_fs();
        ensure_test_home();

        let err = match parse_deeplink_url(url) {
            Err(e) => e,
            Ok(_) => panic!("[{label}] parser should have rejected URL"),
        };
        assert!(
            err.to_string()
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "[{label}] expected error mentioning '{expected_fragment}', got: {err}"
        );
    }
}

#[test]
fn deeplink_parse_rejects_mcp_invalid_app_in_apps_list() {
    let _guard = lock_test_mutex();
    reset_test_fs();
    ensure_test_home();

    let err = parse_deeplink_url(
        "ccswitch://v1/import?resource=mcp&apps=claude,notanapp&config=dGVzdA==",
    )
    .expect_err("invalid app in apps list should be rejected at parser level");
    assert!(
        err.to_string().to_lowercase().contains("invalid app"),
        "expected 'invalid app' error, got: {err}"
    );
}
