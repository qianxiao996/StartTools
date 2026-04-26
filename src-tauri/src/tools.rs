use serde_json::{json, Map, Value};
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use std::{ffi::OsStr};
use std::{fs, u16};
use std::process::Command;
use image::{ RgbaImage};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use winapi::um::winnt::FILE_ATTRIBUTE_DIRECTORY;
use crate::load_data::Tools;
use crate::utils::DB_PATH;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::winuser::SW_SHOW;
use std::ptr::null_mut;
use crate::utils::EXE_PATH;
use tauri::{AppHandle};
use tauri_plugin_opener::OpenerExt;
use crate::{load_data, utils};
use winapi::um::shellapi::{SHGetFileInfoW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES};
use winapi::um::winuser::{
    DestroyIcon, DrawIconEx, GetIconInfo, ICONINFO,
};
use rusqlite::{ Result, ToSql};
use winapi::um::wingdi::DeleteDC;
use winapi::um::wingdi::CreateCompatibleDC;
use winapi::shared::windef::{HICON};
use winapi::um::wingdi::{BITMAPINFOHEADER, BI_RGB};
use image::{ImageBuffer, Rgba};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use winapi::um::wingdi::RGBQUAD;
use winapi::um::wingdi::BITMAPINFO;
const ICON_DRAW_NORMAL: u32 = 0x0003;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuiltinTool {
    pub id: i64,
    pub name: String,
    pub run_type: String,
    pub shell: String,
    pub content: String,
    pub working_dir: String,
    #[serde(default = "current_builtin_tool_os_string")]
    pub os: String,
    pub enabled: bool,
    pub sort: i64,
}

#[tauri::command]
pub fn load_builtin_tools() -> Result<Vec<BuiltinTool>, String> {
    load_builtin_tools_by_os(Some(&configured_builtin_tool_os()))
}

#[tauri::command]
pub fn load_all_builtin_tools() -> Result<Vec<BuiltinTool>, String> {
    load_builtin_tools_by_os(None)
}

#[tauri::command]
pub fn export_builtin_tools(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导出路径不能为空".to_string());
    }
    let tools = load_builtin_tools_by_os(None)?;
    let text = serde_json::to_string_pretty(&tools).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| format!("导出内置工具失败: {}", err))
}

#[tauri::command]
pub fn import_builtin_tools(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导入路径不能为空".to_string());
    }
    let text = fs::read_to_string(&path).map_err(|err| format!("读取内置工具备份失败: {}", err))?;
    let tools: Vec<BuiltinTool> = serde_json::from_str(&text).map_err(|err| format!("内置工具备份格式不正确: {}", err))?;
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.execute("DROP TABLE IF EXISTS builtin_tools_import_backup", [])
        .map_err(|err| format!("清理内置工具备份失败: {}", err))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS builtin_tools_import_backup AS SELECT * FROM builtin_tools",
        [],
    )
    .map_err(|err| format!("备份内置工具失败: {}", err))?;
    conn.execute("DELETE FROM builtin_tools", [])
        .map_err(|err| format!("清理内置工具失败: {}", err))?;
    for tool in tools {
        let enabled = if tool.enabled { 1 } else { 0 };
        conn.execute(
            "INSERT OR REPLACE INTO builtin_tools (id, name, run_type, shell, content, working_dir, os, enabled, sort)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                tool.id,
                tool.name,
                tool.run_type,
                tool.shell,
                tool.content,
                tool.working_dir,
                normalize_builtin_tool_os(&tool.os),
                enabled,
                tool.sort,
            ],
        )
        .map_err(|err| format!("导入内置工具失败: {}", err))?;
    }
    Ok(())
}

fn load_builtin_tools_by_os(os_filter: Option<&str>) -> Result<Vec<BuiltinTool>, String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    let sql = if os_filter.is_some() {
        "SELECT id, name, run_type, shell, content, working_dir, os, enabled, sort
         FROM builtin_tools
         WHERE os = ?1
         ORDER BY sort ASC, id ASC"
    } else {
        "SELECT id, name, run_type, shell, content, working_dir, os, enabled, sort
         FROM builtin_tools
         ORDER BY sort ASC, id ASC"
    };
    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;

    let mut tools = Vec::new();
    if let Some(os) = os_filter {
        let rows = stmt
            .query_map(params![os], row_to_builtin_tool)
            .map_err(|err| err.to_string())?;
        for row in rows {
            tools.push(row.map_err(|err| err.to_string())?);
        }
    } else {
        let rows = stmt
            .query_map([], row_to_builtin_tool)
            .map_err(|err| err.to_string())?;
        for row in rows {
            tools.push(row.map_err(|err| err.to_string())?);
        }
    }
    Ok(tools)
}

fn row_to_builtin_tool(row: &rusqlite::Row<'_>) -> rusqlite::Result<BuiltinTool> {
    Ok(BuiltinTool {
        id: row.get(0)?,
        name: row.get(1)?,
        run_type: row.get(2)?,
        shell: row.get(3)?,
        content: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        working_dir: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        os: normalize_builtin_tool_os(&row.get::<_, Option<String>>(6)?.unwrap_or_default()),
        enabled: row.get::<_, i64>(7)? != 0,
        sort: row.get(8)?,
    })
}

#[tauri::command]
pub fn update_builtin_tool(tool: BuiltinTool) -> Result<(), String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    let enabled = if tool.enabled { 1 } else { 0 };
    let os = normalize_builtin_tool_os(&tool.os);
    if tool.id <= 0 {
        conn.execute(
            "INSERT INTO builtin_tools (name, run_type, shell, content, working_dir, os, enabled, sort)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                tool.name,
                tool.run_type,
                tool.shell,
                tool.content,
                tool.working_dir,
                os,
                enabled,
                tool.sort,
            ],
        )
        .map_err(|err| err.to_string())?;
    } else {
        conn.execute(
            "UPDATE builtin_tools
             SET name = ?1, run_type = ?2, shell = ?3, content = ?4, working_dir = ?5, os = ?6, enabled = ?7, sort = ?8
             WHERE id = ?9",
            params![
                tool.name,
                tool.run_type,
                tool.shell,
                tool.content,
                tool.working_dir,
                os,
                enabled,
                tool.sort,
                tool.id,
            ],
        )
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_builtin_tool(tool_id: i64) -> Result<(), String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.execute("DELETE FROM builtin_tools WHERE id = ?1", params![tool_id])
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn run_builtin_tool(tool: BuiltinTool) -> Result<(), String> {
    if !tool.enabled {
        return Err("工具已禁用".to_string());
    }
    if normalize_builtin_tool_os(&tool.os) != configured_builtin_tool_os() {
        return Err("当前系统不匹配，不能运行该脚本".to_string());
    }
    if tool.content.trim().is_empty() {
        return Err("命令或脚本内容为空".to_string());
    }

    let mut command = system_script_command(&tool.shell, &tool.content);

    if let Some(working_dir) = infer_builtin_tool_working_dir(&tool.content, &tool.working_dir) {
        command.current_dir(working_dir);
    }

    command.spawn().map_err(|err| err.to_string())?;
    Ok(())
}

fn system_script_command(shell: &str, content: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        if shell == "powershell" {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", content]);
            return command;
        }

        let mut command = Command::new("cmd.exe");
        command.args(["/C", content]);
        return command;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let executable = match shell {
            "bash" => "bash",
            "zsh" => "zsh",
            _ => "sh",
        };
        let mut command = Command::new(executable);
        command.args(["-c", content]);
        return command;
    }
}

fn current_builtin_tool_os() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
}

fn current_builtin_tool_os_string() -> String {
    current_builtin_tool_os().to_string()
}

fn configured_builtin_tool_os() -> String {
    let conn = match Connection::open(&*DB_PATH) {
        Ok(conn) => conn,
        Err(_) => return current_builtin_tool_os_string(),
    };
    let value = conn
        .query_row(
            "SELECT value FROM config WHERE name = 'Builtin_Tools_OS' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| current_builtin_tool_os_string());
    normalize_builtin_tool_os(&value)
}

fn normalize_builtin_tool_os(os: &str) -> String {
    match os.trim().to_ascii_lowercase().as_str() {
        "linux" => "linux".to_string(),
        "mac" | "macos" | "darwin" => "macos".to_string(),
        _ => "windows".to_string(),
    }
}

fn infer_builtin_tool_working_dir(content: &str, configured_dir: &str) -> Option<PathBuf> {
    let configured = configured_dir.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(expand_tool_path(configured));
        if path.is_dir() {
            return Some(path);
        }
    }

    if let Some(path) = last_explorer_working_dir() {
        return Some(path);
    }

    if let Some(path) = first_cd_working_dir(content) {
        return Some(path);
    }

    let first_token = first_command_token(content)?;
    let path = PathBuf::from(expand_tool_path(&first_token));
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }
    if path.is_dir() {
        return Some(path);
    }
    default_shell_working_dir()
}

#[cfg(target_os = "windows")]
fn last_explorer_working_dir() -> Option<PathBuf> {
    let script = r#"
$foreground = Add-Type -PassThru -Name Win32ForegroundWindow -Namespace StartToolsNative -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();'
$foregroundHwnd = $foreground::GetForegroundWindow().ToInt64()
$shell = New-Object -ComObject Shell.Application
$windows = @($shell.Windows())
$paths = @()
foreach ($window in $windows) {
  try {
    $path = $window.Document.Folder.Self.Path
    if ($path -and (Test-Path -LiteralPath $path -PathType Container)) {
      if ([int64]$window.HWND -eq $foregroundHwnd) {
        Write-Output $path
        exit
      }
      $paths += $path
    }
  } catch {}
}
if ($paths.Count -gt 0) {
  Write-Output $paths[-1]
}
"#;

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        return None;
    }

    let path = PathBuf::from(path);
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn last_explorer_working_dir() -> Option<PathBuf> {
    None
}

fn expand_tool_path(value: &str) -> String {
    value.replace("%Tools%", &*EXE_PATH)
}

fn first_command_token(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }

    trimmed
        .split_whitespace()
        .next()
        .map(|value| value.trim_matches('"').to_string())
}

fn first_cd_working_dir(content: &str) -> Option<PathBuf> {
    for command in content.split(&['&', '\n', '\r'][..]) {
        let trimmed = command.trim();
        let lower = trimmed.to_lowercase();
        if !lower.starts_with("cd ") && !lower.starts_with("chdir ") {
            continue;
        }

        let mut dir = trimmed
            .splitn(2, char::is_whitespace)
            .nth(1)?
            .trim()
            .to_string();

        if dir.to_lowercase().starts_with("/d ") {
            dir = dir[3..].trim().to_string();
        }
        let dir = dir.trim_matches('"');
        if dir.is_empty() {
            continue;
        }

        let path = PathBuf::from(expand_tool_path(dir));
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}

fn default_shell_working_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let home = PathBuf::from(profile);
            let desktop = home.join("Desktop");
            if desktop.is_dir() {
                return Some(desktop);
            }
            return Some(home);
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

#[tauri::command]
pub fn open_cmd(_data: Map<String, Value>) -> String {
    // let data2 ={"isAdmin":1,"Path":tool.target,"Terminal":store.state.Config.Terminal,"Terminal_Runas_Arguments":store.state.Config.Terminal_Runas_Arguments}; 
    if _data.get("Path").is_none() {
        return "目标文件路径不存在！".to_string();
    }
    let target_str = _data.get("Path").expect("REASON").as_str().unwrap_or("");
    let exepath = &*EXE_PATH;
    // 替换 file 中的 %Tools% 为 exepath
    let newfile = target_str.replace("%Tools%", exepath);
    if !Path::new(&newfile).exists() {
        log::error!("无效的路径: {}", newfile);
        return format!("无效的路径: {}", newfile);
    }
    let filepath = utils::get_file_path(&newfile);
    // if _data.get("Terminal").is_none() {
    //     return "Terminal路径不存在！".to_string();
    // }
    let terminal = if let Some(terminal) = _data.get("Terminal").and_then(|v| v.as_str()) {
        terminal.to_string()
    } else {
        #[cfg(target_os = "windows")]
        {
            "cmd.exe".to_string()
        }
        #[cfg(target_os = "linux")]
        {
            "bash".to_string()
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            log::warn!("不支持的操作系统，未设置默认终端");
            return "不支持的操作系统，未设置默认终端".to_string();
        }
    };
    let terminal_runas_arguments = _data
    .get("Terminal_Runas_Arguments")
    .and_then(|v| v.as_str())
    .unwrap_or("");
    let arguments = terminal_runas_arguments.replace("{DirectoryPath}", &filepath);

    let is_admin = _data
    .get("isAdmin")
    .and_then(|v| v.as_i64()) // Try to convert the value to i64 type
    .map(|s| s == 1) // Check if it equals 1
    .unwrap_or(false); 
// return run_as_admin(is_admin,&terminal, &arguments);
    // return  run_exec(&terminal, &arguments, is_admin, app_handle)
    // if cfg!(target_os = "windows") {
    match run_as_admin(is_admin,&terminal, &arguments) {
        Ok(_) => {
            log::info!("成功打开命令行: {}", filepath);
            return "ok".to_string();
        }
        Err(e) => {
            log::error!("无法打开命令行: {}", e);
            return e;
        }
    }
    // } else if cfg!(target_os = "linux") {

    // } else {
    //     // Handle other operating systems
    //     log::warn!("不支持的操作系统，未设置默认终端");
    //     return "不支持的操作系统，未设置默认终端".to_string();
    // }

}
#[tauri::command]
pub fn open_path(_tools: Map<String, Value>,app_handle: AppHandle) -> String {
    if let Some(target) = _tools.get("target") {
        let target_str = target.as_str().unwrap_or("");
        let exepath = &*EXE_PATH;
        // 替换 file 中的 %Tools% 为 exepath
        let newfile = target_str.replace("%Tools%", exepath);
        if !Path::new(&newfile).exists() {
            log::error!("无效的路径: {}", newfile);
            return format!("无效的路径: {}", newfile);
        }else{
            let result = utils::get_file_path(&newfile);
            return open_directory(&result, app_handle);
        }
    }else{
        log::error!("目标文件不存在！");
        return "目标文件不存在！".to_string();
    }
}
#[tauri::command]
pub fn copy_path(_tools: Map<String, Value>) -> String {
    if let Some(target) = _tools.get("target") {
        let target_str = target.as_str().unwrap_or("");
        let exepath = &*EXE_PATH;
        // 替换 file 中的 %Tools% 为 exepath
        let newfile = target_str.replace("%Tools%", exepath);
        return newfile;
    }else{
        return "目标文件不存在！".to_string();
    }

}

#[tauri::command]
pub fn open_tools(_tools: Map<String, Value>, app_handle: AppHandle) -> String {
    let toolsjson = json!(_tools).to_string();
    log::info!("Received tools: {}", toolsjson);
    if let Some(target) = _tools.get("target") {
        let target_str = target.as_str().unwrap_or("");
        let is_admin = _tools
            .get("isadmin")
            .and_then(|v| v.as_i64()) // 尝试将值转换为 i64 类型
            .map(|s| s == 1) // 判断是否等于 1
            .unwrap_or(false); // 如果转换失败或值不存在，默认返回 false
        let parameters_str = _tools
            .get("parameters")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let result = run_exec(target_str, parameters_str, is_admin, app_handle);
        if result == "ok" {
            if let Some(tool_id) = _tools.get("id").and_then(|v| v.as_i64()) {
                increment_tool_usage(tool_id);
            }
        }
        return result;
    } else {
        log::error!("目标文件不存在！");
        return "目标文件不存在！".to_string();
    }
}

fn increment_tool_usage(tool_id: i64) {
    if tool_id <= 0 {
        return;
    }

    match Connection::open(&*DB_PATH) {
        Ok(conn) => {
            if let Err(err) = conn.execute(
                "UPDATE tools SET number = COALESCE(number, 0) + 1 WHERE id = ?1",
                params![tool_id],
            ) {
                log::error!("Failed to increment tool usage: {}", err);
            }
        }
        Err(err) => {
            log::error!("Failed to open database for tool usage update: {}", err);
        }
    }
}

fn is_valid_executable(file: &str) -> bool {
    let path = Path::new(file);
    path.exists() && path.is_file()
}
fn run_exec(file: &str, args: &str, _isadmin: bool, app_handle: AppHandle) -> String {
    let exepath = &*EXE_PATH;
    // 替换 file 中的 %Tools% 为 exepath
    let newfile = file.replace("%Tools%", exepath);
    // 检查路径是否存在
    if !Path::new(&newfile).exists() {
        log::error!("无效的路径: {}", newfile);
        return format!("无效的路径: {}", newfile);
    }
    // 判断路径是文件还是目录
    match fs::metadata(&newfile) {
        Ok(metadata) => {
            if metadata.is_dir() {
                // 如果是目录，尝试打开它
                // 使用默认的应用来打开文件：
                return open_directory(&newfile, app_handle);
            } else if metadata.is_file() {
                return open_exe(&newfile, args, _isadmin,app_handle);
            } else {
                // 既不是文件也不是目录
                log::error!("路径既不是文件也不是目录: {}", newfile);
                return format!("路径既不是文件也不是目录: {}", newfile);
            }
        }
        Err(e) => {
            // 获取元数据失败
            log::error!("无法获取路径元数据: {} 错误: {}", newfile, e);
            return format!("无法获取路径元数据: {}", newfile);
        }
    }
}

/// 打开目录（支持 Windows 和 Linux、macos）
fn open_directory(path: &str, app_handle: AppHandle) -> String {
    log::info!("检测到目录，正在打开: {}", path);
    // 获取当前 App 实例
    let result = app_handle.opener().open_path(path, None::<&str>);
    match result {
        Ok(_) => "ok".to_string(),
        Err(e) => format!("无法打开目录: {}", e),
    }
    // let result = if cfg!(target_os = "windows") {
    //     // 在 Windows 系统下使用 explorer 命令打开目录
    //     Command::new("explorer")
    //        .arg(path)
    //        .spawn()
    // } else if cfg!(target_os = "linux") {
    //     // 在 Linux 系统下使用 xdg-open 命令打开目录
    //     Command::new("xdg-open")
    //        .arg(path)
    //        .spawn()
    // } else {
    //     log::warn!("打开目录功能仅支持 Windows 和 Linux 系统");
    //     return "打开目录功能仅支持 Windows 和 Linux 系统".to_string();
    // };
    // match result {
    //     Ok(_) => {
    //         // 打开成功，返回 "ok"
    //         "ok".to_string()
    //     }
    //     Err(e) => {
    //         // 打开失败，返回错误信息
    //         format!("无法打开目录: {}", e)
    //     }
    // }
}
/// 使用 ShellExecute 以管理员权限运行程序
#[cfg(target_os = "windows")]
fn run_as_admin(isadmin: bool, path: &str, args: &str) -> Result<(), String> {
    use crate::utils::get_file_path;

    let operation = if isadmin {
        "runas" // 请求管理员权限
    } else {
        "open"
    };
    let working_dir = get_file_path(path);
    let file = OsStr::new(path);
    let parameters = OsStr::new(args);
    let working_dir_os = OsStr::new(&working_dir);
    // 将路径和参数转换为宽字符（UTF-16）
    let wide_path: Vec<u16> = file.encode_wide().chain(Some(0)).collect();
    let wide_operation: Vec<u16> = OsStr::new(operation).encode_wide().chain(Some(0)).collect();
    let wide_parameters: Vec<u16> = parameters.encode_wide().chain(Some(0)).collect();
    let wide_working_dir: Vec<u16> = working_dir_os.encode_wide().chain(Some(0)).collect();
    unsafe {
        let result = ShellExecuteW(
            std::ptr::null_mut(),     // 父窗口句柄
            wide_operation.as_ptr(),  // 操作类型（runas 表示以管理员运行）
            wide_path.as_ptr(),       // 文件路径
            wide_parameters.as_ptr(), // 参数
            wide_working_dir.as_ptr(),         // 默认工作目录
            SW_SHOW,                  // 显示模式
        );

        if result as isize <= 32 {
            return Err(format!("管理员模式: {} 无法运行程序: {} ",isadmin.to_owned().to_string(),path));
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn run_as_admin(isadmin: bool, path: &str, args: &str) -> Result<(), String> {
    // Bug fix: Use path instead of undefined terminal
    let mut cmd = Command::new(path);
    // Bug fix: Use args instead of undefined arguments
    cmd.args(args.split_whitespace());

    if isadmin {
        // 以管理员权限运行
        let mut sudo_cmd = Command::new("sudo");
        sudo_cmd.arg(path);
        sudo_cmd.args(args.split_whitespace());
        match sudo_cmd.spawn() {
            Ok(_) => {
                // Bug fix: Return Ok(()) to match the return type
                log::info!("成功以管理员权限打开命令行: {}", path);
                Ok(())
            }
            Err(e) => {
                // Bug fix: Return Err() to match the return type
                log::error!("无法以管理员权限打开命令行: {}", e);
                Err(format!("无法以管理员权限打开命令行: {}", e))
            }
        }
    } else {
        // 不以管理员权限运行
        match cmd.spawn() {
            Ok(_) => {
                // Bug fix: Return Ok(()) to match the return type
                log::info!("成功打开命令行: {}", path);
                Ok(())
            }
            Err(e) => {
                // Bug fix: Return Err() to match the return type
                log::error!("无法打开命令行: {}", e);
                Err(format!("无法打开命令行: {}", e))
            }
        }
    }
}

/// 打开并运行可执行文件
fn open_exe(newfile: &str, args: &str, isadmin: bool,_app_handle: AppHandle) -> String {
    // 如果是文件，检查是否可执行并运行
    if !is_valid_executable(newfile) {
        log::error!("无效的可执行文件路径: {}", newfile);
        return format!("无效的可执行文件路径: {}", newfile);
    }
    log::info!(
        "正在执行文件：{} 参数：{} 是否管理员：{}",
        newfile,
        args,
        isadmin
    );
    // if cfg!(target_os = "windows") {
    match run_as_admin(isadmin,newfile, args) {
        Ok(_) => {
            log::info!("成功打开命令行: {}", newfile);
            return "ok".to_string();
        }
        Err(e) => {
            log::error!("无法打开命令行: {}", e);
            return e;
        }
    }
    // } else if cfg!(target_os = "linux") {
    //     if isadmin {
    //         // 在 Linux 系统下使用 sudo 以管理员权限执行
    //         let mut sudo_cmd = Command::new("sudo");
    //         sudo_cmd.arg(newfile);
    //         sudo_cmd.args(args.split_whitespace());
    //         match sudo_cmd.spawn() {
    //             Ok(_) => {
    //                 log::info!("命令:{} 参数:{} 管理员:{} 启动成功", newfile, args, isadmin);
    //                 return "ok".to_string();
    //             }
    //             Err(e) => {
    //                 return format!("Failed to start command: {}", e);
    //             }
    //         }
    //     }else{
    //             // // 不以管理员权限运行
    //         let mut cmd = Command::new(newfile);
    //         cmd.args(args.split_whitespace());
    //         match cmd.spawn() {
    //             Ok(_) => {
    //                 log::info!("命令:{} 参数:{} 管理员:{} 执行成功", newfile, args, isadmin);
    //                 return "ok".to_string();
    //             }
    //             Err(e) => {
    //                 return format!("Failed to start command: {}", e);
    //             }
    //         }
    //     }
    // } else {
    //     // Handle other operating systems
    //     log::warn!("不支持的操作系统");
    //     return "不支持的操作系统!".to_string();
    // }
    // if isadmin {
    //     if cfg!(target_os = "windows") {
    //         // 以管理员权限运行
    //         match run_as_admin(true,newfile, args) {
    //             Ok(_) => {
    //                 log::info!("成功以管理员权限运行程序: {}", newfile);
    //                 return "ok".to_string();
    //             }
    //             Err(e) => {
    //                 log::error!("无法以管理员权限运行程序: {}", e);
    //                 return e;
    //             }
    //         }
    //     } else if cfg!(target_os = "linux") {
    //         // 在 Linux 系统下使用 sudo 以管理员权限执行
    //         let mut sudo_cmd = Command::new("sudo");
    //         sudo_cmd.arg(newfile);
    //         sudo_cmd.args(args.split_whitespace());
    //         match sudo_cmd.spawn() {
    //             Ok(_) => {
    //                 log::info!("命令:{} 参数:{} 管理员:{} 启动成功", newfile, args, isadmin);
    //                 return "ok".to_string();
    //             }
    //             Err(e) => {
    //                 return format!("Failed to start command: {}", e);
    //             }
    //         }
    //     } else {
    //         log::warn!("以管理员模式启动功能仅支持 Windows 和 Linux 系统");
    //         return "以管理员模式启动功能仅支持 Windows 和 Linux 系统".to_string();
    //     }
    // } else {
    //     // // 不以管理员权限运行
    //     let mut cmd = Command::new(newfile);
    //     cmd.args(args.split_whitespace());

    //     // 执行命令并获取输出
    //     match cmd.output() {
    //         Ok(res) => {
    //             if res.status.success() {
    //                 // 执行成功，返回 "ok"
    //                 let output_str = String::from_utf8_lossy(&res.stdout);
    //                 log::info!(
    //                     "命令:{} 参数:{} 管理员:{} 执行成功: {}",
    //                     newfile,
    //                     args,
    //                     isadmin,
    //                     output_str
    //                 );
    //                 return "ok".to_string();
    //             } else {
    //                 // 执行失败，返回标准错误内容
    //                 return String::from_utf8_lossy(&res.stderr).to_string();
    //             }
    //         }
    //         Err(e) => {
    //             // 命令执行失败，返回错误信息
    //             return format!("Failed to execute command: {}", e);
    //         }
    //     }

    //     // let shell = app_handle.shell();
    //     // let output = tauri::async_runtime::block_on(async move {
    //     //     shell
    //     //         .command(newfile)
    //     //         .args(args.split_whitespace())
    //     //         .output()
    //     //         .await
    //     //         .unwrap()
    //     // });
    //     // if output.status.success() {
    //     //     log::info!(
    //     //         "命令:{} 参数:{} 管理员:{} 执行成功: {}",
    //     //         newfile,
    //     //         args,
    //     //         isadmin,
    //     //         String::from_utf8_lossy(&output.stdout)
    //     //     );
    //     //     return "ok".to_string();
    //     // } else {
    //     //     log::error!(
    //     //         "命令:{} 参数:{} 管理员:{} 执行失败: {}",
    //     //         newfile,
    //     //         args,
    //     //         isadmin,
    //     //         String::from_utf8_lossy(&output.stderr)
    //     //     );
    //     //     return String::from_utf8_lossy(&output.stderr).to_string();
    //     // }
    //     // if output.status.success() {
    //     //     println!("Result: {:?}", String::from_utf8(output.stdout));
    //     // } else {
    //     //     println!("Exit with code: {}", output.status.code().unwrap());
    //     // }
    // }
}
#[tauri::command]
pub fn get_exe_icon(filename: &str) -> String {
    let exepath = &*EXE_PATH;
    // 替换 file 中的 %Tools% 为 exepath
    let filepath = filename.replace("%Tools%", exepath);
    if let Ok(img) = image::open(&filepath) {
        let resized_img = img.thumbnail(32, 32).to_rgba8();
        let mut icon_img = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
        let offset_x = ((32 - resized_img.width()) / 2) as i64;
        let offset_y = ((32 - resized_img.height()) / 2) as i64;
        image::imageops::overlay(&mut icon_img, &resized_img, offset_x, offset_y);
        
        // Save the resized image to a buffer
        let mut buffer = Vec::new();
        // Use std::io::Cursor to wrap Vec<u8> because Cursor implements the Seek trait
        let mut cursor = std::io::Cursor::new(&mut buffer);
        icon_img.write_to(&mut cursor, image::ImageOutputFormat::Png).expect("Failed to write image to buffer");

        // Convert the buffer to a base64 string
        let base64 = STANDARD.encode(&buffer);
        return "data:image/png;base64,".to_owned()+&base64;
    } else {
        #[cfg(target_os = "windows")]
        return get_windows_exe_icon(&filepath);
    }
}
use winapi::um::wingdi::DIB_RGB_COLORS;
use winapi::um::wingdi::CreateDIBSection;
use winapi::um::wingdi::{DeleteObject, SelectObject};
use std::slice;
pub fn get_windows_exe_icon(filename: &str) -> String {
    match get_file_icon(filename) {
        Some(hicon) => {
            match icon_to_image(hicon) {
                Ok(image) => {
                    // 将图像转换为 Base64 字符串
                    match imagebuffer_to_base64(&image) {
                        Ok(base64_string) => {
                            return  "data:image/png;base64,".to_owned()+&base64_string;
                        }
                        Err(err) => format!("Error: {}", err),
                    }
                }
                Err(e) => format!("Error: {}", e),
            }
        }
        None => "Error: Failed to get file icon".to_string(),
    }
}
/// 将 ImageBuffer 转换为 Base64 字符串
fn imagebuffer_to_base64(image: &RgbaImage) -> Result<String, String> {
    // 创建一个缓冲区来存储 PNG 数据
    let mut png_data = Vec::new();
    // 将 ImageBuffer 编码为 PNG 格式
    // 使用 Cursor 来包装 Vec<u8>，因为 Cursor 实现了 Seek 特性
    let mut cursor = std::io::Cursor::new(&mut png_data);
    if let Err(err) = image.write_to(&mut cursor, image::ImageOutputFormat::Png) {
        return Err(format!("Failed to encode image to PNG: {}", err));
    }
    // 使用 STANDARD 引擎将 PNG 数据编码为 Base64 字符串
    let base64_string = STANDARD.encode(&png_data);
    Ok(base64_string)
}

fn icon_to_image(hicon: HICON) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    unsafe {
        let hdc = CreateCompatibleDC(null_mut());
        if hdc.is_null() {
            return Err("Failed to create compatible DC".to_string());
        }

        let mut icon_info = ICONINFO {
            fIcon: 0,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: null_mut(),
            hbmColor: null_mut(),
        };

        if GetIconInfo(hicon, &mut icon_info) == 0 {
            DeleteDC(hdc);
            DestroyIcon(hicon);
            return Err("Failed to get icon info".to_string());
        }

        let bmp_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 32,
                biHeight: -32, // Negative height to indicate a top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }; 1],
        };

        let mut bits: *mut u8 = null_mut();
        let hbitmap = CreateDIBSection(
            hdc,
            &bmp_info,
            DIB_RGB_COLORS,
            &mut bits as *mut *mut u8 as *mut *mut winapi::ctypes::c_void,
            null_mut(),
            0,
        );

        if hbitmap.is_null() {
            DeleteObject(icon_info.hbmColor as _);
            DeleteObject(icon_info.hbmMask as _);
            DestroyIcon(hicon);
            DeleteDC(hdc);
            return Err("Failed to create DIB section".to_string());
        }

        let old_bitmap = SelectObject(hdc, hbitmap as _);
        if old_bitmap.is_null() {
            DeleteObject(hbitmap as _);
            DeleteObject(icon_info.hbmColor as _);
            DeleteObject(icon_info.hbmMask as _);
            DestroyIcon(hicon);
            DeleteDC(hdc);
            return Err("Failed to select DIB section".to_string());
        }

        let size = (32 * 32 * 4) as usize;
        std::ptr::write_bytes(bits, 0, size);

        if DrawIconEx(hdc, 0, 0, hicon, 32, 32, 0, null_mut(), ICON_DRAW_NORMAL) == 0 {
            SelectObject(hdc, old_bitmap);
            DeleteObject(hbitmap as _);
            DeleteObject(icon_info.hbmColor as _);
            DeleteObject(icon_info.hbmMask as _);
            DestroyIcon(hicon);
            DeleteDC(hdc);
            return Err("Failed to draw icon".to_string());
        }

        let slice = slice::from_raw_parts(bits, size);

        // Convert BGRA to RGBA
        let mut rgba_data = Vec::with_capacity(size);
        for chunk in slice.chunks(4) {
            rgba_data.push(chunk[2]); // R
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[0]); // B
            rgba_data.push(chunk[3]); // A
        }

        let image = ImageBuffer::from_raw(32, 32, rgba_data)
            .ok_or_else(|| "Failed to create image buffer".to_string())?;

        SelectObject(hdc, old_bitmap);
        DeleteObject(hbitmap as _);
        DeleteObject(icon_info.hbmColor as _);
        DeleteObject(icon_info.hbmMask as _);
        DestroyIcon(hicon);
        DeleteDC(hdc);
        Ok(image)
    }
}

/// 使用 SHGetFileInfoW 获取文件图标句柄 (HICON)
fn get_file_icon(file_path: &str) -> Option<HICON> {
    let mut file_info = winapi::um::shellapi::SHFILEINFOW {
        hIcon: null_mut(),
        iIcon: 0,
        dwAttributes: 0,
        szDisplayName: [0; 260],
        szTypeName: [0; 80],
    };

    // 将路径转换为宽字符（UTF-16）
    let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
    let is_directory = Path::new(file_path).is_dir();
    let attributes = if is_directory { FILE_ATTRIBUTE_DIRECTORY } else { 0 };
    let flags = if is_directory {
        SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES
    } else {
        SHGFI_ICON | SHGFI_LARGEICON
    };
    unsafe {
        if SHGetFileInfoW(
            wide_path.as_ptr(),
            attributes,
            &mut file_info,
            std::mem::size_of::<winapi::um::shellapi::SHFILEINFOW>() as u32,
            flags,
        ) != 0
        {
            Some(file_info.hIcon)
        } else {
            None
        }
    }
}



#[tauri::command]
pub fn update_tool(tool: Map<String, Value>) -> String {
    let config_tool = json!(tool).to_string();
    // 使用 info! 宏记录日志
    log::info!("Update Tool: {}", config_tool);
    // 连接到 SQLite 数据库
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    let id = tool.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
    let query = if id <= 0 {
        // 新增记录
        "INSERT INTO tools (name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    } else {
        // 更新记录
        "UPDATE tools SET name = ?1, icon = ?2, target = ?3, parameters = ?4, isadmin = ?5, number = ?6, menu_id = ?7, tags_id = ?8 WHERE id = ?9"
    };
    // let query = "INSERT OR REPLACE INTO tools (id, name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or_default();
    let icon = tool.get("icon").and_then(|v| v.as_str()).unwrap_or_default();
    let target = tool.get("target").and_then(|v| v.as_str()).unwrap_or_default();
    let parameters = tool.get("parameters").and_then(|v| v.as_str()).unwrap_or_default();
    let isadmin = tool.get("isadmin").and_then(|v| v.as_bool()).unwrap_or(false);
    let number = tool.get("number").and_then(|v| v.as_i64()).unwrap_or_default();
    let menu_id = tool.get("menu_id").and_then(|v| v.as_i64()).unwrap_or_default();
    let tags_id = tool.get("tags_id").and_then(|v| v.as_i64()).unwrap_or_default();
    let result;
    if id > 0 {
        result = conn.execute(query, params![
            name,
            icon,
            target,
            parameters,
            isadmin,
            number,
            menu_id,
            tags_id,
            id,
        ]);
    }  else{
        result = conn.execute(query, params![
            name,
            icon,
            target,
            parameters,
            isadmin,
            number,
            menu_id,
            tags_id
        ]);
    }

    if let Err(err) = result {
        log::error!("Failed to update tool: {}", err);
        return format!("error:{}", err).to_string();
    }

    return "ok".to_string(); 
}


#[tauri::command]
pub fn delete_tool(tool_id: i64) -> String {
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    log::info!("Delete Tool: {}", tool_id);
    let sql_str =  "DELETE FROM tools WHERE id = ?1";
    // let query = "INSERT OR REPLACE INTO tools (id, name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let  result = conn.execute(sql_str, params![
        tool_id,
    ]);

    if let Err(err) = result {
        log::error!("Failed to delete tool: {}", err);
        return format!("error:{}", err).to_string();
    }
    return "ok".to_string(); 
}
#[tauri::command]
pub fn delete_menu(menu_id: i64,tags_id_list: Vec<i64>) -> String {
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    log::info!("Delete Menu: {}", menu_id);
    let sql_str =  "DELETE FROM menu WHERE id = ?1";
    // let query = "INSERT OR REPLACE INTO tools (id, name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let  result = conn.execute(sql_str, params![
        menu_id,
    ]);

    if let Err(err) = result {
        log::error!("Failed to delete menu: {}", err);
        return format!("error:{}", err).to_string();
    }
    for tag_id in tags_id_list {
        delete_tag(menu_id, tag_id);
    }
    return "ok".to_string(); 
}
#[tauri::command]
pub fn delete_tag(menu_id:i64,tags_id: i64) -> String {
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    log::info!("Delete Tag: {}", tags_id);
    let sql_str =  "DELETE FROM tags WHERE id = ?1";
    // let query = "INSERT OR REPLACE INTO tools (id, name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let  result = conn.execute(sql_str, params![
        tags_id,
    ]);

    if let Err(err) = result {
        log::error!("Failed to delete tag: {}", err);
        return format!("error:{}", err).to_string();
    }
    //清除工具
    clear_tool(menu_id, tags_id);
    return "ok".to_string(); 
}
#[tauri::command]
pub fn clear_tool(menu_id: i64,tags_id: i64) -> String {
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    log::info!("Clear Tool: menu_id:{} tags_id:{}", menu_id,tags_id);
    let mut sql_str =  "DELETE FROM tools WHERE menu_id =?1 and tags_id =?2 ";
    let result;
    if tags_id <= 0 {
        sql_str = "DELETE FROM tools WHERE menu_id =?1";
        result = conn.execute(sql_str, params![
            menu_id
        ]);
    }else{
        result = conn.execute(sql_str, params![
            menu_id,tags_id
        ]);
    }
    if let Err(err) = result {
        log::error!("Failed to clear tool: {}", err);
        return format!("error:{}", err).to_string();
    }
    return "ok".to_string(); 
}

#[tauri::command]
//清空无效项目

pub fn clear_tool_no(menu_id: i64, tags_id: i64) -> String {
    match load_data::select_tools_by_menu_and_tags(menu_id, tags_id) {
        Ok(tools) => {
            let exepath = &*EXE_PATH;
            let all_delete_tools: Vec<i32> = tools
                .into_iter()
                .filter(|tool| {
                    let newfile = tool.target.replace("%Tools%", exepath);
                    !std::path::Path::new(&newfile).exists()
                })
                .map(|tool| tool.id)
                .collect();

            if all_delete_tools.is_empty() {
                return "ok".to_string();
            }

            let conn = match Connection::open(&*DB_PATH) {
                Ok(c) => c,
                Err(e) => return format!("error:{}", e),
            };
            let placeholders = vec!["?"; all_delete_tools.len()].join(", ");
            let sql_str = format!("DELETE FROM tools WHERE id IN ({})", placeholders);
            // 将 Vec<i32> 转换成 Vec<&dyn ToSql>
            let params: Vec<&dyn ToSql> = all_delete_tools
                .iter()
                .map(|id| id as &dyn ToSql)
                .collect();
            match conn.execute(&sql_str, &params[..]) {
                Ok(_) => {
                    log::info!(
                        "Clear Tool: menu_id:{} tags_id:{} tool:{:?}",
                        menu_id,
                        tags_id,
                        all_delete_tools
                    );
                    "ok".to_string()
                }
                Err(e) => {
                    log::error!("Failed to clear no tool: {}", e);
                    format!("error:{}", e)
                }
            }
        }
        Err(err) => {
            log::error!("查询工具时出错: {}", err);
            format!("查询工具时出错: {}", err)
        }
    }
}



#[tauri::command]
pub fn update_tag(obj: Map<String, Value>) -> String {
    let config_tool = json!(obj).to_string();
    // 使用 info! 宏记录日志
    log::info!("Update Tag: {}", config_tool);
    // 连接到 SQLite 数据库
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    let id = obj.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
    let query = if id <= 0 {
        // 新增记录
        "INSERT INTO tags (name, menu_id, sort) VALUES (?1, ?2, ?3)"
    } else {
        // 更新记录
        "UPDATE tags SET name = ?1, menu_id = ?2, sort = ?3 WHERE id = ?4"
    };
    // let query = "INSERT OR REPLACE INTO tools (id, name, icon, target, parameters, isadmin, number, menu_id, tags_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default();
    let menu_id = obj.get("menu_id").and_then(|v| v.as_i64()).unwrap_or_default();
    let sort = obj.get("sort").and_then(|v| v.as_i64()).unwrap_or_default();
    let result;
    if id > 0 {
        result = conn.execute(query, params![
            name,
            menu_id,
            sort,
            id,
        ]);
    }  else{
        result = conn.execute(query, params![
            name,
            menu_id,
            sort,
        ]);
    }
    if let Err(err) = result {
        log::error!("Failed to update tag: {}", err);
        return format!("error:{}", err).to_string();
    }
    return "ok".to_string(); 
}
#[tauri::command]
pub fn update_menu(obj: Map<String, Value>) -> String {
    let config_tool = json!(obj).to_string();
    // 使用 info! 宏记录日志
    log::info!("Update menu: {}", config_tool);
    // 连接到 SQLite 数据库
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    let id = obj.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
    let query = if id <= 0 {
        // 新增记录
        "INSERT INTO menu (name,sort) VALUES (?1, ?2)"
    } else {
        // 更新记录
        "UPDATE menu SET name = ?1, sort = ?2 WHERE id = ?3"
    };
    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default();
    let sort = obj.get("sort").and_then(|v| v.as_i64()).unwrap_or_default();
    let result;
    if id > 0 {
        result = conn.execute(query, params![
            name,
            sort,
            id,
        ]);
    }  else{
        result = conn.execute(query, params![
            name,
            sort,
        ]);
    }
    if let Err(err) = result {
        log::error!("Failed to update menu: {}", err);
        return format!("error:{}", err).to_string();
    }
    return "ok".to_string(); 
}

#[tauri::command]
pub fn change_bianxie_path(all_tool: Vec<Tools>,is_bianxie:bool) -> String {
    let exepath = &*EXE_PATH;
    log::info!("Change Bianxie Path: {}", exepath);
    let mut all_update_tools: Vec<Tools> = Vec::new();
    for mut  tool in all_tool  {
        let tool_targets = tool.target;
        if is_bianxie{
            //转换为便携路径
            if tool_targets.starts_with(exepath){
                tool.target = tool_targets.replace(exepath,"%Tools%");
                all_update_tools.push(tool);
            } 
        }else{
            //转换为绝对路径
            if tool_targets.starts_with("%Tools%"){
                tool.target = tool_targets.replace("%Tools%",exepath);
                all_update_tools.push(tool);
            }
        }
        
    }
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();
    let sql =   "UPDATE tools SET target = ?1 WHERE id = ?2";
    let mut error_str = String::new();
    for tool in all_update_tools  {
        let result = conn.execute(sql, params![
            tool.target,
            tool.id
        ]);
        if let Err(err) = result {
            log::error!("Failed to update tool: {}", err);
            error_str.push_str(&format!("ID {}: {}\n", tool.id, err));
        }
    }
    if !error_str.is_empty() {
        return format!("Errors occurred:\n{}", error_str);
    }
    return "ok".to_string(); 
}
