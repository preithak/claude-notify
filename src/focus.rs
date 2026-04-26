use anyhow::Result;
use std::process::Command;

#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, IsWindowVisible, SetForegroundWindow,
};

pub fn run(target: Option<&str>) -> Result<()> {
    bring_terminal_forward();
    if let Some(t) = target {
        if let Some((session, window)) = parse_tmux_target(t) {
            let _ = Command::new("wsl.exe")
                .args([
                    "-e",
                    "tmux",
                    "select-window",
                    "-t",
                    &format!("{session}:{window}"),
                ])
                .spawn();
        }
    }
    Ok(())
}

/// Accepts `tmux=session:window` or `claude-notify:tmux=session:window`.
/// Window defaults to "0" if unspecified. Returns None on malformed input.
fn parse_tmux_target(s: &str) -> Option<(String, String)> {
    let s = s.strip_prefix("claude-notify:").unwrap_or(s);
    let rest = s.strip_prefix("tmux=")?;
    let mut parts = rest.splitn(2, ':');
    let session = parts.next()?.to_string();
    let window = parts.next().unwrap_or("0").to_string();
    if session.is_empty() {
        return None;
    }
    Some((session, window))
}

#[cfg(windows)]
fn bring_terminal_forward() {
    let mut hwnd = HWND::default();
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut hwnd as *mut _ as isize));
        if !hwnd.0.is_null() {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(windows))]
fn bring_terminal_forward() {}

#[cfg(windows)]
unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let mut buf = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut buf) as usize;
    if len == 0 {
        return BOOL(1);
    }
    let class = String::from_utf16_lossy(&buf[..len]);
    if class.contains("CASCADIA") {
        let out = lparam.0 as *mut HWND;
        *out = hwnd;
        return BOOL(0);
    }
    BOOL(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_target() {
        assert_eq!(
            parse_tmux_target("tmux=c1:0"),
            Some(("c1".into(), "0".into()))
        );
    }

    #[test]
    fn parses_uri_prefixed_target() {
        assert_eq!(
            parse_tmux_target("claude-notify:tmux=c1:claude"),
            Some(("c1".into(), "claude".into()))
        );
    }

    #[test]
    fn rejects_empty_session() {
        assert_eq!(parse_tmux_target("tmux=:0"), None);
    }

    #[test]
    fn defaults_window_to_zero() {
        assert_eq!(
            parse_tmux_target("tmux=c1"),
            Some(("c1".into(), "0".into()))
        );
    }

    #[test]
    fn rejects_unknown_scheme() {
        assert_eq!(parse_tmux_target("vim=foo"), None);
    }
}
