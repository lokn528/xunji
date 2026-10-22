<#
.SYNOPSIS
    截取桌面或指定窗口的画面，用于自查 UI。

.EXAMPLE
    .\scripts\screenshot.ps1 -OutPath E:\ttmp\shot.png

.EXAMPLE
    .\scripts\screenshot.ps1 -WindowTitle "循迹" -OutPath E:\ttmp\shot.png
#>

[CmdletBinding()]
param(
    [string]$OutPath = "E:\ttmp\shot.png",
    [string]$WindowTitle,
    [string]$ProcessName
)

Add-Type -AssemblyName System.Windows.Forms, System.Drawing

# 必须先声明 per-monitor DPI 感知：否则 GetWindowRect 返回的是逻辑坐标，
# 而 CopyFromScreen 接受物理坐标，两者混用会抓错区域（表现为"右边被裁掉"）。
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Dpi {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public static void Enable() {
    // -4 = PER_MONITOR_AWARE_V2
    try { if (SetProcessDpiAwarenessContext(new IntPtr(-4))) return; } catch {}
    try { SetProcessDPIAware(); } catch {}
  }
}
"@
[Dpi]::Enable()

if ($WindowTitle) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;
public class Win {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public struct RECT { public int Left, Top, Right, Bottom; }

  // 找出该进程下标题匹配且可见的窗口。
  // 不能用 Process.MainWindowTitle：Tauri 会先创建一个空标题的辅助窗口，
  // .NET 会挑中它，导致找不到真正的窗口。
  public static IntPtr FindWindow(uint target, string titleMatch, string procName) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid != target) return true;
      if (!IsWindowVisible(h)) return true;
      int len = GetWindowTextLength(h);
      if (len == 0) return true;
      var sb = new StringBuilder(len + 1);
      GetWindowText(h, sb, sb.Capacity);
      if (sb.ToString().Contains(titleMatch)) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@

    $candidates = Get-Process | Where-Object { $_.ProcessName -eq $ProcessName }
    if (-not $candidates) { throw "找不到进程 $ProcessName" }
    $hwnd = [IntPtr]::Zero
    foreach ($p in $candidates) {
        $hwnd = [Win]::FindWindow([uint32]$p.Id, $WindowTitle, $ProcessName)
        if ($hwnd -ne [IntPtr]::Zero) { break }
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw "找不到标题含「$WindowTitle」的窗口" }
    [void][Win]::ShowWindow($hwnd, 9)   # SW_RESTORE
    [void][Win]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 700
    $r = New-Object Win+RECT
    [void][Win]::GetWindowRect($hwnd, [ref]$r)
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    # 用屏幕抓取而不是 PrintWindow：WebView2 的合成表面在 PrintWindow 下
    # 往往只渲染出一部分，会让人误判成"布局被裁切"。
    # 抓屏前先把窗口提到最前，抓完再放回去。
    $topmost = [IntPtr](-1)
    $notop = [IntPtr](-2)
    [void][Win]::SetWindowPos($hwnd, $topmost, 0, 0, 0, 0, 0x0003)  # SWP_NOMOVE|SWP_NOSIZE
    [void][Win]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 600
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    [void][Win]::SetWindowPos($hwnd, $notop, 0, 0, 0, 0, 0x0003)
}
else {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
}

$dir = Split-Path -Parent $OutPath
if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Host "已保存：$OutPath"
