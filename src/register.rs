use anyhow::{Context, Result};
use std::env;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::ERROR_SUCCESS;
#[cfg(windows)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};

pub const APP_ID: &str = "ClaudeCode.Notify";
pub const PROTOCOL: &str = "claude-notify";

pub fn run() -> Result<()> {
    let exe = env::current_exe().context("could not determine current exe path")?;
    let exe_str = exe.to_string_lossy().into_owned();
    let command = format!("\"{}\" focus --target \"%1\"", exe_str);

    register_appid()?;
    register_protocol(&command)?;

    eprintln!("Registered AppID '{APP_ID}' and protocol '{PROTOCOL}://'.");
    eprintln!("Click handler: {command}");
    Ok(())
}

#[cfg(not(windows))]
fn register_appid() -> Result<()> {
    anyhow::bail!("registration is only supported on Windows builds")
}

#[cfg(not(windows))]
fn register_protocol(_: &str) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn register_appid() -> Result<()> {
    write_string(
        HKEY_CURRENT_USER,
        &format!("Software\\Classes\\AppUserModelId\\{APP_ID}"),
        Some("DisplayName"),
        "Claude Code",
    )
}

#[cfg(windows)]
fn register_protocol(command: &str) -> Result<()> {
    let root = format!("Software\\Classes\\{PROTOCOL}");
    write_string(HKEY_CURRENT_USER, &root, None, "URL:claude-notify")?;
    write_string(HKEY_CURRENT_USER, &root, Some("URL Protocol"), "")?;
    write_string(
        HKEY_CURRENT_USER,
        &format!("{root}\\shell\\open\\command"),
        None,
        command,
    )?;
    Ok(())
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn write_string(parent: HKEY, subkey: &str, value_name: Option<&str>, data: &str) -> Result<()> {
    let subkey_w = to_wide(subkey);
    let mut hkey = HKEY::default();
    unsafe {
        let res = RegCreateKeyExW(
            parent,
            PCWSTR(subkey_w.as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        );
        if res != ERROR_SUCCESS {
            anyhow::bail!("RegCreateKeyExW failed for HKCU\\{subkey}: {res:?}");
        }

        let name_w = value_name.map(to_wide);
        let name_ptr = name_w
            .as_ref()
            .map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));
        let data_w = to_wide(data);
        let bytes = data_w.len() * std::mem::size_of::<u16>();
        let data_slice = std::slice::from_raw_parts(data_w.as_ptr() as *const u8, bytes);

        let res = RegSetValueExW(hkey, name_ptr, 0, REG_SZ, Some(data_slice));
        let _ = RegCloseKey(hkey);
        if res != ERROR_SUCCESS {
            anyhow::bail!(
                "RegSetValueExW failed for HKCU\\{subkey} value {value_name:?}: {res:?}"
            );
        }
    }
    Ok(())
}
