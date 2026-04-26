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
        let menus = [
            (1, "数据库工具", 0),
            (2, "WebShell", 1),
            (4, "漏洞利用", 2),
            (5, "抓包代理", 3),
            (7, "我的工具", 4),
            (10, "综合扫描", 5),
            (19, "字典社工", 6),
            (9, "信息收集", 7),
            (11, "CTF", 8),
            (12, "编码解码", 9),
            (14, "移动端", 10),
            (15, "局域网攻击", 11),
            (13, "应急查杀", 12),
            (16, "代码审计", 13),
            (17, "内网渗透", 14),
            (18, "逆向破解", 15),
            (20, "无线审计", 16),
            (21, "其他文件", 17),
        ];

        for (id, name, sort) in menus {
            conn.execute(
                "INSERT OR IGNORE INTO menu (id, name, sort) VALUES (?1, ?2, ?3)",
                params![id, name, sort],
            )?;
        }
    }

    let tag_count: i64 = conn.query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
    if tag_count == 0 {
        let tags = [
            (1, "管理工具", 2, 0),
            (2, "Shell", 2, 1),
            (3, "端口扫描", 9, 0),
            (4, "目录扫描", 9, 1),
            (6, "子域名", 9, 2),
            (12, "采集收集", 9, 3),
            (13, "指纹识别", 9, 4),
            (14, "暴力破解", 9, 5),
            (7, "Web", 11, 0),
            (8, "Crypto", 11, 1),
            (9, "Misc", 11, 2),
            (10, "Re", 11, 3),
            (11, "Pwn", 11, 4),
            (26, "通用", 11, 5),
            (15, "Android", 14, 0),
            (16, "IOS", 14, 1),
            (17, "HarmonyOS", 14, 2),
            (18, "Applet", 14, 3),
            (28, "其他", 14, 4),
            (42, "WSA", 14, 5),
            (19, "SQL注入", 1, 0),
            (20, "数据管理", 1, 1),
            (21, "数据库提权", 1, 2),
            (22, "JNDI注入", 1, 3),
            (23, "环境搭建", 1, 4),
            (24, "抓包工具", 5, 0),
            (25, "代理工具", 5, 1),
            (29, "字典", 19, 0),
            (30, "Windows", 13, 0),
            (31, "Linux", 13, 1),
            (32, "其他工具", 13, 2),
            (33, "内网工具", 17, 0),
            (34, "密码工具", 17, 1),
            (35, "横向移动", 17, 2),
            (36, "清理", 17, 3),
            (37, "权限提升", 17, 4),
            (38, "权限维持", 17, 5),
            (39, "文件下载", 17, 6),
            (40, "免杀", 17, 7),
            (41, "近源渗透", 19, 1),
        ];

        for (id, name, menu_id, sort) in tags {
            conn.execute(
                "INSERT OR IGNORE INTO tags (id, name, menu_id, sort) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, menu_id, sort],
            )?;
        }
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
             VALUES (1, 'feature', 'starttools', 'starttools', 'StartTools 工具启动器', 0)",
            [],
        )?;
        conn.execute(
            "INSERT INTO legacy_starttools_plugin_structure (id, entry_kind, name, code, explain, sort)
             VALUES (2, 'feature', 'starttools-run', 'starttools-run', '搜索并运行 StartTools 工具', 1)",
            [],
        )?;
    }

    Ok(())
}
