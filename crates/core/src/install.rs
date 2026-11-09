/// Version of one `app-<major>.<minor>.<patch>` folder. Field order makes the derived `Ord` numeric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppVersion(pub u32, pub u32, pub u32);

impl AppVersion {
    /// Parses a folder name such as `app-4.23118.0`. Anything else is `None`.
    pub fn from_dir_name(name: &str) -> Option<Self> {
        let mut parts = name.strip_prefix("app-")?.split('.').map(|p| p.parse::<u32>().ok());
        let v = AppVersion(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(v)
    }

    pub fn dir_name(&self) -> String {
        format!("app-{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The newest app folder among the names found in the Claude install.
pub fn newest_app<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<AppVersion> {
    names.into_iter().filter_map(AppVersion::from_dir_name).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app_folder_name() {
        assert_eq!(AppVersion::from_dir_name("app-4.23118.0"), Some(AppVersion(4, 23118, 0)));
    }

    #[test]
    fn rejects_other_names() {
        for n in ["packages", "app-4.1", "app-4.1.0.0", "app-x.1.0", "xapp-4.1.0", "app-4.1.-1"] {
            assert_eq!(AppVersion::from_dir_name(n), None, "{n}");
        }
    }

    #[test]
    fn newest_compares_numerically_not_as_text() {
        let names = ["app-4.16336.0", "app-4.23118.0", "app-4.4212.0", "packages", "Update.exe"];
        assert_eq!(newest_app(names), Some(AppVersion(4, 23118, 0)));
    }

    #[test]
    fn newest_of_nothing_is_none() {
        assert_eq!(newest_app(["packages"]), None);
    }

    #[test]
    fn dir_name_round_trips() {
        assert_eq!(AppVersion(4, 23118, 0).dir_name(), "app-4.23118.0");
    }
}
