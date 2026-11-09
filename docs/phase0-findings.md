# Phase 0 — feasibility findings

Claude Desktop under test: `%LOCALAPPDATA%\AnthropicClaude\app-4.23118.0` (Squirrel install, Electron).

## Settled

| Question | Answer | Evidence |
|---|---|---|
| Is the single-copy lock per data folder? | **Yes** | `app.asar`: the lock code reads `app.getPath("userData")`, then calls `requestSingleInstanceLock()` |
| `CLAUDE_USER_DATA_DIR` env override? | **No — ignored in signed builds** | `app.asar` logs: "CLAUDE_USER_DATA_DIR is set and signed builds ignore it" |
| `--user-data-dir=<dir>` switch? | **Works** | 2026-11-09: launched `claude.exe --user-data-dir=%LOCALAPPDATA%\ClaudeHouse-phase0\profileA`. A second window (own PID) opened next to the running copy and filled the new folder (`Local State`, `config.json`, `lockfile`, `logs\`…) |
| Does a second copy re-register `claude://`? | **No** | `HKCU\Software\Classes\claude\shell\open\command` stays `"…\app-4.23118.0\claude.exe" "%1"` after launch — no profile argument |

## Login routing (tested 2026-11-09)

- Google sign-in runs in the system browser (`[Auth] Using system browser for: /login/app-google-auth`) and returns through `claude://login/google-auth?code=…&hop_nonce=…`.
- With the stock handler, that link goes to the default copy, and the isolated copy stays stuck on the login screen. **Confirmed.**
- Running `claude.exe --user-data-dir=<profile dir> "<claude:// link>"` while that profile's copy is running hands the link to that copy, and the login completes there. **Confirmed.** Electron's single-instance handoff works per data folder.
- So phase 3 is needed, and its design holds: Claude House owns `claude://` and relaunches `claude.exe --user-data-dir=<profile that started the login> "%1"`.
- Changing `HKCU\Software\Classes\claude` is a persistent change that needs the user's approval (auto mode blocked it). The installer, or an explicit user action, must make it, and uninstall must restore it.

## Log folder depends on where the data folder is (found 2026-11-09)

- On Windows, if the data folder is under `%APPDATA%`, Claude calls `setAppLogsPath(join(LOCALAPPDATA, basename(userData), "logs"))`. The stock copy (`%APPDATA%\Claude`) logs to `%LOCALAPPDATA%\Claude\Logs`; a profile at `%APPDATA%\ClaudeHouse\profiles\second` logged to `%LOCALAPPDATA%\second\logs`.
- If the data folder is under `%LOCALAPPDATA%`, logs stay in `<data folder>\logs` (the phase 0 test profile).
- Consequence: Claude House keeps its data in `%LOCALAPPDATA%\ClaudeHouse`. With the first layout, login routing never saw a profile's login start and sent every login to the stock copy, and a profile id like `temp` would have logged into `%LOCALAPPDATA%\Temp`.

## Open

1. ~~Does login finish inside the isolated copy?~~ Yes, with routing (above).
2. Does the session survive a restart of the isolated copy?
3. Do the Code tab and Cowork stay separate? (`~/.claude` is shared unless `CLAUDE_CONFIG_DIR` is set per profile.)
4. Do two copies clash during a Squirrel update?

## Consequence for the design

- Launch contract: `app-<newest>\claude.exe --user-data-dir=<profile dir>`. No env override needed for the data folder.
- `claude://` routing (phase 3) is required for Google sign-in. Open design point: which profile gets a link when several are waiting on a login. Simplest rule: only one login may be pending at a time, and the link goes to that profile.
