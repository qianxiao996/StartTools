use crate::utils::DB_PATH;
use rusqlite::{params, Connection, Result};

pub fn init_database() -> Result<()> {
    let conn = Connection::open(&*DB_PATH)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS config (
            name TEXT NOT NULL PRIMARY KEY,
            value TEXT
        );

        CREATE TABLE IF NOT EXISTS menu (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            sort INTEGER
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            menu_id INTEGER,
            sort INTEGER
        );

        CREATE TABLE IF NOT EXISTS tools (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            icon TEXT,
            target TEXT,
            parameters TEXT,
            isadmin INTEGER,
            number INTEGER,
            menu_id INTEGER,
            tags_id INTEGER
        );

        CREATE TABLE IF NOT EXISTS builtin_tools (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            run_type TEXT NOT NULL DEFAULT 'command',
            shell TEXT NOT NULL DEFAULT 'cmd',
            content TEXT,
            working_dir TEXT,
            os TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            sort INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS starttools_plugin_structure (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            entry_kind TEXT NOT NULL,
            plugin_id TEXT,
            main TEXT,
            logo TEXT,
            preload TEXT,
            single INTEGER,
            height INTEGER,
            name TEXT,
            code TEXT,
            explain TEXT,
            cmd_kind TEXT,
            value TEXT,
            label TEXT,
            match_rule TEXT,
            min_length INTEGER,
            max_length INTEGER,
            window_mode TEXT,
            window_width INTEGER,
            window_height INTEGER,
            sort INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(parent_id) REFERENCES starttools_plugin_structure(id) ON DELETE CASCADE
        );
        ",
    )?;

    ensure_column(&conn, "menu", "sort", "INTEGER")?;
    ensure_column(&conn, "tags", "sort", "INTEGER")?;
    ensure_column(&conn, "builtin_tools", "os", "TEXT")?;
    backfill_builtin_tool_os(&conn)?;
    ensure_column(&conn, "starttools_plugin_structure", "plugin_id", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "main", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "logo", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "preload", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "single", "INTEGER")?;
    ensure_column(&conn, "starttools_plugin_structure", "height", "INTEGER")?;
    ensure_column(&conn, "starttools_plugin_structure", "name", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "window_mode", "TEXT")?;
    ensure_column(&conn, "starttools_plugin_structure", "window_width", "INTEGER")?;
    ensure_column(&conn, "starttools_plugin_structure", "window_height", "INTEGER")?;
    drop_starttools_open_mode_column(&conn)?;
    seed_defaults(&conn)?;
    seed_config(&conn)?;
    migrate_main_window_hotkey_default(&conn)?;
    migrate_search_hotkey_default(&conn)?;
    seed_starttools_plugin(&conn)?;

    Ok(())
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

fn backfill_builtin_tool_os(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE builtin_tools SET os = ?1 WHERE os IS NULL OR TRIM(os) = ''",
        params![current_builtin_tool_os()],
    )?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, data_type: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for col in columns {
        if col? == column {
            return Ok(());
        }
    }

    conn.execute(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, data_type), [])?;
    Ok(())
}

fn seed_starttools_plugin(conn: &Connection) -> Result<()> {
    let plugin_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM starttools_plugin_structure WHERE entry_kind = 'plugin'",
        [],
        |row| row.get(0),
    )?;

    if plugin_count == 0 {
        conn.execute(
            "INSERT INTO starttools_plugin_structure
             (entry_kind, plugin_id, name, main, logo, preload, single, height, window_mode, window_width, window_height, sort)
             VALUES ('plugin', 'starttools', 'StartTools', 'index.html', 'logo.svg', 'preload.js', 1, 640, 'default', 1080, 680, 0)",
            [],
        )?;
    }

    Ok(())
}

fn drop_starttools_open_mode_column(conn: &Connection) -> Result<()> {
    if !column_exists(conn, "starttools_plugin_structure", "open_mode")? {
        return Ok(());
    }

    conn.execute_batch(
        "
        PRAGMA foreign_keys=off;

        CREATE TABLE starttools_plugin_structure_new (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            parent_id INTEGER,
            entry_kind TEXT NOT NULL,
            plugin_id TEXT,
            main TEXT,
            logo TEXT,
            preload TEXT,
            single INTEGER,
            height INTEGER,
            name TEXT,
            code TEXT,
            explain TEXT,
            cmd_kind TEXT,
            value TEXT,
            label TEXT,
            match_rule TEXT,
            min_length INTEGER,
            max_length INTEGER,
            window_mode TEXT,
            window_width INTEGER,
            window_height INTEGER,
            sort INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(parent_id) REFERENCES starttools_plugin_structure(id) ON DELETE CASCADE
        );

        INSERT INTO starttools_plugin_structure_new (
            id, parent_id, entry_kind, plugin_id, main, logo, preload,
            single, height, name, code, explain, cmd_kind, value, label,
            match_rule, min_length, max_length, window_mode, window_width, window_height, sort
        )
        SELECT
            id, parent_id, entry_kind, plugin_id, main, logo, preload,
            single, height, name, code, explain, cmd_kind, value, label,
            match_rule, min_length, max_length,
            COALESCE(window_mode, 'default'),
            COALESCE(window_width, 1080),
            COALESCE(window_height, 680),
            sort
        FROM starttools_plugin_structure;

        DROP TABLE starttools_plugin_structure;
        ALTER TABLE starttools_plugin_structure_new RENAME TO starttools_plugin_structure;

        PRAGMA foreign_keys=on;
        ",
    )?;

    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for col in columns {
        if col? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn seed_defaults(conn: &Connection) -> Result<()> {
    let menu_count: i64 = conn.query_row("SELECT COUNT(*) FROM menu", [], |row| row.get(0))?;
    if menu_count == 0 {
        conn.execute(
            "INSERT OR IGNORE INTO menu (id, name, sort) VALUES (1, '默认分类', 0)",
            [],
        )?;
    }

    let tag_count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
    if tag_count == 0 {
        conn.execute(
            "INSERT OR IGNORE INTO tags (id, name, menu_id, sort) VALUES (1, '默认标签', 1, 0)",
            [],
        )?;
    }

    Ok(())
}
fn seed_config(conn: &Connection) -> Result<()> {
    let defaults = [
        ("X", "0"),
        ("Y", "77"),
        ("Width", "443"),
        ("Height", "887"),
        ("Skin", "light"),
        ("Current_Menu_Id", "1"),
        ("Current_Tags_Id", "-1"),
        ("Tools_Order_Field", "number"),
        ("Tools_Order_Type", "desc"),
        ("HotKey_Show_Hidden", "Alt+2"),
        ("HotKey_Search", "Alt+3"),
        ("Startup", "true"),
        ("Startup_Background", "false"),
        ("Locked_Position", "false"),
        ("Locked_Size", "false"),
        ("Terminal", "C:\\Windows\\System32\\cmd.exe"),
        ("Terminal_Runas_Arguments", "/K cd /D \"{DirectoryPath}\""),
        ("LeftWidth", "80"),
        ("Top_Window", "true"),
        ("Plugin_Http_Host", "127.0.0.1"),
        ("Plugin_Http_Port", "13678"),
    ];

    for (name, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO config (name, value) VALUES (?1, ?2)",
            params![name, value],
        )?;
    }

    Ok(())
}

fn migrate_search_hotkey_default(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE config SET value = 'Alt+3' WHERE name = 'HotKey_Search' AND value = 'Ctrl+Alt+F'",
        [],
    )?;
    Ok(())
}

fn migrate_main_window_hotkey_default(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE config SET value = 'Alt+2' WHERE name = 'HotKey_Show_Hidden' AND value = 'Ctrl+Q'",
        [],
    )?;
    Ok(())
}

#[allow(dead_code)]
fn seed_legacy_starttools_plugin(conn: &Connection) -> Result<()> {
    let feature_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM legacy_starttools_plugin_structure WHERE entry_kind = 'feature'",
        [],
        |row| row.get(0),
    )?;

    if feature_count == 0 {
        conn.execute(
            "INSERT INTO legacy_starttools_plugin_structure (entry_kind, main, logo, preload, single, height, sort)
             VALUES ('config', 'index.html', 'logo.svg', 'preload.js', 1, 640, 0)",
            [],
        )?;

        conn.execute(
            "INSERT INTO legacy_starttools_plugin_structure (id, entry_kind, name, code, explain, sort)
             VALUES (1, 'feature', 'starttools', 'starttools', 'StartTools 宸ュ叿鍚姩鍣?, 0)",
            [],
        )?;
        conn.execute(
            "INSERT INTO legacy_starttools_plugin_structure (id, entry_kind, name, code, explain, sort)
             VALUES (2, 'feature', 'starttools-run', 'starttools-run', '鎼滅储骞惰繍琛?StartTools 宸ュ叿', 1)",
            [],
        )?;
    }

    Ok(())
}

