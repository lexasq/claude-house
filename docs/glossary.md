# Glossary

One name per concept. Code, CLI, UI and docs use these words.

| Term | Meaning | In code |
|---|---|---|
| **Claude House** | This app. Starts and manages several Claude Desktop copies, one per account. | |
| **Copy** | One running Claude Desktop with its own data folder. | |
| **Main account** | The Claude Desktop the user already had: data in `%APPDATA%\Claude`, started by its own shortcut, not by Claude House. Kept exactly as it is (same sign-in, same sessions). Shown only once Claude Desktop has been used on the machine. | `Target::Main`, `MAIN_NAME`, `main_spec` |
| **Profile** | Every other account, set up in Claude House. Has an id, a display name, a colour and an "isolate Code tab" flag. The UI calls profiles *accounts*. | `Profile`, `profile_spec` |
| **Profile id** | Short lowercase slug (`a-z`, `0-9`, `-`, 1–32 chars). Never changes. The profile folder is derived from it. | `ProfileId` |
| **House root** | `%LOCALAPPDATA%\ClaudeHouse`. Holds `profiles.json`, `profiles\<id>\`, `routing-enabled`, `ch-link.log`. Must not be under `%APPDATA%`: Claude moves the logs of a data folder there to `%LOCALAPPDATA%\<folder name>\logs`. | `House::root` |
| **Profile folder** | `<house root>\profiles\<id>`. Passed to Claude as `--user-data-dir`. Two profiles can never share one, because it comes from the unique id. | `profile_dir` |
| **Code config folder** | `<profile folder>\claude-code`. Passed as `CLAUDE_CONFIG_DIR` when a profile isolates its Code tab settings from `~/.claude`. | `code_config_dir` |
| **Claude install** | `%LOCALAPPDATA%\AnthropicClaude`. Holds side-by-side `app-<version>` folders. | `install_root` |
| **App folder** | One `app-<major>.<minor>.<patch>` folder inside the Claude install. Claude House launches the newest. | `AppVersion`, `newest_app` |
| **Launch spec** | The program, arguments and environment for starting one copy (or handing it a link). Built by a pure function. | `LaunchSpec` |
| **Claude link** | A `claude://` URL handed to Claude by Windows. A **login link** has host `login` (e.g. `claude://login/google-auth?code=…`) and carries a one-time code: never logged or echoed. | `ClaudeLink` |
| **Login start** | A `[Auth] Using system browser for: /login/…` line in a copy's `main.log`. Profiles log to `<profile folder>\logs\main.log`; the Main account to `%LOCALAPPDATA%\Claude\Logs\main.log`. | `last_login_start` |
| **Route** | Where a Claude link goes. A login link goes to the copy with the newest login start (no time limit); other links, and logins nobody started, go to the Main account. | `route`, `Target` |
| **Login routing** | The feature: Google sign-in reaches the copy that started it. Needs Claude House to own the `claude://` handler. Off until the user turns it on (remembered in `routing-enabled`). | `routing_wanted` |
| **Handler** | The `claude://` open command in `HKCU\Software\Classes\claude\shell\open\command`. Claude Desktop re-registers itself there every time `claude.exe` starts. | `handler_command`, `registry.rs` |
| **Handler guard** | The loop that keeps the handler pointed at `ch-link.exe`, putting it back whenever Claude takes it. Runs inside the app while login routing is on, or in a terminal as `ch watch`. | `guard_handler`, `HandlerGuard` |
| **ch-link** | `ch-link.exe`, the windowless program Windows runs for each `claude://` link while login routing is on. Must stay next to `claude-house.exe` / `ch.exe`. | `ch_link_main`, `link_exe` |
| **Sign-in state** | Never opened / not signed in / signed in / unreadable, from two fields of a copy's `config.json`. | `SignIn`, `sign_in_state` |
