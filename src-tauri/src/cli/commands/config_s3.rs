use clap::Subcommand;

use crate::cli::ui::{highlight, info, mask_secret_for_display, success, warning};
use crate::error::AppError;
use crate::services::S3SyncService;
use crate::settings::{get_s3_sync_settings, set_s3_sync_settings, S3SyncSettings};

#[derive(Subcommand, Debug, Clone)]
pub enum S3Command {
    /// Show current S3 sync settings
    Show {
        /// Print stored secrets in full instead of masking them
        #[arg(long)]
        reveal: bool,
    },

    /// Create or update S3 sync settings
    Set {
        #[arg(long)]
        region: Option<String>,

        #[arg(long)]
        bucket: Option<String>,

        #[arg(long)]
        access_key_id: Option<String>,

        #[arg(long)]
        secret_access_key: Option<String>,

        #[arg(long)]
        endpoint: Option<String>,

        #[arg(long)]
        remote_root: Option<String>,

        #[arg(long)]
        profile: Option<String>,

        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        #[arg(long, conflicts_with = "enable")]
        disable: bool,
    },

    /// Clear stored S3 sync settings
    Clear,

    /// Check whether the current S3 settings can connect successfully
    CheckConnection,

    /// Upload the current local snapshot to S3
    Upload,

    /// Download the current remote snapshot from S3
    Download,
}

pub fn execute(command: S3Command) -> Result<(), AppError> {
    match command {
        S3Command::Show { reveal } => show(reveal),
        S3Command::Set {
            region,
            bucket,
            access_key_id,
            secret_access_key,
            endpoint,
            remote_root,
            profile,
            enable,
            disable,
        } => set(
            region,
            bucket,
            access_key_id,
            secret_access_key,
            endpoint,
            remote_root,
            profile,
            enable,
            disable,
        ),
        S3Command::Clear => clear(),
        S3Command::CheckConnection => check_connection(),
        S3Command::Upload => upload(),
        S3Command::Download => download(),
    }
}

fn show(reveal: bool) -> Result<(), AppError> {
    print!("{}", format_show(reveal)?);
    Ok(())
}

fn format_show(reveal: bool) -> Result<String, AppError> {
    let Some(settings) = get_s3_sync_settings() else {
        return Ok(format!(
            "{}\n",
            info(crate::t!("S3 sync is not configured.", "S3 同步尚未配置。"))
        ));
    };

    Ok(format!(
        "{}\n{}\nEnabled:           {}\nRegion:            {}\nBucket:            {}\nAccess Key ID:     {}\nSecret Access Key: {}\nEndpoint:          {}\nRemote Root:       {}\nProfile:           {}\nLast Sync:         {}\nLast Error:        {}\n",
        highlight(crate::t!("S3 Compatible Sync", "S3 兼容同步")),
        "═".repeat(60),
        yes_no(settings.enabled),
        blank_as_na(&settings.region),
        blank_as_na(&settings.bucket),
        secret_for_show(&settings.access_key_id, reveal),
        secret_for_show(&settings.secret_access_key, reveal),
        blank_as_aws(&settings.endpoint),
        settings.remote_root,
        settings.profile,
        settings
            .status
            .last_sync_at
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        settings
            .status
            .last_error
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("N/A"),
    ))
}

#[allow(clippy::too_many_arguments)]
fn set(
    region: Option<String>,
    bucket: Option<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    endpoint: Option<String>,
    remote_root: Option<String>,
    profile: Option<String>,
    enable: bool,
    disable: bool,
) -> Result<(), AppError> {
    let settings = merged_settings(
        get_s3_sync_settings(),
        region,
        bucket,
        access_key_id,
        secret_access_key,
        endpoint,
        remote_root,
        profile,
        enable,
        disable,
    );
    set_s3_sync_settings(Some(settings))?;
    println!(
        "{}",
        success(crate::t!("✓ S3 settings saved.", "✓ S3 设置已保存。"))
    );
    Ok(())
}

fn clear() -> Result<(), AppError> {
    set_s3_sync_settings(None)?;
    println!(
        "{}",
        success(crate::t!("✓ S3 settings cleared.", "✓ S3 设置已清空。"))
    );
    Ok(())
}

fn check_connection() -> Result<(), AppError> {
    S3SyncService::check_connection()?;
    println!(
        "{}",
        success(crate::t!("✓ S3 connection succeeded.", "✓ S3 连接成功。"))
    );
    Ok(())
}

fn upload() -> Result<(), AppError> {
    let summary = S3SyncService::upload()?;
    println!("{}", success(&summary.message));
    Ok(())
}

fn download() -> Result<(), AppError> {
    let (summary, post_sync_warning) = S3SyncService::download_with_warning()?;
    if let Some(error) = &post_sync_warning {
        let en = format!("Live config sync after S3 restore failed: {error}");
        let zh = format!("S3 恢复后同步 live 配置失败: {error}");
        println!("{}", warning(crate::t!(&en, &zh)));
    }
    println!("{}", success(&summary.message));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn merged_settings(
    current: Option<S3SyncSettings>,
    region: Option<String>,
    bucket: Option<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    endpoint: Option<String>,
    remote_root: Option<String>,
    profile: Option<String>,
    enable: bool,
    disable: bool,
) -> S3SyncSettings {
    let mut settings = current.unwrap_or_default();
    if let Some(value) = region {
        settings.region = value;
    }
    if let Some(value) = bucket {
        settings.bucket = value;
    }
    if let Some(value) = access_key_id {
        settings.access_key_id = value;
    }
    if let Some(value) = secret_access_key {
        settings.secret_access_key = value;
    }
    if let Some(value) = endpoint {
        settings.endpoint = value;
    }
    if let Some(value) = remote_root {
        settings.remote_root = value;
    }
    if let Some(value) = profile {
        settings.profile = value;
    }
    if enable {
        settings.enabled = true;
    }
    if disable {
        settings.enabled = false;
    }
    // A CLI process is not resident, so this release must not persist a toggle
    // that has no worker behind it.
    settings.auto_sync = false;
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

fn blank_as_aws(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "AWS default"
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::{format_show, merged_settings};
    use crate::cli::ui::mask_secret_for_display;
    use crate::settings::{set_s3_sync_settings, S3SyncSettings, WebDavSyncStatus};
    use crate::test_support::TestEnvGuard;
    use serial_test::serial;

    const ACCESS_KEY_ID: &str = "AKIAEXAMPLEKEYID99";
    const SECRET_ACCESS_KEY: &str = "wJalrXUtnFEMI/K7MDENG";

    #[test]
    fn merged_settings_updates_only_supplied_fields_and_preserves_status() {
        let current = S3SyncSettings {
            enabled: false,
            auto_sync: true,
            region: "us-east-1".to_string(),
            bucket: "old-bucket".to_string(),
            access_key_id: "AKID".to_string(),
            secret_access_key: "SECRET".to_string(),
            endpoint: String::new(),
            remote_root: "sync-root".to_string(),
            profile: "default".to_string(),
            status: WebDavSyncStatus {
                last_error: Some("old error".to_string()),
                ..WebDavSyncStatus::default()
            },
        };

        let merged = merged_settings(
            Some(current),
            None,
            Some("new-bucket".to_string()),
            None,
            None,
            Some("https://s3.example.com".to_string()),
            None,
            None,
            true,
            false,
        );

        assert!(merged.enabled);
        assert!(!merged.auto_sync);
        assert_eq!(merged.region, "us-east-1");
        assert_eq!(merged.bucket, "new-bucket");
        assert_eq!(merged.secret_access_key, "SECRET");
        assert_eq!(merged.endpoint, "https://s3.example.com");
        assert_eq!(merged.status.last_error.as_deref(), Some("old error"));
    }

    fn seed_s3() {
        set_s3_sync_settings(Some(S3SyncSettings {
            enabled: true,
            auto_sync: false,
            region: "us-east-1".to_string(),
            bucket: "demo-bucket".to_string(),
            access_key_id: ACCESS_KEY_ID.to_string(),
            secret_access_key: SECRET_ACCESS_KEY.to_string(),
            endpoint: String::new(),
            remote_root: "sync-root".to_string(),
            profile: "default".to_string(),
            status: WebDavSyncStatus::default(),
        }))
        .expect("save s3 settings");
    }

    #[test]
    #[serial(home_settings)]
    fn show_masks_access_keys_by_default() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let _env = TestEnvGuard::isolated(temp.path());
        seed_s3();

        let out = format_show(false).expect("format s3 show");
        let masked_id = mask_secret_for_display(ACCESS_KEY_ID);
        let masked_secret = mask_secret_for_display(SECRET_ACCESS_KEY);
        assert!(
            !out.contains(ACCESS_KEY_ID),
            "default show must not print the raw access key id: {out}"
        );
        assert!(
            !out.contains(SECRET_ACCESS_KEY),
            "default show must not print the raw secret access key: {out}"
        );
        assert!(
            out.contains(&masked_id),
            "default show should print the masked access key id {masked_id}: {out}"
        );
        assert!(
            out.contains(&masked_secret),
            "default show should print the masked secret access key {masked_secret}: {out}"
        );
    }

    #[test]
    #[serial(home_settings)]
    fn show_reveal_prints_full_access_keys() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let _env = TestEnvGuard::isolated(temp.path());
        seed_s3();

        let out = format_show(true).expect("format s3 show --reveal");
        assert!(
            out.contains(ACCESS_KEY_ID),
            "--reveal should print the stored access key id: {out}"
        );
        assert!(
            out.contains(SECRET_ACCESS_KEY),
            "--reveal should print the stored secret access key: {out}"
        );
    }
}
