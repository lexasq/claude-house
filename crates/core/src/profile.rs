use std::path::{Path, PathBuf};

/// A profile id: 1–32 chars of `a-z`, `0-9`, `-`, not starting or ending with `-`.
/// The only way to get one is `ProfileId::parse`, so every id in the program is valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileIdError {
    Empty,
    TooLong,
    BadChar(char),
    EdgeDash,
}

impl ProfileId {
    pub fn parse(raw: &str) -> Result<Self, ProfileIdError> {
        if raw.is_empty() {
            return Err(ProfileIdError::Empty);
        }
        if raw.len() > 32 {
            return Err(ProfileIdError::TooLong);
        }
        if let Some(c) = raw
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(ProfileIdError::BadChar(c));
        }
        if raw.starts_with('-') || raw.ends_with('-') {
            return Err(ProfileIdError::EdgeDash);
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `<house root>\profiles\<id>` — the folder Claude gets as `--user-data-dir`.
pub fn profile_dir(house_root: &Path, id: &ProfileId) -> PathBuf {
    house_root.join("profiles").join(id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lowercase_slug() {
        assert_eq!(ProfileId::parse("work-2").unwrap().as_str(), "work-2");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(ProfileId::parse(""), Err(ProfileIdError::Empty));
    }

    #[test]
    fn rejects_over_32_chars() {
        assert!(ProfileId::parse(&"a".repeat(32)).is_ok());
        assert_eq!(ProfileId::parse(&"a".repeat(33)), Err(ProfileIdError::TooLong));
    }

    #[test]
    fn rejects_uppercase_spaces_and_path_chars() {
        assert_eq!(ProfileId::parse("Work"), Err(ProfileIdError::BadChar('W')));
        assert_eq!(ProfileId::parse("a b"), Err(ProfileIdError::BadChar(' ')));
        assert_eq!(ProfileId::parse("..\\x"), Err(ProfileIdError::BadChar('.')));
    }

    #[test]
    fn rejects_leading_or_trailing_dash() {
        assert_eq!(ProfileId::parse("-a"), Err(ProfileIdError::EdgeDash));
        assert_eq!(ProfileId::parse("a-"), Err(ProfileIdError::EdgeDash));
    }

    #[test]
    fn profile_dir_is_under_profiles() {
        let id = ProfileId::parse("work").unwrap();
        assert_eq!(
            profile_dir(Path::new(r"C:\H"), &id),
            Path::new(r"C:\H").join("profiles").join("work")
        );
    }
}
