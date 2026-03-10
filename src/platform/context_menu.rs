#[cfg(target_os = "windows")]
mod windows_impl {
    use winreg::enums::*;
    use winreg::RegKey;

    use crate::media::detect;

    const SHELL_KEY_NAME: &str = "ConvertWithMediaForge";

    pub fn register_context_menu() -> Result<(), Box<dyn std::error::Error>> {
        let exe_path = std::env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // Register for each supported file extension
        for ext in detect::supported_extensions() {
            let ext_key_path = format!(
                "Software\\Classes\\SystemFileAssociations\\.{ext}\\shell\\{SHELL_KEY_NAME}"
            );

            let (key, _) = hkcu.create_subkey(&ext_key_path)?;
            key.set_value("", &"Convert with MediaForge")?;
            key.set_value("Icon", &format!("\"{exe_str}\",0"))?;

            let (cmd_key, _) = hkcu.create_subkey(format!("{ext_key_path}\\command"))?;
            cmd_key.set_value("", &format!("\"{exe_str}\" --files \"%1\""))?;
        }

        // Register for directories
        let dir_key_path = format!("Software\\Classes\\Directory\\shell\\{SHELL_KEY_NAME}");
        let (dir_key, _) = hkcu.create_subkey(&dir_key_path)?;
        dir_key.set_value("", &"Convert folder with MediaForge")?;
        dir_key.set_value("Icon", &format!("\"{exe_str}\",0"))?;

        let (dir_cmd_key, _) = hkcu.create_subkey(format!("{dir_key_path}\\command"))?;
        dir_cmd_key.set_value("", &format!("\"{exe_str}\" --folder \"%1\""))?;

        Ok(())
    }

    pub fn unregister_context_menu() -> Result<(), Box<dyn std::error::Error>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        for ext in detect::supported_extensions() {
            let ext_key_path = format!(
                "Software\\Classes\\SystemFileAssociations\\.{ext}\\shell\\{SHELL_KEY_NAME}"
            );
            let _ = hkcu.delete_subkey_all(&ext_key_path);
        }

        let dir_key_path = format!("Software\\Classes\\Directory\\shell\\{SHELL_KEY_NAME}");
        let _ = hkcu.delete_subkey_all(&dir_key_path);

        Ok(())
    }

    pub fn is_registered() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let check_path = format!(
            "Software\\Classes\\SystemFileAssociations\\.png\\shell\\{SHELL_KEY_NAME}"
        );
        hkcu.open_subkey(&check_path).is_ok()
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(not(target_os = "windows"))]
mod unix_impl {
    pub fn register_context_menu() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn unregister_context_menu() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn is_registered() -> bool {
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub use unix_impl::*;
