<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>你的私人工作黑匣子 — 自动记录、智能回顾、一键复盘。</strong>
</p>

<p align="center">
  <a href="https://github.com/wm94i/Work_Review/releases/latest">
    <img src="https://img.shields.io/github/v/release/wm94i/Work_Review?style=flat-square&color=blue" alt="Release">
  </a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/github/license/wm94i/Work_Review?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Rust-orange?style=flat-square" alt="Stack">
</p>

---

Work Review 在后台安静地记录你正在使用的应用、访问的网页和屏幕内容，然后帮你把这些碎片拼成**可回溯的工作轨迹**。

- 不需要手动打卡，不需要填表
- 问一句"今天做了什么"就能拿到答案
- 日报、周报、复盘，数据都替你备好了

> 全部数据本地存储，不上传任何服务器。AI 功能可选，关掉也完全可用。

---

## 核心能力

### 自动记录

| 记录维度 | 说明 |
|---------|------|
| 应用追踪 | 自动检测前台应用，记录使用时长和窗口标题 |
| 网页追踪 | 识别浏览器 URL，按域名 / 页面 / 浏览器聚合统计 |
| 屏幕截图 | 定时截图 + OCR 文字提取，留存工作现场 |
| 空闲检测 | 键鼠 + 屏幕双重检测，挂机不计时 |

### 智能分析

| 能力 | 说明 |
|-----|------|
| 工作助手 | 对话式问答，问"今天做了什么""上周总结""最近有什么待办"直接出结果 |
| 时间范围识别 | 自然语言自动提取时间，"昨天""本周""最近3天"无需手动选日期 |
| Session 聚合 | 将零散活动合并为连续工作段，识别深度工作时段 |
| 意图识别 | 自动归类工作方向（开发 / 设计 / 沟通 / 阅读...） |
| 待办提取 | 从工作轨迹中提炼可能的跟进事项 |
| AI 日报 | 基于当日数据生成结构化工作总结，支持历史日期回看 |

### 隐私控制

- 按应用设置 `正常 / 脱敏 / 忽略`
- 敏感关键词自动过滤
- 域名黑名单
- 锁屏自动暂停
- 手动暂停 / 恢复

---

## 页面一览

| 页面 | 做什么 |
|------|-------|
| **概览** | 今日统计仪表盘 — 总时长、工作时长、应用和网站分布 |
| **时间线** | 按时间轴回看活动、截图、窗口标题和小时摘要 |
| **问答** | 对话式工作助手 — 输入问题，得到基于数据的回答 |
| **日报** | 查看 / 生成任意日期的工作日报 |
| **设置** | 常规、AI 模型、外观、隐私、存储 |

---

## AI 模型

Work Review 的核心是**本地记录**，AI 只负责让数据更好读。

| 模式 | 说明 |
|------|------|
| **基础模板** | 零配置，直接输出结构化统计，不依赖任何外部服务 |
| **AI 增强** | 调用你配置的模型，生成更自然的总结和对话回答 |

支持的提供商：Ollama (本地) / OpenAI 兼容 / DeepSeek / 通义千问 / 智谱 / Kimi / 豆包 / SiliconFlow / Gemini / Claude

> 不启用 AI 时，记录、概览、时间线、工作助手（基础模式）全部正常可用。

---

## 安装

从 [Releases](https://github.com/wm94i/Work_Review/releases/latest) 下载最新版。

| 平台 | 安装包 |
|------|--------|
| macOS Apple Silicon | `.dmg` |
| macOS Intel | `.dmg` |
| Windows | `.exe` |

<details>
<summary>macOS 首次打开提示"已损坏"？</summary>

```bash
sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"
```

然后到 `系统设置 > 隐私与安全性 > 屏幕录制` 为 Work Review 开启权限。
</details>

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 后端 | Rust |
| 前端 | Svelte 4 + Vite |
| 样式 | Tailwind CSS |
| 存储 | SQLite |

---

## 开发

```bash
npm install
npm run tauri:dev    # 开发
npm run tauri:build  # 构建
```

要求：Node.js 18+ / Rust stable / Tauri 2 CLI

```text
src/                  Svelte 前端
src/routes/           页面（概览 / 时间线 / 问答 / 日报 / 设置）
src/lib/              组件、store、工具函数
src-tauri/src/        Rust 后端（监控、数据库、分析、隐私、更新）
```

---

## 相关文档

- [CHANGELOG.md](CHANGELOG.md)
- [docs/WINDOWS_OCR.md](docs/WINDOWS_OCR.md)

## 友链

- [linux.do](https://linux.do/)

## License

MIT
