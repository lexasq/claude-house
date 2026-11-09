//! Telling Claude copies apart from a list of running processes.

use std::path::Path;

/// What we know about one running process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub exe: String,
    pub cmd: String,
}

/// Whether the main (stock) Claude Desktop is running: a Claude Desktop browser process
/// (program under the install folder, no Chromium `--type=`) that is not one of our profiles.
/// Claude Code's own `claude.exe` lives elsewhere and does not count.
pub fn main_claude_running(processes: &[ProcessInfo], install_root: &Path, house_root: &Path) -> bool {
    let install = format!("{}\\", install_root.display()).to_lowercase();
    let profiles = house_root.join("profiles").display().to_string().to_lowercase();
    processes.iter().any(|p| {
        let cmd = p.cmd.to_lowercase();
        p.exe.to_lowercase().starts_with(&install) && !cmd.contains("--type=") && !cmd.contains(&profiles)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALL: &str = r"C:\Users\x\AppData\Local\AnthropicClaude";
    const HOUSE: &str = r"C:\Users\x\AppData\Local\ClaudeHouse";
    const APP_EXE: &str = r"C:\Users\x\AppData\Local\AnthropicClaude\app-4.23118.0\claude.exe";

    fn p(exe: &str, cmd: &str) -> ProcessInfo {
        ProcessInfo { exe: exe.into(), cmd: cmd.into() }
    }

    fn running(ps: &[ProcessInfo]) -> bool {
        main_claude_running(ps, Path::new(INSTALL), Path::new(HOUSE))
    }

    #[test]
    fn main_browser_process_counts() {
        assert!(running(&[p(APP_EXE, &format!("\"{APP_EXE}\""))]));
    }

    #[test]
    fn install_folder_is_matched_ignoring_case() {
        let upper = APP_EXE.to_uppercase();
        assert!(running(&[p(&upper, "claude.exe")]));
    }

    #[test]
    fn helpers_profiles_and_other_programs_do_not_count() {
        let profile = format!("\"{APP_EXE}\" \"--user-data-dir={HOUSE}\\profiles\\work\"");
        let profile_lower = profile.to_lowercase();
        let helper = format!("\"{APP_EXE}\" --type=renderer --user-data-dir=C:\\Users\\x\\AppData\\Roaming\\Claude");
        let code_cli = r"C:\Users\x\AppData\Roaming\Claude\claude-code\2.1.317\a\claude.exe";
        let elsewhere = r"C:\Users\x\AppData\Local\AnthropicClaudeOld\app-1.0.0\claude.exe";
        assert!(!running(&[
            p(APP_EXE, &profile),
            p(APP_EXE, &profile_lower),
            p(APP_EXE, &helper),
            p(code_cli, code_cli),
            p(elsewhere, elsewhere),
        ]));
        assert!(!running(&[]));
    }
}
