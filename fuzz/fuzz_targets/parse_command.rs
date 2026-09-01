#![no_main]
use libfuzzer_sys::fuzz_target;
use greek_core::uninstaller::StandardUninstallStrategy;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let strat = StandardUninstallStrategy::new();
        // Must never panic; may return Ok or Err
        let _ = strat.parse_command_string(s);
        // Also test the public sanitizer doesn't panic
        let _ = StandardUninstallStrategy::sanitize_output(s.as_bytes());
    }
});
