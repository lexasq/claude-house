//! Which account a Claude copy is signed into, read from its `config.json`.
//!
//! Only two fields are read: `windowSizeWasSignedIn` and `lastKnownAccountUuid`. The file also
//! holds the login token cache; it is never copied out (serde skips fields not in `ConfigFields`).

use serde::Deserialize;

/// Claude's internal account id. Opaque: only compared, never shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignIn {
    /// No `config.json` yet: the copy was never started.
    NeverOpened,
    /// Started, but nobody is signed in (never was, or signed out).
    SignedOut,
    SignedIn(AccountId),
    /// The file exists but is not the JSON we expect.
    Unreadable,
}

#[derive(Deserialize)]
struct ConfigFields {
    #[serde(rename = "windowSizeWasSignedIn")]
    signed_in: Option<bool>,
    #[serde(rename = "lastKnownAccountUuid")]
    account: Option<String>,
}

/// Sign-in state from the text of `config.json`, or `None` when the file does not exist.
pub fn sign_in_state(config: Option<&str>) -> SignIn {
    let Some(text) = config else { return SignIn::NeverOpened };
    match serde_json::from_str::<ConfigFields>(text) {
        Err(_) => SignIn::Unreadable,
        Ok(ConfigFields { signed_in: Some(true), account: Some(id) }) if !id.is_empty() => SignIn::SignedIn(AccountId(id)),
        Ok(_) => SignIn::SignedOut,
    }
}

/// For each key, the other keys signed into the same account. Keys with no shared account are left out.
pub fn shared_accounts<K: Clone>(states: &[(K, SignIn)]) -> Vec<(K, Vec<K>)> {
    let account = |s: &SignIn| match s {
        SignIn::SignedIn(id) => Some(id.clone()),
        _ => None,
    };
    states
        .iter()
        .enumerate()
        .filter_map(|(i, (key, state))| {
            let mine = account(state)?;
            let others: Vec<K> = states
                .iter()
                .enumerate()
                .filter(|(j, (_, other))| *j != i && account(other).as_ref() == Some(&mine))
                .map(|(_, (k, _))| k.clone())
                .collect();
            (!others.is_empty()).then(|| (key.clone(), others))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "11111111-1111-1111-1111-111111111111";
    const B: &str = "22222222-2222-2222-2222-222222222222";

    fn signed(id: &str) -> SignIn {
        sign_in_state(Some(&format!(r#"{{"windowSizeWasSignedIn":true,"lastKnownAccountUuid":"{id}"}}"#)))
    }

    #[test]
    fn signed_in_needs_flag_and_account() {
        assert_eq!(signed(A), SignIn::SignedIn(AccountId(A.into())));
        let with_tokens = format!(r#"{{"oauth:tokenCacheV2":"secret","windowSizeWasSignedIn":true,"lastKnownAccountUuid":"{A}"}}"#);
        assert_eq!(sign_in_state(Some(&with_tokens)), SignIn::SignedIn(AccountId(A.into())));
    }

    #[test]
    fn missing_flag_or_account_is_signed_out() {
        assert_eq!(sign_in_state(Some(r#"{"windowSizeWasSignedIn":false}"#)), SignIn::SignedOut);
        assert_eq!(sign_in_state(Some(&format!(r#"{{"windowSizeWasSignedIn":false,"lastKnownAccountUuid":"{A}"}}"#))), SignIn::SignedOut);
        assert_eq!(sign_in_state(Some(r#"{"windowSizeWasSignedIn":true}"#)), SignIn::SignedOut);
        assert_eq!(sign_in_state(Some(r#"{"windowSizeWasSignedIn":true,"lastKnownAccountUuid":""}"#)), SignIn::SignedOut);
        assert_eq!(sign_in_state(Some("{}")), SignIn::SignedOut);
    }

    #[test]
    fn no_file_is_never_opened_and_bad_json_is_unreadable() {
        assert_eq!(sign_in_state(None), SignIn::NeverOpened);
        assert_eq!(sign_in_state(Some("{oops")), SignIn::Unreadable);
        assert_eq!(sign_in_state(Some(r#"{"windowSizeWasSignedIn":"yes"}"#)), SignIn::Unreadable);
    }

    #[test]
    fn shared_accounts_lists_the_others_on_the_same_account() {
        let states = [
            ("main", signed(A)),
            ("work", signed(B)),
            ("second", signed(A)),
            ("third", signed(A)),
            ("new", SignIn::NeverOpened),
            ("out", SignIn::SignedOut),
        ];
        assert_eq!(
            shared_accounts(&states),
            vec![("main", vec!["second", "third"]), ("second", vec!["main", "third"]), ("third", vec!["main", "second"])]
        );
    }

    #[test]
    fn no_sharing_gives_nothing() {
        assert!(shared_accounts(&[("a", signed(A)), ("b", signed(B)), ("c", SignIn::SignedOut)]).is_empty());
    }
}
