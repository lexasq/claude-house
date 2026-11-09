# Glossary

| Term | Meaning |
|---|---|
| **Claude House** | This app. Starts and manages several Claude Desktop copies, one per account. |
| **Profile** | One Claude account set up in Claude House. Has an id, a display name, a colour and an "isolate Code tab" flag. |
| **Profile id** | Short lowercase slug (`a-z`, `0-9`, `-`, 1–32 chars). Never changes. The profile folder is derived from it. |
| **House root** | `%LOCALAPPDATA%\ClaudeHouse`. Holds `profiles.json`, `profiles\<id>\`, `routing-enabled`, `ch-link.log`. Must not be under `%APPDATA%`: Claude moves the logs of a data folder there to `%LOCALAPPDATA%\<folder name>\logs`. |
| **Profile folder** | `<house root>\profiles\<id>`. Passed to Claude as `--user-data-dir`. Two profiles can never share one, because it comes from the unique id. |
| **Code config folder** | `<profile folder>\claude-code`. Passed as `CLAUDE_CONFIG_DIR` when the profile isolates its Code tab settings from `~/.claude`. |
| **Claude install** | `%LOCALAPPDATA%\AnthropicClaude`. Holds side-by-side `app-<version>` folders. |
| **App folder** | One `app-<major>.<minor>.<patch>` folder inside the Claude install. Claude House launches the newest. |
| **Launch spec** | The program, arguments and environment for starting one profile's copy. Built by a pure function. |
| **Claude link** | A `claude://` URL handed to Claude by Windows. A **login link** has host `login` (e.g. `claude://login/google-auth?code=…`). |
| **Login start** | A `[Auth] Using system browser for: /login/…` line in a copy's `main.log`. Profiles log to `<profile folder>\logs\main.log`; the stock copy to `%LOCALAPPDATA%\Claude\Logs\main.log`. |
| **Stock copy / Main Claude** | The ordinary Claude Desktop copy using `%APPDATA%\Claude`, not managed as a profile. In the intended flow every account is a profile and Main Claude stays closed. |
| **Target / route** | Where a Claude link goes: a profile or the stock copy. A login link goes to the copy with the newest login start (no time limit); other links, and logins nobody started, go to the stock copy. |
