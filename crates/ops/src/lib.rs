//! Claude House operations with effects (files, processes, registry), shared by `ch`, `ch-link` and the app.
//! They return data; callers decide how to show it. Rules live in `claude-house-core`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use claude_house_core::account::{shared_accounts, sign_in_state, SignIn};
use claude_house_core::handler::{handler_command, is_handled_by};
use claude_house_core::install::{newest_app, AppVersion};
use claude_house_core::launch::{claude_exe, profile_spec, stock_spec, LaunchSpec};
use claude_house_core::link::{last_login_start, route, ClaudeLink, LogTime, Target};
use claude_house_core::profile::{profile_dir, Color, Profile, ProfileId, ProfileName};
use claude_house_core::store::{self, ProfileList, StoreError};

mod registry;

/// Machine locations, read once from the environment.
pub struct House {
    pub root: PathBuf,
    install_root: PathBuf,
    /// The stock copy's log, where it records login starts.
    stock_log: PathBuf,
    /// The stock copy's `config.json`, where it records who is signed in.
    stock_config: PathBuf,
}

/// What the stock Claude Desktop is called: the account the user already uses, kept as it is.
pub const MAIN_NAME: &str = "Main account";

/// One Claude copy's sign-in state and the other copies on the same account.
pub struct CopyStatus {
    pub sign_in: SignIn,
    pub same_account_as: Vec<Target>,
}

/// The main (stock) Claude and every profile, with sign-in states.
pub struct Overview {
    pub main: CopyStatus,
    pub profiles: Vec<(Profile, CopyStatus)>,
}

impl Overview {
    /// Display names of `targets` (`MAIN_NAME` for the stock copy).
    pub fn names(&self, targets: &[Target]) -> Vec<String> {
        targets
            .iter()
            .map(|t| match t {
                Target::Stock => MAIN_NAME.to_owned(),
                Target::Profile(id) => {
                    self.profiles.iter().find(|(p, _)| &p.id == id).map_or(id.as_str(), |(p, _)| p.name.as_str()).to_owned()
                }
            })
            .collect()
    }
}

/// Who `claude://` links currently go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerOwner {
    ClaudeHouse,
    ClaudeDesktop,
    Other(Option<String>),
}

impl House {
    pub fn from_process() -> Result<Self, String> {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from).ok_or(format!("{k} is not set"));
        let (local, roaming) = (var("LOCALAPPDATA")?, var("APPDATA")?);
        Ok(House {
            // Not under APPDATA: Claude would move a profile's logs to %LOCALAPPDATA%\<id>\logs,
            // outside the profile folder (app.asar: setAppLogsPath(join(LOCALAPPDATA, basename(userData), "logs"))).
            root: local.join("ClaudeHouse"),
            install_root: local.join("AnthropicClaude"),
            stock_log: local.join("Claude").join("Logs").join("main.log"),
            stock_config: roaming.join("Claude").join("config.json"),
        })
    }

    /// The main Claude and every profile with sign-in states and shared accounts.
    pub fn overview(&self) -> Result<Overview, String> {
        let list = self.profiles()?;
        let states: Vec<(Target, SignIn)> = std::iter::once((Target::Stock, read_sign_in(&self.stock_config)))
            .chain(list.iter().map(|p| {
                let config = profile_dir(&self.root, &p.id).join("config.json");
                (Target::Profile(p.id.clone()), read_sign_in(&config))
            }))
            .collect();
        let shared = shared_accounts(&states);
        let mut copies = states.into_iter().map(|(target, sign_in)| {
            let same_account_as = shared.iter().find(|(t, _)| *t == target).map(|(_, o)| o.clone()).unwrap_or_default();
            CopyStatus { sign_in, same_account_as }
        });
        let main = copies.next().expect("the stock copy is always first");
        Ok(Overview { main, profiles: list.iter().cloned().zip(copies).collect() })
    }

    /// Opens the main Claude (or brings it to the front), like its own shortcut does.
    pub fn launch_main(&self) -> Result<(), String> {
        let exe = self.stock_exe()?;
        spawn(&LaunchSpec { program: exe, args: vec![], env: vec![] })
    }

    fn list_path(&self) -> PathBuf {
        self.root.join("profiles.json")
    }

    pub fn profiles(&self) -> Result<ProfileList, String> {
        store::load(&self.list_path()).map_err(|e| store_error(&self.list_path(), e))
    }

    fn save(&self, list: &ProfileList) -> Result<(), String> {
        std::fs::create_dir_all(&self.root).map_err(|e| format!("cannot create {}: {e}", self.root.display()))?;
        store::save(&self.list_path(), list).map_err(|e| store_error(&self.list_path(), e))
    }

    pub fn add(&self, id: &str, name: Option<&str>, color: &str, isolate_code: bool) -> Result<Profile, String> {
        let profile = Profile {
            id: parse_id(id)?,
            name: ProfileName::parse(name.unwrap_or(id)).ok_or("invalid name: use 1-64 characters")?,
            color: Color::parse(color).ok_or(format!("invalid color {color:?}: use #rrggbb"))?,
            isolate_code,
        };
        let mut list = self.profiles()?;
        list.add(profile.clone()).map_err(|_| format!("profile {id:?} already exists"))?;
        self.save(&list)?;
        Ok(profile)
    }

    /// Forgets the profile. Returns its data folder, which is kept.
    pub fn remove(&self, id: &str) -> Result<PathBuf, String> {
        let id = parse_id(id)?;
        let mut list = self.profiles()?;
        list.remove(&id);
        self.save(&list)?;
        Ok(profile_dir(&self.root, &id))
    }

    pub fn launch_spec(&self, id: &str) -> Result<LaunchSpec, String> {
        let list = self.profiles()?;
        let profile = list.get(&parse_id(id)?).ok_or(format!("no profile {id:?}"))?;
        Ok(profile_spec(&self.install_root, self.installed_app()?, &self.root, profile, None))
    }

    /// Starts the profile's copy; if it is running, the new process hands over to it and exits.
    pub fn launch(&self, id: &str) -> Result<(), String> {
        spawn(&self.launch_spec(id)?)
    }

    /// Where `url` goes and the command that delivers it.
    pub fn link_plan(&self, url: &str) -> Result<(Target, LaunchSpec), String> {
        // The link is never echoed back: a link handed to us may carry a one-time login code,
        // and this message can end up in ch-link.log.
        let link = ClaudeLink::parse(url).map_err(|_| "that is not a claude:// link".to_owned())?;
        let list = self.profiles()?;
        let target = route(&link, &self.login_starts(&list));
        let app = self.installed_app()?;
        let spec = match &target {
            Target::Stock => stock_spec(&self.install_root, app, &link),
            Target::Profile(id) => {
                let profile = list.get(id).expect("route only returns listed profiles");
                profile_spec(&self.install_root, app, &self.root, profile, Some(&link))
            }
        };
        Ok((target, spec))
    }

    pub fn open_link(&self, url: &str) -> Result<Target, String> {
        let (target, spec) = self.link_plan(url)?;
        spawn(&spec)?;
        Ok(target)
    }

    /// Last login start of the stock copy and of every profile, from their logs. Unreadable logs count as none.
    fn login_starts(&self, list: &ProfileList) -> Vec<(Target, LogTime)> {
        let last = |log: &Path| last_login_start_in_file(log, 64 * 1024);
        let profiles = list
            .iter()
            .map(|p| (Target::Profile(p.id.clone()), profile_dir(&self.root, &p.id).join("logs").join("main.log")));
        std::iter::once((Target::Stock, self.stock_log.clone()))
            .chain(profiles)
            .filter_map(|(target, log)| last(&log).map(|at| (target, at)))
            .collect()
    }

    fn installed_app(&self) -> Result<AppVersion, String> {
        let names: Vec<String> = std::fs::read_dir(&self.install_root)
            .map(|rd| rd.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect())
            .unwrap_or_default();
        newest_app(names.iter().map(String::as_str))
            .ok_or(format!("Claude Desktop is not installed in {}", self.install_root.display()))
    }

    fn stock_exe(&self) -> Result<PathBuf, String> {
        Ok(claude_exe(&self.install_root, self.installed_app()?))
    }

    pub fn handler_owner(&self) -> HandlerOwner {
        let current = registry::read_handler();
        if link_exe().is_ok_and(|exe| is_handled_by(current.as_deref(), &exe)) {
            HandlerOwner::ClaudeHouse
        } else if self.stock_exe().is_ok_and(|exe| is_handled_by(current.as_deref(), &exe)) {
            HandlerOwner::ClaudeDesktop
        } else {
            HandlerOwner::Other(current)
        }
    }

    /// Points `claude://` back at Claude Desktop.
    pub fn restore_handler(&self) -> Result<(), String> {
        write_handler(&handler_command(&self.stock_exe()?))
    }
}

/// `ch-link.exe`, installed next to the running program.
pub fn link_exe() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("ch-link.exe");
    exe.is_file().then_some(exe.clone()).ok_or(format!("{} is missing; build it with cargo build -p claude-house-cli", exe.display()))
}

/// Points `claude://` at Claude House once.
pub fn take_handler() -> Result<(), String> {
    write_handler(&handler_command(&link_exe()?))
}

/// Keeps `claude://` pointed at Claude House until `stop` is set (checked at least every second,
/// so a caller that joins the thread waits at most that long).
/// Calls `on_take` with the command it replaced each time it takes the handler back.
pub fn keep_handler(stop: &AtomicBool, mut on_take: impl FnMut(Option<String>)) -> Result<(), String> {
    let ours = handler_command(&link_exe()?);
    while !stop.load(Ordering::Relaxed) {
        let current = registry::read_handler();
        if current.as_deref() != Some(ours.as_str()) {
            write_handler(&ours)?;
            on_take(current);
        }
        registry::wait_for_change(std::time::Duration::from_secs(1));
    }
    Ok(())
}

/// The newest login start in a log file, reading backwards in `chunk`-byte steps and stopping at
/// the first match. Claude's main.log grows to ~10 MB; the newest start is usually near its end.
/// A missing or unreadable file has none.
fn last_login_start_in_file(path: &Path, chunk: u64) -> Option<LogTime> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let mut end = file.metadata().ok()?.len();
    // Bytes after `end` that belong to a line whose start we have not read yet.
    let mut tail: Vec<u8> = Vec::new();
    while end > 0 {
        let start = end.saturating_sub(chunk);
        let mut buf = vec![0; usize::try_from(end - start).ok()?];
        file.seek(SeekFrom::Start(start)).ok()?;
        file.read_exact(&mut buf).ok()?;
        buf.extend_from_slice(&tail);
        // Unless we reached the file start, the text before the first newline may be a partial line:
        // keep it for the next (earlier) chunk and search only what follows it.
        let cut = if start == 0 { 0 } else { buf.iter().position(|&b| b == b'\n').unwrap_or(buf.len()) };
        if let Some(found) = last_login_start(&String::from_utf8_lossy(&buf[cut..])) {
            return Some(found);
        }
        tail = buf[..cut].to_vec();
        end = start;
    }
    None
}

/// A missing file means the copy was never opened; any other read failure is unreadable.
fn read_sign_in(config: &Path) -> SignIn {
    match std::fs::read_to_string(config) {
        Ok(text) => sign_in_state(Some(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => sign_in_state(None),
        Err(_) => SignIn::Unreadable,
    }
}

fn write_handler(command: &str) -> Result<(), String> {
    registry::write_handler(command).map_err(|e| format!("cannot write the claude:// handler: {e}"))
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

/// The one line of effect for processes: hand the spec to Windows.
fn spawn(spec: &LaunchSpec) -> Result<(), String> {
    Command::new(&spec.program)
        .args(&spec.args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| format!("cannot start {}: {e}", spec.program.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "[info] [Auth] Using system browser for: /login/app-google-auth";

    fn newest(text: &str, chunk: u64) -> Option<LogTime> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.log");
        std::fs::write(&path, text).unwrap();
        last_login_start_in_file(&path, chunk)
    }

    #[test]
    fn backward_scan_matches_a_whole_file_read_for_every_chunk_size() {
        let log = format!(
            "2026-11-09 10:00:00 {START}\n2026-11-09 10:00:01 [info] other\n2026-11-09 11:00:00 {START}\n2026-11-09 11:00:05 [info] last line without newline"
        );
        let expected = last_login_start(&log);
        assert!(expected.is_some());
        // Tiny chunks split lines at every possible place, including inside the timestamp.
        for chunk in 1..=log.len() as u64 + 1 {
            assert_eq!(newest(&log, chunk), expected, "chunk size {chunk}");
        }
    }

    #[test]
    fn no_start_missing_file_and_empty_file_give_none() {
        assert_eq!(newest("2026-11-09 10:00:00 [info] other\n", 8), None);
        assert_eq!(newest("", 8), None);
        assert_eq!(last_login_start_in_file(Path::new(r"C:\no\such\main.log"), 8), None);
    }
}
