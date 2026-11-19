//! 轻量诊断日志，供各层共用（类似 shared kernel，避免 domain 反向依赖 infrastructure）。
//!
//! Windows GUI 声明 `windows_subsystem = "windows"` 后与控制台分离，stderr 不再可见；
//! 因此 Windows 下把诊断信息追加到本地日志文件，其它平台维持原有 stderr 输出。

/// 输出一条诊断日志。
pub fn log(message: &str) {
    #[cfg(target_os = "windows")]
    write_file(message);
    #[cfg(not(target_os = "windows"))]
    eprintln!("slimbrave: {message}");
}

#[cfg(target_os = "windows")]
fn write_file(message: &str) {
    use std::io::Write;

    let Some(path) = dirs::cache_dir().map(|dir| dir.join("slimbrave").join("slimbrave.log"))
    else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "slimbrave: {message}");
    }
}
