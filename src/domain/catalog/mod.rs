//! 策略目录（`domain` 层）。
//!
//! 由 `assets/catalog.json` 内置清单驱动，支持用户覆盖层与远程模板层合并。
//! 子模块按职责拆分：
//! - [`model`]：领域模型与目录合并行为
//! - [`schema`]：JSON 序列化结构（内置 / 用户 / 远程）
//! - [`store`]：进程级单例与内置 / 用户文件加载

mod model;
mod schema;
mod store;

#[cfg(test)]
mod tests;

pub use model::{Feature, PolicyValue};
#[cfg(not(target_os = "windows"))]
pub use schema::RemotePolicy;
pub use schema::{RemoteCatalog, UserCatalog};
pub use store::{init, load_remote, reload};
