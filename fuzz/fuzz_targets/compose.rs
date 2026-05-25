#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let parser = crush_compat::compose::ComposeParser::new();
        let _ = parser.parse(&s, std::path::Path::new("fuzz.yml"));
    }
});
