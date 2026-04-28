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
    tag: Option<&str>,
    group: Option<&str>,
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
    show(&xml_str, tag, group)?;

    if let Some(path) = play_path.as_deref() {
        play_sync(path);
    }
    Ok(())
}

/// Suppress the toast when *either* of these is true:
///   1. The foreground window's class is CASCADIA_HOSTING_WINDOW_CLASS and
///      its title contains the expected tab name (active focus on the tab).
///   2. The cursor is hovering over a Windows Terminal window whose title
///      contains the expected tab name (you're looking at the terminal even
///      if focus drifted to another window like a browser tab).
///
/// `CLAUDE_NOTIFY_ALWAYS=1` overrides both checks. `CLAUDE_NOTIFY_DEBUG=<path>`
/// appends decisions to a log for diagnosis.
#[cfg(windows)]
fn should_skip(expected_title: &str) -> bool {
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowTextW,
        WindowFromPoint, GA_ROOT,
    };

    if std::env::var_os("CLAUDE_NOTIFY_ALWAYS").is_some() {
        return false;
    }
    let debug_path = std::env::var("CLAUDE_NOTIFY_DEBUG").ok();
    let log = |msg: &str| {
        if let Some(p) = &debug_path {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(f, "[ts={ts}] {msg}");
            }
        }
    };

    let class_and_title = |hwnd: HWND| -> (String, String) {
        if hwnd.0.is_null() {
            return (String::new(), String::new());
        }
        unsafe {
            let mut cb = [0u16; 256];
            let cl = GetClassNameW(hwnd, &mut cb) as usize;
            let mut tb = [0u16; 512];
            let tl = GetWindowTextW(hwnd, &mut tb) as usize;
            (
                String::from_utf16_lossy(&cb[..cl]),
                String::from_utf16_lossy(&tb[..tl]),
            )
        }
    };
    let matches_wt = |class: &str, title: &str| -> bool {
        class.contains("CASCADIA") && title.contains(expected_title)
    };

    unsafe {
        // Check 1: foreground window
        let fg = GetForegroundWindow();
        let (fg_class, fg_title) = class_and_title(fg);
        log(&format!(
            "[skip-debug] expected={expected_title:?} fg.class={fg_class:?} fg.title={fg_title:?}"
        ));
        if matches_wt(&fg_class, &fg_title) {
            log("[skip-debug] foreground is matching WT -> skip");
            return true;
        }

        // Check 2: cursor is over a WT window
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_ok() {
            let under = WindowFromPoint(cursor);
            let root = GetAncestor(under, GA_ROOT);
            let (cur_class, cur_title) = class_and_title(root);
            log(&format!(
                "[skip-debug] cursor.class={cur_class:?} cursor.title={cur_title:?}"
            ));
            if matches_wt(&cur_class, &cur_title) {
                log("[skip-debug] cursor over matching WT -> skip");
                return true;
            }
        } else {
            log("[skip-debug] GetCursorPos failed");
        }

        log("[skip-debug] neither check matched -> fire");
        false
    }
}

#[cfg(not(windows))]
fn should_skip(_: &str) -> bool {
    false
}

#[cfg(windows)]
fn show(xml_str: &str, tag: Option<&str>, group: Option<&str>) -> Result<()> {
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(xml_str))
        .context("failed to parse toast XML")?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    if let Some(t) = tag {
        // Tag is limited to 64 characters; truncate to be safe.
        let trimmed: String = t.chars().take(64).collect();
        toast.SetTag(&HSTRING::from(trimmed.as_str()))?;
    }
    if let Some(g) = group {
        let trimmed: String = g.chars().take(64).collect();
        toast.SetGroup(&HSTRING::from(trimmed.as_str()))?;
    }
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?;
    notifier.Show(&toast)?;
    Ok(())
}

#[cfg(not(windows))]
fn show(_xml_str: &str, _tag: Option<&str>, _group: Option<&str>) -> Result<()> {
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
