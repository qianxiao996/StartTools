use serde::{Deserialize, Serialize};
use crate::utils::DB_PATH;
use serde_json::Value;
use rusqlite::{Connection, Result, Row};
#[derive(Debug, Serialize, Deserialize)]
pub struct Menu {
    pub id: i32,
    pub name: String,
    pub sort: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub id: i32,
    pub name: String,
    pub menu_id: Option<i32>,
    pub sort: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tools {
    pub id: i32,
    pub name: String,
    pub icon: String,
    pub target: String,
    pub parameters: String,
    pub isadmin: Option<i32>,
    pub number: Option<i32>,
    pub menu_id: Option<i32>,
    pub tags_id: Option<i32>,
}

// 泛型查询数据库函数
fn query_database<T, F>(query: &str, mapper: F) -> Result<Vec<Value>>
    where
        F: Fn(&Row) -> Result<T> + Copy,
        T: Serialize,
    {
        // let current_dir = std::env::current_dir().expect("Failed to get current directory");
        // let db_path = current_dir.join("data.db");
        let conn = Connection::open(&*DB_PATH)?;
        let mut stmt = conn.prepare(query)?;
        let person_iter = stmt.query_map([], mapper)?;

        let mut data = Vec::new();
        for person in person_iter {
            if let Ok(entity) = person {
                let json_val = serde_json::to_value(entity).expect("Serialization failed"); // 将结构转换为JSON值
                data.push(json_val);
            }
        }
        Ok(data)
    }

#[tauri::command]
pub fn load_menus() -> Vec<Value> {
    let query = "SELECT id, name, sort FROM menu";
    match query_database(query, |row| {
        Ok(Menu {
            id: row.get(0)?,
            name: row.get(1)?,
            sort: row.get(2)?,
        })
    }) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to load menus: {}", e);
            Vec::new()
        }
    }
}

#[tauri::command]
pub fn load_tags() -> Vec<Value> {
    let query = "SELECT id, name, menu_id,sort FROM tags"; // 修正查询语句以包含正确的列
    match query_database(query, |row| {
        Ok(Tags {
            id: row.get(0)?,
            name: row.get(1)?,
            menu_id: row.get(2)?,
            sort: row.get(3)?
        })
    }) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to load tags: {}", e);
            Vec::new()
        }
    }
}

#[tauri::command]
pub fn load_tools() -> Vec<Value> {
    let query = "SELECT id,name, icon,target,parameters,isadmin,number,menu_id,tags_id FROM tools";
    match query_database(query, |row| {
        Ok(Tools {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            target: row.get(3)?,
            parameters: row.get(4)?,
            isadmin: row.get(5)?,
            number: row.get(6)?,
            menu_id: row.get(7)?,
            tags_id: row.get(8)?,
        })
    }) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to load tools: {}", e);
            Vec::new()
        }
    }
}

pub fn select_tools_by_menu_and_tags(menu_id: i64, tags_id: i64) -> Result<Vec<Tools>, rusqlite::Error> {
    let conn = Connection::open(&*DB_PATH)?;
    let query = if tags_id <= 0 {
        "SELECT id, name, icon, target, parameters, isadmin, number, menu_id, tags_id FROM tools WHERE menu_id = ?1"
    } else {
        "SELECT id, name, icon, target, parameters, isadmin, number, menu_id, tags_id FROM tools WHERE menu_id = ?1 AND tags_id = ?2"
    };
    let mut tools = Vec::new();
    {
        let mut stmt = conn.prepare(query)?;
        let tools_iter = if tags_id <= 0 {
            stmt.query_map([menu_id], map_tool_row)?
        } else {
            stmt.query_map([menu_id, tags_id], map_tool_row)?
        };
        for tool_result in tools_iter {
            tools.push(tool_result?);
        }
    }
    Ok(tools)
}
fn map_tool_row(row: &rusqlite::Row) -> Result<Tools, rusqlite::Error> {
    Ok(Tools {
        id: row.get("id")?,
        name: row.get("name")?,
        icon: row.get("icon")?,
        target: row.get("target")?,
        parameters: row.get("parameters")?,
        isadmin: row.get("isadmin")?,
        number: row.get("number")?,
        menu_id: row.get("menu_id")?,
        tags_id: row.get("tags_id")?,
    })
}