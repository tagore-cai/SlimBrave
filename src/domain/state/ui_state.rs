//! 内存中的 UI 状态容器：与目录等长的勾选 / 选择向量。

use crate::domain::catalog::init as catalog;

#[derive(Debug, Clone)]
pub struct UiState {
    pub checked: Vec<bool>,
    pub permissions: Vec<usize>,
    pub safe_browsing: Option<usize>,
    pub dns_mode: usize,
    pub dns_template: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            checked: vec![false; catalog().features.len()],
            permissions: vec![0; catalog().permissions.len()],
            safe_browsing: None,
            dns_mode: 0,
            dns_template: String::new(),
        }
    }
}
