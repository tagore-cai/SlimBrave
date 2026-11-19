//! macOS 配置描述文件（.mobileconfig）：构造、写入、安装、查询。
//!
//! - [`builder`]：纯函数式 plist 构造（含稳定 UUID 与值转换）
//! - [`install`]：文件 IO 与 `profiles` / `open` 系统命令

mod builder;
mod install;

#[cfg(test)]
mod tests;

pub use builder::build_mobileconfig;
pub use install::{open_config, profile_installed, remove_profile, write_mobileconfig};
