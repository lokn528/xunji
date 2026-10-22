//! Windows 右键「发送到 循迹」菜单。
//!
//! 通过写注册表实现，免额外常驻进程。安装与卸载都只影响 `HKCU`，
//! 不动系统级 `HKLM`，不需要管理员权限。

/// 菜单显示名。
pub const MENU_LABEL: &str = "发送到 循迹";

/// 菜单的内部标识（注册表子键名，避免中文编码问题）。
const VERB_KEY: &str = "Xunji.SendTo";

/// 被作用的类根键。
const CLASSES: &str = r"Software\Classes";

/// 需要注册菜单的类键：`*` 为所有文件，`Directory` 为文件夹。
const TARGETS: [&str; 2] = ["*", "Directory"];

/// 安装右键菜单。`exe_path` 为循迹可执行文件的完整路径。
#[cfg(windows)]
pub fn install(exe_path: &str) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for target in TARGETS {
        let class = hkcu
            .open_subkey_with_flags(CLASSES, KEY_WRITE)
            .map_err(|e| format!("打开 {CLASSES} 失败：{e}"))?;
        // `create_subkey` 返回 `(RegKey, RegDisposition)`，取其中的 RegKey。
        let (verb, _) = class
            .create_subkey(format!("{target}\\shell\\{VERB_KEY}"))
            .map_err(|e| format!("创建 {target} 菜单项失败：{e}"))?;
        verb.set_value("MUIVerb", &MENU_LABEL)
            .map_err(|e| format!("写入菜单名失败：{e}"))?;
        verb.set_value("", &MENU_LABEL)
            .map_err(|e| format!("写入菜单名失败：{e}"))?;

        let (cmd, _) = verb
            .create_subkey("command")
            .map_err(|e| format!("创建 command 子键失败：{e}"))?;
        cmd.set_value("", &format!("\"{exe_path}\" \"%1\""))
            .map_err(|e| format!("写入命令失败：{e}"))?;
    }
    Ok(())
}

/// 卸载右键菜单。
#[cfg(windows)]
pub fn uninstall() -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for target in TARGETS {
        let class = match hkcu.open_subkey_with_flags(CLASSES, KEY_WRITE) {
            Ok(k) => k,
            Err(_) => continue,
        };
        let _ = class.delete_subkey_all(format!("{target}\\shell\\{VERB_KEY}"));
    }
    Ok(())
}

/// 非 Windows 平台返回不支持。
#[cfg(not(windows))]
pub fn install(_exe_path: &str) -> Result<(), String> {
    Err("右键菜单仅支持 Windows".into())
}

/// 非 Windows 平台返回不支持。
#[cfg(not(windows))]
pub fn uninstall() -> Result<(), String> {
    Err("右键菜单仅支持 Windows".into())
}

/// 安装右键菜单（供设置页调用）。
#[tauri::command]
pub fn install_shell_menu() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("找不到程序路径：{e}"))?
        .to_string_lossy()
        .to_string();
    install(&exe)?;
    Ok(format!("已注册「{MENU_LABEL}」"))
}

/// 卸载右键菜单（供设置页调用）。
#[tauri::command]
pub fn uninstall_shell_menu() -> Result<String, String> {
    uninstall()?;
    Ok(format!("已移除「{MENU_LABEL}」"))
}

/// 查询右键菜单是否已注册。
#[tauri::command]
pub fn shell_menu_installed() -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(CLASSES)
            .and_then(|c| c.open_subkey(format!("*\\shell\\{VERB_KEY}")))
            .is_ok()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 菜单名非空且含循迹() {
        assert!(MENU_LABEL.contains("循迹"));
        assert!(!MENU_LABEL.is_empty());
    }

    #[test]
    fn 内部标识为纯_ascii() {
        assert!(VERB_KEY.is_ascii(), "注册表子键名应避免中文编码问题");
    }

    #[test]
    fn 目标包含文件与文件夹() {
        assert!(TARGETS.contains(&"*"));
        assert!(TARGETS.contains(&"Directory"));
    }
}
