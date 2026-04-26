# claude-notify

A small native Windows toast notifier, designed to be called from WSL. Built for [Claude Code](https://claude.com/claude-code) hook integration but useful for any WSL workflow that wants to surface Linux-side events as Windows toasts.

Hits the OS floor for latency (~10-30ms per toast), supports custom audio, custom AppID branding, and click-to-focus on a specific tmux window.

## Features

- Native Windows toast via WinRT (`Windows.UI.Notifications`)
- Custom audio per toast (built-in `ms-winsoundevent:` names or `file:///` paths)
- Click activation that brings Windows Terminal forward and switches to a specific tmux session/window
- Custom AppID branding so toasts read "Claude Code"
- Single static binary, ~450 KB, no runtime dependencies on the Windows side

## Build

This produces a Windows `.exe`. Either cross-compile from WSL or build natively on Windows.

### Cross-compile from WSL

```bash
sudo apt install -y mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# Output: target/x86_64-pc-windows-gnu/release/claude-notify.exe
```

### Build on Windows

```powershell
cargo build --release
# Output: target\release\claude-notify.exe
```

## Usage

```bash
# Show a toast
claude-notify.exe send \
    --title "api-server" \
    --body "Task complete" \
    --sound "ms-winsoundevent:Notification.IM"

# Show a clickable toast that focuses a tmux window when clicked
claude-notify.exe send \
    --title "api-server" \
    --body "Needs input" \
    --sound "ms-winsoundevent:Notification.IM" \
    --click "claude-notify:tmux=c1:0"

# One-time setup: register the AppID and the claude-notify:// URL protocol in HKCU
claude-notify.exe register

# Invoked by Windows when a toast is clicked
claude-notify.exe focus --target "tmux=c1:0"
```

### Click URI format

`--click` accepts a URI of the form:

```
claude-notify:tmux=<session>:<window>
```

The `register` subcommand wires this scheme to call `claude-notify.exe focus --target "%1"`, so the click handler receives the URI verbatim and parses out the target.

### Sound options

Any value accepted by Windows `<audio src="...">`:

- `ms-winsoundevent:Notification.Default`
- `ms-winsoundevent:Notification.IM`
- `ms-winsoundevent:Notification.Mail`
- `ms-winsoundevent:Notification.Reminder`
- `ms-winsoundevent:Notification.SMS`
- `file:///C:/path/to/custom.wav` (any size; played via `PlaySoundW` directly because `file://` audio in toast XML is dropped for unpackaged AppIDs)

### Focus suppression

`--skip-if-title <name>` suppresses both the toast and the audio when the foreground Windows Terminal window's title contains `<name>` AND the cursor is on the same monitor as that window. Useful so you don't get a toast for the tab you're already looking at.

Override with `CLAUDE_NOTIFY_ALWAYS=1` to always fire regardless of focus.

Multi-monitor caveat: if WT is the OS-level foreground but on a different monitor than your eyes (you walked away or are reading on another screen without clicking elsewhere), the cursor-monitor check usually catches it, but a passive-reading edge case still loses. Use `CLAUDE_NOTIFY_ALWAYS=1` if it bothers you.

### Debug logging

Set `CLAUDE_NOTIFY_DEBUG=<windows-path-to-log-file>` (e.g. `C:\Users\me\AppData\Local\Temp\notify.log`) and the suppression check will append its decisions to that file.

## Status

- [x] `send` with title, body, audio (built-in `ms-winsoundevent:` or custom `file://` WAV via PlaySound)
- [x] Custom AppID
- [x] `register`: HKCU AppID DisplayName + URL protocol writes
- [x] `focus`: brings any visible Windows Terminal (CASCADIA class) forward and runs `wsl.exe tmux select-window -t session:window`
- [x] `--skip-if-title` + cursor-monitor check + `CLAUDE_NOTIFY_ALWAYS` opt-out
- [ ] Toast `Tag`/`Group` so repeated Notifications for the same tab replace each other instead of stacking
- [ ] Custom icon registration

## License

MIT, see [LICENSE](LICENSE).
