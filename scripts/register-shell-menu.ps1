<#
.SYNOPSIS
    注册或移除资源管理器右键菜单「发送到 循迹」。

.DESCRIPTION
    在 HKCU\Software\Classes 下为 `*`（所有文件）与 `Directory`（文件夹）
    写入 shell 菜单项。免额外常驻进程，不需要管理员权限。

    卸载：把 -Uninstall 传给脚本。

.EXAMPLE
    .\scripts\register-shell-menu.ps1 -ExePath "C:\Apps\循迹\xunji.exe"

.EXAMPLE
    .\scripts\register-shell-menu.ps1 -Uninstall
#>

[CmdletBinding()]
param(
    # 循迹可执行文件的完整路径
    [string]$ExePath,

    # 移除菜单项
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$VerbKey = 'Xunji.SendTo'
$Label = '发送到 循迹'
$Targets = @('*', 'Directory')

if ($Uninstall) {
    foreach ($target in $Targets) {
        $path = "HKCU:\Software\Classes\$target\shell\$VerbKey"
        if (Test-Path $path) {
            Remove-Item -LiteralPath $path -Recurse -Force
            Write-Host "已移除：$target" -ForegroundColor Yellow
        }
    }
    Write-Host "右键菜单「$Label」已移除。" -ForegroundColor Green
    return
}

if (-not $ExePath) {
    Write-Error "请用 -ExePath 指定循迹可执行文件的完整路径，或用 -Uninstall 移除菜单。"
}

if (-not (Test-Path -LiteralPath $ExePath)) {
    Write-Error "找不到文件：$ExePath"
}

$ExePath = (Resolve-Path -LiteralPath $ExePath).Path

foreach ($target in $Targets) {
    $key = "HKCU:\Software\Classes\$target\shell\$VerbKey"
    New-Item -Path $key -Force | Out-Null
    New-ItemProperty -Path $key -Name 'MUIVerb' -Value $Label -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $key -Name '' -Value $Label -PropertyType String -Force | Out-Null

    $cmdKey = "$key\command"
    New-Item -Path $cmdKey -Force | Out-Null
    New-ItemProperty -Path $cmdKey -Name '' -Value "`"$ExePath`" `"%1`"" -PropertyType String -Force | Out-Null

    Write-Host "已注册：$target" -ForegroundColor Green
}

Write-Host ""
Write-Host "右键菜单「$Label」已就绪。" -ForegroundColor Green
Write-Host "在资源管理器里右键文件或文件夹 → 发送到 → $Label 即可。"
