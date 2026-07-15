#![no_main]
use libfuzzer_sys::fuzz_target;
use matter_parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut parser = Parser::from_source(s);
        let _ = parser.parse();
    }
});
