use matter_vcpu::{Instruction, VirtualCpu};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cpu = VirtualCpu::new(32);

    cpu.load_program(vec![
        Instruction::LoadConst { reg: 0, value: 10 },
        Instruction::LoadConst { reg: 1, value: 20 },
        Instruction::Add { dst: 2, a: 0, b: 1 },
        Instruction::Print { reg: 2 },
        Instruction::Halt,
    ])?;

    cpu.run()?;

    let stats = cpu.stats();
    println!(
        "cycles={} energy={} pc={}",
        stats.cycles, stats.energy_consumed, stats.program_counter
    );

    Ok(())
}
