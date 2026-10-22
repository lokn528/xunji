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

if ($WindowTitle) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

    $candidates = Get-Process | Where-Object { $_.MainWindowTitle -like "*$WindowTitle*" }
    if ($ProcessName) {
        $candidates = $candidates | Where-Object { $_.ProcessName -eq $ProcessName }
    }
    $proc = $candidates | Select-Object -First 1
    if (-not $proc) { throw "找不到标题含「$WindowTitle」的窗口" }
    [void][Win]::ShowWindow($proc.MainWindowHandle, 9)   # SW_RESTORE
    [void][Win]::SetForegroundWindow($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 700
    $r = New-Object Win+RECT
    [void][Win]::GetWindowRect($proc.MainWindowHandle, [ref]$r)
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    # PrintWindow 直接取窗口内容，不受其他窗口遮挡影响
    $hdc = $g.GetHdc()
    [void][Win]::PrintWindow($proc.MainWindowHandle, $hdc, 2)
    $g.ReleaseHdc($hdc)
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
