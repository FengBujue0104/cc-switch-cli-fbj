![CC-Switch CLI](assets/screenshots/hero.png)

<div align="center">

## CC-Switch CLI (fbj)

**在一个 TUI / CLI 里切换 Claude Code、Codex、Hermes、Pi 的供应商。**

[![Version](https://img.shields.io/badge/version-5.10.6-blue.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64%20%7C%20Linux%20x64-lightgrey.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md) | 中文

</div>

---

## 关于

这是 [CC-Switch CLI](https://github.com/saladday/cc-switch-cli) 的个人轻量化分支（更上游为 [CC-Switch](https://github.com/farion1231/cc-switch)）。只保留 Claude / Codex / Hermes / Pi 的供应商切换，去掉赞助商推广，以及 MCP、Skills、Sessions、Usage 等默认入口。

保留官方 Codex OAuth。统一 Codex 会话历史（`model_provider = custom`）默认开启。已有官方（`openai`）会话仍留在原桶，需要时再执行 `cc-switch settings codex-history migrate-existing`。本地代理仍可选：`cc-switch proxy enable`，用于 API 格式转换。

---

## 截图

<div align="center">
  <img src="assets/screenshots/home-zh.png" alt="首页" width="70%"/>
</div>

---

## 安装

**Linux x86_64**（静态 musl，Debian 12 可用）：

```bash
curl -fsSL https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.sh | bash
```

**Windows x86_64**：

```powershell
irm https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.ps1 | iex
```

Linux 默认装到 `~/.local/bin`（`CC_SWITCH_INSTALL_DIR` 可改）。Windows 装到 `%LOCALAPPDATA%\cc-switch` 并写入用户 PATH。`CC_SWITCH_FORCE=1` 非交互覆盖。安装脚本会下载 `checksums.txt`，SHA-256 对不上就拒绝安装。

**自更新**（本构建及之后的二进制）：

```bash
cc-switch update --check
cc-switch update
```

更新源是 [本仓库的 GitHub Releases](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)，不是上游 SaladDay。`cc-switch update` 会用 `checksums.txt`（或 GitHub 资源 digest）做 SHA-256 校验。当前源码是 `5.10.6`，已发布标签仍是 `v5.10.5-fbj.1`，等下一次本地发版。先用上面的脚本重装一次以拿到 updater 修复，之后的 `v5.10.6` 等标签即可自更新。

本机发版（不走 GitHub Actions）：

```bash
scripts/publish-release.sh v5.10.6
```

---

## 快速开始

```bash
cc-switch                              # TUI
cc-switch --app claude provider list
cc-switch --app claude use <id>
cc-switch --app codex provider list
cc-switch --app codex use <id>
cc-switch auth list                    # ChatGPT / Codex OAuth
cc-switch settings codex-history show  # 会话共享开关
cc-switch env check
cc-switch proxy show                   # 可选本地代理
```

`--app` 仅支持：`claude`（默认）、`codex`、`hermes`、`pi`。

仅 Unix：

```bash
cc-switch start claude <id>
cc-switch start codex <id>
cc-switch start codex <id> --shared-sessions
```

`start` 只影响这一次启动，不改全局当前供应商。`--shared-sessions` 走统一的 `custom` 历史桶。

添加供应商模板：Custom、Official/OAuth、DeepSeek、智谱 GLM、MiniMax、小米 MiMo、OpenRouter。

---

## 从源码编译

需要 Rust 1.91.1+：

```bash
git clone https://github.com/FengBujue0104/cc-switch-cli-fbj.git
cd cc-switch-cli-fbj/src-tauri
cargo build --release
```

Linux 发版包：

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

---

## 许可证

MIT。原作者 Jason Young；CLI 分支 saladday；本 fork FengBujue0104。
