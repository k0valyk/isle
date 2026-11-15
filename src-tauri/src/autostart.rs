pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";

    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(key_path)
        .map_err(|error| error.to_string())?;

    if enabled {
        let value = format!("\"{}\"", exe.display());
        key.set_value("Isle", &value)
            .map_err(|error| error.to_string())
    } else {
        match key.delete_value("Isle") {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[allow(dead_code)]
pub fn installed() -> bool {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
        Ok(key) => key,
        Err(_) => return false,
    };
    key.get_value::<String, _>("Isle").is_ok()
}
