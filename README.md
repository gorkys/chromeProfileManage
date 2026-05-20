<div align="center">
  <img src="src/renderer/assets/logo.png" width="104" height="104" alt="Chrome Profile Manage logo">

  <h1>Chrome Profile Manage</h1>

  <p>
    Lightweight Chrome profile workspace manager for Windows.
    <br>
    轻量级 Windows 本地 Chrome Profile 工作环境管理器。
  </p>

  <p>
    <a href="https://github.com/gorkys/chromeProfileManage/releases">
      <img alt="Release" src="https://img.shields.io/github/v/release/gorkys/chromeProfileManage?style=flat-square">
    </a>
    <a href="LICENSE">
      <img alt="License" src="https://img.shields.io/github/license/gorkys/chromeProfileManage?style=flat-square">
    </a>
    <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-blue?style=flat-square">
    <img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square">
  </p>

  <p>
    <a href="#english">English</a> ·
    <a href="#中文">中文</a>
  </p>
</div>

---

## English

Chrome Profile Manage is a local desktop application for creating and managing isolated Chrome workspaces. Each workspace owns an independent Chrome `User Data` directory, so sessions, cookies, extension state, cache, and browser preferences stay separated by environment.

It is built with Tauri v2 and uses the system WebView instead of bundling Chromium. Chrome isolation is handled by Chrome's native startup flags.

### Features

- Create isolated Chrome environments backed by dedicated `--user-data-dir` directories
- Synchronize a template Chrome Profile into new or existing environments
- Launch a workspace with a configured startup URL
- Configure the storage directory used for newly created profiles
- Edit environment name and startup URL inline with autosave
- Open a managed profile directory from the environment table
- Delete managed environments with confirmation and path safety checks
- Switch between light and dark themes
- Use a compact icon-first interface for frequent actions

### How It Works

Each environment is stored as a standalone Chrome `User Data` directory. When launching Chrome, the app passes:

```text
--user-data-dir=<environment-user-data-dir>
--profile-directory=Default
```

Template synchronization copies the selected source Profile into the target environment's `Default` Profile directory. This keeps Chrome's expected directory structure while still allowing template sources such as `Default`, `Profile 1`, or `Profile 3`.

> [!NOTE]
> Close the source or target Chrome window before synchronizing or deleting a profile. Running Chrome instances may lock profile files or delay writes.

### Actions

| Action | Description |
| --- | --- |
| Launch | Start Chrome with the selected isolated workspace |
| Sync Template | Copy the configured template Profile into the selected workspace |
| Open Profile | Open the workspace profile directory in Windows Explorer |
| Delete | Delete the managed workspace and its profile directory |

### Installation

Download the latest version from the [Releases](https://github.com/gorkys/chromeProfileManage/releases) page.

Assets:

- `ChromeProfileManage.exe` - portable executable
- `ChromeProfileManage-setup.exe` - Windows installer

### Development

Prerequisites:

- Windows
- Node.js
- Rust
- Microsoft Edge WebView2 Runtime

Install dependencies:

```powershell
npm install
```

Run locally:

```powershell
npm run dev
```

Build:

```powershell
npm run build
```

Build outputs:

```text
src-tauri\target\release\
src-tauri\target\release\bundle\nsis\
```

### Project Structure

```text
.
├─ src/
│  └─ renderer/
│     ├─ assets/
│     ├─ index.html
│     ├─ renderer.js
│     └─ styles.css
├─ src-tauri/
│  ├─ icons/
│  ├─ src/
│  │  └─ main.rs
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ package.json
└─ README.md
```

### Data And Safety

- Application settings are stored in the Tauri application data directory
- New environments are created under the configured profile storage path
- Existing environments keep their original profile path if the global storage path changes
- Deletion is restricted to managed profile roots to reduce accidental data loss

---

## 中文

Chrome Profile Manage 是一个 Windows 本地桌面应用，用于创建和管理相互隔离的 Chrome 工作环境。每个环境都拥有独立的 Chrome `User Data` 目录，因此登录状态、Cookie、扩展状态、缓存和浏览器偏好可以按环境隔离保存。

项目基于 Tauri v2 构建，使用系统 WebView，不额外捆绑 Chromium。浏览器隔离能力由 Chrome 原生启动参数提供。

### 功能特性

- 基于独立 `--user-data-dir` 创建隔离的 Chrome 环境
- 支持将母版 Chrome Profile 同步到新环境或已有环境
- 支持为每个环境配置启动网页
- 支持配置新建环境的 Profile 保存路径
- 环境名称和启动网页支持表格内编辑并自动保存
- 支持从环境表格打开对应 Profile 文件夹
- 删除环境时带确认提示和路径安全校验
- 支持浅色/深色主题切换
- 使用紧凑的图标式界面，适合高频操作

### 工作原理

每个环境都会保存为一个独立的 Chrome `User Data` 目录。启动 Chrome 时，应用会传入：

```text
--user-data-dir=<环境 User Data 目录>
--profile-directory=Default
```

同步母版时，应用会将选中的来源 Profile 复制到目标环境的 `Default` Profile 目录中。这样既保持 Chrome 期望的目录结构，也允许使用 `Default`、`Profile 1`、`Profile 3` 等具体 Profile 作为母版来源。

> [!NOTE]
> 同步或删除 Profile 前，建议先关闭相关 Chrome 窗口。运行中的 Chrome 可能会锁定 Profile 文件，或延迟写入配置数据。

### 主要操作

| 操作 | 说明 |
| --- | --- |
| 启动 | 使用选中的隔离环境启动 Chrome |
| 同步母版 | 将全局设置中的母版 Profile 复制到当前环境 |
| 打开 Profile | 在资源管理器中打开当前环境的 Profile 目录 |
| 删除 | 删除当前受管环境及其 Profile 目录 |

### 安装

前往 [Releases](https://github.com/gorkys/chromeProfileManage/releases) 下载最新版本。

发布产物：

- `ChromeProfileManage.exe` - 免安装版本
- `ChromeProfileManage-setup.exe` - Windows 安装包

### 开发

环境要求：

- Windows
- Node.js
- Rust
- Microsoft Edge WebView2 Runtime

安装依赖：

```powershell
npm install
```

本地运行：

```powershell
npm run dev
```

构建：

```powershell
npm run build
```

构建产物：

```text
src-tauri\target\release\
src-tauri\target\release\bundle\nsis\
```

### 项目结构

```text
.
├─ src/
│  └─ renderer/
│     ├─ assets/
│     ├─ index.html
│     ├─ renderer.js
│     └─ styles.css
├─ src-tauri/
│  ├─ icons/
│  ├─ src/
│  │  └─ main.rs
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ package.json
└─ README.md
```

### 数据与安全

- 应用配置保存在 Tauri 应用数据目录
- 新建环境会保存在全局设置指定的 Profile 保存路径下
- 修改全局保存路径不会自动迁移已有环境
- 删除操作仅允许删除受管 Profile 根目录下的环境，降低误删风险
