use anyhow::Result;

pub fn run() -> Result<()> {
    // TODO: write HKCU\Software\Classes\AppUserModelId\ClaudeCode.Notify
    //   - DisplayName = "Claude Code"
    //   - IconUri (optional)
    // TODO: register URI protocol "claude-notify://" pointing back to this exe with `focus --target=...`.
    //   - HKCU\Software\Classes\claude-notify\(Default) = "URL:claude-notify"
    //   - HKCU\Software\Classes\claude-notify\URL Protocol = ""
    //   - HKCU\Software\Classes\claude-notify\shell\open\command\(Default) = "\"<exe>\" focus --target \"%1\""
    eprintln!("register: not yet implemented");
    eprintln!("for now, the AppID 'ClaudeCode.Notify' will be created implicitly on first toast.");
    Ok(())
}
