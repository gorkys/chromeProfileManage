#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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
    start_url: String,
    #[serde(default)]
    extension_paths: Vec<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    chrome_path: String,
    master_profile_path: String,
    default_url: String,
    #[serde(default)]
    profile_storage_path: String,
    environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigPatch {
    chrome_path: Option<String>,
    master_profile_path: Option<String>,
    default_url: Option<String>,
    profile_storage_path: Option<String>,
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
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let user_data_path = Path::new(&local_app_data).join("Google\\Chrome\\User Data");
        let default_profile_path = user_data_path.join("Default");

        if default_profile_path.is_dir() {
            return default_profile_path.to_string_lossy().to_string();
        }

        if user_data_path.is_dir() {
            return user_data_path.to_string_lossy().to_string();
        }
    }

    String::new()
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
    Path::new(&environment.profile_path).join("Default")
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

/// 递归复制目录，跳过 Chrome 运行时锁文件
/// source：源目录路径
/// destination：目标目录路径
/// 返回值：成功时无返回值
fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("创建目标目录失败：{}，路径：{}", error, destination.display()))?;

    for entry_result in fs::read_dir(source)
        .map_err(|error| format!("读取源目录失败：{}，路径：{}", error, source.display()))?
    {
        let entry = entry_result
            .map_err(|error| format!("读取目录项失败：{}，路径：{}", error, source.display()))?;
        let file_name = entry.file_name();
        let file_name_text = file_name.to_string_lossy();

        // Chrome 正在使用 profile 时会生成这些锁文件，复制它们没有意义且容易失败
        if is_profile_runtime_file(&file_name_text) {
            continue;
        }

        let source_path = entry.path();
        let destination_path = destination.join(&file_name);
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取文件类型失败：{}，路径：{}", error, source_path.display()))?;

        if file_type.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
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

/// 打开系统目录选择器
/// title：选择器标题
/// 返回值：选中的目录路径，取消时返回空字符串
#[tauri::command]
fn select_directory(title: Option<String>) -> String {
    rfd::FileDialog::new()
        .set_title(&title.unwrap_or_else(|| "选择目录".to_string()))
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

        copy_dir_all(&master_profile_path, &default_profile_path)?;
    }

    config.environments.insert(0, environment.clone());
    write_config(&app, &config)?;

    Ok(environment)
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
    let profile_path = PathBuf::from(&config.environments[index].profile_path);

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
    let master_profile_path = resolve_master_profile_source(Path::new(&config.master_profile_path))?;
    let default_profile_path = environment_default_profile_dir(environment);

    copy_dir_all(&master_profile_path, &default_profile_path)?;
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
    let profile_path = Path::new(&environment.profile_path);

    if !chrome_path.is_file() {
        return Err("Chrome 程序路径无效，请在全局设置中选择 chrome.exe".to_string());
    }

    fs::create_dir_all(profile_path)
        .map_err(|error| format!("创建 profile 目录失败：{}，路径：{}", error, profile_path.display()))?;

    let mut args = vec![
        format!("--user-data-dir={}", profile_path.display()),
        "--profile-directory=Default".to_string(),
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
            update_environment,
            delete_environment,
            copy_master,
            launch_environment,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败")
}
