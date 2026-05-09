<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>面向个人的、本地优先的工作记录与复盘工具。</strong>
</p>

<p align="center">
  自动整理你一天里使用过的应用、访问过的网站、窗口标题和可选截图记录，生成一条可回看、可追问的工作时间线。
</p>

<p align="center">
  所有数据默认仅保存在本地设备，不上传任何服务器。AI 功能完全可选；关闭后照常使用。
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

## 它适合做什么

Work Review 适合个人用户用来回答这些问题：

- 我今天到底做了什么？
- 这几天主要在推进什么？
- 某个任务大概花了多少时间？
- 我当时看过哪个页面、哪个窗口、哪些上下文？
- 今天的日报怎么快速整理出来？

它的重点不是"监督"，而是帮助你**回忆、整理和复盘**自己的工作过程。

---

## 核心特点

- **自动记录工作轨迹** — 自动整理前台应用、网站访问、窗口标题、可选截图和 OCR 文本，尽量减少手动补记和事后回忆
- **一条统一的工作时间线** — 概览、时间线、工作助手、日报共用同一份底层记录，既能看统计，也能追到具体页面和上下文
- **直接回答工作问题** — 基于本地记录回答"今天做了什么""最近主要在推进什么""有哪些待办"
- **日报生成与导出** — 结构化日报、历史回看、Markdown 导出与自动导出、AI 增强下的附加提示词和段落级编辑
- **桌面化身 Beta** — 迈向个人工作 Agent 的第一步，未来将成为能感知工作上下文、主动提醒和辅助决策的桌面伙伴
- **隐私优先，本地可控** — 数据存本地 SQLite；AI 默认可不启用，模型调用使用你自己的 API Key，不经第三方中转

---

## 界面预览

<p align="center">
  <img src="docs/Introduction_zh/概览.png" alt="概览" width="720" />
</p>

<p align="center">
  <img src="docs/Introduction_zh/助手.png" alt="工作助手" width="720" />
</p>

---

## 隐私与边界

Work Review 从设计上面向个人使用，不适用于：员工监控 · 团队考勤 · 绩效考核 · 隐形追踪

你可以按需控制记录范围：

- 按应用设置为「正常 / 脱敏 / 忽略」，脱敏模式自动跳过截图和 OCR
- 敏感关键词自动过滤 · 域名黑名单
- 锁屏自动暂停 · 手动暂停/恢复
- AI 仅在你主动配置模型后启用，默认关闭

---

## 主要功能

### 自动记录

- 自动识别前台应用，记录使用时长、窗口标题和分类
- 识别浏览器 URL，按站点/页面聚合访问记录
- 定时截图并提取 OCR 文本，支持多屏策略
- 键鼠 + 屏幕空闲检测，减少挂机误记
- 时间线回看某个时段的具体上下文

### 智能整理

- 工作助手：基于本地记录做问答，支持多模型切换
- 自动识别"昨天""本周""最近 N 天"等自然语言时间范围
- 碎片活动聚合为连续工作 Session
- 从页面、窗口标题和上下文中提炼可能的后续待办
- 基础模板与 AI 增强两种回答模式

### 日报与复盘

- 生成结构化日报，支持历史回看
- Markdown 导出与自动导出
- 按小时活跃度汇总
- AI 增强下的附加提示词与段落编辑
- 网站语义分类：修改域名分类后自动回填历史
- 多段工作时间：如上午 + 下午，休息时间不计入

---

## AI 模式

Work Review 的核心始终是**本地记录**。AI 的作用是让记录更容易阅读和复盘，而不是使用前提。

| 模式 | 说明 |
|------|------|
| **基础模板** | 零配置，输出稳定的结构化结果 |
| **AI 增强** | 调用你自行配置的模型服务，让问答和总结更自然 |

支持的提供商：Ollama (本地) / OpenAI 兼容 / DeepSeek / 通义千问 / 智谱 / Kimi / 豆包 / MiniMax / SiliconFlow / Gemini / Claude

---

## 快速开始

1. 从 [Releases](https://github.com/wm94i/Work_Review/releases/latest) 下载对应平台安装包
2. macOS 需授予屏幕录制、辅助功能权限
3. 保持后台运行一段时间
4. 回到概览 / 时间线 / 日报查看当天记录

| 平台 | 安装包 |
|------|--------|
| macOS (Apple Silicon / Intel) | `.dmg` |
| Windows | `.exe` |
| Linux x86_64 (X11 / Wayland) | `.deb` / `.AppImage` |
| Linux ARM64 (aarch64) | `.deb` |

**macOS：** 截图需「屏幕录制」权限，桌宠联动需「辅助功能 + 输入监控」。首次提示"已损坏"时：`sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"`

**Windows：** 依赖 Microsoft Edge WebView2 Runtime。

**Linux：** 截图和窗口追踪依赖当前会话类型与工具链。<details><summary>依赖说明</summary>

```bash
# 基础
sudo apt install xprintidle tesseract-ocr
# X11
sudo apt install xdotool x11-utils scrot
# Wayland: gdbus (GNOME) / kdotool (KDE) / swaymsg (Sway) / hyprctl (Hyprland)
# 截图: grim / gnome-screenshot / spectacle
```

</details>

---

## 扩展能力（Beta）

<details>
<summary>桌面化身</summary>

用独立桌宠窗口反馈待机/办公/阅读/会议/音乐/视频等状态。

<img src="docs/桌宠.png" alt="桌面化身" width="220" />

当前仍在持续完善中，会继续补齐交互联动、表情和预设细节。

</details>

<details>
<summary>Bot 联动（Telegram / 飞书）</summary>

通过本地 API + 多设备注册，从 Telegram / 飞书远程查询记录与生成日报。支持命令：`/devices`、`/report`、`/generate` 等。仅限个人和本人多设备联动使用。

</details>

<details>
<summary>MCP Server</summary>

通过 stdio 协议将工作记录接入 AI 编码工具（Claude Code / Cursor / VS Code Copilot 等）。

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

## 开发

```bash
npm install
npm run tauri:dev    # 开发
npm run tauri:build  # 构建
```

要求：Node.js 18+ / Rust stable / Tauri 2 CLI · 技术栈：Tauri 2 + Rust + Svelte 4 + SQLite

---

## 社区交流

<p align="center"><strong>微信群</strong></p>

<p align="center">
  <img src="docs/group/vx.jpg" alt="微信群" width="220" />
</p>

<p align="center"><small>如果二维码失效，关注下方公众号获取最新进群方式，或者进 TG 群吐槽</small></p>

---

<p align="center"><strong>公众号</strong></p>

<p align="center">
  <img src="docs/group/gzh.jpg" alt="公众号" width="220" />
</p>

---

<p align="center">
  <a href="https://t.me/+stYJLlkZbDYwM2Rl"><img src="https://img.shields.io/badge/Telegram-加入群组-26A5E4?style=flat-square&logo=telegram&logoColor=white" alt="Telegram"></a>
</p>

## 致谢

- 感谢 [linux.do](https://linux.do/) 社区的交流与讨论支持
- 桌面化身 BongoCat 资源改编自 [ayangweb/BongoCat](https://github.com/ayangweb/BongoCat) (MIT License)，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)

## License

MIT

---

## 历史星标

<a href="https://www.star-history.com/#wm94i/Work_Review&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
  </picture>
</a>
