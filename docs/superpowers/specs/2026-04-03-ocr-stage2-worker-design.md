# OCR 第二阶段常驻 Worker 设计

**目标**

将当前 `PaddleOCR` 的“一次识别启动一次 Python 进程”改为“按需启动、进程内复用的 Python 常驻 worker”，避免重复启动解释器和重复初始化 Paddle 模型。

**范围**

- 保持 [ocr.rs](/Users/wmy/data/Pycharm_Project/Work_Review/src-tauri/src/ocr.rs) 对外接口不变
- 仅替换 `extract_with_paddle` 的内部实现方式
- 不增加前端设置项
- 不改变默认引擎选择策略

**非目标**

- 不把默认 macOS / Windows OCR 切换到 Paddle
- 不新增独立守护进程
- 不实现 C++ / ONNX Runtime 版本
- 不修改数据库、前端协议和截图流程

## 当前问题

当前 `extract_with_paddle` 的路径是：

1. Rust 每次 OCR 启动一个 Python 子进程
2. Python 每次导入 `paddleocr`
3. 每次重新初始化 `PaddleOCR()`
4. 识别完成后进程退出

这导致每次请求都承担解释器启动和模型加载成本。

## 方案概述

第二阶段改为“单例 worker + 按需启动 + 异常自恢复”：

1. Rust 侧维护一个全局 `PaddleWorkerClient`
2. 第一次真正需要 `PaddleOCR` 时启动 Python worker
3. worker 启动时只初始化一次 `PaddleOCR`
4. 后续 OCR 请求通过 `stdin/stdout` 的 JSON 行协议发送
5. worker 异常退出时，Rust 侧自动重启一次并重试当前请求

## 组件设计

### 1. `PaddleWorkerClient`

职责：

- 启动 worker 进程
- 持有子进程 `stdin`
- 在后台线程持续读取 `stdout`
- 通过通道把响应返回给同步调用方
- 负责关闭、重启和路径切换

### 2. 全局 worker 管理

使用进程内全局单例管理当前 worker：

- 如果 `python_cmd` 和 `script_path` 未变化，复用现有 worker
- 如果数据目录变化导致脚本路径变化，停止旧 worker，换新 worker
- 如果 worker 已崩溃，重新启动

这样可以兼容数据目录迁移和进程内多次创建 `OcrService` 的现状。

### 3. Python worker 协议

使用 `json line` 协议：

- Rust -> Python：`{"image_path": "..."}`
- Python -> Rust：`{"text": "...", "boxes": [...], "confidence": 0.9, "error": null}`

worker 启动成功后，先输出一行：

- `{"status":"ready"}`

Rust 收到后才把该 worker 视为可用。

### 4. 脚本模式

`paddle_ocr.py` 同时支持两种模式：

- 单次模式：兼容当前直接调用方式
- `--worker` 模式：进入常驻循环并复用初始化后的 OCR 实例

### 5. 错误处理

- 启动超时：返回错误并沿用现有回退逻辑
- 当前请求超时：杀掉 worker，并重启一次重试
- worker 输出非法 JSON：视为 worker 损坏，重启一次
- worker 返回业务错误：记录日志并沿用现有回退逻辑

## 测试策略

第二阶段优先覆盖：

- worker 只在首次请求时启动
- 后续请求复用同一个 worker
- worker 崩溃后能自动重启并完成一次重试
- 当脚本路径变化时，会替换旧 worker

测试不依赖真实 Python/Paddle 环境，而是使用可执行的假 worker 脚本模拟协议行为。

## 后续演进

完成第二阶段后，再根据实际收益评估：

1. 是否引入更轻的本地高精度引擎
2. 是否推进 C++ / ONNX 常驻推理版本
