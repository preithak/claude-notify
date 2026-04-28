use anyhow::Result;
use std::process::Command;

#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, IsWindowVisible, SetForegroundWindow,
};

fn debug_log(msg: &str) {
    if let Ok(p) = std::env::var("CLAUDE_NOTIFY_DEBUG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "[ts={ts}] [focus] {msg}");
        }
    }
}

pub fn run(target: Option<&str>) -> Result<()> {
    debug_log(&format!("invoked, target={target:?}"));
    bring_terminal_forward();
    if let Some(t) = target {
        if let Some((session, window)) = parse_tmux_target(t) {
            debug_log(&format!("spawning wsl.exe tmux select-window -t {session}:{window}"));
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
        if hwnd.0.is_null() {
            debug_log("EnumWindows found NO CASCADIA window");
            return;
        }
        // Read the title for diagnosis
        let mut tb = [0u16; 512];
        let tl = GetWindowTextW(hwnd, &mut tb) as usize;
        let title = String::from_utf16_lossy(&tb[..tl]);
        debug_log(&format!("found WT hwnd={:?} title={title:?}", hwnd.0));
        let result = SetForegroundWindow(hwnd);
        debug_log(&format!("SetForegroundWindow returned {:?}", result));
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
