//! `claude://` links and where they go.
//!
//! Claude House cannot see a login button being clicked. Every Claude copy logs
//! `[Auth] Using system browser for: /login/…` when it sends a login to the browser,
//! so the copy with the newest such line (within `LOGIN_WINDOW_SECS`) gets the login link.
//! Stateless: a crash or a second run gives the same answer.

use crate::profile::ProfileId;

/// How long after a login starts its link may still be routed to that copy.
pub const LOGIN_WINDOW_SECS: i64 = 15 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    Login,
    Other,
}

/// A `claude://` URL received from Windows. Only `ClaudeLink::parse` makes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeLink {
    raw: String,
    kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    NotClaudeScheme,
}

impl ClaudeLink {
    pub fn parse(raw: &str) -> Result<Self, LinkError> {
        const SCHEME: &str = "claude://";
        let rest = raw
            .get(..SCHEME.len())
            .filter(|s| s.eq_ignore_ascii_case(SCHEME))
            .map(|_| &raw[SCHEME.len()..])
            .ok_or(LinkError::NotClaudeScheme)?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let kind = if host.eq_ignore_ascii_case("login") { LinkKind::Login } else { LinkKind::Other };
        Ok(Self { raw: raw.to_owned(), kind })
    }

    pub fn kind(&self) -> &LinkKind {
        &self.kind
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// A Claude copy a link can go to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The stock Claude Desktop copy (`%APPDATA%\Claude`).
    Stock,
    Profile(ProfileId),
}

/// Seconds since 1970-01-01 00:00:00 in the log's own (local) clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogTime(pub i64);

impl LogTime {
    /// Parses `YYYY-MM-DD HH:MM:SS` at the start of a log line.
    pub fn parse(s: &str) -> Option<Self> {
        let b = s.as_bytes().get(..19)?;
        let sep_ok = b[4] == b'-' && b[7] == b'-' && b[10] == b' ' && b[13] == b':' && b[16] == b':';
        let num = |r: std::ops::Range<usize>| -> Option<i64> {
            b[r].iter().try_fold(0i64, |acc, c| c.is_ascii_digit().then(|| acc * 10 + i64::from(c - b'0')))
        };
        let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
        let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
        let in_range = (1..=12).contains(&mo) && (1..=31).contains(&d) && h < 24 && mi < 60 && se < 60;
        (sep_ok && in_range).then(|| LogTime(days_from_civil(y, mo, d) * 86_400 + h * 3_600 + mi * 60 + se))
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Time of the last login start in one copy's `main.log`.
pub fn last_login_start(log: &str) -> Option<LogTime> {
    log.lines()
        .filter(|l| l.contains("[Auth] Using system browser for: /login/"))
        .filter_map(LogTime::parse)
        .max()
}

/// Where a link goes, given each copy's last login start and the current time.
pub fn route(link: &ClaudeLink, starts: &[(Target, LogTime)], now: LogTime) -> Target {
    if link.kind != LinkKind::Login {
        return Target::Stock;
    }
    starts
        .iter()
        .filter(|(_, at)| *at <= now && now.0 - at.0 <= LOGIN_WINDOW_SECS)
        .max_by_key(|(_, at)| *at)
        .map_or(Target::Stock, |(target, _)| target.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str) -> Target {
        Target::Profile(ProfileId::parse(id).unwrap())
    }
    fn t(s: &str) -> LogTime {
        LogTime::parse(s).unwrap()
    }
    fn login() -> ClaudeLink {
        ClaudeLink::parse("claude://login/google-auth?code=x&hop_nonce=y").unwrap()
    }

    #[test]
    fn parses_login_and_other_links() {
        assert_eq!(login().kind(), &LinkKind::Login);
        assert_eq!(ClaudeLink::parse("CLAUDE://login").unwrap().kind(), &LinkKind::Login);
        assert_eq!(ClaudeLink::parse("claude://claude.ai/chat/1").unwrap().kind(), &LinkKind::Other);
        assert_eq!(ClaudeLink::parse("claude://loginx/a").unwrap().kind(), &LinkKind::Other);
        assert_eq!(login().as_str(), "claude://login/google-auth?code=x&hop_nonce=y");
    }

    #[test]
    fn rejects_non_claude_urls() {
        for raw in ["https://claude.ai", "claude:/login", "", "file://x"] {
            assert_eq!(ClaudeLink::parse(raw), Err(LinkError::NotClaudeScheme), "{raw}");
        }
    }

    #[test]
    fn log_time_is_seconds_since_epoch() {
        assert_eq!(t("1970-01-01 00:00:00"), LogTime(0));
        assert_eq!(t("2026-11-09 11:18:08"), LogTime(1_794_223_088));
        assert_eq!(t("2000-03-01 00:00:00").0 - t("2000-02-28 00:00:00").0, 2 * 86_400);
        assert_eq!(t("1970-01-01 23:59:59"), LogTime(86_399));
        for bad in ["", "2026-11-09", "2026-13-01 00:00:00", "2026-11-09 24:00:00", "2026-11-09 23:60:00", "2026-11-09 23:59:60", "2026-11-09T11:18:08", "x026-11-09 11:18:08"] {
            assert_eq!(LogTime::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn finds_last_login_start_in_log() {
        let log = "2026-11-09 11:18:08 [info] [Auth] Using system browser for: /login/app-google-auth\n\
                   2026-11-09 11:19:00 [info] [updater] Update URL: x\n\
                   2026-11-09 11:21:18 [info] [Auth] Using system browser for: /login/app-google-auth\n\
                   2026-11-09 11:22:00 [info] [Auth] something else\n";
        assert_eq!(last_login_start(log), Some(t("2026-11-09 11:21:18")));
        assert_eq!(last_login_start("2026-11-09 11:22:00 [info] hello\n"), None);
    }

    #[test]
    fn login_goes_to_newest_recent_start() {
        let now = t("2026-11-09 12:00:00");
        let starts = [(Target::Stock, t("2026-11-09 11:50:00")), (p("work"), t("2026-11-09 11:58:00")), (p("home"), t("2026-11-09 11:55:00"))];
        assert_eq!(route(&login(), &starts, now), p("work"));
    }

    #[test]
    fn stale_starts_fall_back_to_stock() {
        let now = t("2026-11-09 12:00:00");
        let edge = LogTime(now.0 - LOGIN_WINDOW_SECS);
        assert_eq!(route(&login(), &[(p("work"), edge)], now), p("work"));
        assert_eq!(route(&login(), &[(p("work"), LogTime(edge.0 - 1))], now), Target::Stock);
        assert_eq!(route(&login(), &[], now), Target::Stock);
    }

    #[test]
    fn future_starts_are_ignored() {
        let now = t("2026-11-09 12:00:00");
        assert_eq!(route(&login(), &[(p("work"), LogTime(now.0 + 1))], now), Target::Stock);
    }

    #[test]
    fn other_links_go_to_stock() {
        let now = t("2026-11-09 12:00:00");
        let other = ClaudeLink::parse("claude://claude.ai/chat/1").unwrap();
        assert_eq!(route(&other, &[(p("work"), now)], now), Target::Stock);
    }
}
