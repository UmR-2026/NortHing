$script:OrchTmp = "C:\WINDOWS\TEMP\opencode"
$env:TMP = $script:OrchTmp
$env:TEMP = $script:OrchTmp
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
using System.Collections.Generic;

public struct RECT_W15 { public int Left; public int Top; public int Right; public int Bottom; }

public class Win32W15 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT_W15 lpRect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, int dwExtraInfo);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int v);

    public const uint MOUSEEVENTF_LEFTDOWN = 0x02;
    public const uint MOUSEEVENTF_LEFTUP = 0x04;

    public static void Click(int x, int y) {
        SetCursorPos(x, y);
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }

    public static List<IntPtr> WindowsOf(uint targetPid) {
        var list = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint pid; GetWindowThreadProcessId(h, out pid);
            if (pid == targetPid && IsWindowVisible(h)) {
                var sb = new StringBuilder(256);
                GetWindowText(h, sb, 256);
                if (sb.Length > 0) list.Add(h);
            }
            return true;
        }, IntPtr.Zero);
        return list;
    }

    public static string TitleOf(IntPtr h) {
        var sb = new StringBuilder(256);
        GetWindowText(h, sb, 256);
        return sb.ToString();
    }
}
"@

function Invoke-CDPEval {
    param(
        [string]$wsUrl,
        [string]$expression
    )
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $cts = New-Object System.Threading.CancellationTokenSource(5000)
    $ws.ConnectAsync([Uri]$wsUrl, $cts.Token).Wait()

    $payload = @{
        id = 1
        method = "Runtime.evaluate"
        params = @{
            expression = $expression
            returnByValue = $true
        }
    } | ConvertTo-Json -Depth 5

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($payload)
    $segment = [ArraySegment[byte]]::new($bytes)
    $ws.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).Wait()

    $buffer = [byte[]]::new(16384)
    $recvSegment = [ArraySegment[byte]]::new($buffer)
    $result = $ws.ReceiveAsync($recvSegment, $cts.Token).Result
    $responseJson = [System.Text.Encoding]::UTF8.GetString($buffer, 0, $result.Count)

    $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "Done", $cts.Token).Wait()
    return ($responseJson | ConvertFrom-Json)
}

function Save-WindowScreenshot {
    param([IntPtr]$hwnd, [string]$outPath)
    $r = New-Object RECT_W15
    [Win32W15]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -gt 0 -and $h -gt 0) {
        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
        $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose()
        $bmp.Dispose()
        Write-Output "Screenshot saved: $outPath (${w}x${h})"
    }
}

# 1. Clean up old instances
Write-Output "Stopping any running northhing instances..."
Get-Process northhing -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# 2. Start process
$stdoutLog = "$script:OrchTmp\northhing_stdout.log"
$stderrLog = "$script:OrchTmp\northhing_stderr.log"
Remove-Item $stdoutLog -ErrorAction SilentlyContinue
Remove-Item $stderrLog -ErrorAction SilentlyContinue

Write-Output "Starting target\debug\northhing.exe..."
$proc = Start-Process -FilePath "E:\agent-project\NortHing\target\debug\northhing.exe" -PassThru -RedirectStandardOutput $stdoutLog -RedirectStandardError $stderrLog
Write-Output "Started northhing.exe PID = $($proc.Id). Waiting 20s for UI initialization..."
Start-Sleep -Seconds 20

# 3. Check process
$p = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
if (-not $p -or $p.MainWindowHandle -eq [IntPtr]::Zero) {
    Write-Error "northhing failed to start or has no main window handle"
    Stop-Process -Id $proc.Id -Force
    exit 1
}

$mainHwnd = $p.MainWindowHandle
Write-Output "Main window handle: $mainHwnd, Title: '$($p.MainWindowTitle)', Responding: $($p.Responding)"

# Bring to foreground
[Win32W15]::ShowWindow($mainHwnd, 9) | Out-Null
[Win32W15]::SetForegroundWindow($mainHwnd) | Out-Null
Start-Sleep -Milliseconds 700

# 4. Open Archive window
Write-Output "Attempting to click '档案' (nav-archive) button..."
$clicked = $false
for ($attempt = 1; $attempt -le 3; $attempt++) {
    try {
        $targets = Invoke-RestMethod -Uri "http://127.0.0.1:9222/json" -TimeoutSec 3
        Write-Output "Found $($targets.Count) CDP targets (attempt $attempt)."
        $mainTarget = $targets | Where-Object { $_.type -eq "page" } | Select-Object -First 1
        if ($mainTarget) {
            $wsUrl = $mainTarget.webSocketDebuggerUrl
            $res = Invoke-CDPEval -wsUrl $wsUrl -expression @"
(() => {
    const btn = document.getElementById('nav-archive');
    if (btn) {
        btn.click();
        return 'nav-archive clicked via CDP';
    }
    return 'nav-archive not found';
})()
"@
            Write-Output "CDP Eval result: $($res.result.result.value)"
            if ($res.result.result.value -match "clicked") {
                $clicked = $true
                break
            }
        }
    } catch {
        Write-Output "CDP click attempt $attempt warning: $_"
        Start-Sleep -Seconds 2
    }
}

if (-not $clicked) {
    Write-Output "Fallback: clicking nav-archive by physical coordinates..."
    $r = New-Object RECT_W15
    [Win32W15]::GetWindowRect($mainHwnd, [ref]$r) | Out-Null
    $navX = $r.Left + 330
    $navY = $r.Top + 26
    [Win32W15]::Click($navX, $navY)
    Write-Output "Clicked ($navX, $navY)"
}

Write-Output "Waiting 5s for Archive window to open..."
Start-Sleep -Seconds 5

# 5. Observe for 60s
Write-Output "Observing for 60s (Archive loading & responding status)..."
$archiveHwnd = [IntPtr]::Zero
for ($sec = 0; $sec -le 60; $sec += 10) {
    $cur = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if (-not $cur) {
        Write-Output "t=${sec}s: Process exited!"
        break
    }
    $wins = [Win32W15]::WindowsOf([uint32]$proc.Id)
    $winInfo = @()
    foreach ($w in $wins) {
        $t = [Win32W15]::TitleOf($w)
        $hung = [Win32W15]::IsHungAppWindow($w)
        $winInfo += "'$t' (hung=$hung)"
        if ($t -match "档案|archive") {
            $archiveHwnd = $w
        }
    }
    Write-Output "t=${sec}s: Responding=$($cur.Responding), CPU=$($cur.TotalProcessorTime), Windows=[$($winInfo -join ', ')]"
    if ($sec -lt 60) {
        Start-Sleep -Seconds 10
    }
}

# 6. Check Archive content via CDP if available
if ($archiveHwnd -ne [IntPtr]::Zero) {
    Write-Output "Archive window found with HWND $archiveHwnd"
    [Win32W15]::SetForegroundWindow($archiveHwnd) | Out-Null
    Start-Sleep -Milliseconds 600
    try {
        $targets = Invoke-RestMethod -Uri "http://127.0.0.1:9222/json" -TimeoutSec 3
        foreach ($t in $targets) {
            Write-Output "CDP Target: title='$($t.title)' url='$($t.url)'"
        }
        $archiveTarget = $targets | Where-Object { $_.type -eq "page" -and $_.id -ne $mainTarget.id } | Select-Object -First 1
        if (-not $archiveTarget) {
            $archiveTarget = $targets | Where-Object { $_.title -match "档案|archive" } | Select-Object -First 1
        }
        if ($archiveTarget) {
            $wsUrl = $archiveTarget.webSocketDebuggerUrl
            $archCheck = Invoke-CDPEval -wsUrl $wsUrl -expression @"
(() => {
    const strata = document.querySelectorAll('.stratum');
    const rows = document.querySelectorAll('.row');
    const loading = document.body.innerText.includes('加载中');
    return JSON.stringify({
        strataCount: strata.length,
        rowCount: rows.length,
        hasLoading: loading,
        bodySnippet: document.body.innerText.slice(0, 200)
    });
})()
"@
            Write-Output "Archive CDP DOM inspection: $($archCheck.result.result.value)"
        }
    } catch {
        Write-Output "Archive CDP inspection warning: $_"
    }
} else {
    Write-Output "Warning: Archive window not detected among visible windows."
}

# 7. Bring main window to foreground and send "ping"
Write-Output "Switching focus back to main window..."
[Win32W15]::ShowWindow($mainHwnd, 9) | Out-Null
[Win32W15]::SetForegroundWindow($mainHwnd) | Out-Null
Start-Sleep -Milliseconds 800

$r = New-Object RECT_W15
[Win32W15]::GetWindowRect($mainHwnd, [ref]$r) | Out-Null
$inputX = $r.Left + 500
$inputY = $r.Bottom - 45
Write-Output "Focusing input box at ($inputX, $inputY)..."
[Win32W15]::Click($inputX, $inputY)
Start-Sleep -Milliseconds 400

Write-Output "Typing 'ping'..."
[System.Windows.Forms.SendKeys]::SendWait("ping")
Start-Sleep -Milliseconds 500

$sendBtnX = $r.Right - 55
$sendBtnY = $r.Bottom - 45
Write-Output "Clicking send button at ($sendBtnX, $sendBtnY)..."
[Win32W15]::Click($sendBtnX, $sendBtnY)
Start-Sleep -Milliseconds 500

# 8. Observe for 30s after send
Write-Output "Observing 30s post-send..."
for ($sec = 0; $sec -le 30; $sec += 10) {
    $cur = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if (-not $cur) {
        Write-Output "post-send t=${sec}s: Process exited!"
        break
    }
    $wins = [Win32W15]::WindowsOf([uint32]$proc.Id)
    $winInfo = @()
    foreach ($w in $wins) {
        $t = [Win32W15]::TitleOf($w)
        $hung = [Win32W15]::IsHungAppWindow($w)
        $winInfo += "'$t' (hung=$hung)"
    }
    Write-Output "post-send t=${sec}s: Responding=$($cur.Responding), CPU=$($cur.TotalProcessorTime), Windows=[$($winInfo -join ', ')]"
    if ($sec -lt 30) {
        Start-Sleep -Seconds 10
    }
}

# 9. Take screenshots
New-Item -ItemType Directory -Force "E:\agent-project\NortHing\screenshots" | Out-Null
$mainShot = "E:\agent-project\NortHing\screenshots\w15-1l-main.png"
$archShot = "E:\agent-project\NortHing\screenshots\w15-1l-archive.png"

# Screenshot archive window
if ($archiveHwnd -ne [IntPtr]::Zero) {
    [Win32W15]::ShowWindow($archiveHwnd, 9) | Out-Null
    [Win32W15]::SetForegroundWindow($archiveHwnd) | Out-Null
    Start-Sleep -Milliseconds 600
    Save-WindowScreenshot -hwnd $archiveHwnd -outPath $archShot
}

# Screenshot main window
[Win32W15]::ShowWindow($mainHwnd, 9) | Out-Null
[Win32W15]::SetForegroundWindow($mainHwnd) | Out-Null
Start-Sleep -Milliseconds 600
Save-WindowScreenshot -hwnd $mainHwnd -outPath $mainShot

# 10. Clean up
Write-Output "Stopping northhing process..."
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Write-Output "Runtime verification completed."
