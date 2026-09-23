![CC-Switch CLI](assets/screenshots/hero.png)

<div align="center">

## CC-Switch CLI (fbj)

**Switch Claude Code, Codex, Hermes, and Pi providers from one TUI or CLI.**

[![Version](https://img.shields.io/badge/version-5.11.0-blue.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64%20%7C%20Linux%20x64-lightgrey.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

English | [中文](README_ZH.md)

</div>

---

## About

Personal lightweight fork of [CC-Switch CLI](https://github.com/saladday/cc-switch-cli) (upstream [CC-Switch](https://github.com/farion1231/cc-switch)). It keeps provider switching for Claude, Codex, Hermes, and Pi. Partner promotions and extra surfaces (MCP, Skills, Sessions, Usage) are removed from the default CLI/TUI.

Official Codex OAuth is kept. Unified Codex session history (`model_provider = custom`) is on by default. Existing official (`openai`) sessions stay in that bucket until `cc-switch settings codex-history migrate-existing`. The local proxy remains optional (`cc-switch proxy enable`) for API format conversion.

---

## Screenshots

<div align="center">
  <img src="assets/screenshots/home-en.png" alt="Home" width="70%"/>
</div>

---

## Install

Release packages are **Windows x86_64** and **Linux x86_64** (static musl) only. This fork does not ship macOS binaries.

**Linux x86_64** (static musl, Debian 12+):

```bash
curl -fsSL https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.sh | bash
```

**Windows x86_64**:

```powershell
irm https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.ps1 | iex
```

Linux installs to `~/.local/bin` (`CC_SWITCH_INSTALL_DIR` to override). Windows installs to `%LOCALAPPDATA%\cc-switch` and adds that folder to the user PATH. Set `CC_SWITCH_FORCE=1` to overwrite non-interactively. Both scripts download `checksums.txt` and refuse to install on a SHA-256 mismatch.

**Self-update** (this build and later):

```bash
cc-switch update --check
cc-switch update
```

Updates come from [this repo's GitHub Releases](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases), not upstream SaladDay. `cc-switch update` verifies SHA-256 from `checksums.txt` (or GitHub's asset digest). This source is `5.11.0`; the published tag is still `v5.10.6` until the next local publish. Anything already on `5.10.6` self-updates to `v5.11.0` directly.

Publish a new tag from a dev machine (no GitHub Actions):

```bash
scripts/publish-release.sh v5.11.0
```

---

## Quick start

```bash
cc-switch                              # TUI
cc-switch --app claude provider list
cc-switch --app claude use <id>
cc-switch --app codex provider list
cc-switch --app codex use <id>
cc-switch auth list                    # ChatGPT / Codex OAuth accounts
cc-switch settings codex-history show  # unified session-history toggle
cc-switch env check                    # env conflicts
cc-switch proxy show                   # optional local proxy
```

Supported `--app` values: `claude` (default), `codex`, `hermes`, `pi`.

Unix only:

```bash
cc-switch start claude <id>
cc-switch start codex <id>
cc-switch start codex <id> --shared-sessions
```

`start` launches one session without changing the global current provider. `--shared-sessions` reuses the unified `custom` Codex history bucket.

Templates when adding a provider: Custom, Official/OAuth, DeepSeek, Zhipu GLM, MiniMax, Xiaomi MiMo, OpenRouter.

---

## Build from source

Rust 1.91.1+:

```bash
git clone https://github.com/FengBujue0104/cc-switch-cli-fbj.git
cd cc-switch-cli-fbj/src-tauri
cargo build --release
```

Linux release binary:

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

---

## License

MIT. Original author Jason Young; CLI fork saladday; this personal fork FengBujue0104.
