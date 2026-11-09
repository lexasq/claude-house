# Claude House

Run several Claude Desktop accounts side by side on Windows, each in its own window, each keeping its
own sign-in and Code sessions.

- The Claude Desktop you already use stays exactly as it is: it becomes the **Main account**.
- Every other account runs as a separate Claude Desktop copy with its own data folder (a **profile**).
- Google sign-in comes back to Claude through `claude://` links; Claude House sends each one to the
  window that started the sign-in (**login routing**).

It does not modify Claude Desktop, read your conversations, or talk to the network.

> Only use accounts that are yours, and check that running several at once fits Anthropic's terms for
> your plans. Claude House is not affiliated with Anthropic.

## Requirements

- Windows 10 or 11.
- Claude Desktop installed with its standard installer (it lives in `%LOCALAPPDATA%\AnthropicClaude`).
  The Microsoft Store version has not been tested.
- WebView2 runtime (already part of Windows 11 and current Windows 10).

## Download and use

1. Download **`ClaudeHouse.zip`** from the
   [latest release](https://github.com/lexasq/claude-house/releases/latest)
   (or with the GitHub CLI: `gh release download -R lexasq/claude-house -p ClaudeHouse.zip`).
2. Unzip it somewhere permanent, e.g. `%LOCALAPPDATA%\Programs\ClaudeHouse`.
   **Keep the three programs together**: `claude-house.exe`, `ch-link.exe`, `ch.exe`.
   (Prefer to build it yourself? See [Build](#build).)
3. Run **`claude-house.exe`**. A window opens and a Claude House icon appears in the tray.
   The programs are not code-signed, so the first time Windows may show *"Windows protected your PC"*:
   click **More info → Run anyway**.
4. Turn on **Login routing**. (Only needed for Google sign-in; it can be turned off at any time.)
5. Your existing Claude Desktop appears as **Main account**. Nothing to do there.
6. Under **Add an account**, give the account a name and click **Add account**.
7. Click **Open** on it. A new Claude Desktop window opens on its sign-in screen. Sign in with that account.
   Within a few seconds Claude House shows it as **Signed in**.
8. Repeat 6–7 for every other account.

Later, open any account from the window or from the tray icon's menu. Closing the window keeps Claude
House in the tray, so login routing keeps working; **Quit** in the tray menu stops it.

**Good to know**

- Each account has its own Code tab session list. Signing the same account into two copies gives two
  separate lists; Claude House warns when two copies are on the same account.
- *Separate Code tab settings* gives that account its own `~/.claude` (skills, CLAUDE.md, MCP servers).
  Leave it off to share your settings across accounts.
- **Remove** forgets an account but keeps its data folder, so signing back in is not needed if you add
  it again with the same id.

### Stop using it

1. Turn **Login routing** off (this gives `claude://` links back to Claude Desktop), then **Quit** from the tray.
2. Delete the program folder.
3. Optionally delete `%LOCALAPPDATA%\ClaudeHouse` (the extra accounts' data). Your Main account is
   never touched.

If Claude House was deleted while routing was on, links fix themselves the next time Claude Desktop
starts (it re-registers `claude://` on every start), or run `ch handler uninstall` before deleting.

## Privacy: what Claude House touches

| What | Read / write | Why |
|---|---|---|
| `%LOCALAPPDATA%\ClaudeHouse\profiles.json` | read, write | Your account list: id, name, colour, one setting. |
| `%LOCALAPPDATA%\ClaudeHouse\profiles\<id>\` | Claude writes it | Each extra account's Claude data. Claude House only reads two fields of its `config.json`. |
| `config.json` of each copy (incl. `%APPDATA%\Claude`) | read | Only `windowSizeWasSignedIn` and `lastKnownAccountUuid`, to show *Signed in* and spot duplicates. The account id is compared, never shown or stored. The rest of the file, including Claude's login tokens, is parsed and discarded, never kept or written anywhere. |
| `main.log` of each copy | read | Only the time of the latest `[Auth] Using system browser…` line, to route a sign-in. |
| `HKCU\Software\Classes\claude` | read, write | The `claude://` handler, only while login routing is on. |
| `%LOCALAPPDATA%\ClaudeHouse\routing-enabled` | write | Remembers that you turned routing on. |
| `%LOCALAPPDATA%\ClaudeHouse\ch-link.log` | write | One line per link: time and which account it went to. Never the link itself (it carries a one-time sign-in code). |

No network access, no telemetry, nothing sent anywhere. Your conversations, passwords and tokens are
never read.

## Command line (`ch`)

Everything the window does, from a terminal (PowerShell examples, run from the program folder):

```powershell
.\ch.exe list                                   # accounts and their sign-in state
.\ch.exe add work --name "Work" --color "#3366cc" [--isolate-code]
.\ch.exe launch work                            # open it (or bring it to the front)
.\ch.exe launch work --dry-run                  # print the command instead
.\ch.exe remove work                            # forget it; its data folder is kept
.\ch.exe watch                                  # login routing without the app; Ctrl+C to stop
.\ch.exe handler status | install | uninstall   # who gets claude:// links
.\ch.exe open-link "<claude:// link>" --dry-run # where a link would go
```

## Build

Prerequisites (once):

1. **Rust** (stable) via rustup: `winget install Rustlang.Rustup`
2. **Visual Studio 2022 Build Tools** with the C++ workload (Rust's linker on Windows; Tauri needs it):
   `winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`

Then, from the repository folder:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package.ps1
```

This builds release versions and writes `dist\ClaudeHouse\` (the three programs, this README and the license) and
`dist\ClaudeHouse.zip`. If Claude House is running from `target\release`, quit it first (Windows
cannot replace a running program).

To build by hand: `cargo build --release -p claude-house-cli -p claude-house-app` puts all three
programs in `target\release`.

## How it works

- **Separate copies.** Claude Desktop allows one running copy per data folder. Claude House starts
  `claude.exe --user-data-dir=%LOCALAPPDATA%\ClaudeHouse\profiles\<id>` for each extra account.
  Starting one that is already running just brings it to the front.
- **Login routing.** Every `claude.exe` start registers itself as the `claude://` handler, and that
  handler always reaches the Main account. While routing is on, Claude House keeps the handler pointed
  at `ch-link.exe` (the *handler guard*). For each link, `ch-link.exe` finds the copy whose log shows
  the newest sign-in start and hands the link to it.
- **Sign-in state** comes from two fields of each copy's `config.json`.

The facts behind this, and how each was checked, are in
[docs/phase0-findings.md](docs/phase0-findings.md). A Claude Desktop update can change them.

## Code layout

| Path | What |
|---|---|
| `crates/core` | Rules as pure functions: profile ids and folders, launch commands, link routing, sign-in state, `profiles.json`. Start here. |
| `crates/ops` | Everything with effects: files, processes, the registry (`registry.rs`). Shared by the CLI and the app. |
| `crates/cli` | `ch.exe` (command line) and `ch-link.exe` (the windowless `claude://` handler). |
| `crates/app` | `claude-house.exe`: Tauri 2 window and tray; the UI is plain HTML/JS in `ui/`. |
| `docs/glossary.md` | The terms, one name per concept, and where each lives in code. |
| `docs/phase0-findings.md` | How Claude Desktop behaves, as far as Claude House relies on it. |
| `scripts/package.ps1` | Builds the ready-to-run folder. |

## Develop

```powershell
cargo test                    # unit tests + CLI tests that run the real ch.exe in temp folders
cargo clippy --all-targets
cargo mutants                 # mutation testing (cargo install cargo-mutants)
cargo llvm-cov --summary-only # coverage (cargo install cargo-llvm-cov)
```

Tests never touch the real registry or your Claude copies: they run against fake `APPDATA` /
`LOCALAPPDATA` folders. The registry code (`crates/ops/src/registry.rs`), `ch handler`, `ch watch` and
the app's routing switch are checked by hand.

## Status

Working: separate accounts, login routing, sign-in state, window and tray, command line.
Not yet: an installer, start with Windows, tested behaviour across Claude Desktop updates
(see *Still open* in the findings).

## License

MIT, see [LICENSE](LICENSE).
