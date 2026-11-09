# Claude House

Run several Claude Desktop accounts side by side on Windows. Each account (profile) gets its own Claude data folder, so each stays logged in on its own.

Status: phase 2 of 6 — command-line tool `ch`. Findings: [docs/phase0-findings.md](docs/phase0-findings.md). Terms: [docs/glossary.md](docs/glossary.md).

## How to use it

Every account is a profile; Claude House is the way in.

1. Run `claude-house.exe` and turn on **Login routing** (lets Google sign-in reach the right window).
2. Add an account, click **Open**, sign in in the window that opens. Repeat for each account.
3. Each account keeps its own sign-in and Code sessions; nothing is shared between them.

Already using Claude Desktop? Its sessions live in its own folder (`%APPDATA%\Claude`). Quit it, then
use **Import** in the *Main Claude* section (or `ch import-main <id>`) to copy it into an account. Keep
Main Claude closed afterwards and turn off its "start with Windows" setting: two copies on one sign-in
can sign each other out.

## Build

```powershell
cargo build --release -p claude-house-cli   # -> target\release\ch.exe
```

## Use

```powershell
ch add work --name "Work account" --color "#3366cc" --isolate-code
ch list
ch launch work            # starts it, or brings its window to the front
ch launch work --dry-run  # prints the command instead
ch remove work            # forgets the profile; its data folder is kept
```

Profiles live in `%LOCALAPPDATA%\ClaudeHouse\profiles.json`; each profile's data in `%LOCALAPPDATA%\ClaudeHouse\profiles\<id>`. (Not under `%APPDATA%`: there Claude would put a profile's logs in `%LOCALAPPDATA%\<id>\logs`, where login routing can't find them.)

## Logging in with Google

Google sign-in comes back through a `claude://` link, which Windows still sends to your normal Claude copy. Until phase 3 routes these automatically, see "Login routing" in [docs/phase0-findings.md](docs/phase0-findings.md) for the manual handoff.

## Develop

```powershell
cargo test
cargo clippy --all-targets
cargo mutants          # mutation testing
cargo llvm-cov --summary-only
```
