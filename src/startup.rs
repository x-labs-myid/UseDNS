#[cfg(windows)]
fn startup_command() -> Result<Vec<u16>, String> {
    use std::os::windows::ffi::OsStrExt;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = vec![b'"' as u16];
    command.extend(executable.as_os_str().encode_wide());
    command.extend("\" --background".encode_utf16());
    command.push(0);
    if command.len() > 261 {
        return Err(
            "The executable path is too long for Windows startup. Move UseDNS to a shorter path."
                .into(),
        );
    }
    Ok(command)
}

#[cfg(windows)]
pub fn is_enabled() -> Result<bool, String> {
    use windows::{
        Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW},
        },
        core::w,
    };
    let mut length = 0;
    // SAFETY: This first call queries the buffer size only.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            w!("UseDNS"),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut length),
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    status
        .ok()
        .map_err(|e| format!("Could not read Windows startup settings: {e}"))?;
    let mut value = vec![0u16; (length as usize).div_ceil(2)];
    // SAFETY: value is large enough for the registry data, measured above.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            w!("UseDNS"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut length),
        )
    }
    .ok()
    .map_err(|e| format!("Could not read Windows startup settings: {e}"))?;
    value.truncate((length as usize).div_ceil(2));
    Ok(value == startup_command()?)
}

#[cfg(windows)]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    use windows::{
        Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteValueW,
                RegSetValueExW,
            },
        },
        core::w,
    };

    let mut key = HKEY::default();
    // SAFETY: The key handle is initialized by Windows and closed on every path below.
    unsafe {
        RegCreateKeyW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            &mut key,
        )
        .ok()
        .map_err(|e| format!("Could not open Windows startup settings: {e}"))?;
    }
    let result = if enabled {
        startup_command().and_then(|command| {
            let bytes = command
                .iter()
                .flat_map(|c| c.to_le_bytes())
                .collect::<Vec<_>>();
            // SAFETY: bytes contains the UTF-16, NUL-terminated registry string.
            unsafe { RegSetValueExW(key, w!("UseDNS"), None, REG_SZ, Some(&bytes)) }
                .ok()
                .map_err(|e| format!("Could not enable Windows startup: {e}"))
        })
    } else {
        // SAFETY: key is valid; only this application's named startup value is removed.
        let status = unsafe { RegDeleteValueW(key, w!("UseDNS")) };
        if status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            status
                .ok()
                .map_err(|e| format!("Could not disable Windows startup: {e}"))
        }
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> Result<(), String> {
    Err("Windows startup is supported on Windows only.".into())
}
