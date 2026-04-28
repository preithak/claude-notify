# Open Items

Backlog tracking for `claude-notify`. Each item is deferred (not yet prioritized) until it actually causes friction. The README's Status section links here for detail.

---

## focus: click handler flashes a console window — needs rework

**Symptom:** clicking a toast briefly opens a Windows Terminal-shaped window which immediately closes. Underneath that, `focus` does its job (Windows Terminal is brought forward and `tmux select-window` runs), but the UX is broken.

**Root cause:** `claude-notify.exe` is built with the console subsystem (Rust's default for binaries). When Windows invokes it via URL protocol activation (no parent console exists), it allocates a fresh console window for the process. The console disappears as soon as the process exits, hence the flash.

**Approaches considered:**

1. **Build with `#![windows_subsystem = "windows"]`.**
   - Pro: no console window, ever.
   - Con: `send` and `register` lose stdout/stderr when invoked from a regular shell. We'd need `AttachConsole(ATTACH_PARENT_PROCESS)` in `main()` for CLI subcommands and rewire stdio so `eprintln!` still reaches the calling terminal.

2. **Split into two binaries.**
   - `claude-notify.exe` (console subsystem) for `send` and `register`.
   - `claude-notify-focus.exe` (windows subsystem) for click activation.
   - `register` writes the GUI binary's path into the URL protocol handler.
   - Pro: clean separation, no `AttachConsole` gymnastics.
   - Con: two binaries to build, ship, and reason about.

3. **Wrap the registered command to suppress the window.**
   - e.g. `cmd.exe /c start "" /B "...claude-notify.exe" focus --target "%1"`.
   - Pro: zero code change.
   - Con: still flashes briefly on some Windows versions; depends on `cmd.exe`/`start` behavior; uglier registry value.

**Recommendation:** option 1, single binary with `AttachConsole`. Keeps shipping simple and the UX clean.

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
