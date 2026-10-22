<#
.SYNOPSIS
    注册或移除资源管理器右键菜单「发送到 循迹」。

.DESCRIPTION
    在 HKCU\Software\Classes 下为 `*`（所有文件）与 `Directory`（文件夹）
    写入 shell 菜单项。免额外常驻进程，不需要管理员权限。

    用 .NET 的注册表 API 而不是 PowerShell 的 New-Item：后者会把路径里的
    `*` 当成通配符，导致解析卡死或写错键。

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

function Get-ClassesKey {
    return [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Software\Classes')
}

if ($Uninstall) {
    $classes = Get-ClassesKey
    try {
        foreach ($target in $Targets) {
            $shell = $classes.CreateSubKey("$target\shell")
            if ($shell) {
                if ($shell.GetSubKeyNames() -contains $VerbKey) {
                    $shell.DeleteSubKeyTree($VerbKey)
                    Write-Host "已移除：$target" -ForegroundColor Yellow
                }
                $shell.Close()
            }
        }
    }
    finally { $classes.Close() }
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

$classes = Get-ClassesKey
try {
    foreach ($target in $Targets) {
        $verb = $classes.CreateSubKey("$target\shell\$VerbKey")
        if (-not $verb) { throw "创建 $target 菜单项失败" }
        $verb.SetValue('MUIVerb', $Label, [Microsoft.Win32.RegistryValueKind]::String)
        $verb.SetValue('', $Label, [Microsoft.Win32.RegistryValueKind]::String)
        $verb.Close()

        $cmd = $classes.CreateSubKey("$target\shell\$VerbKey\command")
        if (-not $cmd) { throw "创建 $target command 子键失败" }
        $cmd.SetValue('', "`"$ExePath`" `"%1`"", [Microsoft.Win32.RegistryValueKind]::String)
        $cmd.Close()

        Write-Host "已注册：$target" -ForegroundColor Green
    }
}
finally { $classes.Close() }

Write-Host ""
Write-Host "右键菜单「$Label」已就绪。" -ForegroundColor Green
Write-Host "在资源管理器里右键文件或文件夹 → 发送到 → $Label 即可。"
