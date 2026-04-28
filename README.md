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

`--skip-if-title <name>` suppresses both the toast and the audio when **either**:

1. The foreground window is Windows Terminal and its title contains `<name>` (active focus on the tab), or
2. The cursor is hovering over a Windows Terminal window whose title contains `<name>` (you're looking at the terminal even if focus drifted to a browser tab or another app).

Override with `CLAUDE_NOTIFY_ALWAYS=1` to always fire regardless.

### Tag/Group dedup

`--tag <s>` and `--group <s>` set the toast's `Tag` and `Group` properties. Toasts sharing the same `(tag, group)` replace each other in Action Center instead of stacking. Useful for e.g. Claude Code's `Notification` event, which re-fires periodically while waiting and otherwise piles up. Each value is capped at 64 chars (Windows limit); values longer than that are truncated.

### Debug logging

Set `CLAUDE_NOTIFY_DEBUG=<windows-path-to-log-file>` (e.g. `C:\Users\me\AppData\Local\Temp\notify.log`) and the suppression check will append its decisions to that file.

When invoking from WSL, both `CLAUDE_NOTIFY_DEBUG` and `CLAUDE_NOTIFY_ALWAYS` need to be in `WSLENV` to propagate to the Windows binary. The provided helper script (`notify-windows.sh`) handles that.

## Status

- [x] `send` with title, body, audio (built-in `ms-winsoundevent:` or custom `file://` WAV via PlaySound)
- [x] Custom AppID
- [x] `register`: HKCU AppID DisplayName + URL protocol writes
- [~] `focus`: works (brings WT forward + `tmux select-window`), but click handler flashes a transient console window — **needs rework**
- [x] `--skip-if-title` (foreground-or-cursor-over) + `CLAUDE_NOTIFY_ALWAYS` opt-out
- [x] `--tag` / `--group` for Action Center dedup
- [ ] Custom icon registration

See [`docs/tasks/open-items.md`](docs/tasks/open-items.md) for details on each deferred item.

## License

MIT, see [LICENSE](LICENSE).
