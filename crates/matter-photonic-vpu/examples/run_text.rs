use matter_photonic_vpu::run_program_text;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .ok_or("missing argument: usage run_text <program.pvpu> [x y]")?;

    let x = match args.get(2) {
        Some(v) => v.parse::<usize>()?,
        None => 2,
    };
    let y = match args.get(3) {
        Some(v) => v.parse::<usize>()?,
        None => 0,
    };

    let source = fs::read_to_string(path)?;
    let processor = run_program_text(4, 4, &source)?;

    let intensity = processor.intensity_at(x, y)?;
    println!("intensity({x},{y})={intensity:.4}");
    println!("cycles={}", processor.cycles);
    println!("energy_used={}", processor.energy_used);

    Ok(())
}
