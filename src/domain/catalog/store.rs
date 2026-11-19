//! 进程级目录单例与内置 / 用户 / 远程文件的加载。

use std::sync::{OnceLock, RwLock};

use crate::domain::Browser;

use super::model::Catalog;
use super::schema::{RemoteCatalog, UserCatalog};

pub const BUILTIN_CATALOG: &str = include_str!("../../../assets/catalog.json");

static CATALOG: OnceLock<RwLock<Catalog>> = OnceLock::new();

pub fn init() -> std::sync::RwLockReadGuard<'static, Catalog> {
    CATALOG
        .get_or_init(|| RwLock::new(build_catalog()))
        .read()
        .expect("catalog lock poisoned")
}

pub fn reload() {
    if let Some(lock) = CATALOG.get() {
        let mut catalog = lock.write().expect("catalog lock poisoned");
        *catalog = build_catalog();
    }
}

fn build_catalog() -> Catalog {
    let mut catalog =
        Catalog::from_json(BUILTIN_CATALOG).expect("builtin catalog must be valid JSON");

    if let Some(user) = load_json_file(user_catalog_path()) {
        match serde_json::from_str::<UserCatalog>(&user) {
            Ok(user) => catalog.merge_user(&user),
            Err(err) => log_merge(&format!("user catalog ignored: {err}")),
        }
    }

    catalog
}

fn load_json_file(path: Option<std::path::PathBuf>) -> Option<String> {
    std::fs::read_to_string(path?).ok()
}

pub fn load_remote(browser: Browser) -> Option<RemoteCatalog> {
    let path = dirs::cache_dir()?
        .join("slimbrave")
        .join(browser.cache_file());
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

fn user_catalog_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| home.join(".config/slimbrave/catalog.json"))
}

fn log_merge(message: &str) {
    #[cfg(not(test))]
    crate::logging::log(message);
    #[cfg(test)]
    let _ = message;
}
