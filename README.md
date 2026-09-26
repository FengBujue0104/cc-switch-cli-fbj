<div align="center">

# CC-Switch CLI (fbj)

**在一个 TUI / CLI 里切换 Claude Code、Codex、Hermes、Pi 的供应商。**

中文 ｜ [English](README_EN.md)

[![Version](https://img.shields.io/badge/version-5.11.0-blue.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases/tag/v5.11.0)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64%20%7C%20Linux%20x64-lightgrey.svg)](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

</div>

---

![CC-Switch TUI 首页](assets/screenshots/home.png)

## 这是什么

[CC-Switch CLI](https://github.com/saladday/cc-switch-cli) 的个人轻量化分支（更上游为 [CC-Switch](https://github.com/farion1231/cc-switch)）。**只保留四套日常在用的 harness：Claude Code、Codex、Hermes、Pi**，其余一律裁掉，供应商的增删改查和切换保持原样好用。

## 与上游的差别

- **harness 只留四个。** `-a/--app` 仅接受 `claude`、`codex`、`hermes`、`pi`，`gemini`、`opencode`、`openclaw` 在命令入口就被拒绝（库内部仍保留旧 id 的解析，否则旧数据读不出来）。供应商切换 / live 同步的路径一律不再读写已删 harness 的本地配置文件（`should_sync_live` 恒为 `false`），换回上游或其他分支仍可继续使用。还会碰已删 harness 本地文件的只剩两处：隐藏命令 `cc-switch config openclaw`（显式调用时写 `~/.openclaw/` 下的配置和工作区文件，不接入供应商切换），以及启动时一次性的旧 common-config 凭据清理（只从 `~/.gemini/.env` 删掉泄漏的密钥条目，有标记、不重跑）。
- **不会再自己复活已删 harness。** 以前「自动检测可用 harness」会把已删的 harness 又塞回切换标签栏，这条路径已禁用；`settings visible-apps` 也降级为只读，无法把已删 harness 加回来。
- **CLI 命令收敛。** 可见的顶层命令为 `auth`、`provider`、`use`、`config`、`proxy`、`settings`、`start`、`daemon`、`env`、`update`、`interactive`、`completions`（`start` / `daemon` 仅 Unix；`proxy` 是默认启用的 cargo feature，`--no-default-features --features cli` 可编出不带代理命令的版本）；`skills`、`mcp`、`sessions`、`usage` 等入口已移除。隐藏子命令仍有 `config openclaw`、`config webdav`、`config s3`（后两者做备份同步，不写已删 harness 的 live 文件），以及 `provider speedtest` / `stream-check` / `fetch-models` / `quota` / `usage-query`。
- **TUI 侧栏精简。** 任意应用下都只有「首页 / 供应商 / 设置 / 退出」四项。
- **保留的东西没动。** 官方 Codex OAuth、统一的 Codex 会话历史（`model_provider = custom`）、可选的本地代理（`cc-switch proxy enable`，负责 API 格式转换）都保持原样。

统一 Codex 会话历史默认开启；已有的官方（`openai`）会话仍留在原桶，需要时执行 `cc-switch settings codex-history migrate-existing` 迁移。

## 安装

发版包只有 **Windows x86_64** 和 **Linux x86_64**（静态 musl）两种，本 fork 不提供 macOS 二进制。

**Linux x86_64**（静态 musl，不依赖系统 glibc）：

```bash
curl -fsSL https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.sh | bash
```

**Windows x86_64**：

```powershell
irm https://raw.githubusercontent.com/FengBujue0104/cc-switch-cli-fbj/main/install.ps1 | iex
```

Linux 默认装到 `~/.local/bin`（`CC_SWITCH_INSTALL_DIR` 可改），Windows 装到 `%LOCALAPPDATA%\cc-switch` 并写入用户 PATH。已装过时两个脚本都会先问一句覆盖还是取消：Linux 的 `install.sh` 在没有 TTY 时会直接退出并提示设置 `CC_SWITCH_FORCE=1`；Windows 的 `install.ps1` 没有 TTY 检测，非交互宿主里会直接报错，同样可以用 `CC_SWITCH_FORCE=1` 跳过询问。两个脚本都会下载 `checksums.txt`，SHA-256 对不上就拒绝安装。

## 更新

```bash
cc-switch update --check     # 只看，不动文件
cc-switch update             # 下载并替换当前二进制
```

更新源是[本仓库的 GitHub Releases](https://github.com/FengBujue0104/cc-switch-cli-fbj/releases)，不是上游 SaladDay；`cc-switch update` 会用 `checksums.txt`（或 GitHub 资源 digest）校验 SHA-256。当前最新标签 **v5.11.0**，`5.10.6` 及以上可以直接自更新过来。

> 注意：如果当前安装的二进制编译于 `6c037571` 之前（即本 fork 把 `cc-switch update` 指向自己仓库之前），它的更新源仍是上游 `saladday/cc-switch-cli`——本 fork 唯一的早期包 `v5.10.5-fbj.1` 就属于这种情况。它报的版本号是 `5.10.5`，而上游最新目前也还是 `5.10.5`，所以它现在会一直显示「已是最新」；上游哪天发了更新的版本，它就会直接被更新成上游的构建。用上面的安装命令重装一次，即可切入本仓库的更新通道。

本机发版（不走 GitHub Actions，`<tag>` 填下一个版本号）：

```bash
scripts/publish-release.sh <tag>
```

这个脚本按 WSL 环境写：Windows 侧的 zip 用 `cargo.exe` 打，找不到 `gh` 时回退到 `/mnt/c/Program Files/GitHub CLI/gh.exe`；换机器用 `WIN_CARGO` / `GH` 覆盖这两个路径。

## 快速开始

```bash
cc-switch                              # TUI
cc-switch --app claude provider list
cc-switch --app claude use <id>
cc-switch --app codex provider list
cc-switch --app codex use <id>
cc-switch auth list                    # ChatGPT / Codex OAuth 账号
cc-switch settings codex-history show  # 统一会话历史开关
cc-switch env check                    # 环境变量冲突检查
cc-switch env tools                    # 本地 CLI 工具检测
cc-switch proxy show                   # 可选本地代理
```

`--app` 仅支持：`claude`（默认）、`codex`、`hermes`、`pi`。

仅 Unix：

```bash
cc-switch start claude <id>
cc-switch start codex <id>
cc-switch start codex <id> --shared-sessions
```

`start` 只影响这一次启动，不改全局当前供应商；`--shared-sessions` 走统一的 `custom` 历史桶。

在 TUI 里添加供应商时的模板：Claude 有 Custom、Claude Official、Codex（Codex OAuth）三项；Codex 有 Custom、OpenAI Official；两者外加 Pi 都有内置预设 DeepSeek、智谱 GLM、MiniMax、小米 MiMo、OpenRouter；Hermes 只有 Custom。赞助商条目列表在本 fork 已清空。（命令行的 `provider add --template` 选择面更窄：Claude / Codex 只多一个 DeepSeek，Pi 和 Hermes 只有 Custom。）

TUI 里 `?` 是上下文帮助，`esc` 返回；`p` 在 Claude / Codex 下开关本地代理（Hermes / Pi 不接代理）。

## 从源码编译

需要 Rust 1.91.1+（仓库内 `rust-toolchain.toml` 固定 1.91.1，rustup 会自动装上）：

```bash
git clone https://github.com/FengBujue0104/cc-switch-cli-fbj.git
cd cc-switch-cli-fbj/src-tauri
cargo build --release
```

Linux 发版包：

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

## 许可证

MIT。原作者 Jason Young；CLI 分支 saladday；本 fork FengBujue0104。
