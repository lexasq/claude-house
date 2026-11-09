# Glossary

| Term | Meaning |
|---|---|
| **Claude House** | This app. Starts and manages several Claude Desktop copies, one per account. |
| **Profile** | One Claude account set up in Claude House. Has an id, a display name, a colour and an "isolate Code tab" flag. |
| **Profile id** | Short lowercase slug (`a-z`, `0-9`, `-`, 1–32 chars). Never changes. The profile folder is derived from it. |
| **House root** | `%APPDATA%\ClaudeHouse`. Holds `profiles.json` and `profiles\<id>\`. |
| **Profile folder** | `<house root>\profiles\<id>`. Passed to Claude as `--user-data-dir`. Two profiles can never share one, because it comes from the unique id. |
| **Code config folder** | `<profile folder>\claude-code`. Passed as `CLAUDE_CONFIG_DIR` when the profile isolates its Code tab settings from `~/.claude`. |
| **Claude install** | `%LOCALAPPDATA%\AnthropicClaude`. Holds side-by-side `app-<version>` folders. |
| **App folder** | One `app-<major>.<minor>.<patch>` folder inside the Claude install. Claude House launches the newest. |
| **Launch spec** | The program, arguments and environment for starting one profile's copy. Built by a pure function. |
| **Claude link** | A `claude://` URL handed to Claude by Windows. A **login link** has host `login` (e.g. `claude://login/google-auth?code=…`). |
| **Pending login** | The one profile currently waiting for a login link. At most one at a time. |
| **Route** | Where a Claude link goes: to a profile, or to the stock (default) Claude copy. |
