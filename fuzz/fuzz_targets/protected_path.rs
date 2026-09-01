#![no_main]
use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let path = Path::new(s);
        let protected = vec!["C:\\Windows".to_string(), "/usr".to_string(), "/etc".to_string()];
        let _ = greek_common::is_protected_path(path, &protected);
        let _ = greek_common::is_protected_path(path, &[]);
    }
});
