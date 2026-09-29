//! cn_patch — 中国大陆定制补丁模块
//!
//! 本模块作为独立单元注入 Handy，不修改上游任何源文件。
//! 通过 `patch-apply.ps1` 在 `lib.rs` 中追加 `mod cn_patch;` 启用。

pub mod download_proxy;
pub mod licensing;
pub mod mirror;
pub mod model_gate;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

/// 可见模型信息（传给前端的精简结构）
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VisibleModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_downloaded: bool,
    pub is_system_controlled: bool,
    pub size_mb: u64,
}

/// 激活状态
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ActivationStatus {
    pub is_activated: bool,
    pub tier: String,
    pub device_id: String,
    pub expire_at: Option<String>,
}

// ── Tauri 命令 ─────────────────────────────────────────────────────────────

/// 获取过滤后的模型列表（SenseVoice 根据授权等级隐藏）
#[tauri::command]
#[specta::specta]
pub async fn cn_get_filtered_models(
    app_handle: AppHandle,
) -> Result<Vec<VisibleModelInfo>, String> {
    let tier = licensing::current_tier(&app_handle);
    let models = model_gate::get_visible_models(&app_handle, &tier);
    Ok(models)
}

/// 获取当前激活状态
#[tauri::command]
#[specta::specta]
pub async fn cn_get_activation_status(
    app_handle: AppHandle,
) -> Result<ActivationStatus, String> {
    let status = licensing::check_activation(&app_handle);
    Ok(ActivationStatus {
        is_activated: status.is_activated,
        tier: format!("{:?}", status.tier).to_lowercase(),
        device_id: status.device_id,
        expire_at: status.expire_at.map(|t| t.to_rfc3339()),
    })
}

/// 提交激活码
#[tauri::command]
#[specta::specta]
pub async fn cn_submit_activation_code(
    app_handle: AppHandle,
    code: String,
) -> Result<ActivationStatus, String> {
    let status = licensing::activate(&app_handle, &code)?;
    Ok(ActivationStatus {
        is_activated: status.is_activated,
        tier: format!("{:?}", status.tier).to_lowercase(),
        device_id: status.device_id,
        expire_at: status.expire_at.map(|t| t.to_rfc3339()),
    })
}

/// 获取设备 ID（用于激活页展示）
#[tauri::command]
#[specta::specta]
pub async fn cn_get_device_id() -> Result<String, String> {
    Ok(licensing::generate_device_id())
}