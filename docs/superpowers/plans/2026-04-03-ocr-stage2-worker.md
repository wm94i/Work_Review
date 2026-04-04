# OCR Stage 2 Worker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `PaddleOCR` 的一次一进程实现替换为 Python 常驻 worker，并保持现有 OCR 对外接口不变。

**Architecture:** 在 [ocr.rs](/Users/wmy/data/Pycharm_Project/Work_Review/src-tauri/src/ocr.rs) 内新增全局 `PaddleWorkerClient` 和 `json line` 协议，`extract_with_paddle` 改为通过常驻 worker 请求识别结果，启动失败或运行异常时保留现有回退逻辑。

**Tech Stack:** Rust、标准进程/线程/通道、Python、PaddleOCR

---

### Task 1: 为常驻 worker 行为补失败测试

**Files:**
- Modify: `src-tauri/src/ocr.rs`
- Test: `src-tauri/src/ocr.rs`

- [ ] **Step 1: 写 worker 复用与重启的失败测试**

```rust
#[test]
fn paddle_worker应在多次请求间复用单个进程() {
    reset_paddle_worker_for_tests();
    let fixture = MockWorkerFixture::new();

    let client = acquire_paddle_worker_for_tests(&fixture.python_path, &fixture.script_path).unwrap();
    let first = client.request(Path::new("/tmp/a.png")).unwrap().unwrap();
    let second = client.request(Path::new("/tmp/b.png")).unwrap().unwrap();

    assert!(first.text.contains("mock"));
    assert!(second.text.contains("mock"));
    assert_eq!(fixture.start_count(), 1);
}

#[test]
fn paddle_worker崩溃后应自动重启一次并完成重试() {
    reset_paddle_worker_for_tests();
    let fixture = MockWorkerFixture::crash_once_then_ok();

    let result = request_paddle_via_worker_for_tests(&fixture.python_path, &fixture.script_path, Path::new("/tmp/a.png"))
        .unwrap()
        .unwrap();

    assert!(result.text.contains("recovered"));
    assert_eq!(fixture.start_count(), 2);
}
```

- [ ] **Step 2: 运行测试并确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ocr::tests -- --nocapture`
Expected: 新测试因缺少 worker 管理代码而失败。

### Task 2: 实现 Python 常驻 worker

**Files:**
- Modify: `src-tauri/src/ocr.rs`
- Test: `src-tauri/src/ocr.rs`

- [ ] **Step 1: 为 `paddle_ocr.py` 增加 `--worker` 模式**

```python
def run_worker():
    ocr = build_ocr()
    print(json.dumps({"status": "ready"}), flush=True)
    for line in sys.stdin:
        request = json.loads(line)
        if request.get("command") == "shutdown":
            break
        print(json.dumps(run_ocr(ocr, request["image_path"]), ensure_ascii=False), flush=True)
```

- [ ] **Step 2: 在 Rust 侧实现 `PaddleWorkerClient`**

```rust
struct PaddleWorkerClient {
    python_cmd: String,
    script_path: PathBuf,
    child: Child,
    stdin: ChildStdin,
    response_rx: Receiver<String>,
}
```

- [ ] **Step 3: 实现全局单例获取与重启逻辑**

```rust
fn with_paddle_worker<T>(python_cmd: &str, script_path: &Path, f: impl FnOnce(&mut PaddleWorkerClient) -> Result<T>) -> Result<T> {
    // 相同配置复用，不同配置替换，异常时重启一次
}
```

- [ ] **Step 4: 改造 `extract_with_paddle` 走 worker**

```rust
fn extract_with_paddle(&self, image_path: &Path) -> Result<Option<OcrResult>> {
    with_paddle_worker(&python_cmd, script_path, |worker| worker.request(image_path))
}
```

- [ ] **Step 5: 运行 OCR 单测并确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ocr::tests -- --nocapture`
Expected: `ocr::tests` 全部通过。

### Task 3: 回归验证

**Files:**
- Modify: `src-tauri/src/ocr.rs`（如需整理）

- [ ] **Step 1: 运行截图与主回归测试**

Run: `cargo test --manifest-path src-tauri/Cargo.toml screenshot::tests -- --nocapture`
Expected: `screenshot::tests` 全部通过。

Run: `cargo test --manifest-path src-tauri/Cargo.toml tests:: -- --nocapture`
Expected: 现有主测试全部通过。

- [ ] **Step 2: 格式化并做最终校验**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml`
Expected: 无报错。

Run: `cargo test --manifest-path src-tauri/Cargo.toml ocr::tests screenshot::tests tests:: -- --nocapture`
Expected: 目标测试全部通过。
