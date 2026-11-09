# Claude House

Run several Claude Desktop accounts side by side on Windows. Each account (profile) gets its own Claude data folder, so each stays logged in on its own.

Status: phase 2 of 6 — command-line tool `ch`. Findings: [docs/phase0-findings.md](docs/phase0-findings.md). Terms: [docs/glossary.md](docs/glossary.md).

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

Profiles live in `%APPDATA%\ClaudeHouse\profiles.json`; each profile's data in `%APPDATA%\ClaudeHouse\profiles\<id>`.

## Logging in with Google

Google sign-in comes back through a `claude://` link, which Windows still sends to your normal Claude copy. Until phase 3 routes these automatically, see "Login routing" in [docs/phase0-findings.md](docs/phase0-findings.md) for the manual handoff.

## Develop

```powershell
cargo test
cargo clippy --all-targets
cargo mutants          # mutation testing
cargo llvm-cov --summary-only
```
