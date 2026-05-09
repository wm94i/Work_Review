<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>A personal, local-first work activity log and review tool.</strong>
</p>

<p align="center">
  Automatically organizes the apps you used, websites you visited, window titles, and optional screenshot records throughout your day into a reviewable, question-answerable work timeline.
</p>

<p align="center">
  All data is stored locally by default and never uploaded to any server. AI features are entirely optional; the app works fine with them turned off.
</p>

<p align="center">
  <a href="./README.md">中文</a> · <a href="./README.tw.md">繁體中文</a> · <a href="./README.en.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/wm94i/Work_Review/releases/latest">
    <img src="https://img.shields.io/github/v/release/wm94i/Work_Review?style=flat-square&color=blue" alt="Release">
  </a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/%F0%9F%94%92%20all%20data%20local-green?style=flat-square" alt="All Data Local">
  <img src="https://img.shields.io/github/license/wm94i/Work_Review?style=flat-square" alt="License">
</p>

---

## What Is It For

Work Review is designed for individuals who want to answer questions like these:

- What did I actually do today?
- What have I been focusing on over the past few days?
- Roughly how much time did a particular task take?
- Which pages, windows, and context did I look at back then?
- How can I quickly put together today's daily report?

The focus is not on "monitoring" but on helping you **recall, organize, and review** your own work process.

---

## Core Features

- **Automatic work trail recording** — Automatically captures foreground apps, website visits, window titles, optional screenshots, and OCR text, minimizing manual note-taking and after-the-fact recall
- **A unified work timeline** — Overview, timeline, work assistant, and daily report all share the same underlying records, letting you view statistics and drill down to specific pages and context
- **Direct answers to work questions** — Answers questions like "What did I do today?", "What have I been focusing on recently?", and "What are my pending items?" based on local records
- **Daily report generation and export** — Structured daily reports, historical review, Markdown export with auto-export, AI-enhanced prompt attachments and paragraph-level editing
- **Desktop Avatar Beta** — The first step toward a personal work Agent, evolving into a desktop partner that can sense work context, proactively remind, and assist with decisions
- **Privacy-first, locally controllable** — Data stored in local SQLite; AI can remain disabled by default, model calls use your own API Key without third-party relay

---

## Interface Preview

<p align="center">
  <img src="docs/Introduction_zh/概览.png" alt="Overview" width="720" />
</p>

<p align="center">
  <img src="docs/Introduction_zh/助手.png" alt="Work Assistant" width="720" />
</p>

---

## Privacy and Boundaries

Work Review is designed for personal use from the ground up. It is not intended for: employee monitoring · team attendance · performance evaluation · covert tracking

You can control the recording scope as needed:

- Per-app settings: Normal / Anonymize / Ignore — anonymize mode automatically skips screenshots and OCR
- Automatic sensitive keyword filtering · domain blacklist
- Auto-pause on screen lock · manual pause/resume
- AI only activates after you configure a model; disabled by default

---

## Main Features

### Automatic Recording

- Automatically detects the foreground app, tracking usage duration, window titles, and categories
- Identifies browser URLs and aggregates visit records by site/page
- Periodic screenshots with OCR text extraction, supporting multi-display strategies
- Keyboard/mouse + screen idle detection to reduce false records during idle time
- Timeline replay to review specific context from any time period

### Smart Organization

- Work assistant: Q&A based on local records, with multi-model switching support
- Automatically understands natural language time ranges like "yesterday", "this week", "last N days"
- Fragments grouped into continuous work sessions
- Extracts potential follow-up to-dos from pages, window titles, and context
- Two response modes: basic template and AI-enhanced

### Daily Reports and Review

- Generate structured daily reports with historical review
- Markdown export and automatic export
- Hourly activity summaries
- AI-enhanced prompt attachments and paragraph editing
- Website semantic categorization: changing a domain category automatically backfills history
- Multi-segment work time: e.g. morning + afternoon, break time excluded

---

## AI Modes

The core of Work Review is always **local recording**. AI's role is to make records easier to read and review, not a prerequisite for usage.

| Mode | Description |
|------|------|
| **Basic Template** | Zero configuration, outputs stable structured results |
| **AI Enhanced** | Calls your self-configured model service for more natural Q&A and summaries |

Supported providers: Ollama (local) / OpenAI compatible / DeepSeek / Qwen / Zhipu / Kimi / Doubao / MiniMax / SiliconFlow / Gemini / Claude

---

## Quick Start

1. Download the installer for your platform from [Releases](https://github.com/wm94i/Work_Review/releases/latest)
2. On macOS, grant Screen Recording and Accessibility permissions
3. Let it run in the background for a while
4. Check the Overview / Timeline / Daily Report to see your recorded activity

| Platform | Installer |
|------|--------|
| macOS (Apple Silicon / Intel) | `.dmg` |
| Windows | `.exe` |
| Linux x86_64 (X11 / Wayland) | `.deb` / `.AppImage` |
| Linux ARM64 (aarch64) | `.deb` |

**macOS:** Screenshots require the "Screen Recording" permission, and avatar linkage requires "Accessibility + Input Monitoring". If you see a "damaged" warning on first launch: `sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"`

**Windows:** Depends on Microsoft Edge WebView2 Runtime.

**Linux:** Screenshots and window tracking depend on the current session type and toolchain. <details><summary>Dependency details</summary>

```bash
# Base
sudo apt install xprintidle tesseract-ocr
# X11
sudo apt install xdotool x11-utils scrot
# Wayland: gdbus (GNOME) / kdotool (KDE) / swaymsg (Sway) / hyprctl (Hyprland)
# Screenshots: grim / gnome-screenshot / spectacle
```

</details>

---

## Extended Capabilities (Beta)

<details>
<summary>Desktop Avatar</summary>

Uses a standalone desktop pet window to reflect idle / working / reading / meeting / music / video states.

<img src="docs/桌宠.png" alt="Desktop Avatar" width="220" />

Still being actively refined — interaction linkage, expressions, and preset details will continue to improve.

</details>

<details>
<summary>Bot Integration (Telegram / Feishu)</summary>

Query records and generate daily reports remotely from Telegram / Feishu via local API + multi-device registration. Supported commands: `/devices`, `/report`, `/generate`, etc. Restricted to personal and own multi-device use only.

</details>

<details>
<summary>MCP Server</summary>

Connects work records to AI coding tools (Claude Code / Cursor / VS Code Copilot, etc.) via the stdio protocol.

```bash
cargo build --release -p work-review-mcp-server
```

```json
{
  "mcpServers": {
    "work-review": {
      "command": "/path/to/work-review-mcp-server",
      "env": {
        "WORK_REVIEW_DB_PATH": "/path/to/work_review.db",
        "WORK_REVIEW_CONFIG_PATH": "/path/to/config.json"
      }
    }
  }
}
```

</details>

---

## Development

```bash
npm install
npm run tauri:dev    # Development
npm run tauri:build  # Build
```

Requires: Node.js 18+ / Rust stable / Tauri 2 CLI · Tech stack: Tauri 2 + Rust + Svelte 4 + SQLite

---

## Community

<p align="center"><strong>WeChat Group</strong></p>

<p align="center">
  <img src="docs/group/vx.jpg" alt="WeChat Group" width="220" />
</p>

<p align="center"><small>If the QR code has expired, follow the official account below for the latest group invitation, or join the TG group</small></p>

---

<p align="center"><strong>WeChat Official Account</strong></p>

<p align="center">
  <img src="docs/group/gzh.jpg" alt="Official Account" width="220" />
</p>

---

<p align="center">
  <a href="https://t.me/+stYJLlkZbDYwM2Rl"><img src="https://img.shields.io/badge/Telegram-Join-26A5E4?style=flat-square&logo=telegram&logoColor=white" alt="Telegram"></a>
</p>

## Acknowledgements

- Thanks to the [linux.do](https://linux.do/) community for discussion and feedback
- Desktop Avatar BongoCat resources adapted from [ayangweb/BongoCat](https://github.com/ayangweb/BongoCat) (MIT License), see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)

## License

MIT

---

## Star History

<a href="https://www.star-history.com/#wm94i/Work_Review&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
  </picture>
</a>
