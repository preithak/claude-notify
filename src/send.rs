use anyhow::{Context, Result};

#[cfg(windows)]
use windows::core::HSTRING;
#[cfg(windows)]
use windows::Data::Xml::Dom::XmlDocument;
#[cfg(windows)]
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

pub const APP_ID: &str = "ClaudeCode.Notify";

pub fn run(
    title: &str,
    body: &str,
    sound: Option<&str>,
    click: Option<&str>,
    skip_if_title: Option<&str>,
) -> Result<()> {
    if let Some(t) = skip_if_title {
        if should_skip(t) {
            return Ok(());
        }
    }

    // file:// audio in toast XML is silently dropped for unpackaged apps with custom AppIDs,
    // so we play the WAV ourselves via PlaySound and emit a silent toast (no default ding).
    let (toast_audio, play_path) = match sound {
        Some(s) if s.starts_with("file://") => (None, file_url_to_local_path(s)),
        other => (other, None),
    };

    let xml_str = build_xml(title, body, toast_audio, click, play_path.is_some());
    show(&xml_str)?;

    if let Some(path) = play_path.as_deref() {
        play_sync(path);
    }
    Ok(())
}

/// Suppress the toast when the foreground window is Windows Terminal AND its
/// title contains the expected tab name AND the cursor is on the same monitor
/// as that window. The cursor-monitor check is a cheap mitigation for the
/// "WT held abandoned focus on a different monitor" case in multi-monitor
/// setups (see README).
#[cfg(windows)]
fn should_skip(expected_title: &str) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowTextW,
    };

    if std::env::var_os("CLAUDE_NOTIFY_ALWAYS").is_some() {
        return false;
    }
    // Debug logging when CLAUDE_NOTIFY_DEBUG is set to a Windows-style path
    // (e.g. C:\Users\me\AppData\Local\Temp\notify.log).
    let debug_path = std::env::var("CLAUDE_NOTIFY_DEBUG").ok();
    let log = |msg: &str| {
        if let Some(p) = &debug_path {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let _ = writeln!(f, "{msg}");
            }
        }
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            log("[skip-debug] no foreground window -> fire");
            return false;
        }
        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf) as usize;
        let class = String::from_utf16_lossy(&class_buf[..class_len]);
        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf) as usize;
        let title = String::from_utf16_lossy(&title_buf[..title_len]);
        log(&format!("[skip-debug] expected={expected_title:?} class={class:?} title={title:?}"));

        if class_len == 0 || !class.contains("CASCADIA") {
            log("[skip-debug] class not CASCADIA -> fire");
            return false;
        }
        if title_len == 0 || !title.contains(expected_title) {
            log("[skip-debug] title does not contain expected -> fire");
            return false;
        }
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_err() {
            log("[skip-debug] GetCursorPos failed -> skip");
            return true;
        }
        let cursor_mon = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let win_mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let same = cursor_mon == win_mon;
        log(&format!(
            "[skip-debug] cursor_mon={:?} win_mon={:?} same={} -> {}",
            cursor_mon, win_mon, same, if same { "skip" } else { "fire" }
        ));
        same
    }
}

#[cfg(not(windows))]
fn should_skip(_: &str) -> bool {
    false
}

#[cfg(windows)]
fn show(xml_str: &str) -> Result<()> {
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(xml_str))
        .context("failed to parse toast XML")?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?;
    notifier.Show(&toast)?;
    Ok(())
}

#[cfg(not(windows))]
fn show(_xml_str: &str) -> Result<()> {
    anyhow::bail!("toast notifications are only supported on Windows builds")
}

/// Play a WAV synchronously. Blocks until the sound finishes (the audio
/// service is per-process; an async playback gets torn down with us, so we
/// keep the process alive for the duration).
#[cfg(windows)]
fn play_sync(path: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_FILENAME, SND_NODEFAULT};

    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(wide.as_ptr()),
            None,
            SND_FILENAME | SND_NODEFAULT,
        );
    }
}

#[cfg(not(windows))]
fn play_sync(_path: &str) {}

/// Convert a `file://` URL to a Windows local path. URL-decodes percent-escapes
/// and converts forward slashes to backslashes. Returns None if the input
/// doesn't look like a usable file URL.
fn file_url_to_local_path(url: &str) -> Option<String> {
    let rest = url.strip_prefix("file://")?;
    // file:///C:/... -> /C:/...    file://localhost/C:/... -> /C:/...
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    if rest.is_empty() {
        return None;
    }
    let decoded = percent_decode(rest);
    Some(decoded.replace('/', "\\"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn build_xml(
    title: &str,
    body: &str,
    toast_audio: Option<&str>,
    click: Option<&str>,
    silent: bool,
) -> String {
    let title_x = xml_escape(title);
    let body_x = xml_escape(body);
    let launch = match click {
        Some(uri) => format!(
            " launch=\"{}\" activationType=\"protocol\"",
            xml_escape(uri)
        ),
        None => String::new(),
    };
    let audio = if silent {
        "<audio silent=\"true\" />".to_string()
    } else {
        match toast_audio {
            Some(src) => format!("<audio src=\"{}\" />", xml_escape(src)),
            None => String::new(),
        }
    };
    format!(
        "<toast{launch}><visual><binding template=\"ToastGeneric\"><text>{title_x}</text><text>{body_x}</text></binding></visual>{audio}</toast>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_special_chars() {
        assert_eq!(
            xml_escape("a & b < c > d \" e ' f"),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f"
        );
    }

    #[test]
    fn builds_xml_without_optional_fields() {
        let xml = build_xml("t", "b", None, None, false);
        assert!(xml.contains("<text>t</text>"));
        assert!(xml.contains("<text>b</text>"));
        assert!(!xml.contains("launch="));
        assert!(!xml.contains("<audio"));
    }

    #[test]
    fn builds_xml_with_ms_audio_and_click() {
        let xml = build_xml(
            "t",
            "b",
            Some("ms-winsoundevent:Notification.IM"),
            Some("claude-notify://focus?target=tmux%3Dc1%3A0"),
            false,
        );
        assert!(xml.contains("launch=\"claude-notify://focus?target=tmux%3Dc1%3A0\""));
        assert!(xml.contains("activationType=\"protocol\""));
        assert!(xml.contains("<audio src=\"ms-winsoundevent:Notification.IM\" />"));
    }

    #[test]
    fn builds_xml_silent_when_external_audio() {
        let xml = build_xml("t", "b", None, None, true);
        assert!(xml.contains("<audio silent=\"true\" />"));
    }

    #[test]
    fn parses_simple_file_url() {
        assert_eq!(
            file_url_to_local_path("file:///C:/Users/me/sound.wav"),
            Some("C:\\Users\\me\\sound.wav".to_string())
        );
    }

    #[test]
    fn parses_file_url_with_localhost() {
        assert_eq!(
            file_url_to_local_path("file://localhost/C:/sound.wav"),
            Some("C:\\sound.wav".to_string())
        );
    }

    #[test]
    fn percent_decodes_path() {
        assert_eq!(
            file_url_to_local_path("file:///C:/Windows/Media/Windows%20Notify.wav"),
            Some("C:\\Windows\\Media\\Windows Notify.wav".to_string())
        );
    }

    #[test]
    fn rejects_non_file_url() {
        assert_eq!(file_url_to_local_path("ms-winsoundevent:Notification.IM"), None);
    }
}
