<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>面向個人的、本地優先的工作記錄與復盤工具。</strong>
</p>

<p align="center">
  自動整理你一天裡使用過的應用、訪問過的網站、窗口標題和可選截圖記錄，生成一條可回看、可追問的工作時間線。
</p>

<p align="center">
  所有數據默認僅保存在本地設備，不上傳任何伺服器。AI 功能完全可選；關閉後照常使用。
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

## 它適合做什麼

Work Review 適合個人用戶用來回答這些問題：

- 我今天到底做了什麼？
- 這幾天主要在推進什麼？
- 某個任務大概花了多少時間？
- 我當時看過哪個頁面、哪個窗口、哪些上下文？
- 今天的日報怎麼快速整理出來？

它的重點不是「監督」，而是幫助你**回憶、整理和復盤**自己的工作過程。

---

## 核心特點

- **自動記錄工作軌跡** — 自動整理前台應用、網站訪問、窗口標題、可選截圖和 OCR 文本，盡量減少手動補記和事後回憶
- **一條統一的工作時間線** — 概覽、時間線、工作助手、日報共用同一份底層記錄，既能看統計，也能追到具體頁面和上下文
- **直接回答工作問題** — 基於本地記錄回答「今天做了什麼」「最近主要在推進什麼」「有哪些待辦」
- **日報生成與匯出** — 結構化日報、歷史回看、Markdown 匯出與自動匯出、AI 增強下的附加提示詞和段落級編輯
- **桌面化身 Beta** — 邁向個人工作 Agent 的第一步，未來將成為能感知工作上下文、主動提醒和輔助決策的桌面夥伴
- **隱私優先，本地可控** — 數據存本地 SQLite；AI 默認可不啟用，模型調用使用你自己的 API Key，不經第三方中轉

---

## 界面預覽

<p align="center">
  <img src="docs/Introduction_zh/概览.png" alt="概覽" width="720" />
</p>

<p align="center">
  <img src="docs/Introduction_zh/助手.png" alt="工作助手" width="720" />
</p>

---

## 隱私與邊界

Work Review 從設計上面向個人使用，不適用於：員工監控 · 團隊考勤 · 績效考核 · 隱形追蹤

你可以按需控制記錄範圍：

- 按應用設置為「正常 / 脫敏 / 忽略」，脫敏模式自動跳過截圖和 OCR
- 敏感關鍵詞自動過濾 · 域名黑名單
- 鎖屏自動暫停 · 手動暫停/恢復
- AI 僅在你主動配置模型後啟用，默認關閉

---

## 主要功能

### 自動記錄

- 自動識別前台應用，記錄使用時長、窗口標題和分類
- 識別瀏覽器 URL，按站點/頁面聚合訪問記錄
- 定時截圖並提取 OCR 文本，支持多屏策略
- 鍵鼠 + 螢幕空閒檢測，減少掛機誤記
- 時間線回看某個時段的具體上下文

### 智能整理

- 工作助手：基於本地記錄做問答，支持多模型切換
- 自動識別「昨天」「本週」「最近 N 天」等自然語言時間範圍
- 碎片活動聚合為連續工作 Session
- 從頁面、窗口標題和上下文中提煉可能的後續待辦
- 基礎模板與 AI 增強兩種回答模式

### 日報與復盤

- 生成結構化日報，支持歷史回看
- Markdown 匯出與自動匯出
- 按小時活躍度彙總
- AI 增強下的附加提示詞與段落編輯
- 網站語義分類：修改域名分類後自動回填歷史
- 多段工作時間：如上午 + 下午，休息時間不計入

---

## AI 模式

Work Review 的核心始終是**本地記錄**。AI 的作用是讓記錄更容易閱讀和復盤，而不是使用前提。

| 模式 | 說明 |
|------|------|
| **基礎模板** | 零配置，輸出穩定的結構化結果 |
| **AI 增強** | 調用你自行配置的模型服務，讓問答和總結更自然 |

支持的提供商：Ollama (本地) / OpenAI 兼容 / DeepSeek / 通義千問 / 智譜 / Kimi / 豆包 / MiniMax / SiliconFlow / Gemini / Claude

---

## 快速開始

1. 從 [Releases](https://github.com/wm94i/Work_Review/releases/latest) 下載對應平台安裝包
2. macOS 需授予螢幕錄製、輔助功能權限
3. 保持後台運行一段時間
4. 回到概覽 / 時間線 / 日報查看當天記錄

| 平台 | 安裝包 |
|------|--------|
| macOS (Apple Silicon / Intel) | `.dmg` |
| Windows | `.exe` |
| Linux x86_64 (X11 / Wayland) | `.deb` / `.AppImage` |
| Linux ARM64 (aarch64) | `.deb` |

**macOS：** 截圖需「螢幕錄製」權限，桌寵聯動需「輔助功能 + 輸入監控」。首次提示"已損壞"時：`sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"`

**Windows：** 依賴 Microsoft Edge WebView2 Runtime。

**Linux：** 截圖和窗口追蹤依賴當前會話類型與工具鏈。<details><summary>依賴說明</summary>

```bash
# 基礎
sudo apt install xprintidle tesseract-ocr
# X11
sudo apt install xdotool x11-utils scrot
# Wayland: gdbus (GNOME) / kdotool (KDE) / swaymsg (Sway) / hyprctl (Hyprland)
# 截圖: grim / gnome-screenshot / spectacle
```

</details>

---

## 擴展能力（Beta）

<details>
<summary>桌面化身</summary>

用獨立桌寵窗口反饋待機/辦公/閱讀/會議/音樂/視頻等狀態。

<img src="docs/桌宠.png" alt="桌面化身" width="220" />

當前仍在持續完善中，會繼續補齊交互聯動、表情和預設細節。

</details>

<details>
<summary>Bot 聯動（Telegram / 飛書）</summary>

通過本地 API + 多設備註冊，從 Telegram / 飛書遠端查詢記錄與生成日報。支持命令：`/devices`、`/report`、`/generate` 等。僅限個人和本人多設備聯動使用。

</details>

<details>
<summary>MCP Server</summary>

通過 stdio 協議將工作記錄接入 AI 編碼工具（Claude Code / Cursor / VS Code Copilot 等）。

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

## 開發

```bash
npm install
npm run tauri:dev    # 開發
npm run tauri:build  # 構建
```

要求：Node.js 18+ / Rust stable / Tauri 2 CLI · 技術棧：Tauri 2 + Rust + Svelte 4 + SQLite

---

## 社群交流

<p align="center"><strong>微信群</strong></p>

<p align="center">
  <img src="docs/group/vx.jpg" alt="微信群" width="220" />
</p>

<p align="center"><small>如果二維碼失效，關注下方公眾號獲取最新進群方式，或者進 TG 群吐槽</small></p>

---

<p align="center"><strong>公眾號</strong></p>

<p align="center">
  <img src="docs/group/gzh.jpg" alt="公眾號" width="220" />
</p>

---

<p align="center">
  <a href="https://t.me/+stYJLlkZbDYwM2Rl"><img src="https://img.shields.io/badge/Telegram-加入群組-26A5E4?style=flat-square&logo=telegram&logoColor=white" alt="Telegram"></a>
</p>

## 致謝

- 感謝 [linux.do](https://linux.do/) 社區的交流與討論支持
- 桌面化身 BongoCat 資源改編自 [ayangweb/BongoCat](https://github.com/ayangweb/BongoCat) (MIT License)，詳見 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)

## License

MIT

---

## 歷史星標

<a href="https://www.star-history.com/#wm94i/Work_Review&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=wm94i/Work_Review&type=Date" />
  </picture>
</a>
