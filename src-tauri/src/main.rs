#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use tauri::Manager;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Environment {
    id: String,
    name: String,
    profile_path: String,
    #[serde(default = "default_profile_directory")]
    profile_directory: String,
    #[serde(default = "default_true")]
    managed: bool,
    start_url: String,
    #[serde(default)]
    extension_paths: Vec<String>,
    created_at: String,
    updated_at: String,
}

/// 控制同步母版时需要带入的浏览数据
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncOptions {
    /// 是否同步 Cookie 与登录态数据库
    sync_cookies: bool,
    /// 是否同步网站本地存储数据
    sync_site_storage: bool,
    /// 是否同步插件本体与插件配置
    sync_extensions: bool,
    /// 是否同步缓存数据
    sync_cache: bool,
    /// 是否同步浏览器会话恢复数据
    sync_sessions: bool,
    /// 是否同步浏览历史和下载记录
    sync_history: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncOptionsInput {
    sync_cookies: Option<bool>,
    sync_site_storage: Option<bool>,
    sync_extensions: Option<bool>,
    sync_cache: Option<bool>,
    sync_sessions: Option<bool>,
    sync_history: Option<bool>,
    exclude_cookies: Option<bool>,
    exclude_site_storage: Option<bool>,
    exclude_extensions: Option<bool>,
    exclude_cache: Option<bool>,
    exclude_sessions: Option<bool>,
    exclude_history: Option<bool>,
}

impl<'de> Deserialize<'de> for SyncOptions {
    /// 反序列化同步选项并兼容旧的排除字段
    /// deserializer：Serde 反序列化器
    /// 返回值：标准同步选项
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input = SyncOptionsInput::deserialize(deserializer)?;

        Ok(Self {
            sync_cookies: input
                .sync_cookies
                .or_else(|| input.exclude_cookies.map(|value| !value))
                .unwrap_or(true),
            sync_site_storage: input
                .sync_site_storage
                .or_else(|| input.exclude_site_storage.map(|value| !value))
                .unwrap_or(true),
            sync_extensions: input
                .sync_extensions
                .or_else(|| input.exclude_extensions.map(|value| !value))
                .unwrap_or(true),
            sync_cache: input
                .sync_cache
                .or_else(|| input.exclude_cache.map(|value| !value))
                .unwrap_or(true),
            sync_sessions: input
                .sync_sessions
                .or_else(|| input.exclude_sessions.map(|value| !value))
                .unwrap_or(true),
            sync_history: input
                .sync_history
                .or_else(|| input.exclude_history.map(|value| !value))
                .unwrap_or(true),
        })
    }
}

impl Default for SyncOptions {
    /// 生成默认同步选项
    /// 返回值：默认全部同步的同步选项
    fn default() -> Self {
        Self {
            sync_cookies: true,
            sync_site_storage: true,
            sync_extensions: true,
            sync_cache: true,
            sync_sessions: true,
            sync_history: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    chrome_path: String,
    master_profile_path: String,
    default_url: String,
    #[serde(default)]
    profile_storage_path: String,
    #[serde(default)]
    sync_options: SyncOptions,
    environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigPatch {
    chrome_path: Option<String>,
    master_profile_path: Option<String>,
    default_url: Option<String>,
    profile_storage_path: Option<String>,
    sync_options: Option<SyncOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateEnvironmentPayload {
    name: String,
    start_url: Option<String>,
    copy_master: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentPatch {
    name: Option<String>,
    start_url: Option<String>,
}

/// 返回默认 Chrome profile 名称
/// 返回值：Default
fn default_profile_directory() -> String {
    "Default".to_string()
}

/// 返回默认开启状态
/// 返回值：true
fn default_true() -> bool {
    true
}

/// 返回应用数据根目录，用于保存配置和 profile
/// app：Tauri 应用句柄
/// 返回值：可写入的本地数据目录
fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .or_else(|_| env::current_dir().map(|dir| dir.join("data")))
        .map_err(|error| format!("无法获取应用数据目录：{error}"))
}

/// 返回配置文件路径
/// app：Tauri 应用句柄
/// 返回值：config.json 的完整路径
fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("config.json"))
}

/// 返回默认环境 profile 根目录
/// app：Tauri 应用句柄
/// 返回值：默认 profiles 目录完整路径
fn default_profiles_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("profiles"))
}

/// 获取默认环境 profile 保存路径
/// app：Tauri 应用句柄
/// 返回值：默认 profile 保存路径文本
fn default_profile_storage_path(app: &tauri::AppHandle) -> Result<String, String> {
    Ok(default_profiles_dir(app)?.to_string_lossy().to_string())
}

/// 根据配置获取环境 profile 保存根目录
/// app：Tauri 应用句柄
/// config：当前配置对象
/// 返回值：环境 profile 保存根目录
fn profile_storage_dir(app: &tauri::AppHandle, config: &Config) -> Result<PathBuf, String> {
    let profile_storage_path = config.profile_storage_path.trim();

    if profile_storage_path.is_empty() {
        return default_profiles_dir(app);
    }

    Ok(PathBuf::from(profile_storage_path))
}

/// 确保配置目录存在
/// app：Tauri 应用句柄
/// 返回值：成功时无返回值
fn ensure_config_dir(app: &tauri::AppHandle) -> Result<(), String> {
    let path = data_dir(app)?;
    fs::create_dir_all(&path).map_err(|error| {
        format!(
            "创建配置目录失败：{}，路径：{}",
            error,
            path.display()
        )
    })
}

/// 确保环境 profile 保存目录存在
/// app：Tauri 应用句柄
/// config：当前配置对象
/// 返回值：可写入的环境 profile 保存目录
fn ensure_profile_storage_dir(app: &tauri::AppHandle, config: &Config) -> Result<PathBuf, String> {
    let path = profile_storage_dir(app, config)?;
    fs::create_dir_all(&path).map_err(|error| {
        format!(
            "创建 profile 保存目录失败：{}，路径：{}",
            error,
            path.display()
        )
    })?;
    Ok(path)
}

/// 获取当前秒级时间戳字符串
/// 返回值：RFC3339 本地时间字符串
fn current_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// 获取用于生成 id 的毫秒时间戳
/// 返回值：Unix 毫秒级时间戳
fn current_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

/// 探测 Windows 常见 Chrome 安装路径
/// 返回值：找到的 chrome.exe 路径，找不到时返回空字符串
fn find_chrome_path() -> String {
    let mut candidates = Vec::new();

    if let Ok(program_files) = env::var("ProgramFiles") {
        candidates.push(Path::new(&program_files).join("Google\\Chrome\\Application\\chrome.exe"));
    }

    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        candidates.push(Path::new(&program_files_x86).join("Google\\Chrome\\Application\\chrome.exe"));
    }

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates.push(Path::new(&local_app_data).join("Google\\Chrome\\Application\\chrome.exe"));
    }

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 探测当前用户默认 Chrome profile 目录
/// 返回值：优先返回 Default profile，找不到时返回 User Data，仍找不到时返回空字符串
fn find_default_profile_path() -> String {
    default_chrome_user_data_dir()
        .and_then(|user_data_path| {
            let default_profile_path = user_data_path.join("Default");

            if default_profile_path.is_dir() {
                return Some(default_profile_path.to_string_lossy().to_string());
            }

            if user_data_path.is_dir() {
                return Some(user_data_path.to_string_lossy().to_string());
            }

            None
        })
        .unwrap_or_default()
}

/// 探测当前用户默认 Chrome User Data 目录
/// 返回值：默认 Chrome User Data 目录，找不到时返回 None
fn default_chrome_user_data_dir() -> Option<PathBuf> {
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        return Some(Path::new(&local_app_data).join("Google\\Chrome\\User Data"));
    }

    None
}

/// 生成默认配置
/// app：Tauri 应用句柄
/// 返回值：包含 Chrome 路径、母版目录和环境列表的配置对象
fn default_config(app: &tauri::AppHandle) -> Result<Config, String> {
    Ok(Config {
        chrome_path: find_chrome_path(),
        master_profile_path: find_default_profile_path(),
        default_url: String::new(),
        profile_storage_path: default_profile_storage_path(app)?,
        sync_options: SyncOptions::default(),
        environments: Vec::new(),
    })
}

/// 读取本地配置文件
/// app：Tauri 应用句柄
/// 返回值：当前配置对象
fn read_config(app: &tauri::AppHandle) -> Result<Config, String> {
    ensure_config_dir(app)?;

    let path = config_path(app)?;

    if !path.exists() {
        let config = default_config(app)?;
        write_config(app, &config)?;
        return Ok(config);
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("读取配置文件失败：{}，路径：{}", error, path.display()))?;
    let mut config: Config = serde_json::from_str(&content)
        .map_err(|error| format!("解析配置文件失败：{}，路径：{}", error, path.display()))?;

    if config.chrome_path.is_empty() {
        config.chrome_path = find_chrome_path();
    }

    if config.master_profile_path.is_empty() {
        config.master_profile_path = find_default_profile_path();
    }

    if config.profile_storage_path.is_empty() {
        config.profile_storage_path = default_profile_storage_path(app)?;
    }

    Ok(config)
}

/// 写入本地配置文件
/// app：Tauri 应用句柄
/// config：要持久化的配置对象
/// 返回值：成功时无返回值
fn write_config(app: &tauri::AppHandle, config: &Config) -> Result<(), String> {
    ensure_config_dir(app)?;

    let path = config_path(app)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|error| format!("序列化配置失败：{error}"))?;

    fs::write(&path, format!("{content}\r\n"))
        .map_err(|error| format!("写入配置文件失败：{}，路径：{}", error, path.display()))
}

/// 将用户输入的环境名转换为安全目录名
/// name：环境名称
/// 返回值：适合 Windows 目录名的字符串
fn create_safe_name(name: &str) -> String {
    let safe: String = name
        .trim()
        .chars()
        .map(|char_item| match char_item {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            char_item if char_item.is_control() => '_',
            char_item => char_item,
        })
        .collect();

    if safe.is_empty() {
        format!("env_{}", current_timestamp())
    } else {
        safe
    }
}

/// 判断是否为 Chrome profile 运行时锁文件
/// name：文件名
/// 返回值：是锁文件时返回 true
fn is_profile_runtime_file(name: &str) -> bool {
    matches!(
        name,
        "SingletonCookie" | "SingletonLock" | "SingletonSocket" | "Lockfile"
    )
}

/// 提取相对路径中的普通路径片段
/// relative_path：相对 profile 根目录的路径
/// 返回值：全部转为小写的路径片段
fn relative_path_parts(relative_path: &Path) -> Vec<String> {
    relative_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// 判断 profile 相对路径是否等于指定片段序列
/// parts：已经标准化的相对路径片段
/// pattern：要匹配的路径片段
/// 返回值：完全匹配时返回 true
fn profile_path_matches(parts: &[String], pattern: &[&str]) -> bool {
    parts.len() == pattern.len()
        && parts
            .iter()
            .zip(pattern.iter())
            .all(|(part, expected)| part.as_str() == *expected)
}

/// 判断 profile 相对路径是否以指定片段序列开头
/// parts：已经标准化的相对路径片段
/// pattern：要匹配的路径前缀片段
/// 返回值：前缀匹配时返回 true
fn profile_path_starts_with(parts: &[String], pattern: &[&str]) -> bool {
    parts.len() >= pattern.len()
        && parts
            .iter()
            .take(pattern.len())
            .zip(pattern.iter())
            .all(|(part, expected)| part.as_str() == *expected)
}

/// 判断当前 IndexedDB 条目是否属于网站来源
/// parts：已经标准化的相对路径片段
/// 返回值：属于网站来源时返回 true
fn is_site_indexeddb_entry(parts: &[String]) -> bool {
    if !profile_path_starts_with(parts, &["indexeddb"]) || parts.len() < 2 {
        return false;
    }

    let origin_name = &parts[1];

    !origin_name.starts_with("chrome-extension_")
}

/// 判断当前 IndexedDB 条目是否属于 Chrome 插件来源
/// parts：已经标准化的相对路径片段
/// 返回值：属于插件来源时返回 true
fn is_extension_indexeddb_entry(parts: &[String]) -> bool {
    if !profile_path_starts_with(parts, &["indexeddb"]) || parts.len() < 2 {
        return false;
    }

    parts[1].starts_with("chrome-extension_")
}

/// 判断同步母版时是否需要跳过指定 profile 条目
/// file_name：当前条目的文件名
/// relative_path：当前条目相对 profile 根目录的路径
/// sync_options：同步过滤选项
/// 返回值：需要跳过时返回 true
fn should_skip_profile_entry(
    file_name: &str,
    relative_path: &Path,
    sync_options: &SyncOptions,
) -> bool {
    if is_profile_runtime_file(file_name) {
        return true;
    }

    let parts = relative_path_parts(relative_path);

    // Cookie 数据通常保存网站登录态，未勾选同步时不复制到环境
    if !sync_options.sync_cookies {
        let cookie_paths: &[&[&str]] = &[
            &["cookies"],
            &["cookies-journal"],
            &["network", "cookies"],
            &["network", "cookies-journal"],
            &["safe browsing cookies"],
            &["safe browsing cookies-journal"],
        ];

        if cookie_paths
            .iter()
            .any(|pattern| profile_path_matches(&parts, pattern))
        {
            return true;
        }
    }

    // 插件本体和插件配置分散在多个扩展目录，未勾选同步时统一跳过
    if !sync_options.sync_extensions {
        let extension_paths: &[&[&str]] = &[
            &["extensions"],
            &["extension rules"],
            &["extension scripts"],
            &["extension state"],
            &["local extension settings"],
            &["managed extension settings"],
            &["sync extension settings"],
            &["extension cookies"],
            &["extension cookies-journal"],
        ];

        if is_extension_indexeddb_entry(&parts)
            || extension_paths
            .iter()
            .any(|pattern| profile_path_starts_with(&parts, pattern))
        {
            return true;
        }
    }

    // 站点存储包含网页写入的本地数据，IndexedDB 会保留 chrome-extension 来源
    if !sync_options.sync_site_storage {
        let site_storage_paths: &[&[&str]] = &[
            &["local storage"],
            &["session storage"],
            &["service worker"],
            &["file system"],
            &["databases"],
            &["blob_storage"],
            &["shared storage"],
            &["shared_proto_db"],
            &["storage", "default"],
            &["storage", "buckets"],
        ];

        if is_site_indexeddb_entry(&parts)
            || site_storage_paths
                .iter()
                .any(|pattern| profile_path_starts_with(&parts, pattern))
        {
            return true;
        }
    }

    // 缓存数据可由 Chrome 自动重建，未勾选同步时跳过
    if !sync_options.sync_cache {
        let cache_paths: &[&[&str]] = &[
            &["cache"],
            &["code cache"],
            &["gpucache"],
            &["dawncache"],
            &["shadercache"],
            &["grshadercache"],
            &["media cache"],
            &["optimizationhints"],
        ];

        if cache_paths
            .iter()
            .any(|pattern| profile_path_starts_with(&parts, pattern))
        {
            return true;
        }
    }

    // 会话文件会恢复上次窗口和标签页，未勾选同步时跳过
    if !sync_options.sync_sessions {
        let session_paths: &[&[&str]] = &[
            &["sessions"],
            &["current session"],
            &["current tabs"],
            &["last session"],
            &["last tabs"],
        ];

        if session_paths
            .iter()
            .any(|pattern| profile_path_starts_with(&parts, pattern))
        {
            return true;
        }
    }

    // 历史记录和常用站点属于使用痕迹，未勾选同步时跳过
    if !sync_options.sync_history {
        let history_paths: &[&[&str]] = &[
            &["history"],
            &["history-journal"],
            &["visited links"],
            &["top sites"],
            &["top sites-journal"],
            &["shortcuts"],
            &["shortcuts-journal"],
            &["favicons"],
            &["favicons-journal"],
        ];

        if history_paths
            .iter()
            .any(|pattern| profile_path_matches(&parts, pattern))
        {
            return true;
        }
    }

    false
}

/// 判断目录是否像一个 Chrome profile 目录
/// target_path：待判断目录
/// 返回值：包含 profile 关键文件或扩展目录时返回 true
fn is_chrome_profile_dir(target_path: &Path) -> bool {
    target_path.join("Preferences").is_file()
        || target_path.join("Secure Preferences").is_file()
        || target_path.join("Extensions").is_dir()
}

/// 将用户选择的母版路径解析为实际 profile 目录
/// master_profile_path：用户配置的母版路径
/// 返回值：可复制的具体 profile 目录
fn resolve_master_profile_source(master_profile_path: &Path) -> Result<PathBuf, String> {
    if !master_profile_path.is_dir() {
        return Err("母版 profile 目录不存在，请先在全局设置中选择有效目录".to_string());
    }

    // 用户选择 Default/Profile 3 这类具体 profile 目录时，直接复制该目录内容
    if is_chrome_profile_dir(master_profile_path) {
        return Ok(master_profile_path.to_path_buf());
    }

    let default_profile_path = master_profile_path.join("Default");

    // 用户误选 User Data 根目录时，优先取其中的 Default profile，避免复制整个 User Data
    if default_profile_path.is_dir() {
        return Ok(default_profile_path);
    }

    Ok(master_profile_path.to_path_buf())
}

/// 获取环境实际使用的 Chrome profile 目录
/// environment：环境配置对象
/// 返回值：环境 User Data 下的 Default profile 目录
fn environment_default_profile_dir(environment: &Environment) -> PathBuf {
    if environment.managed {
        return Path::new(&environment.profile_path).join(&environment.profile_directory);
    }

    PathBuf::from(&environment.profile_path)
}

/// 获取 Chrome 启动所需的 user-data-dir
/// environment：环境配置对象
/// 返回值：传给 Chrome 的 user-data-dir 路径
fn environment_user_data_dir(environment: &Environment) -> PathBuf {
    if environment.managed {
        return PathBuf::from(&environment.profile_path);
    }

    Path::new(&environment.profile_path)
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&environment.profile_path))
}

/// 获取 Chrome 启动所需的 profile-directory
/// environment：环境配置对象
/// 返回值：传给 Chrome 的 profile-directory 名称
fn environment_profile_directory(environment: &Environment) -> String {
    if environment.profile_directory.trim().is_empty() {
        return default_profile_directory();
    }

    environment.profile_directory.clone()
}

/// 判断是否为可扫描的常规 Chrome profile 名称
/// profile_directory：profile 目录名
/// 返回值：常规 profile 返回 true，Guest/System 等内部 profile 返回 false
fn is_regular_profile_directory(profile_directory: &str) -> bool {
    let normalized = profile_directory.trim().to_ascii_lowercase();

    normalized.starts_with("profile ")
}

/// 判断是否为不能删除磁盘目录的外部 profile
/// profile_directory：profile 目录名
/// 返回值：Default、Guest Profile、System Profile 返回 true
fn is_protected_external_profile_directory(profile_directory: &str) -> bool {
    matches!(
        profile_directory.trim().to_ascii_lowercase().as_str(),
        "default" | "guest profile" | "system profile"
    )
}

/// 解析扫描到的外部 Chrome profile 目录
/// profile_path：扫描到的 profile 目录
/// 返回值：规范化后的 profile 路径和 profile-directory 名称
fn resolve_external_profile(profile_path: &Path) -> Result<(PathBuf, String), String> {
    if !profile_path.is_dir() {
        return Err("外部 Profile 路径不存在".to_string());
    }

    if !is_chrome_profile_dir(profile_path) {
        return Err("扫描到的目录不是有效 Chrome Profile".to_string());
    }

    let profile_directory = profile_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "无法识别外部 Profile 的目录名".to_string())?;

    Ok((profile_path.to_path_buf(), profile_directory))
}

/// 确认待删除 profile 位于应用管理目录内
/// app：Tauri 应用句柄
/// target_path：待删除目录
/// 返回值：通过校验时无返回值
fn ensure_managed_profile_path(app: &tauri::AppHandle, target_path: &Path) -> Result<(), String> {
    if !target_path.exists() {
        return Ok(());
    }

    let config = read_config(app)?;
    let mut allowed_roots = vec![default_profiles_dir(app)?];
    let configured_root = profile_storage_dir(app, &config)?;

    if !allowed_roots.iter().any(|root| root == &configured_root) {
        allowed_roots.push(configured_root);
    }

    let canonical_target = target_path.canonicalize().map_err(|error| {
        format!(
            "读取待删除 profile 目录失败：{}，路径：{}",
            error,
            target_path.display()
        )
    })?;

    for root in allowed_roots {
        if !root.exists() {
            continue;
        }

        let canonical_root = root.canonicalize().map_err(|error| {
            format!(
                "读取环境根目录失败：{}，路径：{}",
                error,
                root.display()
            )
        })?;

        if canonical_target.starts_with(&canonical_root) {
            return Ok(());
        }
    }

    Err("出于安全限制，只允许删除本工具创建的 profile 目录".to_string())
}

/// 确认外部 profile 可被删除
/// target_path：待删除的外部 profile 目录
/// profile_directory：profile-directory 名称
/// 返回值：通过校验时无返回值
fn ensure_deletable_external_profile_path(
    target_path: &Path,
    profile_directory: &str,
) -> Result<(), String> {
    if is_protected_external_profile_directory(profile_directory) {
        return Err("Default、Guest Profile、System Profile 只能移出管理列表".to_string());
    }

    if !target_path.exists() {
        return Ok(());
    }

    let user_data_path = default_chrome_user_data_dir()
        .ok_or_else(|| "未找到默认 Chrome User Data 目录，无法确认外部 Profile 删除范围".to_string())?;
    let canonical_root = user_data_path.canonicalize().map_err(|error| {
        format!(
            "读取默认 Chrome User Data 目录失败：{}，路径：{}",
            error,
            user_data_path.display()
        )
    })?;
    let canonical_target = target_path.canonicalize().map_err(|error| {
        format!(
            "读取待删除外部 Profile 目录失败：{}，路径：{}",
            error,
            target_path.display()
        )
    })?;

    if !canonical_target.starts_with(&canonical_root) {
        return Err("出于安全限制，只允许删除默认 Chrome User Data 目录下的外部 Profile".to_string());
    }

    Ok(())
}

/// 递归复制目录，按同步选项跳过敏感浏览数据
/// source：源目录路径
/// destination：目标目录路径
/// relative_root：当前目录相对 profile 根目录的路径
/// sync_options：同步过滤选项
/// 返回值：成功时无返回值
fn copy_dir_filtered(
    source: &Path,
    destination: &Path,
    relative_root: &Path,
    sync_options: &SyncOptions,
) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("创建目标目录失败：{}，路径：{}", error, destination.display()))?;

    for entry_result in fs::read_dir(source)
        .map_err(|error| format!("读取源目录失败：{}，路径：{}", error, source.display()))?
    {
        let entry = entry_result
            .map_err(|error| format!("读取目录项失败：{}，路径：{}", error, source.display()))?;
        let file_name = entry.file_name();
        let file_name_text = file_name.to_string_lossy();
        let source_path = entry.path();
        let destination_path = destination.join(&file_name);
        let relative_path = relative_root.join(&file_name);
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取文件类型失败：{}，路径：{}", error, source_path.display()))?;

        // 运行时锁文件、Cookie、站点数据等条目会在这里统一过滤
        if should_skip_profile_entry(&file_name_text, &relative_path, sync_options) {
            continue;
        }

        if file_type.is_dir() {
            copy_dir_filtered(&source_path, &destination_path, &relative_path, sync_options)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "复制文件失败：{}，源：{}，目标：{}",
                    error,
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

/// 从母版 profile 复制到目标 profile
/// source：源 profile 目录路径
/// destination：目标 profile 目录路径
/// sync_options：同步过滤选项
/// 返回值：成功时无返回值
fn copy_dir_all(source: &Path, destination: &Path, sync_options: &SyncOptions) -> Result<(), String> {
    copy_dir_filtered(source, destination, Path::new(""), sync_options)
}

/// 根据 id 查找环境下标
/// config：配置对象
/// id：环境 id
/// 返回值：环境在列表中的下标
fn find_environment_index(config: &Config, id: &str) -> Result<usize, String> {
    config
        .environments
        .iter()
        .position(|environment| environment.id == id)
        .ok_or_else(|| format!("未找到环境：{id}"))
}

/// 打开系统文件选择器选择 chrome.exe
/// 返回值：选中的文件路径，取消时返回空字符串
#[tauri::command]
fn select_chrome_file() -> String {
    rfd::FileDialog::new()
        .set_title("选择 chrome.exe")
        .add_filter("Chrome", &["exe"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 解析目录选择器的默认打开目录
/// initial_path：用户当前输入的目录路径
/// 返回值：存在的默认打开目录，不存在时返回 None
fn resolve_initial_directory(initial_path: Option<String>) -> Option<PathBuf> {
    let value = initial_path?.trim().to_string();

    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);

    if path.is_dir() {
        return Some(path);
    }

    path.parent()
        .filter(|parent| parent.is_dir())
        .map(|parent| parent.to_path_buf())
}

/// 打开系统目录选择器
/// title：选择器标题
/// initial_path：默认打开的目录路径
/// 返回值：选中的目录路径，取消时返回空字符串
#[tauri::command]
fn select_directory(title: Option<String>, initial_path: Option<String>) -> String {
    let mut dialog = rfd::FileDialog::new().set_title(&title.unwrap_or_else(|| "选择目录".to_string()));

    if let Some(path) = resolve_initial_directory(initial_path) {
        dialog = dialog.set_directory(path);
    }

    dialog
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 读取前端所需配置
/// app：Tauri 应用句柄
/// 返回值：当前配置对象
#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<Config, String> {
    read_config(&app)
}

/// 更新全局设置
/// app：Tauri 应用句柄
/// patch：要更新的全局字段
/// 返回值：更新后的配置对象
#[tauri::command]
fn update_config(app: tauri::AppHandle, patch: ConfigPatch) -> Result<Config, String> {
    let mut config = read_config(&app)?;

    if let Some(chrome_path) = patch.chrome_path {
        config.chrome_path = chrome_path.trim().to_string();
    }

    if let Some(master_profile_path) = patch.master_profile_path {
        config.master_profile_path = master_profile_path.trim().to_string();
    }

    if let Some(default_url) = patch.default_url {
        config.default_url = default_url.trim().to_string();
    }

    if let Some(profile_storage_path) = patch.profile_storage_path {
        config.profile_storage_path = profile_storage_path.trim().to_string();
    }

    if let Some(sync_options) = patch.sync_options {
        config.sync_options = sync_options;
    }

    write_config(&app, &config)?;
    Ok(config)
}

/// 创建新的 Chrome 环境
/// app：Tauri 应用句柄
/// payload：创建环境所需字段
/// 返回值：新建的环境对象
#[tauri::command]
fn create_environment(
    app: tauri::AppHandle,
    payload: CreateEnvironmentPayload,
) -> Result<Environment, String> {
    let mut config = read_config(&app)?;
    let name = payload.name.trim().to_string();

    if name.is_empty() {
        return Err("环境名称不能为空".to_string());
    }

    let safe_name = create_safe_name(&name);
    let profiles_root = ensure_profile_storage_dir(&app, &config)?;
    let mut profile_path = profiles_root.join(&safe_name);
    let mut counter = 1;

    while profile_path.exists() {
        profile_path = profiles_root.join(format!("{safe_name}_{counter}"));
        counter += 1;
    }

    fs::create_dir_all(&profile_path)
        .map_err(|error| format!("创建 profile 目录失败：{}，路径：{}", error, profile_path.display()))?;

    let now = current_timestamp();
    let environment = Environment {
        id: format!("{}_{}", current_millis(), config.environments.len() + 1),
        name,
        profile_path: profile_path.to_string_lossy().to_string(),
        profile_directory: default_profile_directory(),
        managed: true,
        start_url: payload
            .start_url
            .unwrap_or_else(|| config.default_url.clone())
            .trim()
            .to_string(),
        extension_paths: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };

    if payload.copy_master {
        let master_profile_path = resolve_master_profile_source(Path::new(&config.master_profile_path))?;
        let default_profile_path = environment_default_profile_dir(&environment);

        copy_dir_all(&master_profile_path, &default_profile_path, &config.sync_options)?;
    }

    config.environments.insert(0, environment.clone());
    write_config(&app, &config)?;

    Ok(environment)
}

/// 扫描默认 Chrome User Data 下的已有 Profile
/// app：Tauri 应用句柄
/// 返回值：新增到管理列表的环境对象
#[tauri::command]
fn scan_existing_profiles(app: tauri::AppHandle) -> Result<Vec<Environment>, String> {
    let mut config = read_config(&app)?;
    let user_data_path = default_chrome_user_data_dir()
        .ok_or_else(|| "未找到默认 Chrome User Data 目录".to_string())?;

    if !user_data_path.is_dir() {
        return Err(format!(
            "默认 Chrome User Data 目录不存在：{}",
            user_data_path.display()
        ));
    }

    let mut external_environments = Vec::new();

    for entry_result in fs::read_dir(&user_data_path)
        .map_err(|error| format!("读取 Chrome User Data 目录失败：{}，路径：{}", error, user_data_path.display()))?
    {
        let entry = entry_result
            .map_err(|error| format!("读取 Chrome User Data 目录项失败：{}，路径：{}", error, user_data_path.display()))?;
        let profile_path = entry.path();

        if !profile_path.is_dir() || !is_chrome_profile_dir(&profile_path) {
            continue;
        }

        let (profile_path, profile_directory) = resolve_external_profile(&profile_path)?;

        if !is_regular_profile_directory(&profile_directory) {
            continue;
        }

        let canonical_profile_path = profile_path.canonicalize().map_err(|error| {
            format!(
                "读取已有 Profile 路径失败：{}，路径：{}",
                error,
                profile_path.display()
            )
        })?;
        let already_exists = config.environments.iter().any(|environment| {
            Path::new(&environment.profile_path)
                .canonicalize()
                .map(|path| path == canonical_profile_path)
                .unwrap_or(false)
        });

        if already_exists {
            continue;
        }

        let now = current_timestamp();
        let environment = Environment {
            id: format!(
                "{}_{}",
                current_millis(),
                config.environments.len() + external_environments.len() + 1
            ),
            name: profile_directory.clone(),
            profile_path: profile_path.to_string_lossy().to_string(),
            profile_directory,
            managed: false,
            start_url: config.default_url.trim().to_string(),
            extension_paths: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };

        external_environments.push(environment);
    }

    if !external_environments.is_empty() {
        for environment in external_environments.iter().rev() {
            config.environments.insert(0, environment.clone());
        }

        write_config(&app, &config)?;
    }

    Ok(external_environments)
}

/// 更新指定环境
/// app：Tauri 应用句柄
/// id：环境 id
/// patch：要更新的字段
/// 返回值：更新后的环境对象
#[tauri::command]
fn update_environment(
    app: tauri::AppHandle,
    id: String,
    patch: EnvironmentPatch,
) -> Result<Environment, String> {
    let mut config = read_config(&app)?;
    let index = find_environment_index(&config, &id)?;
    let mut environment = config.environments[index].clone();

    if let Some(name) = patch.name {
        let next_name = name.trim().to_string();

        if next_name.is_empty() {
            return Err("环境名称不能为空".to_string());
        }

        environment.name = next_name;
    }

    if let Some(start_url) = patch.start_url {
        environment.start_url = start_url.trim().to_string();
    }

    environment.updated_at = current_timestamp();
    config.environments[index] = environment.clone();
    write_config(&app, &config)?;

    Ok(environment)
}

/// 删除指定环境记录及其磁盘 profile 目录
/// app：Tauri 应用句柄
/// id：环境 id
/// 返回值：成功时返回 true
#[tauri::command]
fn delete_environment(app: tauri::AppHandle, id: String) -> Result<bool, String> {
    let mut config = read_config(&app)?;
    let index = find_environment_index(&config, &id)?;
    let environment = config.environments[index].clone();
    let profile_path = PathBuf::from(&environment.profile_path);

    if environment.managed {
        ensure_managed_profile_path(&app, &profile_path)?;

        if profile_path.exists() {
            fs::remove_dir_all(&profile_path).map_err(|error| {
                format!(
                    "删除 profile 目录失败：{}，路径：{}",
                    error,
                    profile_path.display()
                )
            })?;
        }
    } else if !is_protected_external_profile_directory(&environment_profile_directory(&environment)) {
        ensure_deletable_external_profile_path(&profile_path, &environment_profile_directory(&environment))?;

        if profile_path.exists() {
            fs::remove_dir_all(&profile_path).map_err(|error| {
                format!(
                    "删除外部 Profile 目录失败：{}，路径：{}",
                    error,
                    profile_path.display()
                )
            })?;
        }
    }

    config.environments.retain(|environment| environment.id != id);
    write_config(&app, &config)?;
    Ok(true)
}

/// 将母版 Profile 同步到指定环境
/// app：Tauri 应用句柄
/// id：环境 id
/// 返回值：成功时返回 true
#[tauri::command]
fn copy_master(app: tauri::AppHandle, id: String) -> Result<bool, String> {
    let config = read_config(&app)?;
    let index = find_environment_index(&config, &id)?;
    let environment = &config.environments[index];

    if !environment.managed
        && is_protected_external_profile_directory(&environment_profile_directory(environment))
    {
        return Err("Default、Guest Profile、System Profile 不支持同步母版，避免覆盖内置 Profile 数据".to_string());
    }

    let master_profile_path = resolve_master_profile_source(Path::new(&config.master_profile_path))?;
    let default_profile_path = environment_default_profile_dir(environment);

    copy_dir_all(&master_profile_path, &default_profile_path, &config.sync_options)?;
    Ok(true)
}

/// 启动指定 Chrome 环境
/// app：Tauri 应用句柄
/// id：环境 id
/// 返回值：成功时返回 true
#[tauri::command]
fn launch_environment(app: tauri::AppHandle, id: String) -> Result<bool, String> {
    let config = read_config(&app)?;
    let index = find_environment_index(&config, &id)?;
    let environment = &config.environments[index];
    let chrome_path = Path::new(&config.chrome_path);
    let user_data_dir = environment_user_data_dir(environment);
    let profile_directory = environment_profile_directory(environment);

    if !chrome_path.is_file() {
        return Err("Chrome 程序路径无效，请在全局设置中选择 chrome.exe".to_string());
    }

    fs::create_dir_all(&user_data_dir)
        .map_err(|error| format!("创建 profile 目录失败：{}，路径：{}", error, user_data_dir.display()))?;

    let mut args = vec![
        format!("--user-data-dir={}", user_data_dir.display()),
        format!("--profile-directory={profile_directory}"),
        "--no-first-run".to_string(),
        "--disable-default-apps".to_string(),
    ];

    if !environment.start_url.trim().is_empty() {
        args.push(environment.start_url.trim().to_string());
    }

    let mut command = Command::new(chrome_path);
    command.args(args);

    #[cfg(windows)]
    {
        command.creation_flags(0x08000000);
    }

    command
        .spawn()
        .map_err(|error| format!("启动 Chrome 失败：{}，路径：{}", error, chrome_path.display()))?;

    Ok(true)
}

/// 使用系统资源管理器打开路径
/// target_path：待打开的文件或目录路径
/// 返回值：成功时返回 true
#[tauri::command]
fn open_path(target_path: String) -> Result<bool, String> {
    if target_path.trim().is_empty() {
        return Ok(false);
    }

    #[cfg(windows)]
    let result: io::Result<_> = Command::new("explorer").arg(target_path).spawn();

    #[cfg(not(windows))]
    let result: io::Result<_> = Command::new("xdg-open").arg(target_path).spawn();

    result.map_err(|error| format!("打开路径失败：{error}"))?;
    Ok(true)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            select_chrome_file,
            select_directory,
            load_config,
            update_config,
            create_environment,
            scan_existing_profiles,
            update_environment,
            delete_environment,
            copy_master,
            launch_environment,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败")
}
