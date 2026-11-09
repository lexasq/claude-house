# Phase 0 — feasibility findings

Claude Desktop under test: `%LOCALAPPDATA%\AnthropicClaude\app-4.23118.0` (Squirrel install, Electron).

## Settled

| Question | Answer | Evidence |
|---|---|---|
| Is the single-copy lock per data folder? | **Yes** | `app.asar`: the lock code reads `app.getPath("userData")`, then calls `requestSingleInstanceLock()` |
| `CLAUDE_USER_DATA_DIR` env override? | **No — ignored in signed builds** | `app.asar` logs: "CLAUDE_USER_DATA_DIR is set and signed builds ignore it" |
| `--user-data-dir=<dir>` switch? | **Works** | 2026-11-09: launched `claude.exe --user-data-dir=%LOCALAPPDATA%\ClaudeHouse-phase0\profileA`. A second window (own PID) opened next to the running copy and filled the new folder (`Local State`, `config.json`, `lockfile`, `logs\`…) |
| Does a second copy re-register `claude://`? | **No** | `HKCU\Software\Classes\claude\shell\open\command` stays `"…\app-4.23118.0\claude.exe" "%1"` after launch — no profile argument |

## Open (needs a real login)

1. Does login in the isolated copy finish inside that copy, or does it bounce through a `claude://` link to the default copy?
2. Does the session survive a restart of the isolated copy?
3. Do the Code tab and Cowork stay separate? (`~/.claude` is shared unless `CLAUDE_CONFIG_DIR` is set per profile.)
4. Do two copies clash during a Squirrel update?

## Consequence for the design

- Launch contract: `app-<newest>\claude.exe --user-data-dir=<profile dir>`. No env override needed for the data folder.
- `claude://` routing (phase 3) is needed only if answer 1 is "bounces to the default copy".
