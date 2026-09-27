use serial_test::serial;
use std::path::Path;
use std::process::{Command, Output};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, lock_test_mutex, reset_test_fs};

fn run_cc_switch(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cc-switch"))
        .args(args)
        .env("HOME", home)
        .env("CC_SWITCH_CONFIG_DIR", home.join(".cc-switch"))
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env("CODEX_HOME", home.join(".codex"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_RUNTIME_DIR", home.join(".runtime"))
        .env("XDG_STATE_HOME", home.join(".state"))
        .env("NO_COLOR", "1")
        .output()
        .expect("run cc-switch")
}

fn assert_success(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    stdout
}

#[test]
#[serial]
fn config_show_masks_seeded_api_key() {
    let _lock = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();

    let provider_key = "sk-provider-plaintext-123456";
    assert_success(&run_cc_switch(
        home,
        &[
            "provider",
            "add",
            "--name",
            "Plaintext Provider",
            "--id",
            "plaintext-provider",
            "--base-url",
            "https://api.example.com",
            "--api-key",
            provider_key,
            "--model",
            "claude-sonnet-4-5",
        ],
    ));

    let show = assert_success(&run_cc_switch(home, &["config", "show"]));
    assert!(
        show.contains('{') && show.contains("plaintext-provider"),
        "config show should print JSON-like config: {show}"
    );
    assert!(
        !show.contains(provider_key),
        "config show must not print the raw API key: {show}"
    );
    assert!(
        show.contains("********3456"),
        "config show should print the masked secret form: {show}"
    );

    let json_start = show.find('{').expect("config show JSON object");
    let value: serde_json::Value =
        serde_json::from_str(&show[json_start..]).expect("config show JSON should parse");
    let obj = value.as_object().expect("config show JSON object");
    for dropped in ["gemini", "opencode", "openclaw", "mcp", "prompts", "skills"] {
        assert!(
            !obj.contains_key(dropped),
            "config show must not emit {dropped}: {show}"
        );
    }
    for kept in ["claude", "codex", "hermes", "pi", "version"] {
        assert!(
            obj.contains_key(kept),
            "config show must keep {kept}: {show}"
        );
    }
}

const WEBDAV_PASSWORD: &str = "super-secret-webdav-password-9999";
const S3_ACCESS_KEY_ID: &str = "AKIAEXAMPLEKEYID99";
const S3_SECRET_ACCESS_KEY: &str = "wJalrXUtnFEMI/K7MDENG";

fn seed_webdav(home: &Path) {
    assert_success(&run_cc_switch(
        home,
        &[
            "config",
            "webdav",
            "set",
            "--base-url",
            "https://dav.example.com/root",
            "--username",
            "demo",
            "--password",
            WEBDAV_PASSWORD,
            "--enable",
        ],
    ));
}

fn seed_s3(home: &Path) {
    assert_success(&run_cc_switch(
        home,
        &[
            "config",
            "s3",
            "set",
            "--region",
            "us-east-1",
            "--bucket",
            "demo-bucket",
            "--access-key-id",
            S3_ACCESS_KEY_ID,
            "--secret-access-key",
            S3_SECRET_ACCESS_KEY,
            "--enable",
        ],
    ));
}

#[test]
#[serial]
fn config_webdav_show_masks_password_by_default() {
    let _lock = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();
    seed_webdav(home);

    let show = assert_success(&run_cc_switch(home, &["config", "webdav", "show"]));
    assert!(
        !show.contains(WEBDAV_PASSWORD),
        "webdav show must not print the raw password: {show}"
    );
    assert!(
        show.contains("********9999"),
        "webdav show should print the masked password form: {show}"
    );
}

#[test]
#[serial]
fn config_webdav_show_reveal_prints_full_password() {
    let _lock = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();
    seed_webdav(home);

    let show = assert_success(&run_cc_switch(
        home,
        &["config", "webdav", "show", "--reveal"],
    ));
    assert!(
        show.contains(WEBDAV_PASSWORD),
        "webdav show --reveal should print the stored password: {show}"
    );
}

#[test]
#[serial]
fn config_s3_show_masks_access_keys_by_default() {
    let _lock = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();
    seed_s3(home);

    let show = assert_success(&run_cc_switch(home, &["config", "s3", "show"]));
    assert!(
        !show.contains(S3_ACCESS_KEY_ID),
        "s3 show must not print the raw access key id: {show}"
    );
    assert!(
        !show.contains(S3_SECRET_ACCESS_KEY),
        "s3 show must not print the raw secret access key: {show}"
    );
    assert!(
        show.contains("********ID99"),
        "s3 show should print the masked access key id: {show}"
    );
    assert!(
        show.contains("********DENG"),
        "s3 show should print the masked secret access key: {show}"
    );
}

#[test]
#[serial]
fn config_s3_show_reveal_prints_full_access_keys() {
    let _lock = lock_test_mutex();
    reset_test_fs();
    let home = ensure_test_home();
    seed_s3(home);

    let show = assert_success(&run_cc_switch(home, &["config", "s3", "show", "--reveal"]));
    assert!(
        show.contains(S3_ACCESS_KEY_ID),
        "s3 show --reveal should print the stored access key id: {show}"
    );
    assert!(
        show.contains(S3_SECRET_ACCESS_KEY),
        "s3 show --reveal should print the stored secret access key: {show}"
    );
}
