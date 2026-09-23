use clap::{Subcommand, ValueEnum};
use serde_json::json;

use crate::app_config::AppType;
use crate::cli::i18n::{self, Language};
use crate::cli::ui::{highlight, info, success, to_json, warning};
use crate::error::AppError;

#[derive(Subcommand, Debug, Clone)]
pub enum SettingsCommand {
    /// Show persisted cc-switch settings
    Show {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Get or set the TUI language
    Language {
        /// Language to persist (en|zh)
        #[arg(value_enum)]
        language: Option<LanguageArg>,
    },

    /// Manage visible apps shown in the TUI
    #[command(name = "visible-apps", subcommand)]
    VisibleApps(VisibleAppsCommand),

    /// Show, skip, or require Claude onboarding
    #[command(name = "claude-onboarding", subcommand)]
    ClaudeOnboarding(ClaudeOnboardingCommand),

    /// Show, enable, or disable Claude plugin integration
    #[command(name = "claude-plugin", subcommand)]
    ClaudePlugin(ClaudePluginCommand),

    /// Manage Codex official login preservation for direct provider switches
    #[command(name = "codex-auth-preservation", subcommand)]
    CodexAuthPreservation(CodexAuthPreservationCommand),

    /// Manage unified Codex session history
    #[command(name = "codex-history", subcommand)]
    CodexHistory(CodexHistoryCommand),

    /// Manage the persistent global outbound proxy
    #[command(name = "outbound-proxy", subcommand)]
    OutboundProxy(OutboundProxyCommand),
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageArg {
    En,
    Zh,
}

impl From<LanguageArg> for Language {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::En => Language::English,
            LanguageArg::Zh => Language::Chinese,
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum VisibleAppsCommand {
    /// Show visible app settings
    Show {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ClaudeOnboardingCommand {
    /// Show current onboarding skip setting
    Show,
    /// Mark Claude onboarding as completed and skip the first-run prompt
    Skip,
    /// Clear the completed marker so Claude onboarding can run again
    Require,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ClaudePluginCommand {
    /// Show current Claude plugin integration setting
    Show,
    /// Enable Claude plugin integration and sync current provider state
    Enable,
    /// Disable Claude plugin integration and sync current provider state
    Disable,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CodexAuthPreservationCommand {
    /// Show whether direct third-party switches preserve the Codex official login
    Show {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Preserve the Codex official login on future direct third-party switches
    Enable,
    /// Replace the Codex official login on future direct third-party switches
    Disable,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CodexHistoryCommand {
    /// Show unified Codex session history setting
    Show {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Enable unified Codex session history
    Enable {
        /// Also migrate existing official Codex sessions into the shared history bucket
        #[arg(long)]
        migrate_existing: bool,
    },
    /// Disable unified Codex session history
    Disable {
        /// Restore previously migrated official sessions from backups
        #[arg(long)]
        restore: bool,
    },
    /// Migrate existing official Codex sessions into the shared bucket
    #[command(name = "migrate-existing")]
    MigrateExisting,
    /// Restore migrated official Codex sessions from backups
    Restore,
}

#[derive(Subcommand, Clone)]
pub enum OutboundProxyCommand {
    /// Show the saved outbound proxy and effective source
    Show {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Save a global outbound proxy
    Set {
        /// Proxy URL (http, https, socks5, or socks5h)
        url: String,
        /// Optional proxy username
        #[arg(long)]
        username: Option<String>,
        /// Optional proxy password
        #[arg(long)]
        password: Option<String>,
    },
    /// Clear the saved proxy and fall back to environment proxy variables
    Clear,
}

impl std::fmt::Debug for OutboundProxyCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Show { json } => formatter.debug_struct("Show").field("json", json).finish(),
            Self::Set {
                url,
                username,
                password,
            } => formatter
                .debug_struct("Set")
                .field("url", &crate::proxy::http_client::mask_url(url))
                .field("username", &username.as_ref().map(|_| "***"))
                .field("password", &password.as_ref().map(|_| "***"))
                .finish(),
            Self::Clear => formatter.write_str("Clear"),
        }
    }
}

pub fn execute(cmd: SettingsCommand) -> Result<(), AppError> {
    match cmd {
        SettingsCommand::Show { json } => show_settings(json),
        SettingsCommand::Language { language } => language_cmd(language),
        SettingsCommand::VisibleApps(cmd) => visible_apps_cmd(cmd),
        SettingsCommand::ClaudeOnboarding(cmd) => claude_onboarding_cmd(cmd),
        SettingsCommand::ClaudePlugin(cmd) => claude_plugin_cmd(cmd),
        SettingsCommand::CodexAuthPreservation(cmd) => codex_auth_preservation_cmd(cmd),
        SettingsCommand::CodexHistory(cmd) => codex_history_cmd(cmd),
        SettingsCommand::OutboundProxy(cmd) => outbound_proxy_cmd(cmd),
    }
}

fn outbound_proxy_cmd(cmd: OutboundProxyCommand) -> Result<(), AppError> {
    match cmd {
        OutboundProxyCommand::Show { json } => show_outbound_proxy(json),
        OutboundProxyCommand::Set {
            url,
            username,
            password,
        } => set_outbound_proxy(url, username, password),
        OutboundProxyCommand::Clear => clear_outbound_proxy(),
    }
}

fn show_outbound_proxy(json_output: bool) -> Result<(), AppError> {
    let state = crate::AppState::try_new()?;
    let saved = crate::services::global_proxy::load(&state.db)?;
    let environment_variables = crate::services::global_proxy::configured_environment_variables();
    let effective_source = if saved.is_some() {
        "saved"
    } else if environment_variables.is_empty() {
        "direct"
    } else {
        "environment"
    };

    if json_output {
        let payload = json!({
            "configured": saved.is_some(),
            "url": saved.as_ref().map(|config| config.url.as_str()),
            "username": saved.as_ref().map(|config| config.username.as_str()),
            "password": saved.as_ref().map(|config| config.password.as_str()),
            "effectiveSource": effective_source,
            "environmentVariables": environment_variables,
        });
        println!(
            "{}",
            to_json(&payload).map_err(|error| AppError::Message(error.to_string()))?
        );
        return Ok(());
    }

    println!("{}", highlight("Global Outbound Proxy"));
    match saved {
        Some(config) => {
            println!("URL: {}", config.url);
            println!("Username: {}", config.username);
            println!("Password: {}", config.password);
        }
        None => println!("Saved proxy: (not set)"),
    }
    println!("Effective source: {effective_source}");
    if !environment_variables.is_empty() {
        println!(
            "Environment variables: {}",
            environment_variables.join(", ")
        );
    }
    Ok(())
}

fn set_outbound_proxy(
    url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), AppError> {
    let state = crate::AppState::try_new()?;
    let config = outbound_proxy_config(url, username, password)?;
    if config.to_full_url()?.is_empty() {
        return clear_outbound_proxy_with_state(&state);
    }
    let environment_variables = crate::services::global_proxy::configured_environment_variables();
    if !environment_variables.is_empty() {
        println!(
            "{}",
            warning(&format!(
                "Environment proxy variables are set ({}). The saved outbound proxy takes precedence.",
                environment_variables.join(", ")
            ))
        );
    }

    let daemon_warning = crate::services::global_proxy::set(&state, &config)?;
    println!("{}", success("Global outbound proxy saved"));
    print_daemon_reload_warning(daemon_warning);
    Ok(())
}

fn clear_outbound_proxy() -> Result<(), AppError> {
    let state = crate::AppState::try_new()?;
    clear_outbound_proxy_with_state(&state)
}

fn clear_outbound_proxy_with_state(state: &crate::AppState) -> Result<(), AppError> {
    let daemon_warning = crate::services::global_proxy::clear(state)?;
    println!("{}", success("Global outbound proxy cleared"));
    let environment_variables = crate::services::global_proxy::configured_environment_variables();
    if !environment_variables.is_empty() {
        println!(
            "{}",
            info(&format!(
                "Environment proxy variables remain active: {}",
                environment_variables.join(", ")
            ))
        );
    }
    print_daemon_reload_warning(daemon_warning);
    Ok(())
}

fn outbound_proxy_config(
    url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<crate::services::GlobalOutboundProxyConfig, AppError> {
    let mut config = crate::services::GlobalOutboundProxyConfig::from_full_url(&url)?;
    if let Some(username) = username {
        config.username = username;
    }
    if let Some(password) = password {
        config.password = password;
    }
    config.to_full_url()?;
    Ok(config)
}

fn print_daemon_reload_warning(message: Option<String>) {
    if let Some(message) = message {
        println!("{}", warning(&message));
    }
}

fn show_settings(json_output: bool) -> Result<(), AppError> {
    let settings = crate::settings::get_settings();
    if json_output {
        let payload = json!({
            "language": settings.language.as_deref().unwrap_or(Language::English.code()),
            "visibleApps": settings.visible_apps,
            "skipClaudeOnboarding": settings.skip_claude_onboarding,
            "enableClaudePluginIntegration": settings.enable_claude_plugin_integration,
            "preserveCodexOfficialAuthOnSwitch": settings.preserve_codex_official_auth_on_switch,
            "unifyCodexSessionHistory": settings.unify_codex_session_history,
            "unifyCodexMigrateExisting": settings.unify_codex_migrate_existing.unwrap_or(false),
            "hasCodexHistoryUnifyBackup": crate::codex_history_migration::has_codex_official_history_unify_backup(),
            "piConfigDir": settings.pi_config_dir,
            "preferredEditor": settings.preferred_editor,
        });
        println!(
            "{}",
            to_json(&payload).map_err(|err| AppError::Message(err.to_string()))?
        );
        return Ok(());
    }

    println!("{}", highlight("Settings"));
    println!("Language: {}", i18n::current_language().code());
    print_visible_apps_summary();
    println!(
        "Skip Claude onboarding: {}",
        yes_no(settings.skip_claude_onboarding)
    );
    println!(
        "Claude plugin integration: {}",
        yes_no(settings.enable_claude_plugin_integration)
    );
    println!(
        "Preserve Codex official login: {}",
        yes_no(settings.preserve_codex_official_auth_on_switch)
    );
    println!(
        "Unified Codex session history: {}",
        yes_no(settings.unify_codex_session_history)
    );
    println!(
        "Pi config dir: {}",
        settings.pi_config_dir.as_deref().unwrap_or("(default)")
    );
    println!(
        "Preferred editor: {}",
        settings.preferred_editor.as_deref().unwrap_or("(not set)")
    );
    Ok(())
}

fn language_cmd(language: Option<LanguageArg>) -> Result<(), AppError> {
    let Some(language) = language else {
        println!("Language: {}", i18n::current_language().code());
        return Ok(());
    };

    let language = Language::from(language);
    i18n::set_language(language)?;
    println!(
        "{}",
        success(&format!("Language set to {}", language.display_name()))
    );
    Ok(())
}

fn visible_apps_cmd(cmd: VisibleAppsCommand) -> Result<(), AppError> {
    match cmd {
        VisibleAppsCommand::Show { json } => show_visible_apps(json),
    }
}

fn show_visible_apps(json_output: bool) -> Result<(), AppError> {
    let settings = crate::settings::get_settings();
    if json_output {
        let payload = json!({
            "apps": settings.visible_apps,
            "enabled": enabled_app_labels(&settings.visible_apps),
        });
        println!(
            "{}",
            to_json(&payload).map_err(|err| AppError::Message(err.to_string()))?
        );
        return Ok(());
    }

    print_visible_apps_summary();
    Ok(())
}

fn claude_onboarding_cmd(cmd: ClaudeOnboardingCommand) -> Result<(), AppError> {
    match cmd {
        ClaudeOnboardingCommand::Show => {
            println!(
                "Skip Claude onboarding: {}",
                yes_no(crate::settings::get_skip_claude_onboarding())
            );
            Ok(())
        }
        ClaudeOnboardingCommand::Skip => set_skip_claude_onboarding(true),
        ClaudeOnboardingCommand::Require => set_skip_claude_onboarding(false),
    }
}

fn set_skip_claude_onboarding(enabled: bool) -> Result<(), AppError> {
    crate::settings::set_skip_claude_onboarding(enabled)?;
    println!(
        "{}",
        success(&format!(
            "Skip Claude onboarding {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    );
    Ok(())
}

fn claude_plugin_cmd(cmd: ClaudePluginCommand) -> Result<(), AppError> {
    match cmd {
        ClaudePluginCommand::Show => {
            println!(
                "Claude plugin integration: {}",
                yes_no(crate::settings::get_enable_claude_plugin_integration())
            );
            Ok(())
        }
        ClaudePluginCommand::Enable => set_claude_plugin_integration(true),
        ClaudePluginCommand::Disable => set_claude_plugin_integration(false),
    }
}

fn set_claude_plugin_integration(enabled: bool) -> Result<(), AppError> {
    crate::settings::set_enable_claude_plugin_integration(enabled)?;
    if let Err(err) = crate::claude_plugin::sync_claude_plugin_on_settings_toggle(enabled) {
        println!(
            "{}",
            warning(&format!(
                "Claude plugin integration setting saved, but plugin sync failed: {err}"
            ))
        );
    }
    println!(
        "{}",
        success(&format!(
            "Claude plugin integration {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    );
    Ok(())
}

fn codex_auth_preservation_cmd(cmd: CodexAuthPreservationCommand) -> Result<(), AppError> {
    match cmd {
        CodexAuthPreservationCommand::Show { json } => show_codex_auth_preservation(json),
        CodexAuthPreservationCommand::Enable => set_codex_auth_preservation(true),
        CodexAuthPreservationCommand::Disable => set_codex_auth_preservation(false),
    }
}

fn show_codex_auth_preservation(json_output: bool) -> Result<(), AppError> {
    let enabled = crate::settings::preserve_codex_official_auth_on_switch();
    if json_output {
        let payload = json!({
            "preserveCodexOfficialAuthOnSwitch": enabled,
        });
        println!(
            "{}",
            to_json(&payload).map_err(|err| AppError::Message(err.to_string()))?
        );
        return Ok(());
    }

    println!(
        "Preserve Codex official login on direct switch: {}",
        yes_no(enabled)
    );
    println!(
        "{}",
        info("Proxy takeover always preserves the Codex official login.")
    );
    Ok(())
}

fn set_codex_auth_preservation(enabled: bool) -> Result<(), AppError> {
    crate::settings::set_preserve_codex_official_auth_on_switch(enabled)?;
    println!(
        "{}",
        success(&format!(
            "Codex official login preservation {}",
            if enabled { "enabled" } else { "disabled" }
        ))
    );
    println!(
        "{}",
        info(
            "Applies to future direct third-party switches; proxy takeover always preserves the Codex official login.",
        )
    );
    Ok(())
}

fn codex_history_cmd(cmd: CodexHistoryCommand) -> Result<(), AppError> {
    match cmd {
        CodexHistoryCommand::Show { json } => show_codex_history(json),
        CodexHistoryCommand::Enable { migrate_existing } => {
            set_codex_history_enabled(true, migrate_existing, false)
        }
        CodexHistoryCommand::Disable { restore } => {
            set_codex_history_enabled(false, false, restore)
        }
        CodexHistoryCommand::MigrateExisting => migrate_codex_history_existing(),
        CodexHistoryCommand::Restore => restore_codex_history(),
    }
}

fn official_codex_sessions_need_migrate(settings: &crate::settings::AppSettings) -> bool {
    settings.unify_codex_session_history
        && !settings.unify_codex_migrate_existing.unwrap_or(false)
        && settings
            .local_migrations
            .as_ref()
            .and_then(|migrations| migrations.codex_official_history_unify_v1.as_ref())
            .is_none()
}

fn show_codex_history(json_output: bool) -> Result<(), AppError> {
    let settings = crate::settings::get_settings();
    let has_backup = crate::codex_history_migration::has_codex_official_history_unify_backup();
    let migration = settings
        .local_migrations
        .as_ref()
        .and_then(|migrations| migrations.codex_official_history_unify_v1.as_ref());
    let needs_migrate = official_codex_sessions_need_migrate(&settings);

    if json_output {
        let payload = json!({
            "enabled": settings.unify_codex_session_history,
            "migrateExistingRequested": settings.unify_codex_migrate_existing.unwrap_or(false),
            "hasBackup": has_backup,
            "migration": migration,
            "officialSessionsNeedMigrate": needs_migrate,
        });
        println!(
            "{}",
            to_json(&payload).map_err(|err| AppError::Message(err.to_string()))?
        );
        return Ok(());
    }

    println!("{}", highlight("Codex History"));
    println!(
        "Unified session history: {}",
        yes_no(settings.unify_codex_session_history)
    );
    println!(
        "Migrate existing requested: {}",
        yes_no(settings.unify_codex_migrate_existing.unwrap_or(false))
    );
    println!("Restore backup available: {}", yes_no(has_backup));
    if let Some(migration) = migration {
        println!(
            "Last migration: jsonl_files={}, state_rows={}",
            migration.migrated_jsonl_files, migration.migrated_state_rows
        );
    }
    if needs_migrate {
        println!(
            "{}",
            warning(
                "Official Codex sessions stay in the openai history bucket. Migrate them with: cc-switch settings codex-history migrate-existing"
            )
        );
    }
    Ok(())
}

fn set_codex_history_enabled(
    enabled: bool,
    migrate_existing: bool,
    restore: bool,
) -> Result<(), AppError> {
    let state = crate::store::AppState::try_new()?;
    let outcome = crate::services::codex_history::set_unified_session_history_enabled(
        &state,
        enabled,
        migrate_existing,
    )?;
    if !outcome.changed {
        println!(
            "{}",
            info(&format!(
                "Unified Codex session history already {}",
                if enabled { "enabled" } else { "disabled" }
            ))
        );
        return Ok(());
    }

    if enabled {
        if migrate_existing {
            let migration =
                crate::codex_history_migration::maybe_migrate_codex_official_history_to_unified_bucket(
                )?;
            print_codex_history_migration_outcome(&migration);
        }
        println!("{}", success("Unified Codex session history enabled"));
    } else {
        if restore {
            let restore =
                crate::codex_history_migration::restore_codex_official_history_from_backups()?;
            print_codex_history_restore_outcome(&restore);
        }
        println!("{}", success("Unified Codex session history disabled"));
    }

    Ok(())
}

fn migrate_codex_history_existing() -> Result<(), AppError> {
    let mut settings = crate::settings::get_settings();
    if !settings.unify_codex_session_history {
        return Err(AppError::InvalidInput(
            "Enable unified Codex session history before migrating existing sessions".to_string(),
        ));
    }
    settings.unify_codex_migrate_existing = Some(true);
    crate::settings::update_settings(settings)?;

    let state = crate::store::AppState::try_new()?;
    crate::services::provider::reapply_current_codex_official_live(&state)?;
    let outcome =
        crate::codex_history_migration::maybe_migrate_codex_official_history_to_unified_bucket()?;
    print_codex_history_migration_outcome(&outcome);
    Ok(())
}

fn restore_codex_history() -> Result<(), AppError> {
    let outcome = crate::codex_history_migration::restore_codex_official_history_from_backups()?;
    print_codex_history_restore_outcome(&outcome);
    Ok(())
}

fn print_codex_history_restore_outcome(
    outcome: &crate::codex_history_migration::CodexOfficialHistoryRestoreOutcome,
) {
    if let Some(reason) = &outcome.skipped_reason {
        println!(
            "{}",
            info(&format!("Codex official history restore skipped: {reason}"))
        );
    } else {
        println!(
            "{}",
            success(&format!(
                "Codex official history restored: jsonl_files={}, state_rows={}",
                outcome.restored_jsonl_files, outcome.restored_state_rows
            ))
        );
    }
}

fn print_codex_history_migration_outcome(
    outcome: &crate::codex_history_migration::CodexHistoryProviderBucketMigrationOutcome,
) {
    if let Some(reason) = outcome.skipped_reason.as_deref() {
        println!(
            "{}",
            info(&format!(
                "Codex official history migration skipped: {reason}"
            ))
        );
    } else {
        println!(
            "{}",
            success(&format!(
                "Codex official history migrated: jsonl_files={}, state_rows={}",
                outcome.migrated_jsonl_files, outcome.migrated_state_rows
            ))
        );
    }
}

fn print_visible_apps_summary() {
    let settings = crate::settings::get_settings();
    println!(
        "Visible apps: {}",
        enabled_app_labels(&settings.visible_apps).join(", ")
    );
}

fn enabled_app_labels(visible_apps: &crate::settings::VisibleApps) -> Vec<&'static str> {
    AppType::all()
        .filter(|app| visible_apps.is_enabled_for(app))
        .map(|app| app.as_str())
        .collect()
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        official_codex_sessions_need_migrate, set_codex_auth_preservation, OutboundProxyCommand,
    };
    use crate::settings::AppSettings;
    use crate::test_support::TestEnvGuard;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    struct SettingsTestGuard {
        _env: TestEnvGuard,
        _temp: TempDir,
    }

    #[test]
    fn official_codex_sessions_need_migrate_when_unify_is_on_without_opt_in() {
        let mut settings = AppSettings::default();
        assert!(official_codex_sessions_need_migrate(&settings));

        settings.unify_codex_migrate_existing = Some(true);
        assert!(!official_codex_sessions_need_migrate(&settings));

        settings.unify_codex_migrate_existing = None;
        settings.unify_codex_session_history = false;
        assert!(!official_codex_sessions_need_migrate(&settings));
    }

    #[test]
    fn outbound_proxy_command_debug_redacts_credentials() {
        let command = OutboundProxyCommand::Set {
            url: "http://embedded:secret@127.0.0.1:7890".to_string(),
            username: Some("alice".to_string()),
            password: Some("other-secret".to_string()),
        };

        let debug = format!("{command:?}");
        assert!(!debug.contains("embedded"), "{debug}");
        assert!(!debug.contains("alice"), "{debug}");
        assert!(!debug.contains("secret"), "{debug}");
    }

    impl SettingsTestGuard {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("create temp dir");
            let env = TestEnvGuard::isolated(temp.path());
            Self {
                _env: env,
                _temp: temp,
            }
        }
    }

    #[test]
    #[serial(home_settings)]
    fn codex_auth_preservation_updates_only_the_setting() {
        let _guard = SettingsTestGuard::new();
        let mut settings = crate::settings::get_settings();
        settings.skip_claude_onboarding = true;
        crate::settings::update_settings(settings).expect("seed unrelated setting");

        let codex_dir = crate::codex_config::get_codex_config_dir();
        fs::create_dir_all(&codex_dir).expect("create Codex config dir");
        let auth_path = crate::codex_config::get_codex_auth_path();
        let config_path = crate::codex_config::get_codex_config_path();
        fs::write(&auth_path, b"{\n  \"auth_mode\": \"chatgpt\"\n}\n").expect("seed auth.json");
        fs::write(&config_path, b"model_provider = \"custom\"\n").expect("seed config.toml");
        let auth_before = fs::read(&auth_path).expect("read auth.json before setting change");
        let config_before = fs::read(&config_path).expect("read config.toml before setting change");

        set_codex_auth_preservation(true).expect("enable preservation");

        let settings = crate::settings::get_settings();
        assert!(settings.preserve_codex_official_auth_on_switch);
        assert!(settings.skip_claude_onboarding);
        assert_eq!(
            fs::read(&auth_path).expect("read auth.json after enabling"),
            auth_before
        );
        assert_eq!(
            fs::read(&config_path).expect("read config.toml after enabling"),
            config_before
        );

        set_codex_auth_preservation(false).expect("disable preservation");

        let settings = crate::settings::get_settings();
        assert!(!settings.preserve_codex_official_auth_on_switch);
        assert!(settings.skip_claude_onboarding);
        assert_eq!(
            fs::read(&auth_path).expect("read auth.json after disabling"),
            auth_before
        );
        assert_eq!(
            fs::read(&config_path).expect("read config.toml after disabling"),
            config_before
        );
    }
}
