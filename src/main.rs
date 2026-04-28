use clap::{Parser, Subcommand};

mod focus;
mod register;
mod send;

#[derive(Parser)]
#[command(name = "claude-notify", about, version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show a Windows toast notification.
    Send {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        /// Audio source. Examples:
        ///   ms-winsoundevent:Notification.IM
        ///   ms-winsoundevent:Notification.Mail
        ///   file:///C:/Users/you/sounds/claude.wav
        #[arg(long)]
        sound: Option<String>,
        /// URI invoked when the toast is clicked (e.g. claude-notify://focus?target=tmux%3Dc1%3A0).
        /// Omit to make the toast dismiss-only.
        #[arg(long)]
        click: Option<String>,
        /// Suppress the toast (and audio) when either the foreground window
        /// is Windows Terminal with this string in its title, OR the cursor is
        /// hovering over a Windows Terminal window with this string in its
        /// title. Set CLAUDE_NOTIFY_ALWAYS=1 to override and always fire.
        #[arg(long)]
        skip_if_title: Option<String>,
        /// Tag for Action Center deduplication (max 64 chars). Toasts sharing
        /// the same (tag, group) replace each other instead of stacking.
        #[arg(long)]
        tag: Option<String>,
        /// Group for Action Center deduplication (max 64 chars).
        #[arg(long)]
        group: Option<String>,
    },
    /// Handle a click activation. Brings Windows Terminal forward and switches the tmux window if `--target` is given.
    Focus {
        #[arg(long)]
        target: Option<String>,
    },
    /// Register the AppID and URI protocol in HKCU. Run once after first install.
    Register,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Send {
            title,
            body,
            sound,
            click,
            skip_if_title,
            tag,
            group,
        } => send::run(
            &title,
            &body,
            sound.as_deref(),
            click.as_deref(),
            skip_if_title.as_deref(),
            tag.as_deref(),
            group.as_deref(),
        ),
        Command::Focus { target } => focus::run(target.as_deref()),
        Command::Register => register::run(),
    }
}
