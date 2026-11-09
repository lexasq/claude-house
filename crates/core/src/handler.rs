//! The `claude://` handler command in `HKCU\Software\Classes\claude\shell\open\command`.
//!
//! Every `claude.exe` start re-registers itself there (Claude passes `keepExistingDefault: false`),
//! so Claude House cannot set it once; a watcher must put it back after each change.
//! Claude Desktop's own value is always `"<claude.exe>" "%1"`, so restoring needs no backup.

use std::path::Path;

/// The registry command that sends `claude://` links to `exe`.
pub fn handler_command(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

/// Whether the registered command already sends links to `exe` (Windows paths ignore case).
pub fn is_handled_by(current: Option<&str>, exe: &Path) -> bool {
    current.is_some_and(|c| c.eq_ignore_ascii_case(&handler_command(exe)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Users\x\ClaudeHouse\ch-link.exe";

    #[test]
    fn command_quotes_exe_and_link() {
        assert_eq!(handler_command(Path::new(EXE)), r#""C:\Users\x\ClaudeHouse\ch-link.exe" "%1""#);
    }

    #[test]
    fn recognises_own_command_ignoring_case() {
        let ours = handler_command(Path::new(EXE));
        assert!(is_handled_by(Some(&ours), Path::new(EXE)));
        assert!(is_handled_by(Some(&ours.to_uppercase()), Path::new(EXE)));
    }

    #[test]
    fn claude_desktop_or_missing_command_is_not_ours() {
        let claude_desktop = r#""C:\L\AnthropicClaude\app-4.23118.0\claude.exe" "%1""#;
        assert!(!is_handled_by(Some(claude_desktop), Path::new(EXE)));
        assert!(!is_handled_by(None, Path::new(EXE)));
    }
}
