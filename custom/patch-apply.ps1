# patch-apply.ps1 — 应用/回退中国大陆定制补丁
#
# 用法:
#   .\custom\patch-apply.ps1            # 应用补丁
#   .\custom\patch-apply.ps1 -Revert   # 回退补丁

param(
    [switch]$Revert
)

$ErrorActionPreference = "Stop"
$WorkspaceRoot = Resolve-Path "$PSScriptRoot\.."
$LibRs = Join-Path $WorkspaceRoot "src-tauri\src\lib.rs"
$ModMarker = "mod cn_patch;"
$CommandsMarker = "cn_patch::cn_get_filtered_models"

Write-Host "=== Handy CN Patch Script ===" -ForegroundColor Cyan

if ($Revert) {
    Write-Host "Reverting patches..." -ForegroundColor Yellow

    # 移除 mod cn_patch;
    $content = Get-Content $LibRs -Raw -Encoding UTF8
    $content = $content -replace "mod cn_patch;`r?`n", ""
    $content = $content -replace "cn_patch::cn_get_filtered_models,`r?`n", ""
    $content = $content -replace "cn_patch::cn_get_activation_status,`r?`n", ""
    $content = $content -replace "cn_patch::cn_submit_activation_code,`r?`n", ""
    $content = $content -replace "cn_patch::cn_get_device_id,`r?`n", ""
    $content = $content -replace "cn_patch::download_proxy::cn_download_model,`r?`n", ""
    $content | Set-Content $LibRs -Encoding UTF8 -NoNewline

    Write-Host "Reverted: removed cn_patch module declarations from lib.rs" -ForegroundColor Green
    Write-Host "Note: custom/ directory left intact for re-application." -ForegroundColor DarkGray
    exit 0
}

# ── 应用补丁 ─────────────────────────────────────────────────────────────

Write-Host "Applying patches to: $LibRs"

$content = Get-Content $LibRs -Raw -Encoding UTF8
$modified = $false

# 1. 注入 mod cn_patch; （放在 mod 声明区末尾，utils 之后）
if ($content -notmatch [regex]::Escape($ModMarker)) {
    # 找到最后一个 "mod xxx;" 行的位置，在其后追加
    $pattern = "(?m)^(mod\s+\w+;.*\r?\n)"
    $lastMatch = [regex]::Matches($content, $pattern) | Select-Object -Last 1
    if ($lastMatch) {
        $insertPos = $lastMatch.Index + $lastMatch.Value.Length
        $content = $content.Substring(0, $insertPos) + $ModMarker + "`n" + $content.Substring($insertPos)
    } else {
        # 找不到的话，在第一行插入
        $content = $ModMarker + "`n" + $content
    }
    $modified = $true
    Write-Host "  [+] Added: $ModMarker" -ForegroundColor Green
} else {
    Write-Host "  [=] Already present: $ModMarker" -ForegroundColor DarkGray
}

# 2. 在 collect_commands! 列表中注入 cn_patch 命令
$cnCommands = @(
    "cn_patch::cn_get_filtered_models,",
    "cn_patch::cn_get_activation_status,",
    "cn_patch::cn_submit_activation_code,",
    "cn_patch::cn_get_device_id,",
    "cn_patch::download_proxy::cn_download_model,"
)

foreach ($cmd in $cnCommands) {
    $cmdTrimmed = $cmd.TrimEnd(',')
    if ($content -notmatch [regex]::Escape($cmdTrimmed)) {
        # 找到 collect_commands! 块的第一个命令行
        $pattern = "collect_commands!\s*\["
        $m = [regex]::Match($content, $pattern)
        if ($m.Success) {
            $insertPos = $m.Index + $m.Value.Length
            $content = $content.Substring(0, $insertPos) + "`n            " + $cmd + $content.Substring($insertPos)
            $modified = $true
            Write-Host "  [+] Added: $cmd" -ForegroundColor Green
        }
    } else {
        Write-Host "  [=] Already present: $cmd" -ForegroundColor DarkGray
    }
}

# 3. 复制 custom/src-tauri/src/cn_patch/ 到 src-tauri/src/cn_patch/
$srcPatchDir = Join-Path $WorkspaceRoot "custom\src-tauri\src\cn_patch"
$dstPatchDir = Join-Path $WorkspaceRoot "src-tauri\src\cn_patch"

if (Test-Path $dstPatchDir) {
    Remove-Item $dstPatchDir -Recurse -Force
}
Copy-Item $srcPatchDir $dstPatchDir -Recurse
Write-Host "  [+] Copied cn_patch/ to src-tauri/src/" -ForegroundColor Green

# 4. 复制前端补丁文件
$srcFrontendDir = Join-Path $WorkspaceRoot "custom\src\cn_patch"
$dstFrontendDir = Join-Path $WorkspaceRoot "src\cn_patch"
if (Test-Path $dstFrontendDir) {
    Remove-Item $dstFrontendDir -Recurse -Force
}
Copy-Item $srcFrontendDir $dstFrontendDir -Recurse
Write-Host "  [+] Copied cn_patch/ to src/" -ForegroundColor Green

if ($modified) {
    $content | Set-Content $LibRs -Encoding UTF8 -NoNewline
    Write-Host "  [+] Updated lib.rs" -ForegroundColor Green
}

# 5. 在 Cargo.toml 中添加 补丁所需的依赖
$cargoToml = Join-Path $WorkspaceRoot "src-tauri\Cargo.toml"
$cargoContent = Get-Content $cargoToml -Raw -Encoding UTF8

if ($cargoContent -notmatch 'base64') {
    $cargoContent = $cargoContent -replace '(sha2 = "0\.10")', '$1' + "`n" + 'base64 = "0.22"'
    $cargoContent | Set-Content $cargoToml -Encoding UTF8 -NoNewline
    Write-Host "  [+] Added base64 dependency to Cargo.toml" -ForegroundColor Green
} else {
    Write-Host "  [=] base64 already in Cargo.toml" -ForegroundColor DarkGray
}

Write-Host "`n=== Patch applied successfully ===" -ForegroundColor Cyan
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Set CN_MIRROR_BASE env var (optional, defaults to mirror.cn.example.com)"
Write-Host "  2. Set SHIPPED_TIER env var (free/standard/premium) for build"
Write-Host "  3. Run: bun run tauri dev"
Write-Host ""
Write-Host "To revert: .\custom\patch-apply.ps1 -Revert"