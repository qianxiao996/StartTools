mod config;
mod db;
mod load_data;
mod starttools_plugin;
mod tools;
pub mod utils;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri::Emitter;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use winapi::um::winuser::GetCursorPos;
use winapi::shared::windef::POINT;
use winapi::um::winuser::{
    ChangeWindowMessageFilterEx, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, WM_COPYDATA,
    WM_DROPFILES,
};
use winapi::um::winuser::GetDpiForSystem;
#[cfg(target_os = "windows")]
use winapi::um::shellapi::DragAcceptFiles;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
// #[tauri::command]
// fn greet(name: &str) -> String {
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }
// 托盘图标
// 导入系统托盘所需的依赖, 导入的全都是
use tauri::{
    tray::{
        TrayIconBuilder,
        MouseButtonState,
        MouseButton,
        TrayIconEvent
    },
    menu::{
        Menu,
        MenuItem
    },
    LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl, WebviewWindowBuilder
};

static REGISTERED_SEARCH_HOTKEY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static REGISTERED_MAIN_WINDOW_HOTKEY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static IS_CREATING_SEARCH_WINDOW: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
const SEARCH_WINDOW_WIDTH: f64 = 640.0;
const SEARCH_WINDOW_COLLAPSED_HEIGHT: f64 = 42.0;
const SEARCH_WINDOW_Y_OFFSET: i32 = 120;

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let lower = part.to_lowercase();
            match lower.as_str() {
                "ctrl" | "control" => "CommandOrControl".to_string(),
                "cmd" | "command" | "meta" | "win" => "Super".to_string(),
                "esc" => "Escape".to_string(),
                _ if part.len() == 1 => part.to_uppercase(),
                _ => part.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn open_search_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("search") {
        let _ = win.set_decorations(false);
        let _ = win.set_size(Size::Logical(LogicalSize::new(
            SEARCH_WINDOW_WIDTH,
            SEARCH_WINDOW_COLLAPSED_HEIGHT,
        )));
        let _ = win.unminimize();
        position_search_window(&win);
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    {
        let mut is_creating = match IS_CREATING_SEARCH_WINDOW.lock() {
            Ok(lock) => lock,
            Err(_) => return,
        };
        if *is_creating {
            return;
        }
        *is_creating = true;
    }

    std::thread::spawn(move || {
        let result = WebviewWindowBuilder::new(
            &app,
            "search",
            WebviewUrl::App("/global-search".into()),
        )
        .title("搜索")
        .inner_size(SEARCH_WINDOW_WIDTH, SEARCH_WINDOW_COLLAPSED_HEIGHT)
        .min_inner_size(420.0, SEARCH_WINDOW_COLLAPSED_HEIGHT)
        .center()
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .focused(true)
        .visible(false)
        .build();

        if let Ok(win) = result {
            position_search_window(&win);
        }
        if let Ok(mut is_creating) = IS_CREATING_SEARCH_WINDOW.lock() {
            *is_creating = false;
        }
    });
}

fn position_search_window(win: &tauri::WebviewWindow) {
    let _ = win.center();
    if let Ok(position) = win.outer_position() {
        let next_y = position.y.saturating_sub(SEARCH_WINDOW_Y_OFFSET);
        let _ = win.set_position(Position::Physical(PhysicalPosition::new(position.x, next_y)));
    }
}

fn is_settings_window_visible(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("config")
        .and_then(|win| win.is_visible().ok())
        .unwrap_or(false)
}

fn toggle_main_window(app: tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    match win.is_visible() {
        Ok(true) => {
            let _ = win.hide();
            let _ = app.emit("isclose-edge", true);
        }
        Ok(false) => {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_focus();
            let _ = app.emit("isclose-edge", false);
        }
        Err(error) => {
            log::warn!("Failed to toggle main window: {}", error);
        }
    }
}

fn register_search_hotkey_internal(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    let next_shortcut = normalize_shortcut(if shortcut.trim().is_empty() {
        "Alt+3"
    } else {
        shortcut
    });

    let mut registered = REGISTERED_SEARCH_HOTKEY
        .lock()
        .map_err(|_| "Failed to lock search hotkey state".to_string())?;

    if registered.as_deref() == Some(next_shortcut.as_str()) {
        return Ok(());
    }

    if let Some(current) = registered.as_deref() {
        if let Err(error) = app.global_shortcut().unregister(current) {
            log::warn!("Failed to unregister search shortcut {}: {}", current, error);
        }
    }

    let shortcut_for_handler = next_shortcut.clone();
    app.global_shortcut()
        .on_shortcut(next_shortcut.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if is_settings_window_visible(app) {
                    return;
                }
                log::info!("Search shortcut triggered: {}", shortcut_for_handler);
                open_search_window(app.clone());
            }
        })
        .map_err(|error| format!("Failed to register search shortcut {}: {}", next_shortcut, error))?;

    log::info!("Search shortcut registered: {}", next_shortcut);
    *registered = Some(next_shortcut);
    Ok(())
}

#[tauri::command]
fn register_search_hotkey(app: tauri::AppHandle, shortcut: String) -> Result<(), String> {
    register_search_hotkey_internal(&app, &shortcut)
}

fn register_main_window_hotkey_internal(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    let next_shortcut = normalize_shortcut(if shortcut.trim().is_empty() {
        "Alt+2"
    } else {
        shortcut
    });

    let mut registered = REGISTERED_MAIN_WINDOW_HOTKEY
        .lock()
        .map_err(|_| "Failed to lock main window hotkey state".to_string())?;

    if registered.as_deref() == Some(next_shortcut.as_str()) {
        return Ok(());
    }

    if let Some(current) = registered.as_deref() {
        if let Err(error) = app.global_shortcut().unregister(current) {
            log::warn!("Failed to unregister main window shortcut {}: {}", current, error);
        }
    }

    let shortcut_for_handler = next_shortcut.clone();
    app.global_shortcut()
        .on_shortcut(next_shortcut.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if is_settings_window_visible(app) {
                    return;
                }
                log::info!("Main window shortcut triggered: {}", shortcut_for_handler);
                toggle_main_window(app.clone());
            }
        })
        .map_err(|error| format!("Failed to register main window shortcut {}: {}", next_shortcut, error))?;

    log::info!("Main window shortcut registered: {}", next_shortcut);
    *registered = Some(next_shortcut);
    Ok(())
}

#[tauri::command]
fn register_main_window_hotkey(app: tauri::AppHandle, shortcut: String) -> Result<(), String> {
    register_main_window_hotkey_internal(&app, &shortcut)
}
#[cfg(target_os = "windows")]
fn get_global_mouse_position() -> (i32, i32) {
    unsafe {
        let mut point: POINT = std::mem::zeroed();
        if GetCursorPos(&mut point) == 0 {
            return (0, 0);
        } else {
            (point.x, point.y)
        }
    }
}

#[cfg(target_os = "windows")]
fn allow_windows_file_drop_messages(win: &tauri::WebviewWindow) {
    const WM_COPYGLOBALDATA: u32 = 0x0049;
    const MSGFLT_ALLOW: u32 = 1;

    if let Ok(hwnd) = win.hwnd() {
        unsafe {
            let raw_hwnd = hwnd.0 as _;
            DragAcceptFiles(raw_hwnd, 1);
            for message in [WM_DROPFILES, WM_COPYDATA, WM_COPYGLOBALDATA] {
                ChangeWindowMessageFilterEx(
                    raw_hwnd,
                    message,
                    MSGFLT_ALLOW,
                    std::ptr::null_mut(),
                );
            }
        }
    }
}
#[cfg(target_os = "macos")]
fn get_global_mouse_position() -> (i32, i32) {
    use core_graphics::event::CGEvent;
    use core_graphics::geometry::CGPoint;

    let event = CGEvent::new().unwrap();
    let location: CGPoint = event.location();
    (location.x as i32, location.y as i32)
}

#[cfg(target_os = "linux")]
fn get_global_mouse_position() -> (i32, i32) {
    use x11::xlib::{XQueryPointer, Display, RootWindow};
    use std::ptr;

    unsafe {
        let display = x11::xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return (0, 0); // 如果无法打开显示，返回默认值
        }

        let root = x11::xlib::XDefaultRootWindow(display);
        let mut root_x = 0;
        let mut root_y = 0;
        let mut win_x = 0;
        let mut win_y = 0;
        let mut mask = 0;

        x11::xlib::XQueryPointer(
            display,
            root,
            &mut root,
            &mut root,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
        );

        x11::xlib::XCloseDisplay(display);

        (root_x, root_y)
    }
}

// 检查鼠标是否靠近屏幕边缘
fn is_mouse_near_edge(x: i32, y: i32) -> bool {
    // log::info!("x: {},y:{}", x,y);
    
    const EDGE_THRESHOLD: i32 = 10;
    const BASE_DPI: u32 = 96; // Windows 系统的基准 DPI

    // 动态获取屏幕宽度和高度
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    // 获取系统的 DPI
    let system_dpi = unsafe { GetDpiForSystem() };
    // 计算缩放比例
    let scale_factor = system_dpi as f64 / BASE_DPI as f64;

    // 根据缩放比例调整阈值
    let adjusted_threshold = (EDGE_THRESHOLD as f64 * scale_factor) as i32;

    x <= adjusted_threshold || x >= screen_width - adjusted_threshold ||
    y <= adjusted_threshold || y >= screen_height - adjusted_threshold
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // env_logger::init();
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info) // 设置日志级别为 Info
        .init();
    if let Err(err) = db::init_database() {
        log::error!("Failed to initialize database: {}", err);
    }
    if let Err(err) = starttools_plugin::start_plugin_http_server() {
        log::error!("Failed to start StartTools plugin HTTP server: {}", err);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init()) // 注册 Dialog 插件
        .plugin(tauri_plugin_opener::init())
        // 更新 invoke_handler，使用 config 模块中的函数
        .invoke_handler(tauri::generate_handler![
            config::load_config,
            load_data::load_menus,
            load_data::load_tags,
            load_data::load_tools,
            config::update_config,
            config::export_data,
            config::import_data,
            tools::open_tools,
            tools::load_builtin_tools,
            tools::load_all_builtin_tools,
            tools::export_builtin_tools,
            tools::import_builtin_tools,
            tools::update_builtin_tool,
            tools::delete_builtin_tool,
            tools::run_builtin_tool,
            tools::copy_path,
            tools::open_path,
            tools::open_cmd,
            tools::get_exe_icon,
            tools::update_tool,
            tools::delete_tool,
            tools::delete_menu,
            tools::delete_tag,
            tools::clear_tool,
            tools::clear_tool_no,
            tools::update_tag,
            tools::update_menu,
            tools::change_bianxie_path,
            starttools_plugin::load_starttools_plugins,
            starttools_plugin::export_starttools_plugins,
            starttools_plugin::import_starttools_plugins,
            starttools_plugin::update_starttools_plugin,
            starttools_plugin::delete_starttools_plugin,
            starttools_plugin::open_starttools_plugin,
            starttools_plugin::open_starttools_plugin_dir,
            starttools_plugin::restart_starttools_plugin_http_server,
            register_search_hotkey,
            register_main_window_hotkey
        ])
        .setup(|app| {
            let config = config::load_config();
            if let Some(shortcut) = config.get("HotKey_Search").and_then(|value| value.as_str()) {
                if let Err(error) = register_search_hotkey_internal(app.handle(), shortcut) {
                    log::error!("{}", error);
                }
            }
            if let Some(shortcut) = config.get("HotKey_Show_Hidden").and_then(|value| value.as_str()) {
                if let Err(error) = register_main_window_hotkey_internal(app.handle(), shortcut) {
                    log::error!("{}", error);
                }
            }
            #[cfg(target_os = "windows")]
            if let Some(win) = app.get_webview_window("main") {
                allow_windows_file_drop_messages(&win);
            }

            let handle = app.handle().clone();
            // 启动一个线程监听全局鼠标事件
            std::thread::spawn(move || {
                loop {
                    // 调用系统 API 获取鼠标位置
                    let (x, y) = get_global_mouse_position(); // 自定义函数
                    // 检查鼠标是否靠近屏幕边缘
                    if is_mouse_near_edge(x, y) {
                        handle.emit("mouse-near-edge", serde_json::json!({ "x": x, "y": y })).unwrap();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100)); // 避免占用过多 CPU
                }
            });
            let show_i = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
            // 创建系统托盘
            let _tray = TrayIconBuilder::new()
                // 添加托盘图标
                .icon(app.default_window_icon().unwrap().clone())
                // 添加菜单
                .menu(&menu)
                // 禁用鼠标左键点击图标显示托盘菜单
                // .menu_on_left_click(false)
                // 监听托盘图标发出的鼠标事件
                .on_tray_icon_event(|tray, event| match event {
                    // 左键点击托盘图标显示窗口
                    TrayIconEvent::Click {
                        id: _,
                        position: _,
                        rect: _,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                    } => {
                        let app_handle = tray.app_handle();
                        let win = tray
                            .app_handle()
                            .get_webview_window("main")
                            .expect("REASON");
                        match win.is_visible() {
                            Ok(visible) if !visible => {
                                win.show().unwrap();
                                app_handle.emit("isclose-edge", false).unwrap();
                                win.set_focus().unwrap();
                            }
                            Err(e) => eprintln!("{}", e),
                            _ => (),
                        };
                        // 获取窗口焦点
                        win.set_focus().unwrap();
                    }
                    _ => {}
                })
                // 监听菜单事件
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let handle = app.app_handle().clone();
                        let win = app.get_webview_window("main").unwrap();
                        match win.is_visible() {
                            Ok(visible) if !visible => {
                                win.show().unwrap();
                            }
                            Err(e) => eprintln!("{}", e),
                            _ => (),
                        };
                        // 获取窗口焦点
                        win.set_focus().unwrap();
                        handle.emit("isclose-edge", false).unwrap();

                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
