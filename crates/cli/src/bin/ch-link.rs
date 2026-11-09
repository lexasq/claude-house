// No console window: Windows runs this for every claude:// link.
#![windows_subsystem = "windows"]

fn main() -> std::process::ExitCode {
    claude_house_cli::ch_link_main()
}
