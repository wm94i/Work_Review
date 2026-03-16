# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.7] - 2026-03-16

### 修复
- 修复 macOS 睡眠/Power Nap 唤醒时产生幽灵活动记录的问题
  - 新增 CGSessionCopyCurrentDictionary 锁屏检测（系统级 API，最可靠）
  - 解决合盖睡眠期间 WPS Office 等应用被错误记录使用时长
- 修复时段摘要中卡片显示"1分钟"但文本显示"0分钟"的矛盾
  - 后端摘要文本时长改为向上取整，与前端 Math.round() 对齐

## [1.0.6] - 2026-03-15

### 新增
- 视频画中画（PiP）小窗使用时间统计
  - 通过 macOS `CGWindowListCopyWindowInfo` 检测浮动窗口（layer > 0）
  - 自动识别腾讯视频、爱奇艺、QuickTime Player 等应用的 PiP 小窗
  - 浮动窗口时长独立累计，不与前台应用重复计时
  - 自动排除系统进程（Dock、控制中心等）和过小窗口
  - 受隐私过滤规则约束，支持脱敏和跳过
- 自定义背景图片功能
  - 设置页新增「外观」标签，支持上传自定义背景图
  - 可调节背景不透明度和模糊程度，实时预览
  - 图片自动压缩（最大 1920px 宽，JPEG 格式）
- 设置页新增「AI 模型」独立标签页

### 修复
- 修复 README 中 Releases 下载链接指向错误仓库名的问题

### 优化
- 设置页 UI 精简：标题栏和按钮样式统一，去除多余渐变和阴影
- 概览页、时间线、报告等页面样式微调
- 侧边栏布局和交互优化
- 关于页按钮和技术栈标签样式统一

## [1.0.5] - 2026-03-13

### 新增
- 开机自启动：设置页新增「开机自启动」开关，支持 macOS（LaunchAgent）和 Windows（注册表）
  - 集成 `tauri-plugin-autostart` 官方插件
  - 初始化时自动查询系统实际状态并同步

### 修复
- 修复侧边栏左下角版本号与关于页不一致的问题
  - 所有版本号展示统一通过 Tauri `getVersion()` API 动态读取
  - 版本唯一来源为 `tauri.conf.json`，不再硬编码

### 优化
- AI 模型设置 UI 重构
  - 模式选择从大卡片改为紧凑分段按钮
  - 表单从混合网格统一为单列布局
  - 测试按钮与提供商选择同行，操作更直觉
  - 非 AI 模式时折叠配置面板，减少干扰

## [1.0.4] - 2026-03-13

### 新增
- 多 AI 提供商支持：新增 Claude、Gemini API，统一 AI 调用接口
- macOS 构建支持 ApplicationServices 框架

### 修复
- 修复小时统计 24 点溢出问题
- 修复 macOS 编译错误（`AppError` 导入缺失）

### 优化
- 截图兼容性和监控模块稳定性优化
- 数据库查询与存储逻辑优化
- 存储管理清理策略改进

## [1.0.3] - 2026-03-12

### 新增
- 智能空闲检测：自动解决"应用挂着不用但时长继续累加"的问题
  - 两阶段检测机制：
    - 第一阶段：键鼠活动检测（Windows: `GetLastInputInfo`，macOS: `CGEventSourceSecondsSinceLastEventType`）
    - 第二阶段：屏幕内容变化检测（截图哈希比对）
  - 智能判断逻辑：
    - 有键鼠操作 → 活跃，正常计时
    - 无键鼠操作但屏幕有变化 → 活跃（终端运行程序、视频播放等场景）
    - 无键鼠操作且屏幕连续 3 次无变化 → 空闲，暂停计时
  - 跨平台支持：Windows 和 macOS 双平台完整实现
  - 无需配置：固定 3 分钟阈值，自动工作，用户无感知

### 修复
- 修复 Windows 10 截图和应用统计不工作的问题
  - 新增 GDI BitBlt 截图备用方案，当 Windows Graphics Capture API 失败时自动降级
  - Windows 11 优先使用 WGC API（高性能），Windows 10 自动切换到 GDI 方案（兼容性优先）
  - 解决了 Win10 与 Win11 截图 API 不兼容导致的记录缺失问题

### 优化
- 新增国产浏览器 URL 获取支持：360 安全/极速浏览器、QQ 浏览器、搜狗浏览器、2345 浏览器、猎豹浏览器、傲游浏览器等
- Windows 窗口控制按钮改为原生风格：右上角方形按钮（最小化/最大化/关闭），不再使用 macOS 风格的圆形按钮

## [1.0.2] - 2026-03-11

### 修复
- 修复 Windows 10 不被记录的问题
  - 改用 `PROCESS_QUERY_LIMITED_INFORMATION` 替代高权限标志，覆盖 UAC 保护进程和 Microsoft Store 应用
  - 增加 `QueryFullProcessImageNameW` 和窗口标题推断两级备用方案，基本消除 Unknown 进程记录
  - 新增 `winbase` feature 支持
- 修复活动总时长严重偏低的问题（Unknown 进程时长全部堆积到同一条记录）

### 优化
- 概览页布局优化：网站访问和应用使用区块始终渲染，消除加载时的 layout shift
- 关于页删除无意义的"数据安全保障"说明卡片
- 侧边栏 Logo 区域视觉优化（图标和间距收紧）

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
