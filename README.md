<div align="center">
  <img src="src/renderer/assets/logo.png" width="96" height="96" alt="Chrome Profile Manage logo">

  <h1>Chrome Profile Manage</h1>

  <p>
    A lightweight Windows desktop workspace manager for isolated Chrome profiles.
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
</div>

## Overview

Chrome Profile Manage is a local desktop application for creating and managing isolated Chrome workspaces. Each environment owns an independent Chrome `User Data` directory, allowing separate cookies, sessions, extension state, cache, and browser preferences.

The application is built with Tauri v2 and uses the system WebView instead of bundling Chromium, keeping the desktop shell small while delegating browser isolation to Chrome's native startup flags.

## Features

- Create isolated Chrome environments backed by independent `--user-data-dir` directories
- Synchronize a template Chrome Profile into new or existing environments
- Launch environments with a dedicated startup URL
- Configure the storage directory used for newly created profiles
- Edit environment name and startup URL inline with autosave
- Open an environment's profile directory from the table
- Delete managed environments with a confirmation step and path safety checks
- Switch between light and dark themes
- Use a compact icon-first interface for routine workspace operations

## How It Works

Chrome Profile Manage stores each environment as a standalone Chrome `User Data` directory. When an environment is launched, Chrome receives:

```text
--user-data-dir=<environment-user-data-dir>
--profile-directory=Default
```

Template synchronization copies the selected source Profile into the environment's `Default` Profile directory. This preserves Chrome's expected directory layout while allowing users to choose source profiles such as `Default`, `Profile 1`, or `Profile 3`.

## Screens and Actions

The main environment table provides these actions:

| Action | Description |
| --- | --- |
| Launch | Start Chrome using the selected isolated environment |
| Sync Template | Copy the configured template Profile into the environment |
| Open Profile | Open the environment's profile directory in Explorer |
| Delete | Delete the managed environment and its profile directory |

Global settings include:

- Chrome executable path
- Template Profile path
- Profile storage path for new environments
- Default startup URL

## Installation

Download the latest release from:

```text
https://github.com/gorkys/chromeProfileManage/releases
```

Release assets:

- `ChromeProfileManage.exe` - portable executable
- `ChromeProfileManage-setup.exe` - Windows installer

## Development

### Prerequisites

- Windows
- Node.js
- Rust
- Microsoft Edge WebView2 Runtime

### Install Dependencies

```powershell
npm install
```

### Run In Development

```powershell
npm run dev
```

### Build

```powershell
npm run build
```

Build outputs are created under:

```text
src-tauri\target\release\
src-tauri\target\release\bundle\nsis\
```

## Project Structure

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

## Data And Safety

- Application configuration is stored in the Tauri application data directory
- New profiles are created under the configured profile storage path
- Existing environments keep their original profile path when the global storage path changes
- Deletion is restricted to managed profile roots to reduce accidental data loss
- Running Chrome instances can lock profile files; close the target environment before synchronizing or deleting it

## Technology

- [Tauri v2](https://tauri.app/)
- Rust
- HTML, CSS, and vanilla JavaScript
- Chrome `--user-data-dir` and `--profile-directory`

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
