use crate::app_config::AppType;
use crate::cli::failover_policy::{
    ensure_auto_failover_queue_ready, ensure_auto_failover_ready, inspect_auto_failover_gate,
};
use crate::cli::i18n::texts;
use crate::error::AppError;

use super::super::app::ToastKind;
use super::super::data::{load_state, UiData};
use super::super::runtime_systems::ManagedAuthReq;
use super::RuntimeActionContext;

pub(super) fn set_global_outbound_proxy(
    ctx: &mut RuntimeActionContext<'_>,
    config: crate::services::GlobalOutboundProxyConfig,
) -> Result<(), AppError> {
    let state = load_state()?;
    let full_url = config.to_full_url()?;
    let daemon_warning = if full_url.is_empty() {
        crate::services::global_proxy::clear(&state)?
    } else {
        crate::services::global_proxy::set(&state, &config)?
    };
    let persisted = (!full_url.is_empty())
        .then(|| crate::services::GlobalOutboundProxyConfig::from_full_url(&full_url))
        .transpose()?;
    ctx.data.config.global_outbound_proxy = persisted.clone();
    ctx.app.global_outbound_proxy_draft = Some(persisted.unwrap_or_default());
    ctx.data.mark_current_app_data_changed();
    ctx.app.push_toast(
        if full_url.is_empty() {
            crate::t!("Global outbound proxy cleared.", "全局出站代理已清除。")
        } else {
            crate::t!("Global outbound proxy saved.", "全局出站代理已保存。")
        },
        ToastKind::Success,
    );
    if let Some(message) = daemon_warning {
        ctx.app.push_toast(message, ToastKind::Warning);
    }
    Ok(())
}

pub(super) fn set_proxy_enabled(
    ctx: &mut RuntimeActionContext<'_>,
    enabled: bool,
) -> Result<(), AppError> {
    let state = load_state()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Message(format!("failed to create async runtime: {e}")))?;
    runtime
        .block_on(
            state
                .proxy_service
                .set_managed_session_for_app(ctx.app.app_type.as_str(), enabled),
        )
        .map_err(AppError::Message)?;

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        if enabled {
            crate::t!("Local proxy enabled.", "本地代理已开启。")
        } else {
            crate::t!("Local proxy disabled.", "本地代理已关闭。")
        },
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

pub(super) fn set_proxy_listen_address(
    ctx: &mut RuntimeActionContext<'_>,
    address: String,
) -> Result<(), AppError> {
    update_proxy_config(ctx, |config| {
        config.listen_address = address;
    })
}

pub(super) fn set_proxy_listen_port(
    ctx: &mut RuntimeActionContext<'_>,
    port: u16,
) -> Result<(), AppError> {
    let state = load_state()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Message(format!("failed to create async runtime: {e}")))?;
    let status = runtime.block_on(state.proxy_service.get_status());
    let app_running = status
        .active_workers
        .iter()
        .any(|worker| worker.app_type == ctx.app.app_type.as_str());
    if app_running {
        *ctx.data = UiData::load(&ctx.app.app_type)?;
        ctx.app.push_toast(
            texts::tui_toast_proxy_settings_stop_app_route_before_edit_port(),
            super::super::app::ToastKind::Info,
        );
        return Ok(());
    }

    state
        .db
        .set_app_proxy_preferred_port(ctx.app.app_type.as_str(), port)?;

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        texts::tui_toast_proxy_settings_saved(),
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

pub(super) fn set_proxy_auto_failover(
    ctx: &mut RuntimeActionContext<'_>,
    app_type: AppType,
    enabled: bool,
) -> Result<(), AppError> {
    let state = load_state()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Message(format!("failed to create async runtime: {e}")))?;

    if enabled {
        let gate = runtime.block_on(inspect_auto_failover_gate(&state, &app_type))?;
        ensure_auto_failover_ready(&gate)?;
    }

    if enabled {
        runtime
            .block_on(
                state
                    .proxy_service
                    .enable_auto_failover_for_app(app_type.as_str()),
            )
            .map_err(AppError::Message)?;
    } else {
        runtime
            .block_on(
                state
                    .proxy_service
                    .set_auto_failover_for_app(app_type.as_str(), false),
            )
            .map_err(AppError::Message)?;
    }

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        if enabled {
            crate::t!("Automatic failover enabled.", "自动故障转移已开启。")
        } else {
            crate::t!("Automatic failover disabled.", "自动故障转移已关闭。")
        },
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

pub(super) fn enable_proxy_and_auto_failover(
    ctx: &mut RuntimeActionContext<'_>,
    app_type: AppType,
) -> Result<(), AppError> {
    let state = load_state()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Message(format!("failed to create async runtime: {e}")))?;
    let gate = runtime.block_on(inspect_auto_failover_gate(&state, &app_type))?;
    ensure_auto_failover_queue_ready(&gate)?;

    runtime
        .block_on(async {
            state
                .proxy_service
                .enable_proxy_and_auto_failover_for_app(app_type.as_str())
                .await
        })
        .map_err(AppError::Message)?;

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        crate::t!(
            "Proxy routing and automatic failover enabled.",
            "代理路由和自动故障转移已开启。"
        ),
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

pub(super) fn set_pi_config_dir(
    ctx: &mut RuntimeActionContext<'_>,
    path: Option<String>,
) -> Result<(), AppError> {
    let path = validate_pi_config_dir(path)?;
    let mut settings = crate::settings::get_settings();
    settings.pi_config_dir = path;
    crate::settings::update_settings(settings)?;

    let state = load_state()?;
    let import_result =
        crate::services::ProviderService::import_pi_providers_from_live(&state).err();

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        texts::tui_toast_pi_config_dir_saved(),
        super::super::app::ToastKind::Success,
    );
    if let Some(err) = import_result {
        ctx.app.push_toast(
            texts::tui_toast_pi_config_dir_import_failed(&err.to_string()),
            super::super::app::ToastKind::Warning,
        );
    }

    Ok(())
}

fn validate_pi_config_dir(path: Option<String>) -> Result<Option<String>, AppError> {
    let path = path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(raw) = path.as_deref() {
        let resolved = crate::settings::resolve_override_path(raw);
        if !resolved.is_absolute() {
            return Err(AppError::InvalidInput(format!(
                "Pi config directory must resolve to an absolute path: {}",
                resolved.display()
            )));
        }
    }
    Ok(path)
}

#[cfg(test)]
mod pi_config_dir_tests {
    use super::validate_pi_config_dir;

    #[test]
    fn pi_config_dir_is_validated_before_persistence() {
        assert!(validate_pi_config_dir(Some("relative/pi".to_string())).is_err());
        let absolute_pi_dir = std::env::temp_dir().join("pi-agent");
        assert_eq!(
            validate_pi_config_dir(Some(format!(" {} ", absolute_pi_dir.display()))).unwrap(),
            Some(absolute_pi_dir.to_string_lossy().into_owned())
        );
        assert_eq!(
            validate_pi_config_dir(Some("   ".to_string())).unwrap(),
            None
        );
    }
}

pub(super) fn set_preferred_editor(
    ctx: &mut RuntimeActionContext<'_>,
    command: Option<String>,
) -> Result<(), AppError> {
    if let Some(command) = command.as_deref() {
        crate::cli::editor::validate_preferred_editor_command(command)?;
    }

    crate::settings::set_preferred_editor(command)?;
    ctx.app.push_toast(
        texts::tui_toast_preferred_editor_saved(),
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

pub(super) fn set_preserve_codex_official_auth(
    ctx: &mut RuntimeActionContext<'_>,
    enabled: bool,
) -> Result<(), AppError> {
    crate::settings::set_preserve_codex_official_auth_on_switch(enabled)?;
    ctx.app.push_toast(
        texts::tui_toast_codex_official_auth_preservation_toggled(enabled),
        ToastKind::Success,
    );
    Ok(())
}

pub(super) fn managed_auth_refresh(
    ctx: &mut RuntimeActionContext<'_>,
    auth_provider: String,
) -> Result<(), AppError> {
    let Some(tx) = ctx.managed_auth_req_tx else {
        ctx.app.managed_auth_loading = false;
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_worker_unavailable(
                texts::tui_error_managed_auth_worker_unavailable(),
            ),
            ToastKind::Warning,
        );
        return Ok(());
    };

    ctx.app.managed_auth_loading = true;
    if let Err(err) = tx.send(ManagedAuthReq::Refresh { auth_provider }) {
        ctx.app.managed_auth_loading = false;
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_request_failed(&err.to_string()),
            ToastKind::Warning,
        );
    }

    Ok(())
}

pub(super) fn managed_auth_start_login(
    ctx: &mut RuntimeActionContext<'_>,
    auth_provider: String,
) -> Result<(), AppError> {
    let Some(tx) = ctx.managed_auth_req_tx else {
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_worker_unavailable(
                texts::tui_error_managed_auth_worker_unavailable(),
            ),
            ToastKind::Warning,
        );
        return Ok(());
    };

    ctx.app.managed_auth_loading = true;
    if let Err(err) = tx.send(ManagedAuthReq::StartLogin { auth_provider }) {
        ctx.app.managed_auth_loading = false;
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_request_failed(&err.to_string()),
            ToastKind::Warning,
        );
    }

    Ok(())
}

pub(super) fn managed_auth_set_default(
    ctx: &mut RuntimeActionContext<'_>,
    auth_provider: String,
    account_id: String,
) -> Result<(), AppError> {
    let Some(tx) = ctx.managed_auth_req_tx else {
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_worker_unavailable(
                texts::tui_error_managed_auth_worker_unavailable(),
            ),
            ToastKind::Warning,
        );
        return Ok(());
    };

    ctx.app.managed_auth_loading = true;
    if let Err(err) = tx.send(ManagedAuthReq::SetDefault {
        auth_provider,
        account_id,
    }) {
        ctx.app.managed_auth_loading = false;
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_request_failed(&err.to_string()),
            ToastKind::Warning,
        );
    }

    Ok(())
}

pub(super) fn managed_auth_remove(
    ctx: &mut RuntimeActionContext<'_>,
    auth_provider: String,
    account_id: String,
) -> Result<(), AppError> {
    let Some(tx) = ctx.managed_auth_req_tx else {
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_worker_unavailable(
                texts::tui_error_managed_auth_worker_unavailable(),
            ),
            ToastKind::Warning,
        );
        return Ok(());
    };

    ctx.app.managed_auth_loading = true;
    if let Err(err) = tx.send(ManagedAuthReq::Remove {
        auth_provider,
        account_id,
    }) {
        ctx.app.managed_auth_loading = false;
        ctx.app.push_toast(
            texts::tui_toast_managed_auth_request_failed(&err.to_string()),
            ToastKind::Warning,
        );
    }

    Ok(())
}

fn update_proxy_config(
    ctx: &mut RuntimeActionContext<'_>,
    mutate: impl FnOnce(&mut crate::proxy::ProxyConfig),
) -> Result<(), AppError> {
    let state = load_state()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Message(format!("failed to create async runtime: {e}")))?;

    let status = runtime.block_on(state.proxy_service.get_status());
    if status.running {
        *ctx.data = UiData::load(&ctx.app.app_type)?;
        ctx.app.push_toast(
            texts::tui_toast_proxy_settings_stop_proxy_before_edit_address(),
            super::super::app::ToastKind::Info,
        );
        return Ok(());
    }

    let mut config = runtime.block_on(state.proxy_service.get_config())?;
    mutate(&mut config);
    runtime.block_on(state.proxy_service.update_config(&config))?;

    *ctx.data = UiData::load(&ctx.app.app_type)?;
    ctx.app.push_toast(
        texts::tui_toast_proxy_settings_saved(),
        super::super::app::ToastKind::Success,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    use crate::app_config::AppType;
    use crate::cli::tui::app::App;
    use crate::cli::tui::data::UiData;
    use crate::cli::tui::runtime_systems::RequestTracker;
    use crate::cli::tui::terminal::TuiTerminal;
    use crate::test_support::TestEnvGuard;

    #[test]
    fn set_codex_official_auth_preservation_persists_setting() {
        let temp_home = TempDir::new().expect("create temp home");
        let _env = TestEnvGuard::isolated(temp_home.path());

        let mut terminal = TuiTerminal::new_for_test().expect("create test terminal");
        let mut app = App::new(Some(AppType::Codex));
        let mut data = UiData::load(&AppType::Codex).expect("load ui data");
        let mut proxy_loading = RequestTracker::default();
        let mut webdav_loading = RequestTracker::default();
        let mut update_check = RequestTracker::default();
        let mut ctx = RuntimeActionContext {
            terminal: &mut terminal,
            app: &mut app,
            data: &mut data,
            speedtest_req_tx: None,
            stream_check_req_tx: None,
            skills_req_tx: None,
            proxy_req_tx: None,
            proxy_loading: &mut proxy_loading,
            local_env_req_tx: None,
            session_req_tx: None,
            webdav_req_tx: None,
            webdav_loading: &mut webdav_loading,
            update_req_tx: None,
            update_check: &mut update_check,
            model_fetch_req_tx: None,
            managed_auth_req_tx: None,
        };

        set_preserve_codex_official_auth(&mut ctx, true)
            .expect("enable Codex official auth preservation");

        assert!(crate::settings::preserve_codex_official_auth_on_switch());
        assert!(matches!(
            ctx.app.toast.as_ref(),
            Some(toast) if toast.kind == ToastKind::Success
                && toast.message
                    == texts::tui_toast_codex_official_auth_preservation_toggled(true)
        ));
    }
}
