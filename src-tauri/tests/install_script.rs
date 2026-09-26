#![cfg(not(windows))]

use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const RELEASE_TAG: &str = "v5.11.0";
const LINUX_ASSET: &str = "cc-switch-cli-v5.11.0-linux-x64.tar.gz";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root should exist")
        .to_path_buf()
}

fn install_script_path() -> PathBuf {
    repo_root().join("install.sh")
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("script should be written");
    let mut perms = fs::metadata(path)
        .expect("metadata should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("permissions should be updated");
}

fn sha256_file(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum should run");
    assert!(output.status.success(), "sha256sum failed: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .expect("sha256sum should print a hash")
        .to_string()
}

struct Harness {
    _temp: TempDir,
    home: PathBuf,
    fakebin: PathBuf,
    install_dir: PathBuf,
    logs_dir: PathBuf,
    archive_path: PathBuf,
    checksums_path: PathBuf,
    release_json_path: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir should exist");
        let home = temp.path().join("home");
        let fakebin = temp.path().join("fakebin");
        let install_dir = temp.path().join("install");
        let logs_dir = temp.path().join("logs");
        let payload_dir = temp.path().join("payload");
        let archive_path = temp.path().join(LINUX_ASSET);
        let checksums_path = temp.path().join("checksums.txt");
        let release_json_path = temp.path().join("release.json");

        fs::create_dir_all(&home).expect("home should exist");
        fs::create_dir_all(&fakebin).expect("fakebin should exist");
        fs::create_dir_all(&install_dir).expect("install dir should exist");
        fs::create_dir_all(&logs_dir).expect("logs dir should exist");
        fs::create_dir_all(&payload_dir).expect("payload dir should exist");

        write_executable(
            &payload_dir.join("cc-switch"),
            "#!/usr/bin/env bash\necho new build\n",
        );

        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&payload_dir)
            .arg("cc-switch")
            .status()
            .expect("tar should run");
        assert!(status.success(), "tar should create archive");

        let hash = sha256_file(&archive_path);
        fs::write(
            &checksums_path,
            format!("{hash}  {LINUX_ASSET}\n"),
        )
        .expect("checksums should be written");
        fs::write(
            &release_json_path,
            format!(
                r#"{{"assets":[{{"name":"x","tag_name":"v-nested-wrong"}}],"tag_name":"{RELEASE_TAG}","name":"release"}}"#
            ),
        )
        .expect("release json should be written");

        write_executable(
            &fakebin.join("uname"),
            r#"#!/usr/bin/env bash
set -eu
case "${1:-}" in
  -s) printf 'Linux\n' ;;
  -m) printf 'x86_64\n' ;;
  *) /usr/bin/uname "$@" ;;
esac
"#,
        );

        write_executable(
            &fakebin.join("curl"),
            r#"#!/usr/bin/env bash
set -eu
output=''
url=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output|-o)
      output="$2"
      shift 2
      ;;
    -A|--user-agent)
      shift 2
      ;;
    --fail|--location|--silent|--show-error)
      shift
      ;;
    *)
      url="$1"
      shift
      ;;
  esac
done

printf '%s' "$url" > "${CC_SWITCH_TEST_LOG_DIR}/last-url"
printf '%s\n' "$url" >> "${CC_SWITCH_TEST_LOG_DIR}/requested-urls"

respond() {
  local dest="$1"
  local src="$2"
  if [ -n "$dest" ]; then
    cp "$src" "$dest"
  else
    cat "$src"
  fi
}

case "$url" in
  */releases/latest|*/releases/tags/*)
    respond "$output" "${CC_SWITCH_TEST_RELEASE_JSON}"
    ;;
  */checksums.txt)
    if [ "${CC_SWITCH_TEST_BAD_CHECKSUM:-0}" = "1" ]; then
      printf '0000000000000000000000000000000000000000000000000000000000000000  %s\n' \
        "cc-switch-cli-v5.11.0-linux-x64.tar.gz" > "${CC_SWITCH_TEST_LOG_DIR}/bad-checksums.txt"
      respond "$output" "${CC_SWITCH_TEST_LOG_DIR}/bad-checksums.txt"
    else
      respond "$output" "${CC_SWITCH_TEST_CHECKSUMS_PATH}"
    fi
    ;;
  *.tar.gz)
    respond "$output" "${CC_SWITCH_TEST_ARCHIVE_PATH}"
    ;;
  *)
    echo "unexpected url: $url" >&2
    exit 22
    ;;
esac
"#,
        );

        Self {
            _temp: temp,
            home,
            fakebin,
            install_dir,
            logs_dir,
            archive_path,
            checksums_path,
            release_json_path,
        }
    }

    fn run(&self, extra_envs: &[(&str, &str)], extra_path: Option<&Path>) -> Output {
        let mut path_parts = vec![self.fakebin.display().to_string()];
        if let Some(extra) = extra_path {
            path_parts.push(extra.display().to_string());
        }
        path_parts.push(std::env::var("PATH").unwrap_or_default());

        let mut command = Command::new("bash");
        command
            .arg(install_script_path())
            .env("HOME", &self.home)
            .env("CC_SWITCH_INSTALL_DIR", &self.install_dir)
            .env("CC_SWITCH_TEST_ARCHIVE_PATH", &self.archive_path)
            .env("CC_SWITCH_TEST_CHECKSUMS_PATH", &self.checksums_path)
            .env("CC_SWITCH_TEST_RELEASE_JSON", &self.release_json_path)
            .env("CC_SWITCH_TEST_LOG_DIR", &self.logs_dir)
            .env("PATH", path_parts.join(":"));

        for (key, value) in extra_envs {
            command.env(key, value);
        }

        command.output().expect("install script should run")
    }

    fn requested_urls(&self) -> String {
        fs::read_to_string(self.logs_dir.join("requested-urls")).unwrap_or_default()
    }
}

#[test]
#[serial]
fn install_script_requires_force_for_non_tty_overwrite() {
    let harness = Harness::new();
    write_executable(
        &harness.install_dir.join("cc-switch"),
        "#!/usr/bin/env bash\necho old build\n",
    );

    let output = harness.run(&[], None);
    assert!(
        !output.status.success(),
        "overwrite should fail without force"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CC_SWITCH_FORCE=1"), "stderr was: {stderr}");
    assert!(
        harness.requested_urls().is_empty(),
        "no-TTY cancel must exit before downloading"
    );

    let installed = fs::read_to_string(harness.install_dir.join("cc-switch"))
        .expect("existing binary should remain");
    assert!(installed.contains("old build"));
}

#[test]
#[serial]
fn install_script_force_overwrites_and_requests_tagged_linux_x64_asset() {
    let harness = Harness::new();
    let shadow_dir = harness.home.join("shadow-bin");
    fs::create_dir_all(&shadow_dir).expect("shadow dir should exist");
    write_executable(
        &shadow_dir.join("cc-switch"),
        "#!/usr/bin/env bash\necho shadow build\n",
    );
    write_executable(
        &harness.install_dir.join("cc-switch"),
        "#!/usr/bin/env bash\necho old build\n",
    );

    let output = harness.run(&[("CC_SWITCH_FORCE", "1")], Some(&shadow_dir));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "force overwrite should succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let requested = harness.requested_urls();
    assert!(
        requested.contains(LINUX_ASSET),
        "expected tagged linux-x64 tarball, got {requested}"
    );
    assert!(
        requested.contains("checksums.txt"),
        "checksums.txt should be fetched, got {requested}"
    );
    assert!(
        !requested.contains("linux-x64-musl"),
        "fork install.sh must not request musl-named assets: {requested}"
    );
    assert!(
        !requested.contains("aarch64") && !requested.contains("arm64"),
        "fork install.sh is x86_64 only: {requested}"
    );

    let installed = fs::read_to_string(harness.install_dir.join("cc-switch"))
        .expect("installed file should exist");
    assert!(installed.contains("new build"));
}

#[test]
#[serial]
fn install_script_rejects_checksum_mismatch_and_keeps_existing_binary() {
    let harness = Harness::new();
    let installed_path = harness.install_dir.join("cc-switch");
    write_executable(
        &installed_path,
        "#!/usr/bin/env bash\necho old build\n",
    );

    let output = harness.run(
        &[
            ("CC_SWITCH_FORCE", "1"),
            ("CC_SWITCH_TEST_BAD_CHECKSUM", "1"),
        ],
        None,
    );
    assert!(!output.status.success(), "bad checksum must fail the install");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Checksum mismatch"),
        "stderr was: {stderr}"
    );

    let requested = harness.requested_urls();
    assert!(
        requested.contains(LINUX_ASSET),
        "asset should still be requested, got {requested}"
    );

    let installed =
        fs::read_to_string(&installed_path).expect("existing binary should remain readable");
    assert!(
        installed.contains("old build"),
        "checksum failure must leave the previous binary in place"
    );
}

#[test]
fn install_and_publish_scripts_agree_on_tagged_linux_x64_asset_name() {
    let install = fs::read_to_string(install_script_path()).expect("read install.sh");
    let publish = fs::read_to_string(repo_root().join("scripts/publish-release.sh"))
        .expect("read publish-release.sh");
    assert!(
        install.contains("cc-switch-cli-${tag_name}-linux-x64.tar.gz"),
        "install.sh must use the tagged linux-x64 tarball name"
    );
    assert!(
        publish.contains("cc-switch-cli-${TAG}-linux-x64.tar.gz"),
        "publish-release.sh must emit the tagged linux-x64 tarball name"
    );
    assert!(
        publish.contains("cc-switch-cli-${TAG}-windows-x64.zip"),
        "publish-release.sh must emit the tagged windows-x64 zip name"
    );
    assert!(
        publish.contains("checksums.txt"),
        "publish-release.sh must write checksums.txt"
    );
    assert!(
        !install.contains("CC_SWITCH_LINUX_LIBC"),
        "this fork's install.sh has no libc switch"
    );
    assert!(
        !install.contains("linux-x64-musl"),
        "this fork's install.sh must not advertise musl-named assets"
    );
    assert!(
        install.contains("python3 is required to parse the GitHub Releases JSON"),
        "install.sh must require python3 to parse tag_name"
    );
    assert!(
        !install.contains("grep -oE"),
        "install.sh must not grep tag_name out of the Releases JSON"
    );
}
