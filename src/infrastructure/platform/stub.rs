//! 非 macOS / Windows 目标的占位实现（项目声明不支持这些平台）。

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::domain::{RawValue, WriteLevel};

use super::{ApplyReport, Channel};

pub(super) fn is_installed(_channel: &Channel) -> bool {
    false
}

pub(super) fn is_brave_running(_channel: &Channel) -> bool {
    false
}

pub(super) fn close_brave_process(_channel: &Channel) -> bool {
    false
}

pub(super) fn remove_managed_pref_files(
    _channel: &Channel,
    _write_level: WriteLevel,
) -> Vec<String> {
    Vec::new()
}

pub(super) fn strip_legacy_keys(_channel: &Channel) -> Vec<String> {
    Vec::new()
}

pub(super) fn apply_to(
    _channel: &Channel,
    _plan: &crate::domain::ApplyPlan,
    _dry_run: bool,
    _write_level: WriteLevel,
    _target_dir: Option<&Path>,
) -> Result<ApplyReport> {
    Ok(ApplyReport {
        channel: _channel.name,
        path: String::new(),
        written: Vec::new(),
        deleted: Vec::new(),
        dry_run: _dry_run,
        removed_profile: None,
    })
}

pub(super) fn read_from(
    _channel: &Channel,
    _target_dir: Option<&Path>,
) -> Result<BTreeMap<String, RawValue>> {
    Ok(BTreeMap::new())
}
