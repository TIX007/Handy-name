//! 模型访问控制模块
//!
//! 根据授权等级控制用户可见和可选用的模型。
//! SenseVoice 在 Premium 级别下由系统自动管理（用户不可手动切换）。

use log::debug;
use tauri::{AppHandle, Manager};

use super::licensing::LicenseTier;
use super::VisibleModelInfo;

/// 系统控制模型 ID
pub const SYSTEM_CONTROLLED_MODELS: &[&str] = &["sense-voice-int8"];

/// 各等级可见的模型 ID 白名单
///
/// - Free: 基础模型
/// - Standard: + 中级模型
/// - Premium: + 系统控制模型（SenseVoice，由系统自动选择）
pub fn tier_allowed_models(tier: &LicenseTier) -> Vec<&'static str> {
    let base = vec![
        "small",
        "parakeet-tdt-0.6b-v2",
    ];

    match tier {
        LicenseTier::Free => base,
        LicenseTier::Standard => {
            let mut v = base;
            v.extend(vec![
                "medium",
                "parakeet-tdt-0.6b-v3",
                "canary-180m-flash",
            ]);
            v
        }
        LicenseTier::Premium => {
            let mut v = base;
            v.extend(vec![
                "medium",
                "parakeet-tdt-0.6b-v3",
                "canary-180m-flash",
                "sense-voice-int8", // 系统控制，用户不可手动选
                "large",
            ]);
            v
        }
    }
}

/// 获取经过授权过滤的可见模型列表
///
/// 从上游 `ModelManager` 获取全部模型，过滤出当前等级可见的模型，
/// 对系统控制模型（SenseVoice）标记 `is_system_controlled = true`。
pub fn get_visible_models(app_handle: &AppHandle, tier: &LicenseTier) -> Vec<VisibleModelInfo> {
    let allowed = tier_allowed_models(tier);
    debug!("get_visible_models: tier={:?}, allowed={:?}", tier, allowed);

    // 从上游 ModelManager 获取模型列表
    let model_manager = match app_handle.try_state::<std::sync::Arc<crate::managers::model::ModelManager>>() {
        Some(mm) => mm,
        None => {
            debug!("ModelManager not yet initialized");
            return Vec::new();
        }
    };

    let all_models = model_manager.get_available_models();

    all_models
        .into_iter()
        .filter(|m| {
            // 白名单内的模型可见
            allowed.contains(&m.id.as_str())
        })
        .map(|m| {
            let is_system_controlled = SYSTEM_CONTROLLED_MODELS.contains(&m.id.as_str());
            VisibleModelInfo {
                id: m.id,
                name: m.name,
                description: m.description,
                is_downloaded: m.is_downloaded,
                is_system_controlled,
                size_mb: m.size_mb,
            }
        })
        .collect()
}

/// 判断模型是否为系统控制（用于前端额外过滤）
pub fn is_system_controlled(model_id: &str) -> bool {
    SYSTEM_CONTROLLED_MODELS.contains(&model_id)
}

/// 判断当前等级是否允许自动下载/切换 SenseVoice
pub fn can_use_sensevoice(tier: &LicenseTier) -> bool {
    tier == &LicenseTier::Premium
}