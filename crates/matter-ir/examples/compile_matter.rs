use matter_ir::{emit_ir_program, parse_matter_file};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .ok_or("missing argument: usage compile_matter <program.matter>")?;

    let ops = parse_matter_file(path)?;
    let ir_text = emit_ir_program(&ops);
    print!("{}", ir_text);

    Ok(())
}
