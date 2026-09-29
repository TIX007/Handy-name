# Handy 中国大陆定制二开方案

## 一、设计目标

在不修改上游原始源码的前提下，以**补丁模块**方式实现以下定制：

1. **中国大陆网络适配** — 替换 `blob.handy.computer` / `huggingface.co` 为国内镜像 CDN
2. **SenseVoice 模型系统控制** — 隐藏用户选择，由系统按付费等级自动管理
3. **分模型安装包** — 按精简版/标准版/旗舰版打包不同模型组合
4. **设备激活码** — 离线/在线混合验证，绑定硬件设备

## 二、核心设计原则：补丁模块而非 Fork

```
上游代码 (git pull 更新)
    │
    ├── src-tauri/src/lib.rs          ← 不改，仅加一行 mod cn_patch;
    ├── src-tauri/src/main.rs         ← 不改
    ├── src-tauri/src/managers/model.rs ← 不改
    │
    └── custom/                        ← 二开隔离层
        ├── src-tauri/src/cn_patch/     ← Rust 补丁模块
        │   ├── mod.rs
        │   ├── mirror.rs              ← 镜像 URL 重写
        │   ├── licensing.rs           ← 激活码管理
        │   └── model_gate.rs          ← 模型访问控制
        ├── src/cn_patch/             ← 前端补丁
        │   ├── ActivationGate.tsx    ← 激活码 UI
        │   └── modelFilter.ts      ← 模型过滤
        └── patch-apply.ps1          ← 应用补丁脚本
```

**与上游同步的策略**：用脚本在 `lib.rs` 尾部注入 `mod cn_patch;` 一行（幂等），
其余代码全部在 `custom/` 目录内，`git pull` 后重新执行 `patch-apply.ps1` 即可。

## 三、四大功能模块详解

### 模块 1：中国大陆网络适配

**上游机制分析**：
- legacy 模型（SenseVoice 等）：`ModelSource::Url { url: "https://blob.handy.computer/..." }` — 直接 HTTP 下载
- catalog 模型（GGUF）：`ModelSource::HuggingFace { repo_id, revision }` — 经 hf-hub 下载，失败后 fallback 到 `ROOT.mirrors`（`blob.handy.computer`）

**补丁策略**：在 `download_model` 调用链中拦截 URL，将境外域名重写为国内镜像域名。
不修改 `model.rs` 中的模型定义表，而是在运行时替换 URL。

```rust
// custom/src-tauri/src/cn_patch/mirror.rs
// 核心逻辑：URL 重写规则
fn rewrite_url(original: &str) -> String {
    let rules = &[
        ("blob.handy.computer", "mirror.your-cn-cdn.com"),
        ("huggingface.co", "hf-mirror.com"),
    ];
    for (from, to) in rules {
        if original.contains(from) {
            return original.replace(from, to);
        }
    }
    original.to_string()
}
```

### 模块 2：SenseVoice 模型系统控制

**上游机制分析**：
- SenseVoice 是 legacy 模型，id = `"sense-voice-int8"`，`EngineType::SenseVoice`
- 模型列表经 `get_available_models()` 返回前端
- 前端 `ModelDropdown.tsx` 展示已下载模型供用户选择

**补丁策略**：
- 前端：过滤 `get_available_models()` 返回结果，SenseVoice 标记为系统控制模型，不在下拉菜单中显示
- 后端：新增 `cn_get_filtered_models()` 命令，根据激活状态返回可见模型列表
- SenseVoice 的下载/切换由后端 `model_gate.rs` 根据授权等级自动决策

```rust
// 付费等级 → 可用模型
enum LicenseTier {
    Free,       // 仅 Whisper Small / Parakeet V2
    Standard,   // + Whisper Medium / Parakeet V3
    Premium,    // + SenseVoice（系统自动选择，用户不可切换）
}
```

### 模块 3：分模型安装包

**策略**：通过 Cargo features + Vite env 控制打包时包含的模型集合。

```
安装包版本:
├── handy-lite-installer.exe    → Free 级别，不含模型文件，首启下载 Small
├── handy-standard-installer.exe → Standard 级别，预置 Medium
└── handy-premium-installer.exe  → Premium 级别，预置 SenseVoice
```

构建命令示例：
```bash
# 精简版
SHIPPED_TIER=free bun run tauri build

# 标准版
SHIPPED_TIER=standard bun run tauri build

# 旗舰版
SHIPPED_TIER=premium bun run tauri build
```

### 模块 4：设备激活码

**方案**：RSA 签名 + 硬件指纹，支持离线激活。

```
激活流程:
1. 客户端采集硬件指纹 → 设备ID (machine_code)
2. 用户输入激活码 → 客户端验证 RSA 签名
3. 验证通过 → 写入本地激活文件 (license.dat)
4. 每次启动检查激活状态

激活码格式:
  BASE64(RSA_ENCRYPT(json{device_id, tier, expire_at}))
```

硬件指纹组成：
- Windows: 机器 UUID (注册表 `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`) + CPU ID
- macOS: IOPlatformUUID
- Linux: /etc/machine-id

## 四、部署与维护

### 应用补丁

```powershell
# 拉取上游最新代码后执行
powershell -ExecutionPolicy Bypass -File custom/patch-apply.ps1
```

脚本完成两件事：
1. 在 `lib.rs` 的 `mod` 声明区追加 `mod cn_patch;`（已存在则跳过）
2. 在 `lib.rs` 的 `collect_commands!` 列表追加 `cn_patch::commands::*`（已存在则跳过）

### 回退补丁

```powershell
powershell -ExecutionPolicy Bypass -File custom/patch-apply.ps1 -Revert
```

### 构建不同版本

```powershell
# 1. 应用补丁
powershell -ExecutionPolicy Bypass -File custom/patch-apply.ps1

# 2. 构建指定版本
$env:SHIPPED_TIER = "premium"
bun run tauri build
```

## 五、文件清单

| 文件 | 用途 |
|------|------|
| `custom/src-tauri/src/cn_patch/mod.rs` | Rust 补丁模块入口 |
| `custom/src-tauri/src/cn_patch/mirror.rs` | 镜像 URL 重写 |
| `custom/src-tauri/src/cn_patch/licensing.rs` | 激活码验证 |
| `custom/src-tauri/src/cn_patch/model_gate.rs` | 模型访问控制 |
| `custom/src/cn_patch/ActivationGate.tsx` | 前端激活码 UI |
| `custom/src/cn_patch/modelFilter.ts` | 前端模型过滤 |
| `custom/patch-apply.ps1` | 补丁应用/回退脚本 |