<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>面向个人使用的本地工作轨迹记录器。</strong>
</p>

<p align="center">
  <a href="./README.md">中文</a> · <a href="./README.tw.md">繁體中文</a> · <a href="./README.en.md">English</a>
</p>

<p align="center">
  <a href="https://github.com/wm94i/Work_Review/releases/latest">
    <img src="https://img.shields.io/github/v/release/wm94i/Work_Review?style=flat-square&color=blue" alt="Release">
  </a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux%20(X11%20%7C%20Wayland)-blue?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/github/license/wm94i/Work_Review?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Rust-orange?style=flat-square" alt="Stack">
</p>

---

Work Review 会在后台持续记录你当天使用过的应用、访问过的网站、关键窗口和屏幕内容，再把这些离散片段整理成一条**可回看、可追问、可复盘**的工作轨迹。

- 不需要手动打卡，也不用事后回忆今天干了什么
- 概览、时间线、日报、工作助手共用同一份底层记录
- 既能看统计，也能直接追到具体页面、窗口标题和上下文截图
- 支持简体中文 / English / 繁體中文三种界面语言，日报会按当前语言分别生成与切换
- 支持轻量模式、按小时活跃度视图、日报 Markdown 导出和多屏截图策略切换
- 现在还提供 `桌面化身 Beta`，用更轻量的桌宠状态反馈陪你工作

> 全部数据本地存储，不上传任何服务器。AI 功能可选，关掉也完全可用。

---

## 这是什么

它不是传统意义上的考勤工具，也不是只会堆数字的时间统计器。

Work Review 更像一套面向个人的工作留痕系统：

- 自动沉淀工作轨迹：应用、网站、截图、OCR、时段摘要都会被串起来
- 快速回答工作问题：一句“今天做了什么”“这周主要在推进什么”就能直接得到结果
- 为复盘而不是监控设计：重点是帮助你回忆、整理、复用，而不是制造额外负担

---

## 核心能力

### 自动记录

| 记录维度 | 说明 |
|---------|------|
| 应用追踪 | 自动识别前台应用，记录使用时长、窗口标题和分类 |
| 网站追踪 | 识别浏览器 URL，按浏览器 / 站点 / 页面聚合浏览记录 |
| 屏幕留痕 | 定时截图并提取 OCR 文本，支持按活动窗口所在屏幕或整桌面拼接截图 |
| 空闲检测 | 键鼠 + 屏幕双重判断，尽量避免挂机时间被误记为工作 |
| 历史回看 | 通过时间线回放当日轨迹，定位具体时段与上下文 |

### 智能分析

| 能力 | 说明 |
|-----|------|
| 工作助手 | 基于你的真实记录做问答，适合回答“今天做了什么”“最近在推进什么” |
| 时间范围识别 | 自动理解“昨天”“本周”“最近 3 天”等自然语言时间范围 |
| Session 聚合 | 把碎片活动整理为连续工作段，更容易看出完整工作节奏 |
| 待办提取 | 从访问页面、窗口标题和上下文里提炼可能的后续事项 |
| 日报生成 | 生成结构化日报，支持历史回看、按小时活跃度摘要、AI 附加提示词、Markdown 导出，以及按当前语言切换日报版本 |
| 双模式回答 | 可选基础模板或 AI 增强，兼顾零配置和表达质量 |
| 桌面化身 Beta | 用独立桌宠窗口反馈待机 / 办公 / 阅读 / 会议 / 音乐 / 视频 / 生成中等状态 |

### 隐私控制

- 按应用设置 `正常 / 脱敏 / 忽略`
- 敏感关键词自动过滤
- 域名黑名单
- 锁屏自动暂停
- 手动暂停 / 恢复

### 使用控制

- 支持轻量模式：关闭主界面后可释放主 Webview，仅保留后台记录和托盘
- 支持在时间线内直接修改应用默认分类，并回填该应用的历史记录
- 支持迁移本地数据目录，并在迁移后清理旧目录中的应用托管数据

---

## 界面预览

先看界面，再看能力，会更容易建立对产品的整体认知。

### 今日概览

<img src="docs/Introduction_zh/概览.png" alt="Work Review 今日概览" />

概览页会把当天的总时长、办公时长、浏览器使用、网站访问、按小时活跃度和应用分布放在同一屏里，适合先快速判断今天的工作重心。

### 工作助手

<img src="docs/Introduction_zh/助手.png" alt="Work Review 工作助手" />

助手页直接基于你的本地记录回答问题，适合拿来做当天回顾、阶段总结和待办梳理。

### 桌面化身 Beta

<img src="docs/桌宠.png" alt="Work Review 桌面化身 Beta" width="220" />

桌面化身会以独立桌宠的形式悬浮在桌面上，用轻量状态和短气泡反馈当前节奏。它更适合做陪伴式感知，而不是信息面板。

- 支持待机、办公、阅读、开会、听歌、视频、生成中、摸鱼等状态
- 支持桌宠大小与猫体透明度调节
- 当前仍处于 `Beta` 阶段，会继续优化切换速度、表情和视觉细节

---

## 页面结构

| 页面 | 做什么 |
|------|-------|
| **概览** | 聚合今天的总时长、办公时长、浏览器时长、网站访问、按小时活跃度和应用分布 |
| **时间线** | 逐时段回看窗口、截图、OCR 文本和页面访问轨迹，并可修正应用分类 |
| **助手** | 用自然语言直接提问，让记录变成可消费的信息 |
| **日报** | 查看、生成和回看任意日期的日报内容，支持按当前界面语言切换日报版本，并支持附加提示词和 Markdown 导出 |
| **设置** | 管理记录策略、模型、隐私规则、桌面化身、轻量模式、存储位置和更新行为 |

---

## AI 模型

Work Review 的核心始终是**本地记录**，AI 的作用是把这些记录变得更容易阅读、搜索和复盘。

| 模式 | 说明 |
|------|------|
| **基础模板** | 零配置即可使用，直接输出稳定的结构化结果 |
| **AI 增强** | 调用你自己的模型服务，让总结、问答和复盘更自然 |

支持的提供商：Ollama (本地) / OpenAI 兼容 / DeepSeek / 通义千问 / 智谱 / Kimi / 豆包 / MiniMax / SiliconFlow / Gemini / Claude

> 不启用 AI 时，记录、概览、时间线、工作助手（基础模式）和基础模板日报全部正常可用。

> 日报附加提示词仅在 `AI 增强` 模式下生效。

> `Ollama` 提供商支持直接刷新本机模型列表；如果模型未出现在下拉列表中，仍可手动输入模型名称。

---

## 安装

从 [Releases](https://github.com/wm94i/Work_Review/releases/latest) 下载最新版。

| 平台 | 安装包 |
|------|--------|
| macOS Apple Silicon | `.dmg` |
| macOS Intel | `.dmg` |
| Windows | `.exe` |
| Linux (X11 / 主流 Wayland) | `.deb` / `.AppImage` |

<details>
<summary>Linux 依赖</summary>

基础依赖：

```bash
sudo apt install xprintidle tesseract-ocr
```

X11 额外依赖：

```bash
sudo apt install xdotool x11-utils
```

X11 截图工具（至少安装一个）：

```bash
sudo apt install scrot        # 推荐
# 或替代方案：
sudo apt install maim          # 轻量替代
sudo apt install imagemagick   # 提供 import 命令
```

Wayland 常见 provider / 工具：

```bash
# GNOME
gdbus                                # 通常随系统提供

# KDE Plasma
kdotool

# Sway
swaymsg

# Hyprland
hyprctl

# Wayland 截图工具（至少安装一个）
grim / gnome-screenshot / spectacle
```

> **说明：** `loginctl`、`dbus-send`、`pgrep` 在多数发行版中已预装。
> Linux 当前支持 `X11` 以及主流 `Wayland` provider 链（GNOME / KDE Plasma / Sway / Hyprland）。
> Linux 下浏览器 URL 已恢复为**最佳努力链路**：
> Firefox / Zen / LibreWolf / Waterfox 优先走 sessionstore；
> Chromium 系仍主要依赖窗口标题提取与最近记录兜底，不等同于 macOS 的强能力采集。

</details>

<details>
<summary>macOS 首次打开提示"已损坏"？</summary>

```bash
sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"
```

然后到 `系统设置 > 隐私与安全性 > 屏幕录制` 为 Work Review 开启权限。

如果你要使用 `桌面化身 Beta`，还需要同时为 Work Review 开启 `辅助功能` 权限，以便读取前台窗口状态并及时切换桌宠表情。
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

## 微信群

欢迎来吐槽使用体验。

<p align="center">
  <img src="docs/group/vx.png" alt="Work Review 微信群二维码" width="280" />
</p>

> 二维码有效期较短，如果失效了，重新打开仓库最新版本的 README 查看即可。

## 致谢

感谢 [linux.do](https://linux.do/) 社区的交流与讨论支持。

## License

MIT

---

## 历史星标

<a href="https://www.star-history.com/#wm94i/Work_Review&Date">
  <picture>
    <source
      media="(prefers-color-scheme: dark)"
      srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date&theme=dark"
    />
    <source
      media="(prefers-color-scheme: light)"
      srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date"
    />
    <img
      alt="Star History Chart"
      src="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date"
    />
  </picture>
</a>
