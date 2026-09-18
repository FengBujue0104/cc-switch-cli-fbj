use crate::app_config::AppType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Main,
    Providers,
    Usage,
    UsageLogs,
    UsageLogDetail { rowid: i64 },
    Pricing,
    Sessions,
    Mcp,
    Prompts,
    PiSystemPrompts,
    PiPromptTemplates,
    HermesMemory,
    Config,
    ConfigOpenClawWorkspace,
    ConfigOpenClawDailyMemory,
    ConfigOpenClawEnv,
    ConfigOpenClawTools,
    ConfigOpenClawAgents,
    ConfigCloudSync,
    ConfigWebDav,
    ConfigS3,
    Skills,
    SkillsDiscover,
    SkillsRepos,
    SkillDetail { directory: String },
    Settings,
    SettingsProxy,
    SettingsOutboundProxy,
    SettingsManagedAccounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Main,
    Providers,
    Usage,
    Sessions,
    Mcp,
    Prompts,
    PiSystemPrompts,
    PiPromptTemplates,
    HermesMemory,
    Config,
    Skills,
    OpenClawWorkspace,
    OpenClawEnv,
    OpenClawTools,
    OpenClawAgents,
    Settings,
    Exit,
}

impl NavItem {
    pub const ALL: [NavItem; 4] = [
        NavItem::Main,
        NavItem::Providers,
        NavItem::Settings,
        NavItem::Exit,
    ];

    pub const OPENCLAW_ALL: [NavItem; 4] = [
        NavItem::Main,
        NavItem::Providers,
        NavItem::Settings,
        NavItem::Exit,
    ];

    pub const HERMES_ALL: [NavItem; 4] = [
        NavItem::Main,
        NavItem::Providers,
        NavItem::Settings,
        NavItem::Exit,
    ];

    pub const PI_ALL: [NavItem; 4] = [
        NavItem::Main,
        NavItem::Providers,
        NavItem::Settings,
        NavItem::Exit,
    ];

    pub fn all_for_app(app_type: &AppType) -> &'static [NavItem] {
        match app_type {
            AppType::OpenClaw => &Self::OPENCLAW_ALL,
            AppType::Hermes => &Self::HERMES_ALL,
            AppType::Pi => &Self::PI_ALL,
            _ => &Self::ALL,
        }
    }

    pub fn to_route(self) -> Option<Route> {
        match self {
            NavItem::Main => Some(Route::Main),
            NavItem::Providers => Some(Route::Providers),
            NavItem::Usage => Some(Route::Usage),
            NavItem::Sessions => Some(Route::Sessions),
            NavItem::Mcp => Some(Route::Mcp),
            NavItem::Prompts => Some(Route::Prompts),
            NavItem::PiSystemPrompts => Some(Route::PiSystemPrompts),
            NavItem::PiPromptTemplates => Some(Route::PiPromptTemplates),
            NavItem::HermesMemory => Some(Route::HermesMemory),
            NavItem::Config => Some(Route::Config),
            NavItem::Skills => Some(Route::Skills),
            NavItem::OpenClawWorkspace => Some(Route::ConfigOpenClawWorkspace),
            NavItem::OpenClawEnv => Some(Route::ConfigOpenClawEnv),
            NavItem::OpenClawTools => Some(Route::ConfigOpenClawTools),
            NavItem::OpenClawAgents => Some(Route::ConfigOpenClawAgents),
            NavItem::Settings => Some(Route::Settings),
            NavItem::Exit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NavItem, Route};

    #[test]
    fn slim_nav_is_home_providers_settings_exit() {
        assert_eq!(
            NavItem::ALL,
            [
                NavItem::Main,
                NavItem::Providers,
                NavItem::Settings,
                NavItem::Exit,
            ]
        );
        for nav_items in [
            NavItem::ALL.as_slice(),
            NavItem::OPENCLAW_ALL.as_slice(),
            NavItem::HERMES_ALL.as_slice(),
            NavItem::PI_ALL.as_slice(),
        ] {
            assert_eq!(nav_items, NavItem::ALL.as_slice());
        }
    }

    #[test]
    fn pricing_is_not_a_top_level_nav_item() {
        for nav_items in [
            NavItem::ALL.as_slice(),
            NavItem::OPENCLAW_ALL.as_slice(),
            NavItem::HERMES_ALL.as_slice(),
            NavItem::PI_ALL.as_slice(),
        ] {
            assert!(nav_items
                .iter()
                .all(|item| item.to_route() != Some(Route::Pricing)));
        }
    }
}
