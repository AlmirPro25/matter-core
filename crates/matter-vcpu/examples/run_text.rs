use matter_vcpu::{parse_program, VirtualCpu};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: run_text <program.vcpu>");
        std::process::exit(1);
    }

    let source = fs::read_to_string(&args[1])?;
    let program = parse_program(&source)?;

    let mut cpu = VirtualCpu::new(64);
    cpu.load_program(program)?;
    cpu.run()?;

    let stats = cpu.stats();
    println!(
        "cycles={} energy={} pc={}",
        stats.cycles, stats.energy_consumed, stats.program_counter
    );

    Ok(())
}
