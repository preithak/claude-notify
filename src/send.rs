use anyhow::{Context, Result};

#[cfg(windows)]
use windows::core::HSTRING;
#[cfg(windows)]
use windows::Data::Xml::Dom::XmlDocument;
#[cfg(windows)]
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

pub const APP_ID: &str = "ClaudeCode.Notify";

pub fn run(title: &str, body: &str, sound: Option<&str>, click: Option<&str>) -> Result<()> {
    let xml_str = build_xml(title, body, sound, click);
    show(&xml_str)
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

fn build_xml(title: &str, body: &str, sound: Option<&str>, click: Option<&str>) -> String {
    let title_x = xml_escape(title);
    let body_x = xml_escape(body);
    let launch = match click {
        Some(uri) => format!(
            " launch=\"{}\" activationType=\"protocol\"",
            xml_escape(uri)
        ),
        None => String::new(),
    };
    let audio = match sound {
        Some(src) => format!("<audio src=\"{}\" />", xml_escape(src)),
        None => String::new(),
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
        assert_eq!(xml_escape("a & b < c > d \" e ' f"), "a &amp; b &lt; c &gt; d &quot; e &apos; f");
    }

    #[test]
    fn builds_xml_without_optional_fields() {
        let xml = build_xml("t", "b", None, None);
        assert!(xml.contains("<text>t</text>"));
        assert!(xml.contains("<text>b</text>"));
        assert!(!xml.contains("launch="));
        assert!(!xml.contains("<audio"));
    }

    #[test]
    fn builds_xml_with_audio_and_click() {
        let xml = build_xml(
            "t",
            "b",
            Some("ms-winsoundevent:Notification.IM"),
            Some("claude-notify://focus?target=tmux%3Dc1%3A0"),
        );
        assert!(xml.contains("launch=\"claude-notify://focus?target=tmux%3Dc1%3A0\""));
        assert!(xml.contains("activationType=\"protocol\""));
        assert!(xml.contains("<audio src=\"ms-winsoundevent:Notification.IM\" />"));
    }
}
