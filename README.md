# Codex 余量条 / Codex Usage Bar

> A tiny Windows companion that docks to the right side of Codex Desktop and shows your Codex usage at a glance.
>
> 一个贴在 Codex Desktop 右侧的 Windows 余量提醒工具，用来快速查看 5 小时窗口和周限额。

<p align="center">
  <img src="docs/images/hero.svg" alt="Codex Usage Bar preview" width="860">
</p>

<p align="center">
  <a href="#中文说明">中文</a> ·
  <a href="#english">English</a> ·
  <a href="#privacy">Privacy</a> ·
  <a href="#development">Development</a>
</p>

## 中文说明

**Codex 余量条** 是一个 Windows 桌面小组件。它会在 Codex Desktop 处于前台时吸附到窗口右侧，显示 Codex 的 5 小时会话余量和周限额余量。

这个项目基于 [Finesssee/Win-CodexBar](https://github.com/Finesssee/Win-CodexBar) 改造，并保留上游 [CodexBar](https://github.com/steipete/CodexBar) 的 provider 思路。当前版本只保留 Codex 相关内容，减少多 provider 带来的复杂度。

### 特性

- 只显示 Codex，界面更轻。
- 自动吸附到 Codex Desktop 右侧。
- Codex 失焦、最小化或关闭时自动隐藏。
- 展示 5 小时窗口和周限额。
- 中文界面，默认显示剩余额度。
- 本地运行，不上传聊天内容、token 或账号数据。

### 安装

从 GitHub Releases 下载 Windows 安装包：

```text
Codex 余量条_<version>_x64-setup.exe
```

安装后启动 **Codex 余量条**，再打开 Codex Desktop。余量条会在 Codex 窗口右侧出现。

### 使用建议

- 如果余量条没有出现，先确认 Codex Desktop 是当前前台窗口。
- 如果数据为空或不准确，优先检查 Codex 是否已经登录。
- 本工具会尽量读取 Codex 可用的真实用量信息；当真实接口不可用时，会保留诚实降级，不把本地估算伪装成官方账单。

### 免责声明

本项目不是 OpenAI 官方产品，也不隶属于 OpenAI。Codex、OpenAI 等名称归其各自所有者所有。

## English

**Codex Usage Bar** is a small Windows desktop companion for Codex Desktop. When Codex is the active foreground window, the bar docks to its right side and shows the remaining 5-hour session quota and weekly quota.

This project is derived from [Finesssee/Win-CodexBar](https://github.com/Finesssee/Win-CodexBar), which itself ports ideas from [CodexBar](https://github.com/steipete/CodexBar). This fork is intentionally Codex-only.

### Features

- Codex-only surface.
- Docks to the right side of Codex Desktop.
- Hides when Codex is minimized, inactive, or closed.
- Shows 5-hour session and weekly quota.
- Chinese-first UI with remaining-quota semantics.
- Local-first: no chat content, token, or account data is uploaded by this app.

### Install

Download the Windows installer from GitHub Releases:

```text
Codex 余量条_<version>_x64-setup.exe
```

Install it, launch **Codex 余量条**, then bring Codex Desktop to the foreground.

### Notes

- If the bar does not appear, make sure Codex Desktop is the foreground window.
- If usage data is unavailable, make sure Codex Desktop is signed in.
- The app attempts to use live Codex usage data where available. If that data is unavailable, fallback states should be explicit and honest rather than presented as official billing data.

## Privacy

The app is designed as a local Windows companion:

- It does not upload your Codex conversations.
- It does not intentionally log secrets or tokens.
- It reads local Codex/account state only to derive usage display.
- Release builds should avoid printing credential values in logs or UI.

<p align="center">
  <img src="docs/images/architecture.svg" alt="Architecture overview" width="780">
</p>

## Development

Prerequisites:

- Node.js
- pnpm
- Rust toolchain
- Windows with WebView2

Build and check:

```powershell
cd apps/desktop-tauri
pnpm install
pnpm run build
cd ..\..
cargo check
```

Create a Windows installer:

```powershell
cd apps/desktop-tauri
pnpm exec tauri build --bundles nsis
```

The generated installer is placed under:

```text
target/release/bundle/nsis/
```

## Project Lineage

This project stands on the shoulders of:

- [Finesssee/Win-CodexBar](https://github.com/Finesssee/Win-CodexBar)
- [steipete/CodexBar](https://github.com/steipete/CodexBar)

Please keep upstream attribution when redistributing derived builds.

## License

MIT. See [LICENSE](LICENSE).
