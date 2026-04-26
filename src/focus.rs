use anyhow::Result;

pub fn run(target: Option<&str>) -> Result<()> {
    // TODO: bring Windows Terminal foreground via SetForegroundWindow on its
    // top-level HWND (enumerate windows whose class is "CASCADIA_HOSTING_WINDOW_CLASS").
    // TODO: if `target` is "tmux=session:window", spawn `wsl.exe -d <distro> tmux select-window -t <session>:<window>`.
    eprintln!("focus invoked with target = {target:?}");
    eprintln!("(not yet implemented)");
    Ok(())
}
