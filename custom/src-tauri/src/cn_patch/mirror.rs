//! 镜像 URL 重写模块
//!
//! 将上游 `blob.handy.computer` / `huggingface.co` 的下载请求
//! 重定向到国内镜像节点，保证中国大陆网络可用性。
//!
//! 拦截点：在 `ModelManager::download_model` 执行前替换 URL。
//! 安装包构建时通过环境变量 `CN_MIRROR_BASE` 配置镜像地址。

use log::{debug, info};

/// 默认镜像规则，可通过环境变量覆盖
const DEFAULT_RULES: &[(&str, &str)] = &[
    ("blob.handy.computer", "mirror.cn.example.com"),
    ("huggingface.co", "hf-mirror.com"),
];

/// 将原始 URL 重写为国内镜像 URL
///
/// 若 URL 不匹配任何规则，原样返回。
pub fn rewrite_url(original: &str) -> String {
    let mirror_base = std::env::var("CN_MIRROR_BASE").unwrap_or_default();
    let rules: Vec<(String, String)> = if !mirror_base.is_empty() {
        vec![
            ("blob.handy.computer".to_string(), mirror_base.clone()),
            ("huggingface.co".to_string(), "hf-mirror.com".to_string()),
        ]
    } else {
        DEFAULT_RULES
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    };

    for (from, to) in &rules {
        if original.contains(from.as_str()) {
            let rewritten = original.replace(from, to);
            debug!("mirror::rewrite_url: {} -> {}", original, rewritten);
            return rewritten;
        }
    }
    original.to_string()
}

/// 获取当前生效的镜像基址列表（供前端诊断展示）
pub fn active_mirrors() -> Vec<String> {
    let mirror_base = std::env::var("CN_MIRROR_BASE").unwrap_or_default();
    if !mirror_base.is_empty() {
        return vec![mirror_base, "hf-mirror.com".to_string()];
    }
    DEFAULT_RULES.iter().map(|(_, to)| to.to_string()).collect()
}

/// 初始化镜像模块，在启动时记录生效配置
pub fn init() {
    info!(
        "cn_patch::mirror initialized, active mirrors: {:?}",
        active_mirrors()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_blob_url() {
        let url = "https://blob.handy.computer/sense-voice-int8.tar.gz";
        let rewritten = rewrite_url(url);
        assert!(rewritten.contains("mirror.cn.example.com"));
        assert!(rewritten.contains("sense-voice-int8.tar.gz"));
    }

    #[test]
    fn rewrites_hf_url() {
        let url = "https://huggingface.co/handy-computer/whisper-small-gguf/resolve/main/file.gguf";
        let rewritten = rewrite_url(url);
        assert!(rewritten.contains("hf-mirror.com"));
    }

    #[test]
    fn passthrough_unknown_url() {
        let url = "https://example.com/model.bin";
        assert_eq!(rewrite_url(url), url);
    }
}