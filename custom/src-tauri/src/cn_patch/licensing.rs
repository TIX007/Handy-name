//! 设备激活码模块
//!
//! 激活码方案：RSA 签名的离线激活码。
//!
//! 激活码格式 (Base64 编码):
//!   RSA_SIGN(json{device_id, tier, expire_at})
//!
//! 客户端内置公钥验证签名，无需联网。
//! 生成激活码需要对应的私钥（由服务端持有）。
//!
//! 硬件指纹：
//! - Windows: MachineGuid + CPU 处理器名
//! - macOS: IOPlatformUUID
//! - Linux: /etc/machine-id

use chrono::{DateTime, Utc};
use log::{info, warn};
use sha2::{Digest, Sha256};
use tauri::AppHandle;

/// 内部激活状态（不序列化，仅 Rust 侧使用）
pub struct InternalActivationStatus {
    pub is_activated: bool,
    pub tier: LicenseTier,
    pub device_id: String,
    pub expire_at: Option<DateTime<Utc>>,
}

/// 授权等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseTier {
    /// 免费版：Whisper Small / Parakeet V2
    Free,
    /// 标准版：+ Whisper Medium / Parakeet V3
    Standard,
    /// 旗舰版：+ SenseVoice（系统自动管理）
    Premium,
}

/// 激活码内含的授权信息（签名前的明文 JSON）
#[derive(serde::Serialize, serde::Deserialize)]
struct LicensePayload {
    device_id: String,
    tier: String,
    /// ISO 8601 过期时间，None = 永久
    expire_at: Option<String>,
}

/// 生成设备唯一 ID
pub fn generate_device_id() -> String {
    let machine_info = collect_machine_info();
    let mut hasher = Sha256::new();
    hasher.update(machine_info.as_bytes());
    let hash = hasher.finalize();
    // 取前 16 字节格式化为 UUID 风格
    let hex: String = hash.iter().take(16).map(|b| format!("{:02x}", b)).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// 采集机器信息用于生成设备指纹
fn collect_machine_info() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // MachineGuid from registry
        let guid = Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .find(|l| l.contains("MachineGuid"))
                    .and_then(|l| l.split_whitespace().last())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        // CPU name
        let cpu = Command::new("wmic")
            .args(["cpu", "get", "name"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        format!("win:{}:{}", guid, cpu)
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let uuid = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .find(|l| l.contains("IOPlatformUUID"))
                    .and_then(|l| l.split('=').last())
                    .map(|s| s.trim().trim_matches('"').to_string())
            })
            .unwrap_or_default();
        format!("macos:{}", uuid)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/etc/machine-id")
            .map(|s| format!("linux:{}", s.trim()))
            .unwrap_or_else(|_| "linux:unknown".to_string())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unknown:unknown".to_string()
    }
}

/// 获取当前授权等级
pub fn current_tier(app_handle: &AppHandle) -> LicenseTier {
    let status = check_activation(app_handle);
    if status.is_activated {
        status.tier
    } else {
        LicenseTier::Free
    }
}

/// 检查激活状态
pub fn check_activation(app_handle: &AppHandle) -> InternalActivationStatus {
    let device_id = generate_device_id();
    let license_path = license_file_path(app_handle);

    match std::fs::read_to_string(&license_path) {
        Ok(content) => {
            match parse_license(&content, &device_id) {
                Ok(payload) => {
                    let is_expired = payload
                        .expire_at
                        .as_ref()
                        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                        .is_some_and(|t| t.with_timezone(&Utc) < Utc::now());

                    if is_expired {
                        warn!("License expired for device {}", device_id);
                        return InternalActivationStatus {
                            is_activated: false,
                            tier: LicenseTier::Free,
                            device_id,
                            expire_at: None,
                        };
                    }

                    let tier = match payload.tier.as_str() {
                        "standard" => LicenseTier::Standard,
                        "premium" => LicenseTier::Premium,
                        _ => LicenseTier::Free,
                    };

                    let expire_at = payload
                        .expire_at
                        .as_ref()
                        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                        .map(|t| t.with_timezone(&Utc));

                    InternalActivationStatus {
                        is_activated: true,
                        tier,
                        device_id,
                        expire_at,
                    }
                }
                Err(e) => {
                    warn!("License parse failed: {}", e);
                    InternalActivationStatus {
                        is_activated: false,
                        tier: LicenseTier::Free,
                        device_id,
                        expire_at,
                    }
                }
            }
        }
        Err(_) => InternalActivationStatus {
            is_activated: false,
            tier: LicenseTier::Free,
            device_id,
            expire_at: None,
        },
    }
}

/// 提交激活码进行激活
pub fn activate(app_handle: &AppHandle, code: &str) -> Result<InternalActivationStatus, String> {
    let device_id = generate_device_id();

    let payload = verify_activation_code(code, &device_id)?;

    // 持久化
    let license_path = license_file_path(app_handle);
    std::fs::write(&license_path, code.trim())
        .map_err(|e| format!("Failed to save license: {}", e))?;

    info!("Device {} activated as {}", device_id, payload.tier);

    let tier = match payload.tier.as_str() {
        "standard" => LicenseTier::Standard,
        "premium" => LicenseTier::Premium,
        _ => LicenseTier::Free,
    };

    let expire_at = payload
        .expire_at
        .as_ref()
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc));

    Ok(InternalActivationStatus {
        is_activated: true,
        tier,
        device_id,
        expire_at,
    })
}

// ── 内部实现 ─────────────────────────────────────────────────────────────

fn license_file_path(app_handle: &AppHandle) -> std::path::PathBuf {
    let data_dir = crate::portable::app_data_dir(app_handle)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    data_dir.join("license.dat")
}

fn parse_license(content: &str, expected_device_id: &str) -> Result<LicensePayload, String> {
    verify_activation_code(content.trim(), expected_device_id)
}

/// 验证激活码签名并解析授权信息
///
/// 在实际部署中，这里使用 RSA 公钥验签。
/// 开发阶段使用简化方案：HMAC-SHA256 验证。
fn verify_activation_code(code: &str, expected_device_id: &str) -> Result<LicensePayload, String> {
    // 激活码格式：base64(json_payload).base64(hmac_signature)
    let parts: Vec<&str> = code.trim().split('.').collect();
    if parts.len() != 2 {
        return Err("Invalid activation code format".to_string());
    }

    use base64::{engine::general_purpose, Engine as _};

    let payload_bytes = general_purpose::STANDARD
        .decode(parts[0])
        .map_err(|e| format!("Invalid payload encoding: {}", e))?;
    let payload: LicensePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("Invalid payload JSON: {}", e))?;

    // 验证设备 ID 匹配
    if payload.device_id != expected_device_id {
        return Err(format!(
            "Device mismatch: code is for {}, this device is {}",
            payload.device_id, expected_device_id
        ));
    }

    // 验证签名
    // 生产环境使用 RSA 公钥验签，此处用 HMAC 简化
    let secret = std::env::var("CN_LICENSE_SECRET")
        .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string());
    let mut hasher = sha2::Sha256::new();
    hasher.update(parts[0].as_bytes());
    hasher.update(secret.as_bytes());
    let expected_sig = hasher.finalize();
    let expected_sig_b64 = general_purpose::STANDARD.encode(expected_sig);

    if expected_sig_b64 != parts[1] {
        return Err("Invalid activation code signature".to_string());
    }

    Ok(payload)
}