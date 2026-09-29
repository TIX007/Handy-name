//! 镜像下载命令封装
//!
//! 提供一个 Tauri 命令 `cn_download_model`，在上游 `download_model` 之前
//! 重写 URL 为国内镜像地址。前端调用 `cn_download_model` 替代 `download_model`。
//!
//! 原理：
//! 上游 `ModelManager::download_model` 从 `model_info.source` 取 URL 直接下载。
//! 我们在调用前用 `mirror::rewrite_url` 替换 `ModelSource::Url` 中的 URL 字段。
//! 对 `ModelSource::HuggingFace` 类型，需修改 HF 的 endpoint（通过环境变量）。

use tauri::{AppHandle, Manager};

use super::mirror;

/// 镜像化下载模型（替代上游 `download_model`）
///
/// 前端调用方式：
/// ```ts
/// await commands.cnDownloadModel(modelId);
/// ```
#[tauri::command]
#[specta::specta]
pub async fn cn_download_model(
    app_handle: AppHandle,
    model_id: String,
) -> Result<(), String> {
    // 获取 ModelManager
    let model_manager = app_handle
        .state::<std::sync::Arc<crate::managers::model::ModelManager>>()
        .inner()
        .clone();

    // 在下载前重写 URL（针对 legacy 模型）
    // 由于 ModelManager 内部从 source 读取 URL，我们需要在 available_models 中替换
    {
        let mut models = model_manager.available_models.lock().unwrap();
        if let Some(model) = models.get_mut(&model_id) {
            if let crate::managers::model::ModelSource::Url { url, .. } = &model.source {
                let rewritten = mirror::rewrite_url(url);
                if rewritten != *url {
                    log::info!("cn_download_model: rewrote URL for {}", model_id);
                    model.source = crate::managers::model::ModelSource::Url {
                        url: rewritten,
                        sha256: model.source.clone().get_sha256(),
                    };
                }
            }
        }
    }

    // 设置 HF endpoint 环境变量（针对 catalog 模型）
    // hf-hub 库读取 HF_ENDPOINT 环境变量
    if std::env::var("HF_ENDPOINT").is_err() {
        std::env::set_var("HF_ENDPOINT", "https://hf-mirror.com");
    }

    // 调用上游下载逻辑
    model_manager
        .download_model(&model_id)
        .await
        .map_err(|e| e.to_string())
}

/// 辅助 trait：从 ModelSource 提取 sha256
trait ModelSourceExt {
    fn get_sha256(&self) -> Option<String>;
}

impl ModelSourceExt for crate::managers::model::ModelSource {
    fn get_sha256(&self) -> Option<String> {
        match self {
            crate::managers::model::ModelSource::Url { sha256, .. } => sha256.clone(),
            _ => None,
        }
    }
}