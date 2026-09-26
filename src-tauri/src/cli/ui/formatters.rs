use serde::Serialize;
use serde_json::Value;

pub fn to_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Mask a secret for display: empty stays empty, short values become
/// `********`, longer values keep a 4-character tail.
pub(crate) fn mask_secret_for_display(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    let count = value.chars().count();
    if count <= 8 {
        return "********".to_string();
    }
    let tail: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("********{tail}")
}

fn is_secret_container_key(key: &str) -> bool {
    matches!(key.to_ascii_lowercase().as_str(), "env" | "auth")
}

fn is_secret_value_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    matches!(
        key.as_str(),
        "api_key" | "apikey" | "access_token" | "accesstoken" | "password" | "secret" | "token"
    ) || key.ends_with("_api_key")
        || key.ends_with("_token")
        || key.ends_with("_secret")
        || key.ends_with("_password")
}

fn mask_all_strings(value: &mut Value) {
    match value {
        Value::String(text) => *text = mask_secret_for_display(text),
        Value::Array(items) => items.iter_mut().for_each(mask_all_strings),
        Value::Object(map) => map.values_mut().for_each(mask_all_strings),
        _ => {}
    }
}

/// Recursively mask `env` / `auth` objects and `api_key`-like fields in a
/// JSON value. Used by `cc-switch config show`.
pub(crate) fn mask_json_secrets(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(mask_json_secrets),
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let Some(child) = map.get_mut(&key) else {
                    continue;
                };
                if is_secret_container_key(&key) {
                    mask_all_strings(child);
                } else if is_secret_value_key(&key) {
                    if let Value::String(text) = child {
                        *text = mask_secret_for_display(text);
                    } else {
                        mask_json_secrets(child);
                    }
                } else {
                    mask_json_secrets(child);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_json_secrets, mask_secret_for_display};
    use serde_json::json;

    #[test]
    fn mask_secret_for_display_hides_the_body_and_keeps_a_short_tail() {
        assert_eq!(mask_secret_for_display(""), "");
        assert_eq!(mask_secret_for_display("short"), "********");
        assert_eq!(
            mask_secret_for_display("sk-abcdefghijklmnopqrstuvwxyz1234"),
            "********1234"
        );
        assert!(
            !mask_secret_for_display("sk-abcdefghijklmnopqrstuvwxyz1234")
                .contains("sk-abcdefghijklmnopqrstuvwxyz1234")
        );
    }

    #[test]
    fn mask_json_secrets_covers_env_api_key_and_auth() {
        let mut value = json!({
            "claude": {
                "providers": {
                    "p1": {
                        "settingsConfig": {
                            "env": {
                                "ANTHROPIC_API_KEY": "sk-provider-plaintext-123456",
                                "ANTHROPIC_BASE_URL": "https://api.example.com"
                            }
                        }
                    }
                }
            },
            "codex": {
                "providers": {
                    "p2": {
                        "settingsConfig": {
                            "auth": { "OPENAI_API_KEY": "sk-codex-plaintext-123456" }
                        }
                    }
                }
            },
            "meta": { "api_key": "sk-direct-plaintext-123456" }
        });
        mask_json_secrets(&mut value);
        let rendered = value.to_string();
        assert!(!rendered.contains("sk-provider-plaintext-123456"));
        assert!(!rendered.contains("sk-codex-plaintext-123456"));
        assert!(!rendered.contains("sk-direct-plaintext-123456"));
        assert!(rendered.contains("********3456"));
        assert_eq!(
            value["claude"]["providers"]["p1"]["settingsConfig"]["env"]["ANTHROPIC_API_KEY"],
            json!(mask_secret_for_display("sk-provider-plaintext-123456"))
        );
    }
}

pub fn format_bool(value: bool) -> &'static str {
    if value {
        "✓"
    } else {
        "✗"
    }
}
