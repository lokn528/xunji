<#
.SYNOPSIS
    在指定窗口内按屏幕坐标点击/移动，用于自测窗口按钮、拖拽等交互。

.EXAMPLE
    .\scripts\click.ps1 -ProcessName xunji -RelX 1470 -RelY 25
#>

[CmdletBinding()]
param(
    [string]$ProcessName,
    [int]$RelX = 0,
    [int]$RelY = 0,
    [int]$AbsX = -1,
    [int]$AbsY = -1,
    [switch]$DoubleClick,
    [switch]$Drag,
    [switch]$MoveOnly,
    [int]$DragToX = -1,
    [int]$DragToY = -1
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Dpi {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
  public static void Enable() {
    try { SetProcessDpiAwarenessContext(new IntPtr(-4)); } catch {}
  }
}
"@
[Dpi]::Enable()

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Win {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;

  // 按进程找可见且有标题的窗口。Process.MainWindowHandle 会挑中 Tauri 的
  // 空标题辅助窗口，坐标全错。
  public static IntPtr Find(uint target) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid != target) return true;
      if (!IsWindowVisible(h)) return true;
      int len = GetWindowTextLength(h);
      if (len == 0) return true;
      var sb = new StringBuilder(len + 1);
      GetWindowText(h, sb, sb.Capacity);
      if (sb.Length > 0) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@

$proc = Get-Process -Name $ProcessName -ErrorAction Stop | Select-Object -First 1
$hwnd = [Win]::Find([uint32]$proc.Id)
if ($hwnd -eq [IntPtr]::Zero) { throw "找不到主窗口句柄" }

$r = New-Object Win+RECT
[void][Win]::GetWindowRect($hwnd, [ref]$r)

# 保持当前显示状态，别把最大化窗口还原掉
if ([Win]::IsZoomed($hwnd)) { [void][Win]::ShowWindow($hwnd, 3) }   # SW_MAXIMIZE
else { [void][Win]::ShowWindow($hwnd, 9) }                            # SW_RESTORE
[void][Win]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400

if ($AbsX -ge 0) {
    $x = $AbsX
    $y = $AbsY
} else {
    $x = $r.Left + $RelX
    $y = $r.Top + $RelY
}

Write-Host "窗口 rect: $($r.Left),$($r.Top) - $($r.Right),$($r.Bottom)"
Write-Host "点击于: $x,$y"

[void][Win]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 150

if ($MoveOnly) {
    exit 0
}

if ($Drag) {
    if ($DragToX -lt 0) { $DragToX = $x + 120 }
    if ($DragToY -lt 0) { $DragToY = $y + 80 }
    [Win]::mouse_event([Win]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 120
    [void][Win]::SetCursorPos($DragToX, $DragToY)
    Start-Sleep -Milliseconds 120
    [Win]::mouse_event([Win]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 300
    exit 0
}

$clicks = if ($DoubleClick) { 2 } else { 1 }
for ($i = 0; $i -lt $clicks; $i++) {
    [Win]::mouse_event([Win]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [Win]::mouse_event([Win]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
    if ($DoubleClick) { Start-Sleep -Milliseconds 60 }
}
Start-Sleep -Milliseconds 400
