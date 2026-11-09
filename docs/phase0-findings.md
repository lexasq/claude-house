# How Claude Desktop behaves — what Claude House relies on

Facts found while building Claude House, with how each was checked. Claude Desktop is an Electron app
installed with Squirrel; checked against `%LOCALAPPDATA%\AnthropicClaude\app-4.23118.0` (November 2026).
A Claude Desktop update can change any of these; re-check them when something stops working.

## Running several copies

| Question | Answer | Evidence |
|---|---|---|
| Is the "only one copy" lock per data folder? | **Yes** | `app.asar`: the lock code reads `app.getPath("userData")`, then calls `requestSingleInstanceLock()`. |
| Can the data folder be set with `CLAUDE_USER_DATA_DIR`? | **No**, signed builds ignore it | `app.asar` logs "CLAUDE_USER_DATA_DIR is set and signed builds ignore it". |
| Can it be set with `--user-data-dir=<dir>`? | **Yes** | `claude.exe --user-data-dir=<folder>` opened a second window next to the running one and filled the new folder (`Local State`, `config.json`, `lockfile`, `logs\`…). |
| Starting a copy that is already running? | Hands over and exits | A second `claude.exe` with the same `--user-data-dir` passes its arguments (e.g. a `claude://` link) to the running copy, which comes to the front. Launching twice is therefore safe. |

## Where a copy keeps things

- **Data folder:** sign-in, settings and the Code tab's session list (`claude-code-sessions\`) are per data folder, not per account. The same account in two copies has two separate session lists.
- **Logs depend on where the data folder is.** If it is under `%APPDATA%`, Claude calls `setAppLogsPath(join(LOCALAPPDATA, basename(userData), "logs"))`: the Main account (`%APPDATA%\Claude`) logs to `%LOCALAPPDATA%\Claude\Logs`, and a profile at `%APPDATA%\ClaudeHouse\profiles\second` logged to `%LOCALAPPDATA%\second\logs`. If the data folder is under `%LOCALAPPDATA%`, logs stay in `<data folder>\logs`. **That is why the house root is `%LOCALAPPDATA%\ClaudeHouse`.** (The first layout put profiles under `%APPDATA%`: routing never found their login starts, and an id like `temp` would have logged into `%LOCALAPPDATA%\Temp`.)
- **`main.log`** grows to about 10 MB, then rotates to `main1.log`, `main2.log`.
- **Sign-in state** is in `config.json`: `windowSizeWasSignedIn` (bool) and `lastKnownAccountUuid`. Signed-in copies on this machine had both; a copy that never finished signing in had neither. The same file also holds the login token cache (`oauth:tokenCache*`), which Claude House never reads into its own data.
- **Code tab settings** (`~/.claude`: skills, CLAUDE.md, MCP servers, transcripts) are shared by all copies unless `CLAUDE_CONFIG_DIR` is set. Whether the Desktop Code tab fully honours `CLAUDE_CONFIG_DIR` is not yet verified.

## Login and `claude://` links

- Google sign-in runs in the system browser (`[Auth] Using system browser for: /login/app-google-auth` in `main.log`) and comes back through `claude://login/google-auth?code=…&hop_nonce=…`. The code is single-use.
- **Every `claude.exe` start re-registers `claude://` to itself** (`setAsDefaultProtocolClient` with `keepExistingDefault: false`), including the short-lived process that only hands a link to a running copy. The registered command is always `"<claude.exe>" "%1"` with no data folder, so the link reaches the Main account. *(An early test concluded "a second copy doesn't re-register"; that only showed the command looked unchanged, because Claude writes the same value. Reading `app.asar` showed it rewrites it on every start.)*
- Handing a profile's link to it works: `claude.exe --user-data-dir=<profile folder> "<link>"` while that copy runs completes the sign-in there.
- **Consequence:** Claude House must own the handler and keep taking it back (the *handler guard*), and decide per link which copy started the login. Rule: the copy with the newest login start in its `main.log`, no time limit. Stateless, so a crash or a rerun gives the same answer.
- The only switch that stops Claude re-registering (`disableDeepLinkRegistration`) is for organisation-managed installs.
- Changing `HKCU\Software\Classes\claude` is a persistent change to the user's system: Claude House only does it after the user turns login routing on, and gives the handler back when it is turned off.

## Still open

1. Does a profile stay signed in across restarts for weeks, or do refresh tokens expire faster for extra copies?
2. Does the Desktop Code tab honour `CLAUDE_CONFIG_DIR` completely (the "Separate Code tab settings" option)?
3. Do several copies clash during a Squirrel update?
