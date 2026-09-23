use crate::error::AppError;
use crate::settings::{self, VisibleApps, VisibleAppsMode};

/// Pin the visible-app set to the four supported harnesses, every launch.
///
/// This build ships exactly Claude, Codex, Hermes and Pi. Auto detection is
/// gone on purpose: it used to widen the tab bar from whatever CLI happened to
/// sit on `PATH`, which resurrected Gemini/OpenCode/OpenClaw tabs on machines
/// that still had them installed. It also never *narrowed* anything useful,
/// because a machine without one of the four must keep that tab.
///
/// Settings written by an older build (auto mode, a pending first-run prompt,
/// per-app detection snapshots) are normalized here so nothing outside the
/// supported set can reach the tab bar again.
pub fn apply_startup_policy() -> Result<VisibleApps, AppError> {
    let visible_apps = settings::default_visible_apps();
    let mut app_settings = settings::get_settings();
    app_settings.visible_apps = visible_apps.clone();
    app_settings.visible_apps_settings.mode = VisibleAppsMode::Manual;
    app_settings.visible_apps_settings.auto_prompt_decided = true;
    app_settings
        .visible_apps_settings
        .manual_hidden_installed_notices
        .clear();
    settings::update_settings(app_settings)?;

    Ok(visible_apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::AppType;
    use crate::settings::{VisibleAppsMode, VisibleAppsSettings};
    use crate::test_support::{
        lock_test_home_and_settings, set_test_home_override, TestHomeSettingsLock,
    };
    use serial_test::serial;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct EnvGuard {
        _lock: TestHomeSettingsLock,
        old_home: Option<OsString>,
        old_test_home_override: Option<PathBuf>,
        old_userprofile: Option<OsString>,
        old_config_dir: Option<OsString>,
    }

    impl EnvGuard {
        fn set_home(home: &Path) -> Self {
            let lock = lock_test_home_and_settings();
            let old_home = std::env::var_os("HOME");
            let old_test_home_override = crate::test_support::test_home_override();
            let old_userprofile = std::env::var_os("USERPROFILE");
            let old_config_dir = std::env::var_os("CC_SWITCH_CONFIG_DIR");
            std::env::set_var("HOME", home);
            std::env::set_var("USERPROFILE", home);
            std::env::set_var("CC_SWITCH_CONFIG_DIR", home.join(".cc-switch"));
            set_test_home_override(Some(home));
            crate::settings::reload_test_settings();
            Self {
                _lock: lock,
                old_home,
                old_test_home_override,
                old_userprofile,
                old_config_dir,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.old_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
            match &self.old_userprofile {
                Some(value) => std::env::set_var("USERPROFILE", value),
                None => std::env::remove_var("USERPROFILE"),
            }
            match &self.old_config_dir {
                Some(value) => std::env::set_var("CC_SWITCH_CONFIG_DIR", value),
                None => std::env::remove_var("CC_SWITCH_CONFIG_DIR"),
            }
            set_test_home_override(self.old_test_home_override.as_deref());
            crate::settings::reload_test_settings();
        }
    }

    fn visible_apps_with(claude: bool, codex: bool, hermes: bool, pi: bool) -> VisibleApps {
        VisibleApps {
            claude,
            codex,
            gemini: true,
            opencode: true,
            hermes,
            openclaw: true,
            pi,
        }
    }

    #[test]
    #[serial(home_settings)]
    fn startup_policy_pins_the_four_supported_harnesses() {
        let temp_home = TempDir::new().expect("create temp home");
        let _env = EnvGuard::set_home(temp_home.path());
        let mut settings = crate::settings::get_settings();
        // A stale file from an older build: only Pi visible, removed harnesses on.
        settings.visible_apps = visible_apps_with(false, false, false, true);
        crate::settings::update_settings(settings).expect("save settings");

        let visible_apps = apply_startup_policy().expect("apply policy");

        assert_eq!(visible_apps, settings::default_visible_apps());
        assert_eq!(
            visible_apps.ordered_enabled(),
            vec![
                AppType::Claude,
                AppType::Codex,
                AppType::Hermes,
                AppType::Pi
            ],
            "only the four supported harnesses may ever be visible"
        );
        assert_eq!(
            crate::settings::get_visible_apps(),
            settings::default_visible_apps()
        );
    }

    #[test]
    #[serial(home_settings)]
    fn startup_policy_clears_stale_auto_detection_state() {
        let temp_home = TempDir::new().expect("create temp home");
        let _env = EnvGuard::set_home(temp_home.path());
        let mut settings = crate::settings::get_settings();
        settings.visible_apps_settings = VisibleAppsSettings {
            mode: VisibleAppsMode::Auto,
            auto_prompt_decided: false,
            manual_hidden_installed_notices: [("pi".to_string(), true)].into_iter().collect(),
            last_detected_installed: [("gemini".to_string(), true)].into_iter().collect(),
        };
        crate::settings::update_settings(settings).expect("save settings");

        apply_startup_policy().expect("apply policy");

        let persisted = crate::settings::get_visible_apps_settings();
        assert_eq!(persisted.mode, VisibleAppsMode::Manual);
        assert!(
            persisted.auto_prompt_decided,
            "no first-run auto-detect prompt may stay pending"
        );
        assert!(
            persisted.manual_hidden_installed_notices.is_empty(),
            "hidden-installed notices belong to the removed auto-detect flow"
        );
    }

    #[test]
    #[serial(home_settings)]
    fn startup_policy_is_idempotent_across_launches() {
        let temp_home = TempDir::new().expect("create temp home");
        let _env = EnvGuard::set_home(temp_home.path());

        let first = apply_startup_policy().expect("first launch");
        let second = apply_startup_policy().expect("second launch");

        assert_eq!(first, second);
        assert_eq!(second, settings::default_visible_apps());
    }
}
