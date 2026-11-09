//! What to run to start a Claude copy, or to hand it a link.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::install::AppVersion;
use crate::link::ClaudeLink;
use crate::profile::{profile_dir, Profile};

/// Program, arguments and extra environment for one `claude.exe` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
}

/// `<profile folder>\claude-code` — the Code tab's config folder for an isolated profile.
pub fn code_config_dir(house_root: &Path, profile: &Profile) -> PathBuf {
    profile_dir(house_root, &profile.id).join("claude-code")
}

/// `<install root>\app-<version>\claude.exe`.
pub fn claude_exe(install_root: &Path, app: AppVersion) -> PathBuf {
    install_root.join(app.dir_name()).join("claude.exe")
}

/// Start a profile's copy, or, if it is already running, hand it `link`.
pub fn profile_spec(
    install_root: &Path,
    app: AppVersion,
    house_root: &Path,
    profile: &Profile,
    link: Option<&ClaudeLink>,
) -> LaunchSpec {
    let mut data_dir = OsString::from("--user-data-dir=");
    data_dir.push(profile_dir(house_root, &profile.id));
    let args = std::iter::once(data_dir).chain(link.map(|l| OsString::from(l.as_str()))).collect();
    let env = if profile.isolate_code {
        vec![(OsString::from("CLAUDE_CONFIG_DIR"), code_config_dir(house_root, profile).into_os_string())]
    } else {
        vec![]
    };
    LaunchSpec { program: claude_exe(install_root, app), args, env }
}

/// Hand `link` to the stock copy, exactly as the stock `claude://` handler would.
pub fn stock_spec(install_root: &Path, app: AppVersion, link: &ClaudeLink) -> LaunchSpec {
    LaunchSpec { program: claude_exe(install_root, app), args: vec![OsString::from(link.as_str())], env: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Color, ProfileId, ProfileName};

    const INSTALL: &str = r"C:\L\AnthropicClaude";
    const HOUSE: &str = r"C:\R\ClaudeHouse";
    const APP: AppVersion = AppVersion(4, 23118, 0);

    fn profile(isolate_code: bool) -> Profile {
        Profile {
            id: ProfileId::parse("work").unwrap(),
            name: ProfileName::parse("Work").unwrap(),
            color: Color::parse("#d97757").unwrap(),
            isolate_code,
        }
    }

    fn exe() -> PathBuf {
        Path::new(INSTALL).join("app-4.23118.0").join("claude.exe")
    }

    fn data_dir_arg() -> OsString {
        let dir = Path::new(HOUSE).join("profiles").join("work");
        OsString::from(format!("--user-data-dir={}", dir.display()))
    }

    #[test]
    fn profile_launch_points_claude_at_profile_folder() {
        let spec = profile_spec(Path::new(INSTALL), APP, Path::new(HOUSE), &profile(false), None);
        assert_eq!(spec, LaunchSpec { program: exe(), args: vec![data_dir_arg()], env: vec![] });
    }

    #[test]
    fn isolated_profile_gets_its_own_code_config() {
        let spec = profile_spec(Path::new(INSTALL), APP, Path::new(HOUSE), &profile(true), None);
        let code = Path::new(HOUSE).join("profiles").join("work").join("claude-code");
        assert_eq!(spec.env, vec![(OsString::from("CLAUDE_CONFIG_DIR"), code.into_os_string())]);
    }

    #[test]
    fn link_is_passed_after_the_data_folder() {
        let link = ClaudeLink::parse("claude://login/google-auth?code=a&b=c").unwrap();
        let spec = profile_spec(Path::new(INSTALL), APP, Path::new(HOUSE), &profile(false), Some(&link));
        assert_eq!(spec.args, vec![data_dir_arg(), OsString::from(link.as_str())]);
    }

    #[test]
    fn stock_gets_only_the_link() {
        let link = ClaudeLink::parse("claude://login/x").unwrap();
        let spec = stock_spec(Path::new(INSTALL), APP, &link);
        assert_eq!(spec, LaunchSpec { program: exe(), args: vec![OsString::from("claude://login/x")], env: vec![] });
    }
}
