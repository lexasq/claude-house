//! The only code that touches the registry: `HKCU\Software\Classes\claude`.
//! Untested by `cargo test` on purpose — tests must not change the real handler. Checked by hand.

use std::time::Duration;

use winreg::enums::{HKEY_CURRENT_USER, KEY_NOTIFY};
use winreg::RegKey;

const SCHEME_KEY: &str = r"Software\Classes\claude";
const COMMAND_KEY: &str = r"Software\Classes\claude\shell\open\command";

pub fn read_handler() -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(COMMAND_KEY).ok()?.get_value("").ok()
}

/// Registers `claude` as a URL scheme (idempotent) and sets its open command.
pub fn write_handler(command: &str) -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (scheme, _) = hkcu.create_subkey(SCHEME_KEY)?;
    scheme.set_value("", &"URL:claude")?;
    scheme.set_value("URL Protocol", &"")?;
    let (cmd, _) = hkcu.create_subkey(COMMAND_KEY)?;
    cmd.set_value("", &command)
}

/// Blocks until something under the `claude` key changes, or `timeout` passes.
/// The timeout also covers the key being deleted and recreated under our handle.
pub fn wait_for_change(timeout: Duration) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Registry::{RegNotifyChangeKeyValue, HKEY, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME};
    use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(SCHEME_KEY, KEY_NOTIFY) else {
        std::thread::sleep(timeout);
        return;
    };
    // SAFETY: plain Win32 calls on a key handle we own for this scope and an event we close below.
    unsafe {
        let event = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
        if event.is_null() {
            std::thread::sleep(timeout);
            return;
        }
        let filter = REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_CHANGE_NAME;
        // Watch the whole subtree: Claude rewrites `claudeshellopenmmand`, below the key we open.
        let (watch_subtree, asynchronous) = (1, 1);
        if RegNotifyChangeKeyValue(key.raw_handle() as HKEY, watch_subtree, filter, event, asynchronous) == 0 {
            WaitForSingleObject(event, timeout.as_millis() as u32);
        } else {
            std::thread::sleep(timeout);
        }
        CloseHandle(event);
    }
}
