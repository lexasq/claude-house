//! `ch` and `ch-link` — a thin shell over `claude-house-core`, which holds the rules.

mod registry;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use clap::{Parser, Subcommand};
use claude_house_core::handler::{handler_command, is_handled_by};
use claude_house_core::install::newest_app;
use claude_house_core::launch::{claude_exe, profile_spec, stock_spec, LaunchSpec};
use claude_house_core::link::{last_login_start, route, ClaudeLink, LogTime, Target};
use claude_house_core::profile::{profile_dir, Color, Profile, ProfileId, ProfileName};
use claude_house_core::store::{self, ProfileList, StoreError};

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

/// Machine locations, read once from the environment.
struct Env {
    house_root: PathBuf,
    install_root: PathBuf,
    /// The stock copy's log, where it records login starts.
    stock_log: PathBuf,
}

impl Env {
    fn from_process() -> Result<Self, String> {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from).ok_or(format!("{k} is not set"));
        let local = var("LOCALAPPDATA")?;
        Ok(Env {
            house_root: var("APPDATA")?.join("ClaudeHouse"),
            install_root: local.join("AnthropicClaude"),
            stock_log: local.join("Claude").join("Logs").join("main.log"),
        })
    }

    fn list_path(&self) -> PathBuf {
        self.house_root.join("profiles.json")
    }
}

/// Entry point of `ch.exe`.
pub fn ch_main() -> ExitCode {
    let cli = Cli::parse();
    match Env::from_process().and_then(|env| run(cli.command, &env)) {
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

fn run(cmd: Cmd, env: &Env) -> Result<String, String> {
    match cmd {
        Cmd::List => Ok(render(&load(env)?)),
        Cmd::Add { id, name, color, isolate_code } => add(env, &id, name.as_deref(), &color, isolate_code),
        Cmd::Remove { id } => remove(env, &id),
        Cmd::Launch { id, dry_run } => launch(env, &id, dry_run),
        Cmd::OpenLink { url, dry_run } => open_link(env, &url, dry_run),
        Cmd::Handler { action: HandlerAction::Status } => handler_status(env),
        Cmd::Handler { action: HandlerAction::Install } => handler_install(),
        Cmd::Handler { action: HandlerAction::Uninstall } => handler_uninstall(env),
        Cmd::Watch => watch(),
    }
}

/// Entry point of the windowless `ch-link.exe` that Windows runs for claude:// links.
/// It has no console, so the outcome goes to `<house root>\ch-link.log` (without the link, which holds a login code).
pub fn ch_link_main() -> ExitCode {
    let url = std::env::args().nth(1).unwrap_or_default();
    let Ok(env) = Env::from_process() else { return ExitCode::FAILURE };
    let result = open_link(&env, &url, false);
    let line = match &result {
        Ok(out) => out.trim_end().to_owned(),
        Err(e) => format!("error: {e}"),
    };
    let _ = std::fs::create_dir_all(&env.house_root);
    let entry = format!("{} {line}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env.house_root.join("ch-link.log"))
        .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()));
    if result.is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// `ch-link.exe`, installed next to `ch.exe`.
fn link_exe() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("ch-link.exe");
    exe.is_file().then_some(exe.clone()).ok_or(format!("{} is missing; build it with cargo build -p claude-house-cli", exe.display()))
}

fn handler_status(env: &Env) -> Result<String, String> {
    let current = registry::read_handler();
    let owner = if link_exe().is_ok_and(|exe| is_handled_by(current.as_deref(), &exe)) {
        "Claude House"
    } else if stock_exe(env).is_ok_and(|exe| is_handled_by(current.as_deref(), &exe)) {
        "Claude Desktop"
    } else {
        "something else"
    };
    Ok(format!("claude:// is handled by {owner}: {}\n", current.as_deref().unwrap_or("(not registered)")))
}

fn handler_install() -> Result<String, String> {
    let exe = link_exe()?;
    registry::write_handler(&handler_command(&exe)).map_err(|e| format!("cannot write the claude:// handler: {e}"))?;
    Ok("claude:// now goes to Claude House. Claude takes it back each time it starts; run `ch watch` to keep it.\n".into())
}

fn handler_uninstall(env: &Env) -> Result<String, String> {
    let stock = handler_command(&stock_exe(env)?);
    registry::write_handler(&stock).map_err(|e| format!("cannot write the claude:// handler: {e}"))?;
    Ok("claude:// goes to Claude Desktop again. Stop any running `ch watch` first, or it will take it back.\n".into())
}

fn watch() -> Result<String, String> {
    let ours = handler_command(&link_exe()?);
    println!("Keeping claude:// pointed at Claude House. Ctrl+C to stop.");
    loop {
        let current = registry::read_handler();
        if current.as_deref() != Some(ours.as_str()) {
            registry::write_handler(&ours).map_err(|e| format!("cannot write the claude:// handler: {e}"))?;
            println!("{} took claude:// back from: {}", chrono::Local::now().format("%H:%M:%S"), current.as_deref().unwrap_or("(none)"));
        }
        registry::wait_for_change(std::time::Duration::from_secs(5));
    }
}

fn stock_exe(env: &Env) -> Result<PathBuf, String> {
    Ok(claude_exe(&env.install_root, installed_app(env)?))
}

fn load(env: &Env) -> Result<ProfileList, String> {
    store::load(&env.list_path()).map_err(|e| store_error(&env.list_path(), e))
}

fn save(env: &Env, list: &ProfileList) -> Result<(), String> {
    std::fs::create_dir_all(&env.house_root).map_err(|e| format!("cannot create {}: {e}", env.house_root.display()))?;
    store::save(&env.list_path(), list).map_err(|e| store_error(&env.list_path(), e))
}

fn store_error(path: &Path, e: StoreError) -> String {
    match e {
        StoreError::Io(e) => format!("cannot use {}: {e}", path.display()),
        StoreError::Format(m) => format!("{} is invalid: {m}", path.display()),
    }
}

fn parse_id(raw: &str) -> Result<ProfileId, String> {
    ProfileId::parse(raw).map_err(|_| format!("invalid profile id {raw:?}: use 1-32 lowercase letters, digits or dashes"))
}

fn render(list: &ProfileList) -> String {
    let lines: String = list
        .iter()
        .map(|p| {
            let code = if p.isolate_code { "code isolated" } else { "code shared" };
            format!("{}  {}  {}  {code}\n", p.id.as_str(), p.name.as_str(), p.color.as_str())
        })
        .collect();
    if lines.is_empty() { "No profiles. Add one with: ch add <id>\n".to_owned() } else { lines }
}

fn add(env: &Env, id: &str, name: Option<&str>, color: &str, isolate_code: bool) -> Result<String, String> {
    let profile = Profile {
        id: parse_id(id)?,
        name: ProfileName::parse(name.unwrap_or(id)).ok_or("invalid name: use 1-64 characters")?,
        color: Color::parse(color).ok_or(format!("invalid color {color:?}: use #rrggbb"))?,
        isolate_code,
    };
    let mut list = load(env)?;
    list.add(profile).map_err(|_| format!("profile {id:?} already exists"))?;
    save(env, &list)?;
    Ok(format!("Added {id}.\n"))
}

fn remove(env: &Env, id: &str) -> Result<String, String> {
    let id = parse_id(id)?;
    let mut list = load(env)?;
    list.remove(&id);
    save(env, &list)?;
    Ok(format!(
        "Removed {}. Its data is kept in {}\n",
        id.as_str(),
        profile_dir(&env.house_root, &id).display()
    ))
}

fn launch(env: &Env, id: &str, dry_run: bool) -> Result<String, String> {
    let list = load(env)?;
    let profile = list.get(&parse_id(id)?).ok_or(format!("no profile {id:?}"))?;
    let spec = profile_spec(&env.install_root, installed_app(env)?, &env.house_root, profile, None);
    if dry_run {
        return Ok(describe(&spec));
    }
    spawn(&spec).map_err(|e| format!("cannot start {}: {e}", spec.program.display()))?;
    // If the profile is already running, this process hands over to it and exits.
    Ok(format!("Opened {id}. If it was already running, its window comes to the front.\n"))
}

fn open_link(env: &Env, url: &str, dry_run: bool) -> Result<String, String> {
    let link = ClaudeLink::parse(url).map_err(|_| format!("{url:?} is not a claude:// link"))?;
    let list = load(env)?;
    let target = route(&link, &login_starts(env, &list), now());
    let app = installed_app(env)?;
    let (to, spec) = match &target {
        Target::Stock => ("stock".to_owned(), stock_spec(&env.install_root, app, &link)),
        Target::Profile(id) => {
            let profile = list.get(id).expect("route only returns listed profiles");
            (id.as_str().to_owned(), profile_spec(&env.install_root, app, &env.house_root, profile, Some(&link)))
        }
    };
    if dry_run {
        return Ok(format!("to: {to}\n{}", describe(&spec)));
    }
    spawn(&spec).map_err(|e| format!("cannot start {}: {e}", spec.program.display()))?;
    Ok(format!("Sent the link to {to}.\n"))
}

/// Last login start of the stock copy and of every profile, from their logs. Unreadable logs count as none.
fn login_starts(env: &Env, list: &ProfileList) -> Vec<(Target, LogTime)> {
    let last = |log: &Path| std::fs::read_to_string(log).ok().as_deref().and_then(last_login_start);
    let profiles = list
        .iter()
        .map(|p| (Target::Profile(p.id.clone()), profile_dir(&env.house_root, &p.id).join("logs").join("main.log")));
    std::iter::once((Target::Stock, env.stock_log.clone()))
        .chain(profiles)
        .filter_map(|(target, log)| last(&log).map(|at| (target, at)))
        .collect()
}

/// Now, on the same local clock Claude writes its logs with.
fn now() -> LogTime {
    let text = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    LogTime::parse(&text).expect("chrono formats a valid log time")
}

fn installed_app(env: &Env) -> Result<claude_house_core::install::AppVersion, String> {
    let names: Vec<String> = std::fs::read_dir(&env.install_root)
        .map(|rd| rd.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    newest_app(names.iter().map(String::as_str))
        .ok_or(format!("Claude Desktop is not installed in {}", env.install_root.display()))
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

/// The one untested line of effect: hand the spec to Windows.
fn spawn(spec: &LaunchSpec) -> std::io::Result<std::process::Child> {
    Command::new(&spec.program)
        .args(&spec.args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
