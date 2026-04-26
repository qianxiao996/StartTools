use crate::utils::{DB_PATH, EXE_PATH};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
#[cfg(target_os = "windows")]
use winapi::shared::windef::POINT;
#[cfg(target_os = "windows")]
use winapi::um::wingdi::GetPixel;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetCursorPos, GetDC, ReleaseDC};

static PLUGIN_SERVER_PORT: once_cell::sync::Lazy<Mutex<Option<u16>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));
static PLUGIN_SERVER_HOST: once_cell::sync::Lazy<Mutex<Option<String>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));
static PLUGIN_SERVER_GENERATION: once_cell::sync::Lazy<Mutex<u64>> =
    once_cell::sync::Lazy::new(|| Mutex::new(0));

const DEFAULT_PLUGIN_HTTP_HOST: &str = "127.0.0.1";
const DEFAULT_PLUGIN_HTTP_PORT: u16 = 13678;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartToolsPlugin {
    pub id: i64,
    pub plugin_id: String,
    pub name: String,
    pub main: String,
    pub logo: String,
    #[serde(default)]
    pub logo_url: String,
    pub preload: String,
    #[serde(default = "default_window_mode")]
    pub window_mode: String,
    #[serde(default = "default_window_width")]
    pub window_width: i64,
    #[serde(default = "default_window_height")]
    pub window_height: i64,
    pub sort: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct StartToolsPluginBackup {
    version: u32,
    plugins: Vec<StartToolsPlugin>,
    files: Vec<StartToolsPluginBackupFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StartToolsPluginBackupFile {
    path: String,
    data_base64: String,
}

fn default_window_mode() -> String {
    "default".to_string()
}

fn default_window_width() -> i64 {
    1080
}

fn default_window_height() -> i64 {
    680
}

fn normalize_window_mode(mode: &str) -> String {
    if mode.trim().eq_ignore_ascii_case("maximized") {
        "maximized".to_string()
    } else {
        "default".to_string()
    }
}

fn normalize_window_size(value: i64, fallback: i64) -> i64 {
    value.clamp(360, 7680).max(fallback.min(360))
}

#[tauri::command]
pub fn load_starttools_plugins() -> Result<Vec<StartToolsPlugin>, String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, plugin_id, name, main, logo, preload, window_mode, window_width, window_height, sort
             FROM starttools_plugin_structure
             WHERE entry_kind = 'plugin'
             ORDER BY sort ASC, id ASC",
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let mut plugin = StartToolsPlugin {
                id: row.get(0)?,
                plugin_id: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or_else(generate_plugin_id),
                name: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "StartTools".to_string()),
                main: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "index.html".to_string()),
                logo: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "logo.svg".to_string()),
                logo_url: String::new(),
                preload: row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_default(),
                window_mode: normalize_window_mode(
                    &row.get::<_, Option<String>>(6)?
                        .unwrap_or_else(default_window_mode),
                ),
                window_width: normalize_window_size(row.get::<_, Option<i64>>(7)?.unwrap_or_else(default_window_width), default_window_width()),
                window_height: normalize_window_size(row.get::<_, Option<i64>>(8)?.unwrap_or_else(default_window_height), default_window_height()),
                sort: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            };
            plugin.logo_url = plugin_asset_url(&plugin, &plugin.logo).unwrap_or_default();
            Ok(plugin)
        })
        .map_err(|err| err.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn export_starttools_plugins(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导出路径不能为空".to_string());
    }
    let plugins = load_starttools_plugins()?;
    let plugins_dir = PathBuf::from(&*EXE_PATH).join("plugins");
    let mut files = Vec::new();
    if plugins_dir.exists() {
        for plugin in &plugins {
            let plugin_dir = plugins_dir.join(&plugin.plugin_id);
            if plugin_dir.exists() {
                collect_plugin_backup_files(&plugins_dir, &plugin_dir, &mut files)?;
            }
        }
    }
    let backup = StartToolsPluginBackup {
        version: 1,
        plugins,
        files,
    };
    let text = serde_json::to_string(&backup).map_err(|err| format!("生成插件备份失败: {}", err))?;
    fs::write(path, text).map_err(|err| format!("导出插件失败: {}", err))
}

#[tauri::command]
pub fn import_starttools_plugins(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导入路径不能为空".to_string());
    }
    let text = fs::read_to_string(&path).map_err(|err| format!("读取插件备份失败: {}", err))?;
    let backup: StartToolsPluginBackup = serde_json::from_str(&text).map_err(|err| format!("插件备份格式不正确: {}", err))?;
    backup_starttools_plugins()?;

    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.execute(
        "DELETE FROM starttools_plugin_structure WHERE entry_kind = 'plugin'",
        [],
    )
    .map_err(|err| format!("清理插件记录失败: {}", err))?;

    let plugins_dir = PathBuf::from(&*EXE_PATH).join("plugins");
    if plugins_dir.exists() {
        fs::remove_dir_all(&plugins_dir).map_err(|err| format!("清理插件目录失败: {}", err))?;
    }
    fs::create_dir_all(&plugins_dir).map_err(|err| format!("创建插件目录失败: {}", err))?;

    for mut plugin in backup.plugins {
        if plugin.plugin_id.trim().is_empty() {
            plugin.plugin_id = generate_plugin_id();
        }
        plugin.name = if plugin.name.trim().is_empty() {
            plugin.plugin_id.clone()
        } else {
            plugin.name
        };
        plugin.main = if plugin.main.trim().is_empty() {
            "index.html".to_string()
        } else {
            plugin.main
        };
        plugin.logo = if plugin.logo.trim().is_empty() {
            "logo.svg".to_string()
        } else {
            plugin.logo
        };
        plugin.window_mode = normalize_window_mode(&plugin.window_mode);
        plugin.window_width = normalize_window_size(plugin.window_width, default_window_width());
        plugin.window_height = normalize_window_size(plugin.window_height, default_window_height());
        conn.execute(
            "INSERT OR REPLACE INTO starttools_plugin_structure
             (id, entry_kind, plugin_id, name, main, logo, preload, single, height, window_mode, window_width, window_height, sort)
             VALUES (?1, 'plugin', ?2, ?3, ?4, ?5, ?6, 1, 640, ?7, ?8, ?9, ?10)",
            params![
                plugin.id,
                plugin.plugin_id,
                plugin.name,
                plugin.main,
                plugin.logo,
                plugin.preload,
                plugin.window_mode,
                plugin.window_width,
                plugin.window_height,
                plugin.sort,
            ],
        )
        .map_err(|err| format!("导入插件记录失败: {}", err))?;
    }

    for file in backup.files {
        let relative = safe_plugin_relative_path(&file.path)?;
        let target = plugins_dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建插件目录失败: {}", err))?;
        }
        let bytes = BASE64_STANDARD
            .decode(file.data_base64)
            .map_err(|err| format!("解析插件文件失败 {}: {}", file.path, err))?;
        fs::write(&target, bytes).map_err(|err| format!("写入插件文件失败 {}: {}", file.path, err))?;
    }
    Ok(())
}

#[tauri::command]
pub fn update_starttools_plugin(mut plugin: StartToolsPlugin) -> Result<StartToolsPlugin, String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    if plugin.plugin_id.trim().is_empty() {
        plugin.plugin_id = generate_plugin_id();
    }
    if plugin.name.trim().is_empty() {
        plugin.name = plugin.plugin_id.clone();
    }
    if plugin.main.trim().is_empty() {
        plugin.main = "index.html".to_string();
    }
    if plugin.logo.trim().is_empty() {
        plugin.logo = "logo.svg".to_string();
    }
    plugin.preload = plugin.preload.trim().to_string();
    plugin.window_mode = normalize_window_mode(&plugin.window_mode);
    plugin.window_width = normalize_window_size(plugin.window_width, default_window_width());
    plugin.window_height = normalize_window_size(plugin.window_height, default_window_height());
    if plugin.id <= 0 {
        ensure_plugin_directory(&plugin)?;
        conn.execute(
            "INSERT INTO starttools_plugin_structure
             (entry_kind, plugin_id, name, main, logo, preload, single, height, window_mode, window_width, window_height, sort)
             VALUES ('plugin', ?1, ?2, ?3, ?4, ?5, 1, 640, ?6, ?7, ?8, ?9)",
            params![
                plugin.plugin_id,
                plugin.name,
                plugin.main,
                plugin.logo,
                plugin.preload,
                plugin.window_mode,
                plugin.window_width,
                plugin.window_height,
                plugin.sort,
            ],
        )
        .map_err(|err| err.to_string())?;
        plugin.id = conn.last_insert_rowid();
        return Ok(plugin);
    }

    ensure_plugin_directory(&plugin)?;
    conn.execute(
        "UPDATE starttools_plugin_structure
         SET plugin_id = ?1, name = ?2, main = ?3, logo = ?4, preload = ?5, window_mode = ?6, window_width = ?7, window_height = ?8, sort = ?9
         WHERE id = ?10 AND entry_kind = 'plugin'",
        params![
            plugin.plugin_id,
            plugin.name,
            plugin.main,
            plugin.logo,
            plugin.preload,
            plugin.window_mode,
            plugin.window_width,
            plugin.window_height,
            plugin.sort,
            plugin.id,
        ],
    )
    .map_err(|err| err.to_string())?;
    Ok(plugin)
}

#[tauri::command]
pub fn delete_starttools_plugin(plugin_id: i64) -> Result<(), String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.execute(
        "DELETE FROM starttools_plugin_structure WHERE id = ?1 AND entry_kind = 'plugin'",
        params![plugin_id],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_starttools_plugin(_app: AppHandle, plugin_id: String) -> Result<String, String> {
    let plugin = load_starttools_plugin_by_plugin_id(&plugin_id)?;
    ensure_plugin_directory(&plugin)?;
    let entry = resolve_plugin_path(&plugin, &plugin.main);
    if !entry.exists() {
        return Err(format!("插件入口不存在: {}", entry.to_string_lossy()));
    }
    repair_blank_default_index(&entry, &plugin)?;

    let plugin_url = plugin_entry_url(&plugin, &entry)?;
    validate_plugin_url(&plugin_url)?;
    open_plugin_browser_window(&plugin_url, &plugin)?;
    Ok(plugin_url)
}

#[tauri::command]
pub fn open_starttools_plugin_dir(plugin_id: String) -> Result<(), String> {
    let plugin = load_starttools_plugin_by_plugin_id(&plugin_id)?;
    ensure_plugin_directory(&plugin)?;
    let plugin_dir = PathBuf::from(&*EXE_PATH).join("plugins").join(&plugin.plugin_id);
    open_with_system(&plugin_dir)
}

fn collect_plugin_backup_files(root: &Path, current: &Path, files: &mut Vec<StartToolsPluginBackupFile>) -> Result<(), String> {
    for item in fs::read_dir(current).map_err(|err| format!("读取插件目录失败: {}", err))? {
        let item = item.map_err(|err| format!("读取插件目录失败: {}", err))?;
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        if name == ".starttools-browser-profiles" {
            continue;
        }
        if path.is_dir() {
            collect_plugin_backup_files(root, &path, files)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|err| format!("计算插件相对路径失败: {}", err))?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(&path).map_err(|err| format!("读取插件文件失败 {}: {}", relative, err))?;
        files.push(StartToolsPluginBackupFile {
            path: relative,
            data_base64: BASE64_STANDARD.encode(bytes),
        });
    }
    Ok(())
}

fn safe_plugin_relative_path(path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    if relative.is_absolute() {
        return Err(format!("插件路径不能是绝对路径: {}", path));
    }
    let mut safe = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            _ => return Err(format!("插件路径不安全: {}", path)),
        }
    }
    Ok(safe)
}

fn backup_starttools_plugins() -> Result<(), String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.execute("DROP TABLE IF EXISTS starttools_plugin_structure_import_backup", [])
        .map_err(|err| format!("清理插件记录备份失败: {}", err))?;
    conn.execute(
        "CREATE TABLE starttools_plugin_structure_import_backup AS SELECT * FROM starttools_plugin_structure WHERE entry_kind = 'plugin'",
        [],
    )
    .map_err(|err| format!("备份插件记录失败: {}", err))?;

    let plugins_dir = PathBuf::from(&*EXE_PATH).join("plugins");
    if plugins_dir.exists() {
        let backup_dir = PathBuf::from(&*EXE_PATH).join("plugins.import.bak");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).map_err(|err| format!("清理插件目录备份失败: {}", err))?;
        }
        copy_dir_all(&plugins_dir, &backup_dir).map_err(|err| format!("备份插件目录失败: {}", err))?;
    }
    Ok(())
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for item in fs::read_dir(from)? {
        let item = item?;
        let item_type = item.file_type()?;
        let target = to.join(item.file_name());
        if item_type.is_dir() {
            copy_dir_all(&item.path(), &target)?;
        } else if item_type.is_file() {
            fs::copy(item.path(), target)?;
        }
    }
    Ok(())
}

pub fn start_plugin_http_server() -> Result<u16, String> {
    ensure_plugin_http_server()
}

#[tauri::command]
pub fn restart_starttools_plugin_http_server() -> Result<serde_json::Value, String> {
    {
        let mut generation = PLUGIN_SERVER_GENERATION
            .lock()
            .map_err(|err| err.to_string())?;
        *generation = generation.saturating_add(1);
    }
    {
        let mut port = PLUGIN_SERVER_PORT.lock().map_err(|err| err.to_string())?;
        *port = None;
    }
    {
        let mut host = PLUGIN_SERVER_HOST.lock().map_err(|err| err.to_string())?;
        *host = None;
    }

    std::thread::sleep(Duration::from_millis(120));
    let port = ensure_plugin_http_server()?;
    Ok(serde_json::json!({
        "host": plugin_http_host(),
        "port": port,
    }))
}

fn generate_plugin_id() -> String {
    format!("starttools_{}", current_millis())
}

fn ensure_plugin_directory(plugin: &StartToolsPlugin) -> Result<(), String> {
    if PathBuf::from(&plugin.main).is_absolute() {
        return Ok(());
    }

    let plugin_dir = PathBuf::from(&*EXE_PATH).join("plugins").join(&plugin.plugin_id);
    fs::create_dir_all(&plugin_dir).map_err(|err| err.to_string())?;

    write_if_missing(
        &plugin_dir.join("index.html"),
        &default_index_html(&plugin.name),
    )?;
    write_if_missing(
        &plugin_dir.join("preload.js"),
        default_preload_js(),
    )?;
    write_if_missing(
        &plugin_dir.join("logo.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect width="128" height="128" rx="24" fill="#1f2937"/>
  <path d="M30 38h68v16H30zM30 62h42v16H30zM30 86h68v16H30z" fill="#f8fafc"/>
</svg>
"##,
    )?;

    Ok(())
}

fn write_if_missing(path: &PathBuf, content: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, content).map_err(|err| err.to_string())
}

fn repair_blank_default_index(path: &Path, plugin: &StartToolsPlugin) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let compact = content.split_whitespace().collect::<String>();
    let is_blank_generated_page = compact.contains("<divid=\"app\"></div>")
        && !compact.contains("StartToolsPluginReady")
        && !compact.contains("startToolsPluginBridge");

    if is_blank_generated_page {
        fs::write(path, default_index_html(&plugin.name)).map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn default_index_html(title: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
      body {{
        margin: 0;
        font-family: "Segoe UI", "Microsoft YaHei", sans-serif;
        background: #f7f8fb;
        color: #172033;
      }}
      main {{
        max-width: 720px;
        margin: 0 auto;
        padding: 28px;
      }}
      h1 {{
        margin: 0 0 12px;
        font-size: 24px;
      }}
      .panel {{
        display: grid;
        gap: 10px;
        padding: 14px;
        border: 1px solid #d9dee8;
        border-radius: 8px;
        background: #ffffff;
      }}
      code {{
        font-family: Consolas, "Cascadia Mono", monospace;
      }}
    </style>
  </head>
  <body>
    <main>
      <h1>StartToolsPluginReady</h1>
      <section class="panel">
        <div>插件页面已加载。</div>
        <div>preload 注入信息：<code id="pluginMeta">等待检测</code></div>
        <div>桥接对象：<code id="bridgeState">等待检测</code></div>
      </section>
    </main>
    <script>
      document.querySelector('#pluginMeta').textContent = JSON.stringify(window.__STARTTOOLS_PLUGIN__ || {{}});
      document.querySelector('#bridgeState').textContent = window.startToolsPlugin || window.startToolsPluginBridge ? '已存在' : '未检测到';
    </script>
  </body>
</html>
"#,
        title = title
    )
}

fn default_preload_js() -> &'static str {
    r#"(function () {
  const meta = window.__STARTTOOLS_PLUGIN__ || {};
  const apiBase = "/__starttools_api";

  async function nativeCall(name, payload) {
    const response = await fetch(apiBase + "/" + name, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload || {})
    });
    const result = await response.json().catch(() => ({}));
    if (!response.ok || !result.ok) {
      throw new Error(result.error || ("StartTools native call failed: " + name));
    }
    return result.data;
  }

  const dialog = {
    openFile(options) {
      return nativeCall("dialog/open", Object.assign({}, options || {}, {
        multiple: false,
        directory: false,
        save: false
      }));
    },
    openFiles(options) {
      return nativeCall("dialog/open", Object.assign({}, options || {}, {
        multiple: true,
        directory: false,
        save: false
      }));
    },
    openDirectory(options) {
      return nativeCall("dialog/open", Object.assign({}, options || {}, {
        multiple: false,
        directory: true,
        save: false
      }));
    },
    openDirectories(options) {
      return nativeCall("dialog/open", Object.assign({}, options || {}, {
        multiple: true,
        directory: true,
        save: false
      }));
    },
    saveFile(options) {
      return nativeCall("dialog/open", Object.assign({}, options || {}, {
        multiple: false,
        directory: false,
        save: true
      }));
    }
  };

  const fs = {
    readText(path) {
      return nativeCall("fs/read_text", { path });
    },
    writeText(path, content) {
      return nativeCall("fs/write_text", { path, content });
    },
    readBytes(path) {
      return nativeCall("fs/read_bytes", { path });
    },
    writeBytes(path, base64) {
      return nativeCall("fs/write_bytes", { path, base64 });
    },
    exists(path) {
      return nativeCall("fs/exists", { path });
    },
    stat(path) {
      return nativeCall("fs/stat", { path });
    },
    mkdir(path) {
      return nativeCall("fs/mkdir", { path });
    },
    remove(path) {
      return nativeCall("fs/remove", { path });
    },
    copy(from, to) {
      return nativeCall("fs/copy", { from, to });
    },
    rename(from, to) {
      return nativeCall("fs/rename", { from, to });
    },
    readDir(path) {
      return nativeCall("fs/read_dir", { path });
    }
  };

  const path = {
    normalize(value) {
      return String(value || "").replace(/\\/g, "/").replace(/\/+/g, "/");
    },
    join(...parts) {
      return path.normalize(parts.filter((part) => part != null && part !== "").join("/"));
    },
    basename(value) {
      const normalized = path.normalize(value);
      const parts = normalized.split("/").filter(Boolean);
      return parts.length ? parts[parts.length - 1] : "";
    },
    dirname(value) {
      const normalized = path.normalize(value);
      const index = normalized.lastIndexOf("/");
      return index > 0 ? normalized.slice(0, index) : "";
    },
    extname(value) {
      const base = path.basename(value);
      const index = base.lastIndexOf(".");
      return index > 0 ? base.slice(index) : "";
    }
  };

  const shell = {
    openPath(path) {
      return nativeCall("shell/open_path", { path });
    },
    showInFolder(path) {
      return nativeCall("shell/show_in_folder", { path });
    },
    openUrl(url) {
      return nativeCall("shell/open_url", { url });
    },
    exec(command, args, options) {
      return nativeCall("shell/exec", { command, args, options });
    }
  };

  const clipboard = {
    async writeText(text) {
      await navigator.clipboard.writeText(String(text == null ? "" : text));
      return true;
    },
    readText() {
      return navigator.clipboard.readText();
    },
    async writeImage(base64, type) {
      return nativeCall("clipboard/write_image", { base64, type });
    },
    writeFiles(paths) {
      return nativeCall("clipboard/write_files", { paths });
    },
    async readImage() {
      return nativeCall("clipboard/read_image", {});
    },
    clear() {
      return nativeCall("clipboard/clear", {});
    }
  };

  const storagePrefix = "starttools:" + (meta.id || "plugin") + ":";
  const storage = {
    get(key, fallbackValue) {
      const raw = localStorage.getItem(storagePrefix + key);
      if (raw == null) return fallbackValue == null ? null : fallbackValue;
      try {
        return JSON.parse(raw);
      } catch (_) {
        return raw;
      }
    },
    set(key, value) {
      localStorage.setItem(storagePrefix + key, JSON.stringify(value));
      return true;
    },
    remove(key) {
      localStorage.removeItem(storagePrefix + key);
      return true;
    },
    clear() {
      Object.keys(localStorage)
        .filter((key) => key.startsWith(storagePrefix))
        .forEach((key) => localStorage.removeItem(key));
      return true;
    }
  };

  const db = {
    get: storage.get,
    set: storage.set,
    remove: storage.remove,
    clear: storage.clear,
    keys() {
      return Object.keys(localStorage)
        .filter((key) => key.startsWith(storagePrefix))
        .map((key) => key.slice(storagePrefix.length));
    }
  };

  const app = {
    getPath(name) {
      return nativeCall("app/get_path", { name, pluginId: meta.id });
    }
  };

  const system = {
    notification(title, body) {
      return nativeCall("system/notification", { title, body });
    },
    osInfo() {
      return nativeCall("system/os_info", {});
    },
    env(name) {
      return nativeCall("system/env", { name });
    },
    envs() {
      return nativeCall("system/envs", {});
    },
    cpu() {
      return nativeCall("system/cpu", {});
    },
    memory() {
      return nativeCall("system/memory", {});
    },
    disk() {
      return nativeCall("system/disk", {});
    },
    network() {
      return nativeCall("system/network", {});
    },
    battery() {
      return nativeCall("system/battery", {});
    }
  };

  const input = {
    typeText(text) {
      return nativeCall("input/type_text", { text });
    },
    pasteText(text) {
      return nativeCall("input/paste_text", { text });
    },
    pasteFile(path) {
      return nativeCall("input/paste_file", { path });
    }
  };

  const screen = {
    getDisplays() {
      return nativeCall("screen/get_displays", {});
    },
    getPrimaryDisplay() {
      return nativeCall("screen/get_primary_display", {});
    },
    screenshot() {
      return nativeCall("screen/screenshot", {});
    },
    pickColor() {
      return nativeCall("screen/pick_color", {});
    }
  };

  const process = {
    spawn(command, args, options) {
      return nativeCall("process/spawn", { command, args, options });
    },
    kill(pid) {
      return nativeCall("process/kill", { pid });
    },
    list() {
      return nativeCall("process/list", {});
    }
  };

  const image = {
    size(path) {
      return nativeCall("image/size", { path });
    },
    resize(input, output, options) {
      return nativeCall("image/resize", { input, output, options });
    },
    toBase64(path) {
      return nativeCall("image/to_base64", { path });
    },
    fromBase64(base64, output) {
      return nativeCall("image/from_base64", { base64, output });
    }
  };

  function downloadText(filename, content, type) {
    const blob = new Blob([String(content == null ? "" : content)], {
      type: type || "text/plain;charset=utf-8"
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename || "download.txt";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return true;
  }

  const api = {
    meta,
    dialog,
    fs,
    path,
    shell,
    clipboard,
    input,
    window: {},
    screen,
    system,
    process,
    image,
    storage,
    db,
    http: { request: (options) => fetch(options.url || options, options) },
    app,
    downloadText,
    selectFile: dialog.openFile,
    selectFiles: dialog.openFiles,
    selectDirectory: dialog.openDirectory
  };

  window.startTools = Object.assign(window.startTools || {}, api);
  window.startToolsPlugin = Object.assign(window.startToolsPlugin || {}, api);
})();"#
}

fn load_starttools_plugin_by_plugin_id(plugin_id: &str) -> Result<StartToolsPlugin, String> {
    let conn = Connection::open(&*DB_PATH).map_err(|err| err.to_string())?;
    conn.query_row(
        "SELECT id, plugin_id, name, main, logo, preload, window_mode, window_width, window_height, sort
         FROM starttools_plugin_structure
         WHERE entry_kind = 'plugin' AND plugin_id = ?1
         LIMIT 1",
        params![plugin_id],
        |row| {
            Ok(StartToolsPlugin {
                id: row.get(0)?,
                plugin_id: row
                    .get::<_, Option<String>>(1)?
                    .unwrap_or_else(|| plugin_id.to_string()),
                name: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| plugin_id.to_string()),
                main: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "index.html".to_string()),
                logo: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "logo.svg".to_string()),
                logo_url: String::new(),
                preload: row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_default(),
                window_mode: normalize_window_mode(
                    &row.get::<_, Option<String>>(6)?
                        .unwrap_or_else(default_window_mode),
                ),
                window_width: normalize_window_size(row.get::<_, Option<i64>>(7)?.unwrap_or_else(default_window_width), default_window_width()),
                window_height: normalize_window_size(row.get::<_, Option<i64>>(8)?.unwrap_or_else(default_window_height), default_window_height()),
                sort: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            })
        },
    )
    .map_err(|err| err.to_string())
}

fn resolve_plugin_path(plugin: &StartToolsPlugin, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(&*EXE_PATH)
            .join("plugins")
            .join(&plugin.plugin_id)
            .join(path)
    }
}

fn plugin_entry_url(plugin: &StartToolsPlugin, entry: &Path) -> Result<String, String> {
    if PathBuf::from(&plugin.main).is_absolute() {
        return path_to_file_url(entry);
    }

    let port = ensure_plugin_http_server()?;
    let main = normalize_url_path(&plugin.main)?;
    let url_text = format!(
        "http://{}:{}/plugins/{}/{}",
        plugin_http_host(),
        port,
        plugin.plugin_id,
        main
    );
    tauri::Url::parse(&url_text).map_err(|err| err.to_string())?;
    Ok(url_text)
}

fn path_to_file_url(path: &Path) -> Result<String, String> {
    let url = tauri::Url::from_file_path(path)
        .map_err(|_| format!("鏃犳硶杞崲鎻掍欢鍏ュ彛璺緞: {}", path.to_string_lossy()))?;
    Ok(url.to_string())
}

fn plugin_asset_url(plugin: &StartToolsPlugin, value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Ok(String::new());
    }
    if PathBuf::from(value).is_absolute() {
        return path_to_file_url(Path::new(value));
    }

    let port = ensure_plugin_http_server()?;
    let path = normalize_url_path(value)?;
    let url = format!(
        "http://{}:{}/plugins/{}/{}",
        plugin_http_host(),
        port,
        plugin.plugin_id,
        path
    );
    tauri::Url::parse(&url).map_err(|err| err.to_string())?;
    Ok(url)
}

fn open_plugin_browser_window(url: &str, plugin: &StartToolsPlugin) -> Result<(), String> {
    if let Some(browser) = find_app_mode_browser() {
        let profile_dir = PathBuf::from(&*EXE_PATH)
            .join("plugins")
            .join(".starttools-browser-profiles")
            .join(current_millis().to_string());
        fs::create_dir_all(&profile_dir).map_err(|err| err.to_string())?;
        let width = plugin.window_width.clamp(360, 7680) as i32;
        let height = plugin.window_height.clamp(240, 4320) as i32;
        let (left, top) = centered_window_position(width, height);
        let mut command = Command::new(browser);
        command
            .arg("--no-first-run")
            .arg(format!("--user-data-dir={}", profile_dir.to_string_lossy()))
            .arg("--new-window");
        if plugin.window_mode == "maximized" {
            command.arg("--start-maximized");
        } else {
            command
                .arg(format!("--window-size={},{}", width, height))
                .arg(format!("--window-position={},{}", left, top));
        }
        command
            .arg(format!("--app={}", url))
            .spawn()
            .map_err(|err| format!("打开独立浏览器窗口失败: {}", err))?;
        return Ok(());
    }

    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|err| err.to_string())
}

fn find_app_mode_browser() -> Option<PathBuf> {
    browser_candidates().into_iter().find(|path| path.exists())
}

fn browser_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(program_files) = env::var("ProgramFiles") {
        paths.push(PathBuf::from(&program_files).join("Microsoft/Edge/Application/msedge.exe"));
        paths.push(PathBuf::from(&program_files).join("Google/Chrome/Application/chrome.exe"));
    }
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        paths.push(PathBuf::from(&program_files_x86).join("Microsoft/Edge/Application/msedge.exe"));
        paths.push(PathBuf::from(&program_files_x86).join("Google/Chrome/Application/chrome.exe"));
    }
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(&local_app_data).join("Microsoft/Edge/Application/msedge.exe"));
        paths.push(PathBuf::from(&local_app_data).join("Google/Chrome/Application/chrome.exe"));
    }
    paths
}

fn centered_window_position(width: i32, height: i32) -> (i32, i32) {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let left = ((screen_width - width) / 2).max(0);
    let top = ((screen_height - height) / 2).max(0);
    (left, top)
}

fn validate_plugin_url(url: &str) -> Result<(), String> {
    let parsed = tauri::Url::parse(url).map_err(|err| err.to_string())?;
    if parsed.scheme() != "http" {
        return Ok(());
    }

    let port = parsed
        .port()
        .ok_or_else(|| format!("plugin url missing port: {}", url))?;
    let host = parsed.host_str().unwrap_or(DEFAULT_PLUGIN_HTTP_HOST);
    let path = if parsed.path().is_empty() {
        "/"
    } else {
        parsed.path()
    };
    let mut stream = TcpStream::connect((host, port))
        .map_err(|err| format!("plugin http connect failed {}: {}", url, err))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| err.to_string())?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        path, host, port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("plugin http request failed {}: {}", url, err))?;
    let mut response = [0_u8; 128];
    let size = stream
        .read(&mut response)
        .map_err(|err| format!("plugin http read failed {}: {}", url, err))?;
    let head = String::from_utf8_lossy(&response[..size]);
    if !head.starts_with("HTTP/1.1 200") {
        return Err(format!("plugin http returned invalid response {}: {}", url, head));
    }
    Ok(())
}

fn ensure_plugin_http_server() -> Result<u16, String> {
    let mut port = PLUGIN_SERVER_PORT.lock().map_err(|err| err.to_string())?;
    if let Some(port) = *port {
        return Ok(port);
    }

    let (host, configured_port) = plugin_http_config();
    let bind_addr = format!("{}:{}", host, configured_port);
    let listener = match TcpListener::bind(&bind_addr) {
        Ok(listener) => listener,
        Err(err) => {
            log::warn!(
                "Failed to bind StartTools plugin HTTP server at {}: {}, falling back to dynamic port",
                bind_addr,
                err
            );
            TcpListener::bind(format!("{}:0", host)).map_err(|err| err.to_string())?
        }
    };
    listener.set_nonblocking(true).map_err(|err| err.to_string())?;
    let server_port = listener.local_addr().map_err(|err| err.to_string())?.port();
    let generation = PLUGIN_SERVER_GENERATION
        .lock()
        .map_err(|err| err.to_string())?
        .to_owned();
    if let Ok(mut server_host) = PLUGIN_SERVER_HOST.lock() {
        *server_host = Some(host);
    }
    let root = PathBuf::from(&*EXE_PATH);
    std::thread::spawn(move || {
        loop {
            let is_current_generation = PLUGIN_SERVER_GENERATION
                .lock()
                .map(|current| *current == generation)
                .unwrap_or(false);
            if !is_current_generation {
                break;
            }

            match listener.accept() {
                Ok((stream, _)) => handle_plugin_http_request(stream, &root),
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(err) => {
                    log::warn!("StartTools plugin HTTP server accept failed: {}", err);
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });
    *port = Some(server_port);
    Ok(server_port)
}

fn plugin_http_host() -> String {
    PLUGIN_SERVER_HOST
        .lock()
        .ok()
        .and_then(|host| host.clone())
        .unwrap_or_else(|| plugin_http_config().0)
}

fn plugin_http_config() -> (String, u16) {
    let conn = Connection::open(&*DB_PATH);
    let Ok(conn) = conn else {
        return (DEFAULT_PLUGIN_HTTP_HOST.to_string(), DEFAULT_PLUGIN_HTTP_PORT);
    };

    let host = read_config_value(&conn, "Plugin_Http_Host")
        .filter(|value| !value.trim().is_empty())
        .filter(|value| value.parse::<IpAddr>().is_ok())
        .unwrap_or_else(|| DEFAULT_PLUGIN_HTTP_HOST.to_string());
    let port = read_config_value(&conn, "Plugin_Http_Port")
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_PLUGIN_HTTP_PORT);

    (host, port)
}

fn read_config_value(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM config WHERE name = ?1 LIMIT 1",
        params![name],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

fn handle_plugin_http_request(mut stream: TcpStream, root: &Path) {
    let mut buffer = [0_u8; 65536];
    let Ok(size) = stream.read(&mut buffer) else {
        return;
    };
    if size == 0 {
        return;
    }

    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut parts = request.lines().next().unwrap_or_default().split_whitespace();
    let method = parts.next().unwrap_or_default();
    let raw_path = parts.next().unwrap_or_default();
    if method != "GET" && method != "HEAD" && method != "POST" {
        write_http_response(&mut stream, 405, "Method Not Allowed", "text/plain", b"");
        return;
    }

    let path = raw_path.split('?').next().unwrap_or_default();
    let Ok(path) = percent_decode(path.trim_start_matches('/')) else {
        write_http_response(&mut stream, 400, "Bad Request", "text/plain", b"");
        return;
    };
    let Ok(path) = normalize_url_path(&path) else {
        write_http_response(&mut stream, 403, "Forbidden", "text/plain", b"");
        return;
    };
    if path.starts_with("__starttools_api/") {
        if method != "POST" {
            write_http_response(&mut stream, 405, "Method Not Allowed", "application/json", b"");
            return;
        }
        let body = request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or_default();
        handle_starttools_api_request(&mut stream, &path, body);
        return;
    }
    if !path.starts_with("plugins/") {
        write_http_response(&mut stream, 404, "Not Found", "text/plain", b"");
        return;
    }

    let Ok(root) = root.canonicalize() else {
        write_http_response(&mut stream, 500, "Server Error", "text/plain", b"");
        return;
    };
    let file_path = root.join(&path);
    let Ok(file_path) = file_path.canonicalize() else {
        write_http_response(&mut stream, 404, "Not Found", "text/plain", b"");
        return;
    };
    if !file_path.starts_with(&root) || !file_path.is_file() {
        write_http_response(&mut stream, 403, "Forbidden", "text/plain", b"");
        return;
    }

    let Ok(mut content) = fs::read(&file_path) else {
        write_http_response(&mut stream, 500, "Server Error", "text/plain", b"");
        return;
    };
    let content_type = content_type(&file_path);
    if content_type == "text/html" {
        content = inject_preload_into_html(&path, content);
    }
    let body = if method == "HEAD" {
        &[][..]
    } else {
        content.as_slice()
    };
    write_http_response(&mut stream, 200, "OK", content_type, body);
}

fn handle_starttools_api_request(stream: &mut TcpStream, path: &str, body: &str) {
    let action = path.trim_start_matches("__starttools_api/");
    let payload = serde_json::from_str::<Value>(body).unwrap_or(Value::Null);
    let result = match action {
        "dialog/open" => api_dialog_open(&payload),
        "fs/read_text" => api_fs_read_text(&payload),
        "fs/write_text" => api_fs_write_text(&payload),
        "fs/read_bytes" => api_fs_read_bytes(&payload),
        "fs/write_bytes" => api_fs_write_bytes(&payload),
        "fs/exists" => api_fs_exists(&payload),
        "fs/stat" => api_fs_stat(&payload),
        "fs/mkdir" => api_fs_mkdir(&payload),
        "fs/remove" => api_fs_remove(&payload),
        "fs/copy" => api_fs_copy(&payload),
        "fs/rename" => api_fs_rename(&payload),
        "fs/read_dir" => api_fs_read_dir(&payload),
        "shell/open_path" => api_shell_open_path(&payload),
        "shell/show_in_folder" => api_shell_show_in_folder(&payload),
        "shell/open_url" => api_shell_open_url(&payload),
        "shell/exec" => api_shell_exec(&payload),
        "app/get_path" => api_app_get_path(&payload),
        "system/notification" => api_system_notification(&payload),
        "system/os_info" => api_system_os_info(&payload),
        "system/env" => api_system_env(&payload),
        "system/envs" => api_system_envs(&payload),
        "system/cpu" => api_system_cpu(&payload),
        "system/memory" => api_system_memory(&payload),
        "system/disk" => api_system_disk(&payload),
        "system/network" => api_system_network(&payload),
        "system/battery" => api_system_battery(&payload),
        "input/type_text" => api_input_type_text(&payload),
        "input/paste_text" => api_input_paste_text(&payload),
        "input/paste_file" => api_input_paste_file(&payload),
        "clipboard/write_image" => api_clipboard_write_image(&payload),
        "clipboard/write_files" => api_clipboard_write_files(&payload),
        "clipboard/read_image" => api_clipboard_read_image(&payload),
        "clipboard/clear" => api_clipboard_clear(&payload),
        "screen/get_displays" => api_screen_get_displays(&payload),
        "screen/get_primary_display" => api_screen_get_primary_display(&payload),
        "screen/screenshot" => api_screen_screenshot(&payload),
        "screen/pick_color" => api_screen_pick_color(&payload),
        "process/spawn" => api_process_spawn(&payload),
        "process/kill" => api_process_kill(&payload),
        "process/list" => api_process_list(&payload),
        "image/size" => api_image_size(&payload),
        "image/resize" => api_image_resize(&payload),
        "image/to_base64" => api_image_to_base64(&payload),
        "image/from_base64" => api_image_from_base64(&payload),
        _ => Err("unknown StartTools API".to_string()),
    };

    match result {
        Ok(data) => write_json_response(stream, 200, serde_json::json!({ "ok": true, "data": data })),
        Err(error) => write_json_response(stream, 400, serde_json::json!({ "ok": false, "error": error })),
    }
}

fn write_json_response(stream: &mut TcpStream, status: u16, value: Value) {
    let text = if status == 200 { "OK" } else { "Bad Request" };
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    write_http_response(stream, status, text, "application/json", &body);
}

fn api_dialog_open(payload: &Value) -> Result<Value, String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = payload.get("title").and_then(Value::as_str) {
        dialog = dialog.set_title(title);
    }
    if let Some(directory) = payload
        .get("directoryPath")
        .or_else(|| payload.get("defaultPath"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        dialog = dialog.set_directory(directory);
    }
    if let Some(file_name) = payload
        .get("fileName")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        dialog = dialog.set_file_name(file_name);
    }

    let multiple = payload
        .get("multiple")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let directory = payload
        .get("directory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let save = payload.get("save").and_then(Value::as_bool).unwrap_or(false);

    if save {
        return Ok(dialog
            .save_file()
            .map(path_to_json_string)
            .unwrap_or(Value::Null));
    }
    if directory {
        if multiple {
            return Ok(Value::Array(
                dialog
                    .pick_folders()
                    .unwrap_or_default()
                    .into_iter()
                    .map(path_to_json_string)
                    .collect(),
            ));
        }
        return Ok(dialog
            .pick_folder()
            .map(path_to_json_string)
            .unwrap_or(Value::Null));
    }
    if multiple {
        return Ok(Value::Array(
            dialog
                .pick_files()
                .unwrap_or_default()
                .into_iter()
                .map(path_to_json_string)
                .collect(),
        ));
    }
    Ok(dialog
        .pick_file()
        .map(path_to_json_string)
        .unwrap_or(Value::Null))
}

fn api_fs_read_text(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    Ok(Value::String(content))
}

fn api_fs_write_text(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    fs::write(path, content).map_err(|err| err.to_string())?;
    Ok(Value::Bool(true))
}

fn api_fs_read_bytes(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let content = fs::read(path).map_err(|err| err.to_string())?;
    Ok(Value::String(BASE64_STANDARD.encode(content)))
}

fn api_fs_write_bytes(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let base64 = payload
        .get("base64")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = base64.split(',').last().unwrap_or(base64);
    let bytes = BASE64_STANDARD.decode(raw).map_err(|err| err.to_string())?;
    fs::write(path, bytes).map_err(|err| err.to_string())?;
    Ok(Value::Bool(true))
}

fn api_fs_exists(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    Ok(Value::Bool(path.exists()))
}

fn api_fs_stat(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let metadata = fs::metadata(&path).map_err(|err| err.to_string())?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0);
    let created = metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0);
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "isFile": metadata.is_file(),
        "isDir": metadata.is_dir(),
        "isSymlink": metadata.file_type().is_symlink(),
        "len": metadata.len(),
        "modified": modified,
        "created": created,
    }))
}

fn api_fs_mkdir(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    fs::create_dir_all(path).map_err(|err| err.to_string())?;
    Ok(Value::Bool(true))
}

fn api_fs_remove(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let metadata = fs::metadata(&path).map_err(|err| err.to_string())?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|err| err.to_string())?;
    } else {
        fs::remove_file(path).map_err(|err| err.to_string())?;
    }
    Ok(Value::Bool(true))
}

fn api_fs_copy(payload: &Value) -> Result<Value, String> {
    let from = payload_named_path(payload, "from")?;
    let to = payload_named_path(payload, "to")?;
    let bytes = fs::copy(from, to).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({ "bytes": bytes }))
}

fn api_fs_rename(payload: &Value) -> Result<Value, String> {
    let from = payload_named_path(payload, "from")?;
    let to = payload_named_path(payload, "to")?;
    fs::rename(from, to).map_err(|err| err.to_string())?;
    Ok(Value::Bool(true))
}

fn api_fs_read_dir(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let entries = fs::read_dir(path)
        .map_err(|err| err.to_string())?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok();
            serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "path": path.to_string_lossy(),
                "isFile": metadata.as_ref().map(|value| value.is_file()).unwrap_or(false),
                "isDir": metadata.as_ref().map(|value| value.is_dir()).unwrap_or(false),
                "len": metadata.as_ref().map(|value| value.len()).unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    Ok(Value::Array(entries))
}

fn api_shell_open_path(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    open_with_system(&path)?;
    Ok(Value::Bool(true))
}

fn api_shell_show_in_folder(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    show_in_folder(&path)?;
    Ok(Value::Bool(true))
}

fn api_shell_open_url(payload: &Value) -> Result<Value, String> {
    let url = payload
        .get("url")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "missing url".to_string())?;
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|err| err.to_string())?;
    Ok(Value::Bool(true))
}

fn api_shell_exec(payload: &Value) -> Result<Value, String> {
    let command = payload
        .get("command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "missing command".to_string())?;
    let mut process = Command::new(command);
    if let Some(args) = payload.get("args") {
        if let Some(args) = args.as_array() {
            process.args(args.iter().filter_map(Value::as_str));
        } else if let Some(args) = args.as_str() {
            process.args(args.split_whitespace());
        }
    }
    if let Some(cwd) = payload
        .get("options")
        .and_then(|value| value.get("cwd"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        process.current_dir(cwd);
    }
    let output = process.output().map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "status": output.status.code(),
        "success": output.status.success(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    }))
}

fn api_app_get_path(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let path = match name.as_str() {
        "home" => env::var("USERPROFILE").or_else(|_| env::var("HOME")).map(PathBuf::from),
        "desktop" => env::var("USERPROFILE").map(|value| PathBuf::from(value).join("Desktop")),
        "documents" => env::var("USERPROFILE").map(|value| PathBuf::from(value).join("Documents")),
        "downloads" => env::var("USERPROFILE").map(|value| PathBuf::from(value).join("Downloads")),
        "appdata" | "app_data" => env::var("APPDATA").map(PathBuf::from),
        "temp" | "tmp" => Ok(env::temp_dir()),
        "exe" => Ok(PathBuf::from(&*EXE_PATH)),
        "plugin" => {
            let plugin_id = payload
                .get("pluginId")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_default();
            Ok(PathBuf::from(&*EXE_PATH).join("plugins").join(plugin_id))
        }
        _ => Err(env::VarError::NotPresent),
    }
    .map_err(|_| format!("unknown path name: {}", name))?;
    Ok(path_to_json_string(path))
}

fn api_system_notification(payload: &Value) -> Result<Value, String> {
    let title = payload.get("title").and_then(Value::as_str).unwrap_or("StartTools");
    let body = payload.get("body").and_then(Value::as_str).unwrap_or_default();
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(body)
        .show();
    Ok(Value::Bool(true))
}

fn api_system_os_info(_payload: &Value) -> Result<Value, String> {
    Ok(serde_json::json!({
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "family": env::consts::FAMILY,
    }))
}

fn api_system_env(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "missing env name".to_string())?;
    Ok(env::var(name).map(Value::String).unwrap_or(Value::Null))
}

fn api_system_envs(_payload: &Value) -> Result<Value, String> {
    let values = env::vars()
        .map(|(key, value)| (key, Value::String(value)))
        .collect::<serde_json::Map<_, _>>();
    Ok(Value::Object(values))
}

fn api_input_type_text(payload: &Value) -> Result<Value, String> {
    let text = payload.get("text").and_then(Value::as_str).unwrap_or_default();
    send_keys_text(text)?;
    Ok(Value::Bool(true))
}

fn api_input_paste_text(payload: &Value) -> Result<Value, String> {
    let text = payload.get("text").and_then(Value::as_str).unwrap_or_default();
    set_clipboard_text(text)?;
    send_ctrl_v()?;
    Ok(Value::Bool(true))
}

fn api_input_paste_file(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    set_clipboard_text(&path.to_string_lossy())?;
    send_ctrl_v()?;
    Ok(Value::Bool(true))
}

fn api_clipboard_write_files(payload: &Value) -> Result<Value, String> {
    let paths = payload
        .get("paths")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\r\n")
        })
        .unwrap_or_default();
    set_clipboard_text(&paths)?;
    Ok(Value::Bool(true))
}

fn api_clipboard_write_image(payload: &Value) -> Result<Value, String> {
    let base64 = payload
        .get("base64")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = base64.split(',').last().unwrap_or(base64);
    let bytes = BASE64_STANDARD.decode(raw).map_err(|err| err.to_string())?;
    let image = image::load_from_memory(&bytes).map_err(|err| err.to_string())?;
    let tmp_path = env::temp_dir().join(format!("starttools_clipboard_{}.png", current_millis()));
    image.save(&tmp_path).map_err(|err| err.to_string())?;
    set_clipboard_text(&tmp_path.to_string_lossy())?;
    Ok(serde_json::json!({
        "path": tmp_path.to_string_lossy(),
        "note": "image saved to temp file and path copied to clipboard"
    }))
}

fn api_clipboard_read_image(_payload: &Value) -> Result<Value, String> {
    Err("clipboard.readImage via Rust is not implemented yet".to_string())
}

fn api_clipboard_clear(_payload: &Value) -> Result<Value, String> {
    set_clipboard_text("")?;
    Ok(Value::Bool(true))
}

fn api_screen_get_displays(_payload: &Value) -> Result<Value, String> {
    Ok(Value::Array(vec![primary_display_json()]))
}

fn api_screen_get_primary_display(_payload: &Value) -> Result<Value, String> {
    Ok(primary_display_json())
}

fn api_screen_screenshot(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return windows_primary_screenshot();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("screenshot is only implemented on Windows".to_string())
    }
}

fn api_screen_pick_color(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let mut point: POINT = std::mem::zeroed();
        if GetCursorPos(&mut point) == 0 {
            return Err("failed to get cursor position".to_string());
        }
        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            return Err("failed to get screen dc".to_string());
        }
        let color = GetPixel(hdc, point.x, point.y);
        ReleaseDC(std::ptr::null_mut(), hdc);
        let r = color & 0xff;
        let g = (color >> 8) & 0xff;
        let b = (color >> 16) & 0xff;
        return Ok(serde_json::json!({
            "x": point.x,
            "y": point.y,
            "r": r,
            "g": g,
            "b": b,
            "hex": format!("#{:02X}{:02X}{:02X}", r, g, b),
        }));
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("pickColor is only implemented on Windows".to_string())
    }
}

fn api_process_spawn(payload: &Value) -> Result<Value, String> {
    let command = payload
        .get("command")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "missing command".to_string())?;
    let mut process = Command::new(command);
    if let Some(args) = payload.get("args") {
        if let Some(args) = args.as_array() {
            process.args(args.iter().filter_map(Value::as_str));
        } else if let Some(args) = args.as_str() {
            process.args(args.split_whitespace());
        }
    }
    if let Some(cwd) = payload
        .get("options")
        .and_then(|value| value.get("cwd"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        process.current_dir(cwd);
    }
    let child = process.spawn().map_err(|err| err.to_string())?;
    Ok(serde_json::json!({ "pid": child.id() }))
}

fn api_process_kill(payload: &Value) -> Result<Value, String> {
    let pid = payload
        .get("pid")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing pid".to_string())?;
    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map_err(|err| err.to_string())?;
    }
    Ok(Value::Bool(true))
}

fn api_process_list(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Get-Process | Select-Object Id,ProcessName,Path | ConvertTo-Json -Compress",
            ])
            .output()
            .map_err(|err| err.to_string())?;
        let text = String::from_utf8_lossy(&output.stdout);
        if text.trim().is_empty() {
            return Ok(Value::Array(Vec::new()));
        }
        return serde_json::from_str::<Value>(&text).map_err(|err| err.to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps")
            .args(["-eo", "pid,comm"])
            .output()
            .map_err(|err| err.to_string())?;
        let text = String::from_utf8_lossy(&output.stdout);
        let values = text
            .lines()
            .skip(1)
            .filter_map(|line| {
                let mut parts = line.trim().splitn(2, char::is_whitespace);
                Some(serde_json::json!({
                    "Id": parts.next()?.parse::<u32>().ok()?,
                    "ProcessName": parts.next().unwrap_or_default().trim(),
                }))
            })
            .collect::<Vec<_>>();
        Ok(Value::Array(values))
    }
}

fn api_image_size(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let (width, height) = image::image_dimensions(path).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({ "width": width, "height": height }))
}

fn api_image_resize(payload: &Value) -> Result<Value, String> {
    let input = payload_named_path(payload, "input")?;
    let output = payload_named_path(payload, "output")?;
    let options = payload.get("options").unwrap_or(&Value::Null);
    let width = options.get("width").and_then(Value::as_u64).unwrap_or(0) as u32;
    let height = options.get("height").and_then(Value::as_u64).unwrap_or(0) as u32;
    if width == 0 && height == 0 {
        return Err("missing resize width or height".to_string());
    }
    let image = image::open(&input).map_err(|err| err.to_string())?;
    let resized = if width > 0 && height > 0 {
        image.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    } else if width > 0 {
        image.resize(width, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        image.resize(u32::MAX, height, image::imageops::FilterType::Lanczos3)
    };
    resized.save(&output).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({ "path": output.to_string_lossy() }))
}

fn api_image_to_base64(payload: &Value) -> Result<Value, String> {
    let path = payload_path(payload)?;
    let bytes = fs::read(&path).map_err(|err| err.to_string())?;
    let mime = match path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "image/png",
    };
    let base64 = BASE64_STANDARD.encode(bytes);
    Ok(serde_json::json!({
        "base64": base64,
        "type": mime,
        "dataUrl": format!("data:{};base64,{}", mime, base64),
    }))
}

fn api_image_from_base64(payload: &Value) -> Result<Value, String> {
    let output = payload_named_path(payload, "output")?;
    let base64 = payload
        .get("base64")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = base64.split(',').last().unwrap_or(base64);
    let bytes = BASE64_STANDARD.decode(raw).map_err(|err| err.to_string())?;
    fs::write(&output, bytes).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({ "path": output.to_string_lossy() }))
}

fn api_system_cpu(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return powershell_json("Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors,MaxClockSpeed,LoadPercentage | ConvertTo-Json -Compress");
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(serde_json::json!({ "arch": env::consts::ARCH }))
    }
}

fn api_system_memory(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return powershell_json("Get-CimInstance Win32_OperatingSystem | Select-Object TotalVisibleMemorySize,FreePhysicalMemory,TotalVirtualMemorySize,FreeVirtualMemory | ConvertTo-Json -Compress");
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("memory info is only implemented on Windows".to_string())
    }
}

fn api_system_disk(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return powershell_json("Get-CimInstance Win32_LogicalDisk | Select-Object DeviceID,DriveType,FileSystem,Size,FreeSpace,VolumeName | ConvertTo-Json -Compress");
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("disk info is only implemented on Windows".to_string())
    }
}

fn api_system_network(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return powershell_json("Get-NetIPConfiguration | Select-Object InterfaceAlias,InterfaceDescription,IPv4Address,IPv6Address,DNSServer | ConvertTo-Json -Compress -Depth 4");
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("network info is only implemented on Windows".to_string())
    }
}

fn api_system_battery(_payload: &Value) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    {
        return powershell_json("Get-CimInstance Win32_Battery | Select-Object Name,BatteryStatus,EstimatedChargeRemaining,EstimatedRunTime | ConvertTo-Json -Compress");
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("battery info is only implemented on Windows".to_string())
    }
}

fn payload_path(payload: &Value) -> Result<PathBuf, String> {
    payload
        .get("path")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "missing path".to_string())
}

fn payload_named_path(payload: &Value, name: &str) -> Result<PathBuf, String> {
    payload
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {}", name))
}

fn path_to_json_string(path: PathBuf) -> Value {
    Value::String(path.to_string_lossy().to_string())
}

fn open_with_system(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn show_in_folder(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", path.to_string_lossy()))
            .spawn()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let folder = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        open_with_system(folder)?;
    }
    Ok(())
}

fn powershell_json(script: &str) -> Result<Value, String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str::<Value>(&text).map_err(|err| err.to_string())
}

fn set_clipboard_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let escaped = text.replace('\'', "''");
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", escaped),
            ])
            .status()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        return Err("clipboard bridge is only implemented on Windows".to_string());
    }
    Ok(())
}

fn send_ctrl_v() -> Result<(), String> {
    run_send_keys("^v")
}

fn send_keys_text(text: &str) -> Result<(), String> {
    let escaped = text
        .replace('{', "{{}")
        .replace('}', "{}}")
        .replace('+', "{+}")
        .replace('^', "{^}")
        .replace('%', "{%}")
        .replace('~', "{~}")
        .replace('(', "{(}")
        .replace(')', "{)}");
    run_send_keys(&escaped)
}

fn run_send_keys(keys: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let escaped = keys.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('{}')",
            escaped
        );
        Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()
            .map_err(|err| err.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        return Err("input bridge is only implemented on Windows".to_string());
    }
    Ok(())
}

fn primary_display_json() -> Value {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    serde_json::json!({
        "id": "primary",
        "primary": true,
        "x": 0,
        "y": 0,
        "width": width,
        "height": height,
        "scaleFactor": 1,
    })
}

#[cfg(target_os = "windows")]
fn windows_primary_screenshot() -> Result<Value, String> {
    use image::{ImageBuffer, ImageOutputFormat, Rgba};
    use std::io::Cursor;
    use winapi::um::wingdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, RGBQUAD, BI_RGB, SRCCOPY,
    };

    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return Err("failed to get screen dc".to_string());
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if mem_dc.is_null() || bitmap.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err("failed to create screenshot bitmap".to_string());
        }
        SelectObject(mem_dc, bitmap as _);
        if BitBlt(mem_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY) == 0 {
            DeleteObject(bitmap as _);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err("failed to copy screen".to_string());
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
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
            }],
        };
        let mut buffer = vec![0_u8; (width * height * 4) as usize];
        let result = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            buffer.as_mut_ptr() as _,
            &mut info,
            DIB_RGB_COLORS,
        );
        DeleteObject(bitmap as _);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        if result == 0 {
            return Err("failed to read screenshot pixels".to_string());
        }

        for pixel in buffer.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
            .ok_or_else(|| "failed to create screenshot image".to_string())?;
        let mut png = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut png), ImageOutputFormat::Png)
            .map_err(|err| err.to_string())?;
        let base64 = BASE64_STANDARD.encode(png);
        Ok(serde_json::json!({
            "width": width,
            "height": height,
            "type": "image/png",
            "base64": base64,
            "dataUrl": format!("data:image/png;base64,{}", base64),
        }))
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    text: &str,
    content_type: &str,
    body: &[u8],
) {
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status,
        text,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn inject_preload_into_html(path: &str, content: Vec<u8>) -> Vec<u8> {
    let Some(plugin_id) = path.split('/').nth(1) else {
        return content;
    };
    let Ok(plugin) = load_starttools_plugin_by_plugin_id(plugin_id) else {
        return content;
    };
    let preload_script = if plugin.preload.trim().is_empty() {
        String::new()
    } else {
        let preload_path = resolve_plugin_path(&plugin, &plugin.preload);
        if preload_path.exists() {
            fs::read_to_string(&preload_path).unwrap_or_default()
        } else {
            String::new()
        }
    };
    let mut html = match String::from_utf8(content) {
        Ok(html) => html,
        Err(err) => return err.into_bytes(),
    };
    let script = build_preload_injection_script(&plugin, &preload_script);
    let favicon = build_plugin_favicon_link(&plugin);
    let lower = html.to_lowercase();

    if let Some(index) = lower.find("<head>") {
        html.insert_str(index + "<head>".len(), &format!("{}{}", favicon, script));
    } else if let Some(index) = lower.find("<html>") {
        html.insert_str(index + "<html>".len(), &format!("<head>{}{}</head>", favicon, script));
    } else {
        html.insert_str(0, &format!("{}{}", favicon, script));
    }

    html.into_bytes()
}

fn build_plugin_favicon_link(plugin: &StartToolsPlugin) -> String {
    if plugin.logo.trim().is_empty() {
        return String::new();
    }
    let Ok(url) = plugin_asset_url(plugin, &plugin.logo) else {
        return String::new();
    };
    let Ok(url) = serde_json::to_string(&url) else {
        return String::new();
    };
    format!(
        r#"<script>
(() => {{
  const href = {};
  if (!document.querySelector('link[rel~="icon"]')) {{
    const link = document.createElement('link');
    link.rel = 'icon';
    link.href = href;
    document.head.appendChild(link);
  }}
}})();
</script>"#,
        url
    )
}

fn build_preload_injection_script(plugin: &StartToolsPlugin, preload_script: &str) -> String {
    let plugin_meta = serde_json::json!({
        "id": plugin.plugin_id,
        "name": plugin.name,
        "main": plugin.main,
        "preload": plugin.preload,
    });
    let base_preload = default_preload_js();
    let preload_script = if preload_script.trim() == base_preload.trim() {
        base_preload.to_string()
    } else {
        format!("{}\n{}", base_preload, preload_script)
    };
    let preload_source = serde_json::to_string(&preload_script).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"<script>
window.__STARTTOOLS_PLUGIN__ = {};
(() => {{
  const source = {};
  try {{
    if (source.trim()) {{
      (0, eval)(source + "\n//# sourceURL=starttools-plugin-{}.preload.js");
    }}
  }} catch (error) {{
    window.__STARTTOOLS_PRELOAD_ERROR__ = String(error && (error.stack || error.message) || error);
    console.error("[StartTools preload error]", error);
  }}
}})();
</script>"#,
        plugin_meta, preload_source, plugin.plugin_id
    )
}

fn normalize_url_path(path: &str) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("invalid plugin path".to_string());
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return Err("empty plugin path".to_string());
    }
    Ok(parts.join("/"))
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("invalid percent encoding".to_string());
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|err| err.to_string())?;
            let byte = u8::from_str_radix(hex, 16).map_err(|err| err.to_string())?;
            output.push(byte);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|err| err.to_string())
}

fn current_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

