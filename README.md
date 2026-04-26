# claude-notify

Tiny Windows toast notifier for [Claude Code](https://claude.com/claude-code) hooks running under WSL.

Replaces a ~500ms `powershell.exe` call (CLR + WinRT init) with a ~10ms native binary while supporting custom audio, click-to-focus, and custom AppID branding.

## Why

Claude Code can fire `Stop` and `Notification` hooks when a turn ends or input is needed. Under WSL, the easy way to surface these as Windows toasts is to shell out to `powershell.exe`, but that pays ~300-800ms of process startup per toast. This binary hits the OS floor (process create + WinRT init, ~10ms) and leaves room for nicer features on top.

## Features

- Native Windows toast via WinRT (`Windows.UI.Notifications`)
- Custom audio per toast (built-in `ms-winsoundevent:` names or `file:///` paths)
- Click activation: brings Windows Terminal forward and switches to the originating tmux window
- Custom AppID so toasts read "Claude Code", not "Run"

## Build

This is a Windows binary. Two ways to produce it.

### Cross-compile from WSL (recommended for this workflow)

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
# Send a toast
claude-notify.exe send \
    --title "api-server" \
    --body "Task complete" \
    --sound "ms-winsoundevent:Notification.IM"

# Send with click-to-focus on a specific tmux window
claude-notify.exe send \
    --title "api-server" \
    --body "Needs input" \
    --click "claude-notify://focus?target=tmux%3Dc1%3A0"

# One-time setup: register the AppID and URI protocol in HKCU
claude-notify.exe register

# Invoked by Windows on toast click
claude-notify.exe focus --target "tmux=c1:0"
```

## Integrating with Claude Code

After building, place the `.exe` somewhere reachable from WSL (e.g. `/mnt/c/Users/<you>/bin/`) and update `~/.claude/notify-windows.sh` to call it instead of inline PowerShell.

## Status

- [x] `send` with title, body, audio
- [x] Custom AppID
- [ ] `register`: HKCU AppID + URI protocol
- [ ] `focus`: Windows Terminal foreground + tmux window switch
- [ ] Replace toast (vs. stack) when a newer one fires for the same tab

## License

MIT, see [LICENSE](LICENSE).
