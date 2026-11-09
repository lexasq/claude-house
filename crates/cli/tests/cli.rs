//! Runs the real `ch` binary the way a user does, with APPDATA/LOCALAPPDATA pointed at temp folders.

use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox {
    _dir: tempfile::TempDir,
    appdata: PathBuf,
    localappdata: PathBuf,
}

impl Sandbox {
    /// A fake machine with Claude installed as two app folders.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let appdata = dir.path().join("Roaming");
        let localappdata = dir.path().join("Local");
        for app in ["app-4.4212.0", "app-4.23118.0"] {
            std::fs::create_dir_all(localappdata.join("AnthropicClaude").join(app)).unwrap();
        }
        std::fs::create_dir_all(&appdata).unwrap();
        Sandbox { _dir: dir, appdata, localappdata }
    }

    fn ch(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ch"))
            .args(args)
            .env("APPDATA", &self.appdata)
            .env("LOCALAPPDATA", &self.localappdata)
            .output()
            .unwrap()
    }

    fn house(&self) -> PathBuf {
        self.appdata.join("ClaudeHouse")
    }
}

fn ok(out: &Output) -> String {
    assert!(out.status.success(), "ch failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn err(out: &Output) -> String {
    assert!(!out.status.success(), "ch should have failed: {}", String::from_utf8_lossy(&out.stdout));
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn list_is_empty_on_a_fresh_machine() {
    let s = Sandbox::new();
    assert_eq!(ok(&s.ch(&["list"])), "No profiles. Add one with: ch add <id>\n");
}

#[test]
fn add_then_list_shows_profiles_in_order() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work", "--name", "Work account", "--color", "#3366CC", "--isolate-code"]));
    ok(&s.ch(&["add", "home"]));
    assert_eq!(
        ok(&s.ch(&["list"])),
        "work  Work account  #3366cc  code isolated\nhome  home  #d97757  code shared\n"
    );
    assert!(s.house().join("profiles.json").is_file());
}

#[test]
fn add_rejects_bad_input_and_duplicates() {
    let s = Sandbox::new();
    assert!(err(&s.ch(&["add", "Work"])).contains("profile id"));
    assert!(err(&s.ch(&["add", "work", "--color", "red"])).contains("color"));
    assert!(err(&s.ch(&["add", "work", "--name", "  "])).contains("name"));
    ok(&s.ch(&["add", "work"]));
    assert!(err(&s.ch(&["add", "work"])).contains("already exists"));
}

#[test]
fn remove_is_idempotent_and_keeps_the_data_folder() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work"]));
    let folder = s.house().join("profiles").join("work");
    std::fs::create_dir_all(&folder).unwrap();
    assert!(ok(&s.ch(&["remove", "work"])).contains(&folder.display().to_string()));
    ok(&s.ch(&["remove", "work"]));
    assert_eq!(ok(&s.ch(&["list"])), "No profiles. Add one with: ch add <id>\n");
    assert!(folder.is_dir());
}

#[test]
fn launch_dry_run_prints_the_command_for_the_newest_app() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work", "--isolate-code"]));
    let profile = s.house().join("profiles").join("work");
    let exe = s.localappdata.join("AnthropicClaude").join("app-4.23118.0").join("claude.exe");
    assert_eq!(
        ok(&s.ch(&["launch", "work", "--dry-run"])),
        format!(
            "set CLAUDE_CONFIG_DIR={}\n\"{}\" \"--user-data-dir={}\"\n",
            profile.join("claude-code").display(),
            exe.display(),
            profile.display()
        )
    );
}

#[test]
fn launch_unknown_profile_fails() {
    let s = Sandbox::new();
    assert!(err(&s.ch(&["launch", "nope", "--dry-run"])).contains("no profile \"nope\""));
}

#[test]
fn launch_without_claude_installed_fails() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work"]));
    std::fs::remove_dir_all(s.localappdata.join("AnthropicClaude")).unwrap();
    assert!(err(&s.ch(&["launch", "work", "--dry-run"])).contains("Claude Desktop is not installed"));
}

/// Writes a login-start line `mins_ago` minutes before now (local clock, as Claude logs it).
fn log_login(log: &std::path::Path, mins_ago: i64) {
    let at = chrono::Local::now().naive_local() - chrono::Duration::minutes(mins_ago);
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    let line = format!(
        "{} [info] [Auth] Using system browser for: /login/app-google-auth\n",
        at.format("%Y-%m-%d %H:%M:%S")
    );
    std::fs::write(log, line).unwrap();
}

impl Sandbox {
    fn stock_log(&self) -> PathBuf {
        self.localappdata.join("Claude").join("Logs").join("main.log")
    }
    fn profile_log(&self, id: &str) -> PathBuf {
        self.house().join("profiles").join(id).join("logs").join("main.log")
    }
    fn exe(&self) -> PathBuf {
        self.localappdata.join("AnthropicClaude").join("app-4.23118.0").join("claude.exe")
    }
}

const LOGIN: &str = "claude://login/google-auth?code=c&hop_nonce=n";

#[test]
fn login_link_goes_to_profile_with_newest_login_start() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work"]));
    ok(&s.ch(&["add", "home"]));
    log_login(&s.stock_log(), 5);
    log_login(&s.profile_log("work"), 1);
    log_login(&s.profile_log("home"), 3);
    let profile = s.house().join("profiles").join("work");
    assert_eq!(
        ok(&s.ch(&["open-link", LOGIN, "--dry-run"])),
        format!("to: work\n\"{}\" \"--user-data-dir={}\" \"{LOGIN}\"\n", s.exe().display(), profile.display())
    );
}

#[test]
fn login_link_goes_to_stock_when_it_started_last() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work"]));
    log_login(&s.profile_log("work"), 4);
    log_login(&s.stock_log(), 1);
    assert_eq!(
        ok(&s.ch(&["open-link", LOGIN, "--dry-run"])),
        format!("to: stock\n\"{}\" \"{LOGIN}\"\n", s.exe().display())
    );
}

#[test]
fn stale_or_missing_logs_and_other_links_go_to_stock() {
    let s = Sandbox::new();
    ok(&s.ch(&["add", "work"]));
    assert!(ok(&s.ch(&["open-link", LOGIN, "--dry-run"])).starts_with("to: stock\n"));
    log_login(&s.profile_log("work"), 20);
    assert!(ok(&s.ch(&["open-link", LOGIN, "--dry-run"])).starts_with("to: stock\n"));
    log_login(&s.profile_log("work"), 1);
    assert!(ok(&s.ch(&["open-link", "claude://claude.ai/chat/1", "--dry-run"])).starts_with("to: stock\n"));
}

#[test]
fn open_link_rejects_non_claude_urls() {
    let s = Sandbox::new();
    assert!(err(&s.ch(&["open-link", "https://claude.ai", "--dry-run"])).contains("not a claude:// link"));
}

#[test]
fn corrupt_profile_list_is_reported_not_overwritten() {
    let s = Sandbox::new();
    std::fs::create_dir_all(s.house()).unwrap();
    let file = s.house().join("profiles.json");
    std::fs::write(&file, "{oops").unwrap();
    assert!(err(&s.ch(&["add", "work"])).contains("profiles.json"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "{oops");
}
