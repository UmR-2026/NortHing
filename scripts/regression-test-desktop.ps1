#!/usr/bin/env pwsh
# Regression test for northhing desktop (Slint shell)
# Run after any task completion to ensure build integrity

$ErrorActionPreference = "Stop"
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path "$scriptPath\.."

# Ensure MSYS2 dlltool is in PATH for Windows builds
$msysPath = "C:\msys64\mingw64\bin"
if (Test-Path $msysPath) {
    $env:PATH = "$msysPath;$env:PATH"
}

Write-Host "========================================"
Write-Host "Regression Test: northhing Desktop Shell"
Write-Host "========================================"
Write-Host ""

$checksPassed = 0
$checksFailed = 0

function Check-Step {
    param($Name, $ScriptBlock)
    Write-Host "[CHECK] $Name ..." -NoNewline
    try {
        & $ScriptBlock | Out-Null
        Write-Host " OK" -ForegroundColor Green
        $script:checksPassed++
    } catch {
        Write-Host " FAIL" -ForegroundColor Red
        Write-Host "  Error: $_" -ForegroundColor Red
        $script:checksFailed++
    }
}

# 1. Desktop app compiles cleanly (zero warnings)
Check-Step "Desktop app compiles cleanly" {
    $output = & cargo build -p northhing 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed with exit code $LASTEXITCODE. Output: $output"
    }
    # Check for warnings in northhing crate (excluding slint_build warnings)
    $warnLines = $output | Select-String "^warning:.*northhing" | Where-Object { $_ -notmatch "slint_build" }
    if ($warnLines) {
        throw "Build has warnings: $($warnLines -join "; ")"
    }
}

# 2. Desktop app release build succeeds
Check-Step "Desktop app release build" {
    & cargo build -p northhing --release 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Release build failed with exit code $LASTEXITCODE"
    }
}

# 3. All workspace crates compile
Check-Step "Full workspace check" {
    & cargo check --workspace 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Workspace check failed with exit code $LASTEXITCODE"
    }
}

# 4. Transport adapter with slint feature compiles
Check-Step "Transport adapter (slint feature)" {
    & cargo check -p northhing-transport --features slint-adapter 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Transport adapter slint feature check failed with exit code $LASTEXITCODE"
    }
}

# 5. Binary exists
Check-Step "northhing binary exists" {
    $binPath = "$projectRoot\target\debug\northhing.exe"
    if (-not (Test-Path $binPath)) {
        throw "Binary not found at $binPath"
    }
}

# 6. Slint UI files are present
Check-Step "Slint UI files present" {
    $uiFiles = @(
        "src/apps/desktop/src/ui/main.slint",
        "src/apps/desktop/src/ui/theme.slint",
        "src/apps/desktop/src/ui/components/MaterialButton.slint",
        "src/apps/desktop/src/ui/components/MaterialCard.slint",
        "src/apps/desktop/src/ui/components/MaterialIconButton.slint",
        "src/apps/desktop/src/ui/components/MaterialTextField.slint",
        "src/apps/desktop/src/ui/components/MaterialBadge.slint",
        "src/apps/desktop/src/ui/components/MaterialList.slint",
        "src/apps/desktop/src/ui/components/ChatMessageBubble.slint",
        "src/apps/desktop/src/ui/components/CodeBlock.slint",
        "src/apps/desktop/src/ui/components/MarkdownText.slint",
        "src/apps/desktop/src/ui/components/ToolCallCard.slint",
        "src/apps/desktop/src/ui/views/SidebarView.slint",
        "src/apps/desktop/src/ui/views/ChatPaneView.slint",
        "src/apps/desktop/src/ui/views/InspectorView.slint",
        "src/apps/desktop/src/ui/views/StatusBarView.slint"
    )
    foreach ($f in $uiFiles) {
        $path = Join-Path $projectRoot $f
        if (-not (Test-Path $path)) {
            throw "Missing UI file: $f"
        }
    }
}

# 7. Desktop Cargo.toml has correct dependencies
Check-Step "Desktop dependencies valid" {
    & cargo check -p northhing --message-format=short 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Dependency check failed with exit code $LASTEXITCODE"
    }
}

Write-Host ""
Write-Host "========================================"
Write-Host "Results: $checksPassed passed, $checksFailed failed"
Write-Host "========================================"

if ($checksFailed -gt 0) {
    Write-Host "REGRESSION TEST FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "REGRESSION TEST PASSED" -ForegroundColor Green
    exit 0
}
