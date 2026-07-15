#![no_main]
use libfuzzer_sys::fuzz_target;
use matter_bytecode::BytecodeBuilder;
use matter_parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut parser = Parser::from_source(s);
        if let Ok(program) = parser.parse() {
            let builder = BytecodeBuilder::new();
            let _ = builder.compile(&program);
        }
    }
});
