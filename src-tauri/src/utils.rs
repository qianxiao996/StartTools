use once_cell::sync::Lazy;
use std::env;
use std::path::Path;
// 定义 DB_PATH 为一个惰性初始化的静态变量
pub static DB_PATH: Lazy<String> = Lazy::new(|| {
    // 获取程序所在的目录
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let program_dir = exe_path.parent().expect("Failed to get parent directory");

    // 构造数据库路径
    let db_path = program_dir.join("data.db");

    // 转换为字符串并返回
    db_path
        .to_str()
        .expect("Failed to convert path to string")
        .to_string()
});

pub static EXE_PATH: Lazy<String> = Lazy::new(|| {
    // 获取程序所在的目录
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let program_dir = exe_path.parent().expect("Failed to get parent directory");
    // 转换为字符串并返回
    program_dir
        .to_str()
        .expect("Failed to convert path to string")
        .to_string()
});
pub fn get_file_path(newfile: &str) -> String {
    // 新增逻辑：判断 target_str 是否为目录，如果不是则取出目录
    let path = if Path::new(newfile).is_dir() {
        newfile.to_string()
    } else {
        match Path::new(&newfile).parent() {
            Some(parent) => parent.to_str().unwrap_or("").to_string(),
            None => {
                log::error!("无法获取目录: {}", newfile);
                return format!("无法获取目录: {}", newfile);
            }
        }
    };
    return path;
}