#![no_main]
use libfuzzer_sys::fuzz_target;
use matter_lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s);
        loop {
            let token = lexer.next_token();
            if token == matter_lexer::Token::Eof {
                break;
            }
        }
    }
});
