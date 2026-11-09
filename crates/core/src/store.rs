//! `profiles.json`: the profile list. Claude House is its only writer.
//! Outside data is checked once here; the rest of the program gets valid `Profile`s.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::profile::{Color, Profile, ProfileId, ProfileName};

const FORMAT_VERSION: u32 = 1;

/// Profiles with unique ids, in display order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileList(Vec<Profile>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateId(ProfileId);

impl ProfileList {
    pub fn add(&mut self, profile: Profile) -> Result<(), DuplicateId> {
        if self.get(&profile.id).is_some() {
            return Err(DuplicateId(profile.id));
        }
        self.0.push(profile);
        Ok(())
    }

    /// Removes the profile with `id`. Removing a missing id changes nothing.
    pub fn remove(&mut self, id: &ProfileId) {
        self.0.retain(|p| &p.id != id);
    }

    pub fn get(&self, id: &ProfileId) -> Option<&Profile> {
        self.0.iter().find(|p| &p.id == id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.0.iter()
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Format(String),
}

#[derive(Serialize, Deserialize)]
struct FileDto {
    version: u32,
    profiles: Vec<ProfileDto>,
}

#[derive(Serialize, Deserialize)]
struct ProfileDto {
    id: String,
    name: String,
    color: String,
    isolate_code: bool,
}

fn bad(msg: String) -> StoreError {
    StoreError::Format(msg)
}

impl TryFrom<ProfileDto> for Profile {
    type Error = StoreError;

    fn try_from(d: ProfileDto) -> Result<Self, StoreError> {
        Ok(Profile {
            id: ProfileId::parse(&d.id).map_err(|e| bad(format!("profile id {:?}: {e:?}", d.id)))?,
            name: ProfileName::parse(&d.name).ok_or_else(|| bad(format!("profile {:?}: bad name", d.id)))?,
            color: Color::parse(&d.color).ok_or_else(|| bad(format!("profile {:?}: bad color", d.id)))?,
            isolate_code: d.isolate_code,
        })
    }
}

fn parse_list(json: &str) -> Result<ProfileList, StoreError> {
    let file: FileDto = serde_json::from_str(json).map_err(|e| bad(e.to_string()))?;
    if file.version != FORMAT_VERSION {
        return Err(bad(format!("unsupported version {}", file.version)));
    }
    let mut list = ProfileList::default();
    for dto in file.profiles {
        list.add(Profile::try_from(dto)?).map_err(|DuplicateId(id)| bad(format!("duplicate id {:?}", id.as_str())))?;
    }
    Ok(list)
}

fn render_list(list: &ProfileList) -> String {
    let profiles = list
        .iter()
        .map(|p| ProfileDto {
            id: p.id.as_str().to_owned(),
            name: p.name.as_str().to_owned(),
            color: p.color.as_str().to_owned(),
            isolate_code: p.isolate_code,
        })
        .collect();
    serde_json::to_string_pretty(&FileDto { version: FORMAT_VERSION, profiles }).expect("plain strings and bools always serialize")
}

/// Reads the list. A missing file is an empty list.
pub fn load(path: &Path) -> Result<ProfileList, StoreError> {
    match std::fs::read_to_string(path) {
        Ok(json) => parse_list(&json),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProfileList::default()),
        Err(e) => Err(StoreError::Io(e)),
    }
}

/// Writes `<path>.tmp`, then renames it over `path`, so a crash leaves the old or the new list, never half of one.
pub fn save(path: &Path, list: &ProfileList) -> Result<(), StoreError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let write = || -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(render_list(list).as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    };
    write().map_err(StoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prof(id: &str, name: &str) -> Profile {
        Profile {
            id: ProfileId::parse(id).unwrap(),
            name: ProfileName::parse(name).unwrap(),
            color: Color::parse("#d97757").unwrap(),
            isolate_code: id == "work",
        }
    }

    fn two() -> ProfileList {
        let mut l = ProfileList::default();
        l.add(prof("work", "Work")).unwrap();
        l.add(prof("home", "Home")).unwrap();
        l
    }

    fn ids(l: &ProfileList) -> Vec<&str> {
        l.iter().map(|p| p.id.as_str()).collect()
    }

    #[test]
    fn add_keeps_order_and_rejects_duplicate_ids() {
        let mut l = two();
        assert_eq!(ids(&l), ["work", "home"]);
        assert_eq!(l.add(prof("work", "Again")), Err(DuplicateId(ProfileId::parse("work").unwrap())));
        assert_eq!(l.get(&ProfileId::parse("work").unwrap()).unwrap().name.as_str(), "Work");
    }

    #[test]
    fn remove_is_idempotent() {
        let mut l = two();
        let work = ProfileId::parse("work").unwrap();
        l.remove(&work);
        l.remove(&work);
        assert_eq!(ids(&l), ["home"]);
    }

    #[test]
    fn render_then_parse_round_trips() {
        assert_eq!(parse_list(&render_list(&two())).unwrap(), two());
    }

    #[test]
    fn parse_rejects_bad_fields_duplicates_and_versions() {
        let bad = [
            r##"{"version":1,"profiles":[{"id":"Bad","name":"x","color":"#000000","isolate_code":false}]}"##,
            r##"{"version":1,"profiles":[{"id":"a","name":" ","color":"#000000","isolate_code":false}]}"##,
            r##"{"version":1,"profiles":[{"id":"a","name":"x","color":"red","isolate_code":false}]}"##,
            r##"{"version":1,"profiles":[{"id":"a","name":"x","color":"#000000","isolate_code":false},{"id":"a","name":"y","color":"#000000","isolate_code":false}]}"##,
            r##"{"version":2,"profiles":[]}"##,
            "not json",
        ];
        for json in bad {
            assert!(matches!(parse_list(json), Err(StoreError::Format(_))), "{json}");
        }
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("profiles.json")).unwrap(), ProfileList::default());
    }

    #[test]
    fn unreadable_file_is_an_io_error_not_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(load(dir.path()), Err(StoreError::Io(_))));
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");
        save(&path, &ProfileList::default()).unwrap();
        save(&path, &two()).unwrap();
        assert_eq!(load(&path).unwrap(), two());
        assert!(!dir.path().join("profiles.json.tmp").exists());
    }

    #[test]
    fn leftover_temp_file_from_a_crash_does_not_break_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profiles.json");
        std::fs::write(dir.path().join("profiles.json.tmp"), "half-writ").unwrap();
        save(&path, &two()).unwrap();
        assert_eq!(load(&path).unwrap(), two());
    }
}
