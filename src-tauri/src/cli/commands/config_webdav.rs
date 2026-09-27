use clap::Subcommand;

use crate::cli::ui::{highlight, info, mask_secret_for_display, success, warning};
use crate::error::AppError;
use crate::{
    get_webdav_sync_settings, set_webdav_sync_settings, webdav_jianguoyun_preset,
    WebDavSyncService, WebDavSyncSettings,
};

#[derive(Subcommand, Debug, Clone)]
pub enum WebDavCommand {
    /// Show current WebDAV sync settings
    Show {
        /// Print stored secrets in full instead of masking them
        #[arg(long)]
        reveal: bool,
    },

    /// Create or update WebDAV sync settings
    Set {
        #[arg(long)]
        base_url: Option<String>,

        #[arg(long)]
        remote_root: Option<String>,

        #[arg(long)]
        profile: Option<String>,

        #[arg(long)]
        username: Option<String>,

        #[arg(long)]
        password: Option<String>,

        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        #[arg(long, conflicts_with = "enable")]
        disable: bool,

        #[arg(long, conflicts_with = "no_auto_sync")]
        auto_sync: bool,

        #[arg(long, conflicts_with = "auto_sync")]
        no_auto_sync: bool,
    },

    /// Clear stored WebDAV sync settings
    Clear,

    /// Apply Jianguoyun preset settings
    Jianguoyun {
        #[arg(long)]
        username: String,

        #[arg(long)]
        password: String,

        #[arg(long)]
        remote_root: Option<String>,

        #[arg(long)]
        profile: Option<String>,

        #[arg(long, conflicts_with = "no_auto_sync")]
        auto_sync: bool,

        #[arg(long, conflicts_with = "auto_sync")]
        no_auto_sync: bool,
    },

    /// Check whether the current WebDAV settings can connect successfully
    CheckConnection,

    /// Upload the current local snapshot to WebDAV
    Upload,

    /// Download the current remote snapshot from WebDAV
    Download,

    /// Migrate legacy V1 remote data to V2 protocol
    MigrateV1ToV2,
}

pub fn execute(cmd: WebDavCommand) -> Result<(), AppError> {
    match cmd {
        WebDavCommand::Show { reveal } => show(reveal),
        WebDavCommand::Set {
            base_url,
            remote_root,
            profile,
            username,
            password,
            enable,
            disable,
            auto_sync,
            no_auto_sync,
        } => set(
            base_url,
            remote_root,
            profile,
            username,
            password,
            enable,
            disable,
            auto_sync,
            no_auto_sync,
        ),
        WebDavCommand::Clear => clear(),
        WebDavCommand::Jianguoyun {
            username,
            password,
            remote_root,
            profile,
            auto_sync,
            no_auto_sync,
        } => jianguoyun(
            username,
            password,
            remote_root,
            profile,
            auto_sync,
            no_auto_sync,
        ),
        WebDavCommand::CheckConnection => check_connection(),
        WebDavCommand::Upload => upload(),
        WebDavCommand::Download => download(),
        WebDavCommand::MigrateV1ToV2 => migrate_v1_to_v2(),
    }
}

fn show(reveal: bool) -> Result<(), AppError> {
    print!("{}", format_show(reveal)?);
    Ok(())
}

fn format_show(reveal: bool) -> Result<String, AppError> {
    let Some(settings) = get_webdav_sync_settings() else {
        return Ok(format!(
            "{}\n",
            info(crate::t!(
                "WebDAV sync is not configured.",
                "WebDAV 同步尚未配置。"
            ))
        ));
    };

    Ok(format!(
        "{}\n{}\nEnabled:      {}\nBase URL:     {}\nRemote Root:  {}\nProfile:      {}\nUsername:     {}\nPassword:     {}\nAuto Sync:    {}\nLast Sync:    {}\nLast Error:   {}\n",
        highlight(crate::t!("WebDAV Sync", "WebDAV 同步")),
        "═".repeat(60),
        yes_no(settings.enabled),
        settings.base_url,
        settings.remote_root,
        settings.profile,
        blank_as_na(&settings.username),
        secret_for_show(&settings.password, reveal),
        yes_no(settings.auto_sync),
        settings
            .status
            .last_sync_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        settings
            .status
            .last_error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "N/A".to_string()),
    ))
}

#[allow(clippy::too_many_arguments)]
fn set(
    base_url: Option<String>,
    remote_root: Option<String>,
    profile: Option<String>,
    username: Option<String>,
    password: Option<String>,
    enable: bool,
    disable: bool,
    auto_sync: bool,
    no_auto_sync: bool,
) -> Result<(), AppError> {
    let mut settings = merged_settings(
        get_webdav_sync_settings(),
        base_url,
        remote_root,
        profile,
        username,
        password,
        enable,
        disable,
        auto_sync,
        no_auto_sync,
    );
    settings.normalize();
    set_webdav_sync_settings(Some(settings))?;
    println!(
        "{}",
        success(crate::t!(
            "✓ WebDAV settings saved.",
            "✓ WebDAV 设置已保存。"
        ))
    );
    Ok(())
}

fn clear() -> Result<(), AppError> {
    set_webdav_sync_settings(None)?;
    println!(
        "{}",
        success(crate::t!(
            "✓ WebDAV settings cleared.",
            "✓ WebDAV 设置已清空。"
        ))
    );
    Ok(())
}

fn jianguoyun(
    username: String,
    password: String,
    remote_root: Option<String>,
    profile: Option<String>,
    auto_sync: bool,
    no_auto_sync: bool,
) -> Result<(), AppError> {
    let mut settings = webdav_jianguoyun_preset(&username, &password);
    if let Some(remote_root) = remote_root {
        settings.remote_root = remote_root;
    }
    if let Some(profile) = profile {
        settings.profile = profile;
    }
    if auto_sync {
        settings.auto_sync = true;
    }
    if no_auto_sync {
        settings.auto_sync = false;
    }
    settings.normalize();
    set_webdav_sync_settings(Some(settings))?;
    WebDavSyncService::check_connection()?;
    println!(
        "{}",
        success(crate::t!(
            "✓ Jianguoyun WebDAV preset applied.",
            "✓ 已应用坚果云 WebDAV 预设。"
        ))
    );
    Ok(())
}

fn check_connection() -> Result<(), AppError> {
    WebDavSyncService::check_connection()?;
    println!(
        "{}",
        success(crate::t!(
            "✓ WebDAV connection succeeded.",
            "✓ WebDAV 连接成功。"
        ))
    );
    Ok(())
}

fn upload() -> Result<(), AppError> {
    let summary = WebDavSyncService::upload()?;
    println!("{}", success(&summary.message));
    Ok(())
}

fn download() -> Result<(), AppError> {
    let (summary, post_sync_warning) = WebDavSyncService::download_with_warning()?;
    if let Some(error) = &post_sync_warning {
        let en = format!("Live config sync after WebDAV operation failed: {error}");
        let zh = format!("WebDAV 操作后同步 live 配置失败: {error}");
        println!("{}", warning(crate::t!(&en, &zh)));
    }
    println!("{}", success(&summary.message));
    Ok(())
}

fn migrate_v1_to_v2() -> Result<(), AppError> {
    let (summary, post_sync_warning) = WebDavSyncService::migrate_v1_to_v2_with_warning()?;
    if let Some(error) = &post_sync_warning {
        let en = format!("Live config sync after WebDAV operation failed: {error}");
        let zh = format!("WebDAV 操作后同步 live 配置失败: {error}");
        println!("{}", warning(crate::t!(&en, &zh)));
    }
    println!("{}", success(&summary.message));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn merged_settings(
    current: Option<WebDavSyncSettings>,
    base_url: Option<String>,
    remote_root: Option<String>,
    profile: Option<String>,
    username: Option<String>,
    password: Option<String>,
    enable: bool,
    disable: bool,
    auto_sync: bool,
    no_auto_sync: bool,
) -> WebDavSyncSettings {
    let mut settings = current.unwrap_or_default();

    if let Some(base_url) = base_url {
        settings.base_url = base_url;
    }
    if let Some(remote_root) = remote_root {
        settings.remote_root = remote_root;
    }
    if let Some(profile) = profile {
        settings.profile = profile;
    }
    if let Some(username) = username {
        settings.username = username;
    }
    if let Some(password) = password {
        settings.password = password;
    }
    if enable {
        settings.enabled = true;
    }
    if disable {
        settings.enabled = false;
    }
    if auto_sync {
        settings.auto_sync = true;
    }
    if no_auto_sync {
        settings.auto_sync = false;
    }

    settings
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn blank_as_na(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "N/A"
    } else {
        trimmed
    }
}

fn secret_for_show(value: &str, reveal: bool) -> String {
    if blank_as_na(value) == "N/A" {
        "N/A".to_string()
    } else if reveal {
        value.trim().to_string()
    } else {
        mask_secret_for_display(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{format_show, merged_settings};
    use crate::cli::ui::mask_secret_for_display;
    use crate::test_support::TestEnvGuard;
    use crate::{set_webdav_sync_settings, WebDavSyncSettings, WebDavSyncStatus};
    use serial_test::serial;

    const WEBDAV_PASSWORD: &str = "super-secret-webdav-password-9999";

    #[test]
    fn merged_settings_updates_selected_fields_only() {
        let current = WebDavSyncSettings {
            enabled: true,
            base_url: "https://dav.example.com/root".to_string(),
            remote_root: "sync-root".to_string(),
            profile: "default".to_string(),
            username: "demo".to_string(),
            password: "secret".to_string(),
            auto_sync: false,
            status: WebDavSyncStatus {
                last_error: Some("boom".to_string()),
                ..WebDavSyncStatus::default()
            },
        };

        let merged = merged_settings(
            Some(current),
            None,
            Some("next-root".to_string()),
            None,
            None,
            None,
            false,
            false,
            true,
            false,
        );

        assert!(merged.enabled);
        assert_eq!(merged.base_url, "https://dav.example.com/root");
        assert_eq!(merged.remote_root, "next-root");
        assert_eq!(merged.profile, "default");
        assert_eq!(merged.username, "demo");
        assert_eq!(merged.password, "secret");
        assert!(merged.auto_sync);
        assert_eq!(merged.status.last_error.as_deref(), Some("boom"));
    }

    fn seed_webdav(password: &str) {
        set_webdav_sync_settings(Some(WebDavSyncSettings {
            enabled: true,
            base_url: "https://dav.example.com/root".to_string(),
            remote_root: "sync-root".to_string(),
            profile: "default".to_string(),
            username: "demo".to_string(),
            password: password.to_string(),
            auto_sync: false,
            status: WebDavSyncStatus::default(),
        }))
        .expect("save webdav settings");
    }

    #[test]
    #[serial(home_settings)]
    fn show_masks_password_by_default() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let _env = TestEnvGuard::isolated(temp.path());
        seed_webdav(WEBDAV_PASSWORD);

        let out = format_show(false).expect("format webdav show");
        let masked = mask_secret_for_display(WEBDAV_PASSWORD);
        assert!(
            !out.contains(WEBDAV_PASSWORD),
            "default show must not print the raw password: {out}"
        );
        assert!(
            out.contains(&masked),
            "default show should print the masked password {masked}: {out}"
        );
        assert!(out.contains("Password:"));
    }

    #[test]
    #[serial(home_settings)]
    fn show_reveal_prints_full_password() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let _env = TestEnvGuard::isolated(temp.path());
        seed_webdav(WEBDAV_PASSWORD);

        let out = format_show(true).expect("format webdav show --reveal");
        assert!(
            out.contains(WEBDAV_PASSWORD),
            "--reveal should print the stored password: {out}"
        );
    }

    #[test]
    #[serial(home_settings)]
    fn show_prints_na_for_blank_password() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let _env = TestEnvGuard::isolated(temp.path());
        seed_webdav("");

        let out = format_show(false).expect("format webdav show");
        assert!(
            out.contains("Password:     N/A"),
            "blank password should display N/A: {out}"
        );
        let revealed = format_show(true).expect("format webdav show --reveal");
        assert!(
            revealed.contains("Password:     N/A"),
            "blank password stays N/A even with --reveal: {revealed}"
        );
    }
}
