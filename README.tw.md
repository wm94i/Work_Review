<p align="center">
  <img src="src-tauri/icons/icon.png" width="100" alt="Work Review">
</p>

<h1 align="center">Work Review</h1>

<p align="center">
  <strong>面向個人使用的本地工作軌跡記錄器。</strong>
</p>

<p align="center">
  <a href="./README.md">簡體中文</a> · <a href="./README.tw.md">繁體中文</a> · <a href="./README.en.md">English</a>
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

Work Review 會在後台持續記錄你當天使用過的應用、訪問過的網站、關鍵窗口和屏幕內容，再把這些離散片段整理成一條**可回看、可追問、可復盤**的工作軌跡。

- 不需要手動打卡，也不用事後回憶今天幹了什麼
- 概覽、時間線、日報、工作助手共用同一份底層記錄
- 既能看統計，也能直接追到具體頁面、窗口標題和上下文截圖
- 支持簡體中文 / English / 繁體中文三種界面語言，日報會按當前語言分別生成與切換
- 支持輕量模式、按小時活躍度視圖、日報 Markdown 導出和多屏截圖策略切換
- 現在還提供 `桌面化身 Beta`，用更輕量的桌寵狀態反饋陪你工作

> 全部數據本地存儲，不上傳任何服務器。AI 功能可選，關掉也完全可用。

---

## 這是什麼

它不是傳統意義上的考勤工具，也不是只會堆數字的時間統計器。

Work Review 更像一套面向個人的工作留痕系統：

- 自動沉澱工作軌跡：應用、網站、截圖、OCR、時段摘要都會被串起來
- 快速回答工作問題：一句“今天做了什麼”“這週主要在推進什麼”就能直接得到結果
- 為復盤而不是監控設計：重點是幫助你回憶、整理、復用，而不是製造額外負擔

---

## 核心能力

### 自動記錄

| 記錄維度 | 說明 |
|---------|------|
| 應用追踪 | 自動識別前台應用，記錄使用時長、窗口標題和分類 |
| 網站追踪 | 識別瀏覽器 URL，按瀏覽器 / 站點 / 頁面聚合瀏覽記錄 |
| 屏幕留痕 | 定時截圖並提取 OCR 文本，支持按活動窗口所在屏幕或整桌面拼接截圖 |
| 空閒檢測 | 鍵鼠 + 屏幕雙重判斷，盡量避免掛機時間被誤記為工作 |
| 歷史回看 | 通過時間線回放當日軌跡，定位具體時段與上下文 |

### 智能分析

| 能力 | 說明 |
|-----|------|
| 工作助手 | 基於你的真實記錄做問答，適合回答“今天做了什麼”“最近在推進什麼” |
| 時間範圍識別 | 自動理解“昨天”“本週”“最近 3 天”等自然語言時間範圍 |
| Session 聚合 | 把碎片活動整理為連續工作段，更容易看出完整工作節奏 |
| 待辦提取 | 從訪問頁面、窗口標題和上下文裡提煉可能的後續事項 |
| 日報生成 | 生成結構化日報，支持歷史回看、按小時活躍度摘要、AI 附加提示詞、Markdown 導出，以及按當前語言切換日報版本 |
| 雙模式回答 | 可選基礎模板或 AI 增強，兼顧零配置和表達質量 |
| 桌面化身 Beta（持續完善中） | 用獨立桌寵窗口反饋待機 / 辦公 / 閱讀 / 會議 / 音樂 / 視頻 / 生成中等狀態，當前仍在持續補齊互動與預設細節 |

### 隱私控制

- 按應用設置 `正常 / 脫敏 / 忽略`
- 敏感關鍵詞自動過濾
- 域名黑名單
- 鎖屏自動暫停
- 手動暫停 / 恢復

### 使用控制

- 支持輕量模式：關閉主界面後可釋放主 Webview，僅保留後台記錄和托盤
- 支持在時間線內直接修改應用默認分類，並回填該應用的歷史記錄
- 支持遷移本地數據目錄，並在遷移後清理舊目錄中的應用託管數據

---

## 界面預覽

先看界面，再看能力，會更容易建立對產品的整體認知。

### 今日概覽

<img src="docs/Introduction_tw/概览.png" alt="Work Review 今日概覽" />

概覽頁會把當天的總時長、辦公時長、瀏覽器使用、網站訪問、按小時活躍度和應用分佈放在同一屏裡，適合先快速判斷今天的工作重心。

### 工作助手

<img src="docs/Introduction_tw/助手.png" alt="Work Review 工作助手" />

助手頁直接基於你的本地記錄回答問題，適合拿來做當天回顧、階段總結和待辦梳理。

### 桌面化身 Beta

<img src="docs/桌宠.png" alt="Work Review 桌面化身 Beta" width="220" />

桌面化身會以獨立桌寵的形式懸浮在桌面上，用輕量狀態和短氣泡反饋當前節奏。它更適合做陪伴式感知，而不是信息面板。

- 支持待機、辦公、閱讀、開會、聽歌、視頻、生成中、摸魚等狀態
- 支持桌寵大小與貓體透明度調節
- 當前仍處於 `Beta` 階段，會繼續優化切換速度、互動聯動、表情和視覺細節

---

## 頁面結構

| 頁面 | 做什麼 |
|------|-------|
| **概覽** | 聚合今天的總時長、辦公時長、瀏覽器時長、網站訪問、按小時活躍度和應用分佈 |
| **時間線** | 逐時段回看窗口、截圖、OCR 文本和頁面訪問軌跡，並可修正應用分類 |
| **助手** | 用自然語言直接提問，讓記錄變成可消費的信息 |
| **日報** | 查看、生成和回看任意日期的日報內容，支持按當前界面語言切換日報版本，並支持附加提示詞和 Markdown 導出 |
| **設置** | 管理記錄策略、模型、隱私規則、桌面化身、輕量模式、存儲位置和更新行為 |

---

## AI 模型

Work Review 的核心始終是**本地記錄**，AI 的作用是把這些記錄變得更容易閱讀、搜索和復盤。

| 模式 | 說明 |
|------|------|
| **基礎模板** | 零配置即可使用，直接輸出穩定的結構化結果 |
| **AI 增強** | 調用你自己的模型服務，讓總結、問答和復盤更自然 |

支持的提供商：Ollama (本地) / OpenAI 兼容 / DeepSeek / 通義千問 / 智譜 / Kimi / 豆包 / MiniMax / SiliconFlow / Gemini / Claude

> 不啟用 AI 時，記錄、概覽、時間線、工作助手（基礎模式）和基礎模板日報全部正常可用。

> 日報附加提示詞僅在 `AI 增強` 模式下生效。

> `Ollama` 提供商支持直接刷新本機模型列表；如果模型未出現在下拉列表中，仍可手動輸入模型名稱。

---

## 安裝

從 [Releases](https://github.com/wm94i/Work_Review/releases/latest) 下載最新版。

| 平台 | 安裝包 |
|------|--------|
| macOS Apple Silicon | `.dmg` |
| macOS Intel | `.dmg` |
| Windows | `.exe` |
| Linux (X11 / 主流 Wayland) | `.deb` / `.AppImage` |

- `Windows`：截圖和桌寵聯動預設不需要額外系統隱私授權。
- `Linux`：通常不需要額外系統隱私授權，但截圖和桌寵聯動依賴目前會話類型與 provider / 工具鏈是否齊全。
- `macOS`：時間線截圖需要 `螢幕錄製`，桌寵鍵鼠聯動需要 `輔助功能 + 輸入監控`。

<details>
<summary>Linux 依賴</summary>

基礎依賴：

```bash
sudo apt install xprintidle tesseract-ocr
```

X11 額外依賴：

```bash
sudo apt install xdotool x11-utils
```

X11 截圖工具（至少安裝一個）：

```bash
sudo apt install scrot        # 推薦
# 或替代方案：
sudo apt install maim          # 輕量替代
sudo apt install imagemagick   # 提供 import 命令
```

Wayland 常見 provider / 工具：

```bash
# GNOME
gdbus
# 桌寵鍵鼠聯動還需要安裝倉庫內 GNOME Shell 擴充：
# scripts/gnome-shell/work-review-avatar-input@workreview.app

# KDE Plasma
kdotool

# Sway
swaymsg

# Hyprland
hyprctl

# Wayland 截圖工具（至少安裝一個）
grim / gnome-screenshot / spectacle
```

> **說明：** `loginctl`、`dbus-send`、`pgrep` 在多數發行版中已預裝。
> Linux 現在支持 `X11` 與主流 `Wayland` provider 鏈（GNOME / KDE Plasma / Sway / Hyprland）。
> Linux 下瀏覽器 URL 已恢復為**最佳努力鏈路**：
> Firefox / Zen / LibreWolf / Waterfox 優先走 sessionstore；
> Chromium 系仍主要依賴窗口標題提取與最近記錄兜底，不等同於 macOS 的強能力採集。

</details>

<details>
<summary>macOS 首次打開提示"已損壞"？</summary>

```bash
sudo xattr -rd com.apple.quarantine "/Applications/Work Review.app"
```

然後到 `系統設置 > 隱私與安全性` 檢查以下權限：

- `螢幕錄製`：時間線截圖必需
- `輔助功能`：讀取前台窗口狀態必需
- `輸入監控`：桌寵鍵盤和滑鼠聯動必需

如果剛安裝新版本後發現截圖突然沒有了，或者桌寵不再回應鍵鼠，先確認這些權限仍然指向目前這份 `Work Review.app`。
</details>

---

## 技術棧

| 層級 | 技術 |
|------|------|
| 桌面框架 | Tauri 2 |
| 後端 | Rust |
| 前端 | Svelte 4 + Vite |
| 樣式 | Tailwind CSS |
| 存儲 | SQLite |

---

## 開發

```bash
npm install
npm run tauri:dev    # 開發
npm run tauri:build  # 構建
```

要求：Node.js 18+ / Rust stable / Tauri 2 CLI

```text
src/                  Svelte 前端
src/routes/           頁面（概覽 / 時間線 / 問答 / 日報 / 設置）
src/lib/              組件、store、工具函數
src-tauri/src/        Rust 後端（監控、數據庫、分析、隱私、更新）
```

---

## 相關文檔

- [CHANGELOG.md](CHANGELOG.md)
- [docs/WINDOWS_OCR.md](docs/WINDOWS_OCR.md)

## 社群交流

### 微信群

<p align="center">
  <img src="docs/group/vx.png" alt="Work Review 微信群 QR code" width="280" />
</p>

> 若群 QR code 失效，可先關注下方公眾號，再按公眾號內指引進群。

### 公眾號

<p align="center">
  <img src="docs/group/gzh.jpg" alt="Work Review 公眾號 QR code" width="280" />
</p>

> 微信群 QR code 失效時，可透過公眾號取得最新進群方式。

### Telegram

[![Telegram](https://img.shields.io/badge/Telegram-加入群組-26A5E4?style=flat-square&logo=telegram&logoColor=white)](https://t.me/+stYJLlkZbDYwM2Rl)

## 致謝

感謝 [linux.do](https://linux.do/) 社區的交流與討論支持。

- 桌面化身中的 BongoCat 互動資源與部分視覺素材改編自 [ayangweb/BongoCat](https://github.com/ayangweb/BongoCat)，上游專案採用 MIT License。相關歸屬與許可說明見 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## License

MIT

---

## 歷史星標

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
