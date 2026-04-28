# Open Items

Backlog tracking for `claude-notify`. Each item is deferred (not yet prioritized) until it actually causes friction. The README's Status section links here for detail.

---

## focus: click handler flashes a console window — needs rework

**Symptom:** clicking a toast briefly opens a Windows Terminal-shaped window which immediately closes. Underneath that, `focus` does its job — `EnumWindows` finds a CASCADIA window, `SetForegroundWindow` returns success (BOOL(1)), and `wsl.exe tmux select-window` is spawned. Verified end-to-end via `CLAUDE_NOTIFY_DEBUG`. The flash is the visible part; whether the existing terminal actually settles as foreground is unclear because the console window for `claude-notify.exe` itself competes for foreground state during the brief moment it's alive.

**Root cause:** `claude-notify.exe` is built with the console subsystem (Rust's default). When Windows invokes it via URL protocol activation (no parent console), it auto-allocates a console window. The console flashes for the process lifetime and then disappears.

**Approaches considered:**

1. **Build with `#![windows_subsystem = "windows"]`.** ❌ **Tried, rejected.**
   - Pro: no console window, ever.
   - Con: `send` and `register` lose stdout/stderr when invoked from a parent shell unless we wire up `AttachConsole(ATTACH_PARENT_PROCESS)`.
   - **Blocker discovered in testing:** changing the binary's subsystem byte from console (3) to GUI (2), without changing source behavior, was enough to trigger CrowdStrike Falcon. EDR scored "GUI binary that spawns child processes (`wsl.exe`) and manipulates other processes' foreground state, run from a UNC path" as silent-loader malware pattern. Same code, same path; only the subsystem changed. Reverted.

2. **Split into two binaries.** Same EDR risk as option 1 for the GUI-subsystem half (it'd have the same suspicious-pattern surface). Skip.

3. **Wrap the registered command with `cmd /c start "" /B`.** Keeps console subsystem unchanged so EDR is fine, but `cmd.exe` itself flashes a window in URL-protocol activation, so the visible problem just moves up one process. Skip.

4. **Hide the auto-allocated console programmatically inside the binary.** Keep console subsystem (no EDR delta), and as the very first thing in `main()` for the `focus` subcommand, call `ShowWindow(GetConsoleWindow(), SW_HIDE)`. The console flashes from process start until the call (~30ms with optimized release builds), much less perceptible than the current full-process flash. Sketch:

   ```rust
   #[cfg(windows)]
   if matches!(cli.command, Command::Focus { .. }) {
       hide_console();
   }
   #[cfg(windows)]
   fn hide_console() {
       use windows::Win32::System::Console::GetConsoleWindow;
       use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
       unsafe {
           let hwnd = GetConsoleWindow();
           if !hwnd.0.is_null() { let _ = ShowWindow(hwnd, SW_HIDE); }
       }
   }
   ```

   Requires adding the `Win32_System_Console` cargo feature. Doesn't fully eliminate the flash but makes it a brief frame instead of a process-lifetime window.

5. **Code-sign the binary.** Heavyweight (cert acquisition + signing pipeline). Would unblock option 1 with EDR but is overkill for a personal tool.

**Current status:** deferred. Option 4 is the best practical path forward when this is picked up; option 1 is off the table until/unless the binary is code-signed.

**Diagnostic helper:** `focus.rs` ships with opt-in debug logging. Set `CLAUDE_NOTIFY_DEBUG` to a Windows path before clicking a toast and the binary appends what `EnumWindows` found, the `SetForegroundWindow` return value, and the spawned tmux command.

---

## ~~Toast Tag/Group dedup~~ — done

Implemented. `send` now accepts `--tag` and `--group`; the helper passes `--tag "<session>-<window>" --group "<event>"`, so repeated `Notification` events for the same tab replace each other in Action Center while a Stop and a Notification on the same tab remain distinct toasts.

---

## Custom icon registration

**Status:** deferred. Cosmetic.

Toasts currently use the default Windows bell icon because the registered AppID has no `IconUri`. To brand the toast:

1. Ship a `.ico` alongside the binary (or embed it as an `.rc` resource).
2. In `register`, additionally write
   `HKCU\Software\Classes\AppUserModelId\ClaudeCode.Notify\IconUri`
   pointing at the `.ico` file. Windows accepts `file:///` URIs and direct paths.
3. Optionally also write `IconBackgroundColor` (`#RRGGBB`) for a branded accent color.
