//! `ch` and `ch-link` — terminal front ends over `ops`.

pub mod ops;
mod registry;

use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use clap::{Parser, Subcommand};
use claude_house_core::launch::LaunchSpec;
use claude_house_core::link::Target;
use claude_house_core::account::SignIn;
use ops::{HandlerOwner, House, ProfileStatus};

#[derive(Parser)]
#[command(name = "ch", about = "Run several Claude Desktop accounts side by side")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List profiles.
    List,
    /// Add a profile.
    Add {
        /// 1-32 lowercase letters, digits or dashes. Never changes.
        id: String,
        /// Display name (defaults to the id).
        #[arg(long)]
        name: Option<String>,
        /// Badge colour, #rrggbb.
        #[arg(long, default_value = "#d97757")]
        color: String,
        /// Give the Code tab its own settings folder instead of the shared ~/.claude.
        #[arg(long)]
        isolate_code: bool,
    },
    /// Remove a profile from the list. Its data folder is kept.
    Remove { id: String },
    /// Start a profile's Claude copy (or bring it to the front if it is running).
    Launch {
        id: String,
        /// Print the command instead of running it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Send a claude:// link to the Claude copy that started the login.
    OpenLink {
        url: String,
        /// Print where it would go and the command, instead of running it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Show or change who handles claude:// links.
    Handler {
        #[command(subcommand)]
        action: HandlerAction,
    },
    /// Keep claude:// pointed at Claude House. Claude takes it back every time it starts. Stop with Ctrl+C.
    Watch,
}

#[derive(Subcommand)]
enum HandlerAction {
    /// Show the current claude:// handler.
    Status,
    /// Point claude:// at Claude House once (Claude will take it back on its next start; use `ch watch`).
    Install,
    /// Point claude:// back at Claude Desktop.
    Uninstall,
}

/// Entry point of `ch.exe`.
pub fn ch_main() -> ExitCode {
    let cli = Cli::parse();
    match House::from_process().and_then(|house| run(cli.command, &house)) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ch: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cmd: Cmd, house: &House) -> Result<String, String> {
    match cmd {
        Cmd::List => Ok(render(&house.statuses()?)),
        Cmd::Add { id, name, color, isolate_code } => {
            house.add(&id, name.as_deref(), &color, isolate_code).map(|_| format!("Added {id}.\n"))
        }
        Cmd::Remove { id } => house
            .remove(&id)
            .map(|dir| format!("Removed {id}. Its data is kept in {}\n", dir.display())),
        Cmd::Launch { id, dry_run: true } => house.launch_spec(&id).map(|spec| describe(&spec)),
        Cmd::Launch { id, dry_run: false } => house
            .launch(&id)
            .map(|()| format!("Opened {id}. If it was already running, its window comes to the front.\n")),
        Cmd::OpenLink { url, dry_run: true } => house
            .link_plan(&url)
            .map(|(to, spec)| format!("to: {}\n{}", target_name(&to), describe(&spec))),
        Cmd::OpenLink { url, dry_run: false } => house.open_link(&url).map(|to| format!("Sent the link to {}.\n", target_name(&to))),
        Cmd::Handler { action: HandlerAction::Status } => Ok(handler_status(house)),
        Cmd::Handler { action: HandlerAction::Install } => ops::take_handler().map(|()| {
            "claude:// now goes to Claude House. Claude takes it back each time it starts; run `ch watch` to keep it.\n".into()
        }),
        Cmd::Handler { action: HandlerAction::Uninstall } => house.restore_handler().map(|()| {
            "claude:// goes to Claude Desktop again. Stop any running `ch watch` or the app first, or they will take it back.\n".into()
        }),
        Cmd::Watch => watch(),
    }
}

fn target_name(target: &Target) -> &str {
    match target {
        Target::Stock => "stock",
        Target::Profile(id) => id.as_str(),
    }
}

fn handler_status(house: &House) -> String {
    match house.handler_owner() {
        HandlerOwner::ClaudeHouse => "claude:// is handled by Claude House.\n".into(),
        HandlerOwner::ClaudeDesktop => "claude:// is handled by Claude Desktop.\n".into(),
        HandlerOwner::Other(cmd) => format!("claude:// is handled by something else: {}\n", cmd.as_deref().unwrap_or("(not registered)")),
    }
}

fn watch() -> Result<String, String> {
    println!("Keeping claude:// pointed at Claude House. Ctrl+C to stop.");
    ops::keep_handler(&AtomicBool::new(false), |was| {
        println!("{} took claude:// back from: {}", chrono::Local::now().format("%H:%M:%S"), was.as_deref().unwrap_or("(none)"));
    })?;
    Ok(String::new())
}

fn render(list: &[ProfileStatus]) -> String {
    let lines: String = list
        .iter()
        .map(|s| {
            let p = &s.profile;
            let code = if p.isolate_code { "code isolated" } else { "code shared" };
            let sign_in = match s.sign_in {
                SignIn::NeverOpened => "never opened",
                SignIn::SignedOut => "not signed in",
                SignIn::SignedIn(_) => "signed in",
                SignIn::Unreadable => "config unreadable",
            };
            let names = ops::shared_names(list, s);
            let shared = if names.is_empty() { String::new() } else { format!(" (same account as {})", names.join(", ")) };
            format!("{}  {}  {}  {code}  {sign_in}{shared}\n", p.id.as_str(), p.name.as_str(), p.color.as_str())
        })
        .collect();
    if lines.is_empty() { "No profiles. Add one with: ch add <id>\n".to_owned() } else { lines }
}

fn describe(spec: &LaunchSpec) -> String {
    let env: String = spec
        .env
        .iter()
        .map(|(k, v)| format!("set {}={}\n", k.to_string_lossy(), v.to_string_lossy()))
        .collect();
    let args: String = spec.args.iter().map(|a| format!(" \"{}\"", a.to_string_lossy())).collect();
    format!("{env}\"{}\"{args}\n", spec.program.display())
}

/// Entry point of the windowless `ch-link.exe` that Windows runs for claude:// links.
/// It has no console, so the outcome goes to `<house root>\ch-link.log` (without the link, which holds a login code).
pub fn ch_link_main() -> ExitCode {
    let url = std::env::args().nth(1).unwrap_or_default();
    let Ok(house) = House::from_process() else { return ExitCode::FAILURE };
    let result = house.open_link(&url);
    let line = match &result {
        Ok(to) => format!("Sent the link to {}.", target_name(to)),
        Err(e) => format!("error: {e}"),
    };
    let _ = std::fs::create_dir_all(&house.root);
    let entry = format!("{} {line}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(house.root.join("ch-link.log"))
        .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    if result.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

