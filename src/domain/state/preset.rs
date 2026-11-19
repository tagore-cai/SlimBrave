//! 快捷预设：按目录的 privacy / security 标记批量填充状态。

use super::{UiState, NOT_SET};
use crate::domain::catalog::init as catalog;
use crate::domain::Browser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Privacy,
    Security,
}

impl UiState {
    pub fn apply_preset(&mut self, preset: Preset, browser: Browser) {
        self.checked.fill(false);

        for (index, permission) in catalog().permissions.iter().enumerate() {
            let selection = match permission.name.as_str() {
                "JavaScript" => "Allow",
                "Camera" | "Microphone" => "Ask",
                "Images" => NOT_SET,
                _ => {
                    if permission.options.iter().any(|o| o == "Block") {
                        "Block"
                    } else {
                        NOT_SET
                    }
                }
            };
            self.permissions[index] = permission
                .options
                .iter()
                .position(|option| option == selection)
                .unwrap_or(0);
        }

        let keys = match preset {
            Preset::Privacy => &catalog().privacy_keys,
            Preset::Security => &catalog().security_keys,
        };
        for key in keys {
            if !browser.supports_policy(key) {
                continue;
            }
            if let Some(feature) = catalog().feature_by_key(key) {
                if let Some(index) = catalog().features.iter().position(|f| f.key == feature.key) {
                    if let Some(checked) = self.checked.get_mut(index) {
                        *checked = true;
                    }
                }
            }
        }

        match preset {
            Preset::Privacy => {
                self.safe_browsing = Some(1);
                self.dns_mode = 1;
                self.dns_template.clear();
            }
            Preset::Security => {
                self.safe_browsing = Some(0);
                self.dns_mode = 0;
                self.dns_template.clear();
            }
        }
    }
}
