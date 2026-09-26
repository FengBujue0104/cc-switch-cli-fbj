# AGENTS.md

This file mirrors `CLAUDE.md` for Codex and other coding agents. Keep both files aligned when repository guidance changes.

**Product contract is `README.md` / `README_EN.md`.** This is a lightweight personal fork of [SaladDay/cc-switch-cli](https://github.com/saladday/cc-switch-cli). Do not follow leftover upstream copy that talks about a six-harness product.

## Fork constraints (do not violate)

- **Harnesses:** only Claude Code, Codex, Hermes, and Pi. `-a/--app` accepts `claude`, `codex`, `hermes`, `pi`. `gemini`, `opencode`, and `openclaw` are rejected at the command entry point. The library still parses those retired ids so old SQLite / JSON data can load; that is not permission to expose them again.
- **Do not resurrect removed harnesses.** Do not re-enable PATH auto-detection of available harnesses. `settings visible-apps` stays read-only. `should_sync_live` stays `false` for retired ids so provider switch / live sync never reads or writes `~/.gemini`, `~/.config/opencode`, or `~/.openclaw` as part of switching.
- **Live-file exceptions (keep them exceptional):** hidden `cc-switch config openclaw` may write `~/.openclaw/` when called explicitly. Startup may one-shot scrub leaked keys from `~/.gemini/.env` (flagged, never repeats).
- **Visible CLI commands:** `auth`, `provider`, `use`, `config`, `proxy`, `settings`, `start`, `daemon`, `env`, `update`, `interactive`, `completions`. `start` / `daemon` are Unix-only. `proxy` is a default-on cargo feature. Do **not** re-add user-facing `skills`, `mcp`, `sessions`, or `usage` commands.
- **TUI sidebar:** every app is Home / Providers / Settings / Exit. Help text and startup IO must match that four-item nav.
- **Update source:** GitHub Releases for `FengBujue0104/cc-switch-cli-fbj`. Never retarget `cc-switch update` or install scripts at `saladday/cc-switch-cli`.
- **Release platforms:** Windows x86_64 and Linux x86_64 musl only. Publish with `scripts/publish-release.sh`; there is no GitHub Actions release workflow.

Hidden `config webdav` / `config s3` / `config openclaw` stay as hidden subcommands. Internal MCP / skills / sessions / usage **library** modules may still compile; they are not user-facing surfaces.

## Commands

The Rust crate lives in `src-tauri/`. Run Cargo commands from that directory unless a command explicitly targets repository-root assets or scripts.

```bash
cd src-tauri

cargo run                                  # Run cc-switch in interactive mode
cargo run -- provider list                 # Run a specific CLI command
cargo run -- --app codex provider list     # Run a command for a specific app
cargo run -- proxy show                    # Inspect proxy state
cargo run -- env tools                     # Check local CLI tools
cargo build --release                      # Build release binary at target/release/cc-switch

cargo fmt                                  # Format Rust code
cargo fmt --check                          # Check formatting
cargo clippy                               # Run lints
cargo test                                 # Run all tests
cargo test provider_switch                 # Run tests whose names contain provider_switch
cargo test --test provider_commands        # Run a single integration test target
cargo test --features test-hooks           # Run tests with the test-hooks feature enabled
```

The repository pins Rust through `src-tauri/rust-toolchain.toml` to Rust 1.91.1 with `rustfmt` and `clippy`.

## Project overview

CC-Switch CLI (fbj) is a Rust TUI/CLI for switching providers on **Claude Code, Codex, Hermes, and Pi**. It keeps provider CRUD, Codex OAuth, unified Codex session history, an optional local proxy, Unix `start`/`daemon`, env checks, and self-update.

The main crate is `src-tauri/`; the repository root contains docs, assets, install/update scripts, packaging metadata, and Nix files.

Key Rust entry points:

- `src/main.rs` parses CLI arguments, initializes logging, creates startup state for most commands, and dispatches to command handlers.
- `src/lib.rs` declares crate modules and re-exports public types used by integration tests and command code.
- `src/cli/mod.rs` defines the top-level Clap CLI, global `--app` flag, and command enum. Visible commands are listed under Fork constraints.
- `src/cli/commands/` contains command implementations. MCP / prompts / skills / sessions modules may still exist as leftover library code; they are not clap user-facing commands in this fork.
- `src/cli/interactive/` and `src/cli/tui/` contain the interactive ratatui UI. Sidebar nav is four items; do not advertise removed pages in `?` help.
- `src/services/` contains durable business logic used by commands and the TUI.
- `src/database/` is the SQLite persistence layer.
- `src/app_config.rs`, `src/provider.rs`, and app-specific config modules define the shared configuration model. Retired Gemini / OpenCode / OpenClaw adapters remain so old data can parse; live sync for those ids is gated off.
- `src/proxy/` implements the optional local multi-app proxy.
- `src/daemon/` implements the Unix supervisor daemon.
- `src/store.rs` defines `AppState`. Persist/export snapshot loops must walk `AppType::all()` (the four live harnesses).

## State and configuration model

CC-Switch stores core state in SQLite at `~/.cc-switch/cc-switch.db` by default, or under `$CC_SWITCH_CONFIG_DIR/cc-switch.db` when `CC_SWITCH_CONFIG_DIR` is set. `~/.cc-switch/settings.json` stores app settings. `~/.cc-switch/backups/` holds rotating backups.

Legacy `config.json` and `skills.json` are migration/import sources only. `AppState::try_new()` validates and migrates legacy files into SQLite when needed. `AppState::try_new_with_startup_recovery()` also imports live provider configs and recovers proxy takeovers when needed. `AppState::save()` persists the in-memory snapshot back to SQLite.

Live config files are synced only for harnesses where `should_sync_live` is true (Claude, Codex, Hermes, Pi):

- Claude: `~/.claude/settings.json`, `~/.claude.json`, `~/.claude/CLAUDE.md`
- Codex: `~/.codex/auth.json`, `~/.codex/config.toml`, `~/.codex/AGENTS.md`
- Hermes: Hermes config directory from settings or the default app location
- Pi: Pi config directory from settings or the default app location

Environment overrides matter when testing or running commands: `CC_SWITCH_CONFIG_DIR` controls CC-Switch storage, `CLAUDE_CONFIG_DIR` controls Claude config directory, and `CODEX_HOME` controls Codex config. Tests also commonly set `HOME`, `XDG_CONFIG_HOME`, `XDG_RUNTIME_DIR`, and `XDG_STATE_HOME`.

## CLI architecture

Adding or changing a user-facing command usually requires updates in three layers:

1. Define the Clap shape in `src/cli/mod.rs` or the relevant `src/cli/commands/*.rs` file.
2. Implement command I/O and prompts in `src/cli/commands/`, keeping durable logic in `src/services/` when behavior is shared with the TUI or other commands.
3. Add or update tests under `src-tauri/tests/` or module-local `#[cfg(test)]` tests.

The global `--app` flag selects an `AppType`; Claude is the default. Supported app labels are `claude`, `codex`, `hermes`, and `pi`. Retired ids stay `value(skip)` in clap.

Commands that normally create startup state call `AppState::try_new_with_startup_recovery()` before dispatch. `update`, `completions`, `internal`, and Unix `daemon` commands intentionally bypass normal startup state.

## TUI interaction guidance

- Keep primary TUI surfaces focused on fields, current values/status, and available actions.
- Put feature explanations, behavioral caveats, validation rules, and other long-form hints in the contextual `?` help for the focused control.
- Do not add persistent instruction or description panels when the same information can live in `?` help.
- Global `?` help must describe the four-item sidebar only. Do not document MCP / Prompts / Sessions / Skills / Usage pages as reachable.

## Proxy architecture

The proxy command surface is in `src/cli/commands/proxy.rs`, orchestration lives in `src/services/proxy.rs`, and the HTTP server is in `src/proxy/server.rs` and `src/proxy/handlers.rs`. Proxy takeover applies to Claude and Codex; Hermes and Pi do not use the proxy.

## Testing requirements

All test cases should be executed under `src-tauri/`.

When adding integration tests that touch HOME, app config directories, or live config files, isolate filesystem state with helpers in `src-tauri/tests/support.rs`. Use `ensure_test_home()`, `reset_test_fs()`, and `lock_test_mutex()` patterns rather than writing to real user directories. Unit tests inside the crate can also use `src/test_support.rs` helpers for test home/settings isolation.

### IMPORTANT

- **NEVER** change the host configuration in `$CC_SWITCH_CONFIG_DIR/`.
- **NEVER** change the host configuration in `$CLAUDE_CONFIG_DIR/`.
- **NEVER** change the host configuration in `$CODEX_HOME/`.
- Create a sandbox before executing test cases or commands that write app configuration.
- Prefer temporary directories and explicit environment overrides for tests that exercise live config sync/import paths.
