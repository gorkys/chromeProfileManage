# Chrome Profile Manage

Windows 本地 Chrome Profile 环境工作台，用于创建、同步、启动多个相互隔离的 Chrome 工作环境。它不是简单的浏览器多开工具，而是面向多账号、多店铺、多项目的本地工作环境生成器。

## 特性

- 基于 Chrome `--user-data-dir` 创建独立浏览器空间
- 支持从母版 Profile 同步配置、插件和浏览器状态
- 支持创建时同步母版，也支持对已有环境再次同步母版
- 支持为每个环境配置启动网页
- 支持修改新环境的 Profile 保存路径
- 环境名称和启动网页双击编辑并自动保存
- 删除环境时同步删除本工具创建的 Profile 文件夹，并进行二次确认
- 提供深色和浅色主题
- 使用 Tauri v2 构建，体积轻，不捆绑 Chromium

## 界面

左侧为固定图标栏，包含：

- 环境管理
- 全局设置
- 深浅主题切换

环境列表使用表格展示，操作列包含：

- 启动环境
- 同步母版
- 打开 Profile 文件夹
- 删除环境

## 母版 Profile

母版 Profile 是用来复制浏览器工作状态的来源目录。Windows 上 Chrome 默认 Profile 通常位于：

```text
C:\Users\<用户名>\AppData\Local\Google\Chrome\User Data
```

常见具体 Profile 目录：

```text
Default
Profile 1
Profile 2
Profile 3
```

建议在全局设置中选择具体 Profile 目录，例如：

```text
C:\Users\king\AppData\Local\Google\Chrome\User Data\Profile 3
```

同步母版时，程序会把母版 Profile 内容复制到目标环境的 `Default` Profile 中，并在启动 Chrome 时使用：

```text
--user-data-dir=<环境 User Data 目录>
--profile-directory=Default
```

## 关于油猴脚本同步

Tampermonkey 等扩展的脚本数据通常存放在 Profile 内的扩展数据目录中，例如：

- `Extensions`
- `Local Extension Settings`
- `IndexedDB`

因此母版 Profile 同步通常可以带上油猴插件和脚本数据。为提高成功率，请在同步前关闭母版 Chrome 窗口，避免运行中的 Chrome 占用或延迟写入 Profile 数据。

## 技术栈

- Tauri v2
- Rust
- 原生 HTML / CSS / JavaScript
- Chrome `--user-data-dir`

## 开发环境

需要安装：

- Node.js
- Rust
- Windows WebView2 Runtime

安装依赖：

```powershell
npm install
```

启动开发模式：

```powershell
npm run dev
```

## 打包

```powershell
npm run build
```

常见产物：

```text
src-tauri\target\release\chrome-manage.exe
src-tauri\target\release\bundle\nsis\Chrome 环境工作台_0.1.0_x64-setup.exe
```

## 项目结构

```text
src/
  renderer/
    index.html
    renderer.js
    styles.css
    assets/
src-tauri/
  src/main.rs
  icons/
  tauri.conf.json
```

## 数据存储

应用配置和默认 Profile 环境保存在 Tauri 应用数据目录下。也可以在全局设置中修改新建环境的 Profile 保存路径。

已有环境会继续使用创建时记录的 Profile 路径，不会因为全局保存路径变化而自动迁移。

## 安全说明

- 删除环境只允许删除本工具管理目录下的 Profile，避免误删系统 Chrome Profile
- 删除前会二次确认
- 如果目标环境 Chrome 正在运行，删除可能失败，请先关闭对应 Chrome 窗口

## License

MIT
