// Recycle bin integration for REEK Ultimate Uninstaller.
//
// Instead of permanently deleting a directory, move it to the Recycle Bin so
// the user can restore it. Uses SHFileOperationW (Win32_UI_Shell).

use greek_common::{GreekError, Result};
use std::path::Path;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::BOOL;
use windows::Win32::UI::Shell::{
    SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
    SHFILEOPSTRUCTW,
};

/// Move a file or directory to the Recycle Bin.
pub fn move_to_recycle_bin(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // SHFileOperationW expects a double-null-terminated list of paths.
    let mut wide: Vec<u16> = path_str.encode_utf16().collect();
    wide.push(0);
    wide.push(0);

    let mut op = SHFILEOPSTRUCTW {
        hwnd: windows::Win32::Foundation::HWND(std::ptr::null_mut()),
        wFunc: FO_DELETE,
        pFrom: PCWSTR(wide.as_ptr()),
        pTo: PCWSTR(ptr::null()),
        // FOF_ALLOWUNDO is what actually sends items to the Recycle Bin.
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        fAnyOperationsAborted: BOOL(0),
        hNameMappings: ptr::null_mut(),
        lpszProgressTitle: PCWSTR(ptr::null()),
    };

    let result = unsafe { SHFileOperationW(&mut op) };
    if result != 0 {
        return Err(GreekError::SystemError(format!(
            "Failed to move {} to recycle bin (error {})",
            path.display(),
            result
        )));
    }
    if op.fAnyOperationsAborted == BOOL(1) {
        return Err(GreekError::SystemError(format!(
            "Move to recycle bin was aborted: {}",
            path.display()
        )));
    }

    tracing::info!("Moved to recycle bin: {}", path.display());
    Ok(())
}
