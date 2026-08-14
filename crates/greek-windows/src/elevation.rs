// Elevated-user detection for REEK Ultimate Uninstaller.
//
// Some operations (force-remove of protected folders, service deletion,
// registry writes under HKLM) require Administrator privileges. This module
// lets the UI detect that and gate destructive actions accordingly.

use windows::Win32::Foundation::BOOL;
use windows::Win32::UI::Shell::IsUserAnAdmin;

/// Return `true` if the current process is running with Administrator
/// privileges.
pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin() != BOOL(0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_elevated_returns_bool() {
        // Cannot assert a specific value, but must return on the current
        // process without panicking.
        let _ = is_elevated();
    }
}
