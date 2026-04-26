use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use crate::utils::{DB_PATH, EXE_PATH};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
struct BackupFile {
    version: u32,
    db_base64: String,
    plugins: Vec<BackupEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BackupEntry {
    path: String,
    data_base64: String,
}

#[tauri::command]
pub fn load_config() -> Map<String, Value> {
    let mut config = Map::new();
    let conn_result = Connection::open(&*DB_PATH);
    if let Err(_) = conn_result {
        return config;
    }
    let conn = conn_result.unwrap();
    let stmt_result = conn.prepare("SELECT name, value FROM config");
    if let Err(err) = stmt_result {
        log::error!("Load config error: {}", err);
        return config;
    }
    let mut stmt = stmt_result.unwrap();
    let rows_result = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((key, value))
    });
    if let Err(err) = rows_result {
        log::error!("Load config error: {}", err);
        return config;
    }
    let rows = rows_result.unwrap();
    for row in rows {
        if let Ok((key, value)) = row {
            config.insert(key, Value::String(value));
        }
    }
    config
}

#[tauri::command]
pub fn update_config(config: Map<String, Value>) -> String {
    let config_json = json!(config).to_string();
    log::info!("Update config: {}", config_json);

    let conn_result = Connection::open(&*DB_PATH);
    if let Err(err) = conn_result {
        log::error!("Failed to connect to database: {}", err);
        return "error".to_string();
    }
    let conn = conn_result.unwrap();

    for (key, value) in config {
        let value_str = match value {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(n) => n.to_string(),
            Value::Null => "".to_string(),
            _ => {
                log::error!("Invalid value type for key: {}", key);
                continue;
            }
        };
        let query = "INSERT OR REPLACE INTO config (name, value) VALUES (?1, ?2)";
        let result = conn.execute(query, &[&key, &value_str]);
        if let Err(err) = result {
            log::error!("Failed to update config: {}", err);
            return "error".to_string();
        }
    }

    "ok".to_string()
}

#[tauri::command]
pub fn export_data(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导出路径不能为空".to_string());
    }

    let db_bytes = fs::read(&*DB_PATH).map_err(|err| format!("读取数据库失败: {}", err))?;
    let plugins_dir = plugins_dir();
    let mut plugins = Vec::new();
    if plugins_dir.exists() {
        collect_plugin_files(&plugins_dir, &plugins_dir, &mut plugins)?;
    }

    let backup = BackupFile {
        version: 1,
        db_base64: BASE64_STANDARD.encode(db_bytes),
        plugins,
    };
    let text = serde_json::to_string(&backup).map_err(|err| format!("生成备份失败: {}", err))?;
    fs::write(&path, text).map_err(|err| format!("导出备份失败: {}", err))
}

#[tauri::command]
pub fn import_data(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("导入路径不能为空".to_string());
    }
    if !Path::new(&path).exists() {
        return Err("导入文件不存在".to_string());
    }

    let text = fs::read_to_string(&path).map_err(|err| format!("读取备份失败: {}", err))?;
    let backup: BackupFile = serde_json::from_str(&text).map_err(|err| format!("备份文件格式不正确: {}", err))?;
    let db_bytes = BASE64_STANDARD
        .decode(backup.db_base64)
        .map_err(|err| format!("解析数据库失败: {}", err))?;

    backup_current_data()?;
    fs::write(&*DB_PATH, db_bytes).map_err(|err| format!("写入数据库失败: {}", err))?;

    let plugins_dir = plugins_dir();
    if plugins_dir.exists() {
        fs::remove_dir_all(&plugins_dir).map_err(|err| format!("清理插件目录失败: {}", err))?;
    }
    fs::create_dir_all(&plugins_dir).map_err(|err| format!("创建插件目录失败: {}", err))?;

    for entry in backup.plugins {
        let relative = safe_relative_path(&entry.path)?;
        let target = plugins_dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建插件目录失败: {}", err))?;
        }
        let bytes = BASE64_STANDARD
            .decode(entry.data_base64)
            .map_err(|err| format!("解析插件文件失败 {}: {}", entry.path, err))?;
        fs::write(&target, bytes).map_err(|err| format!("写入插件文件失败 {}: {}", entry.path, err))?;
    }

    Ok(())
}

fn plugins_dir() -> PathBuf {
    PathBuf::from(&*EXE_PATH).join("plugins")
}

fn collect_plugin_files(root: &Path, current: &Path, entries: &mut Vec<BackupEntry>) -> Result<(), String> {
    for item in fs::read_dir(current).map_err(|err| format!("读取插件目录失败: {}", err))? {
        let item = item.map_err(|err| format!("读取插件目录失败: {}", err))?;
        let path = item.path();
        let file_name = item.file_name().to_string_lossy().to_string();
        if file_name == ".starttools-browser-profiles" {
            continue;
        }
        if path.is_dir() {
            collect_plugin_files(root, &path, entries)?;
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
        entries.push(BackupEntry {
            path: relative,
            data_base64: BASE64_STANDARD.encode(bytes),
        });
    }
    Ok(())
}

fn safe_relative_path(path: &str) -> Result<PathBuf, String> {
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

fn backup_current_data() -> Result<(), String> {
    if Path::new(&*DB_PATH).exists() {
        let backup_path = format!("{}.bak", &*DB_PATH);
        fs::copy(&*DB_PATH, &backup_path).map_err(|err| format!("备份当前数据库失败: {}", err))?;
    }

    let plugins_dir = plugins_dir();
    if plugins_dir.exists() {
        let backup_dir = PathBuf::from(&*EXE_PATH).join("plugins.bak");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).map_err(|err| format!("清理插件备份失败: {}", err))?;
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
