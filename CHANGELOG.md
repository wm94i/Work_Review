# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.1] - 2026-03-11

### 修复
- 修复 Windows 10 启动后概况数据全为 0 的问题
  - 锁屏检测：使用 OpenInputDesktop 替代不可靠的 GetForegroundWindow/quser 判断
  - 截屏兼容：DrawBorderSettings::WithoutBorder 在 Win10 不支持时自动降级到 WithBorder
  - 窗口获取：GetForegroundWindow 返回 null 时降级为 Desktop，避免跳过轮询

### 优化
- 修复 Rust 后端编译警告
- 修复前端 A11y 可访问性警告
- Clippy 代码风格优化

## [1.0.0] 

### 核心功能
- 智能工作追踪：自动记录应用使用情况
- 活动合并：同一应用连续使用自动合并为单条记录
- 浏览器 URL 追踪：记录访问的网站
- 时间线视图：可视化查看每日工作轨迹
- AI 日报生成：支持本地 Ollama 和云端 API

### 隐私保护
- 应用规则：跳过、模糊或正常记录指定应用
- 敏感关键词过滤：自动过滤包含敏感词的活动
- 域名黑名单：不记录指定网站
- 锁屏检测：锁屏时自动暂停记录

### 平台支持
- macOS 10.13+：屏幕录制权限检测
- Windows 10+：完整功能支持

### 技术规格
- 后端：Rust + Tauri 2
- 前端：Svelte 4 + TailwindCSS
- 数据库：SQLite
