use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::HybridRuntime;
use matter_vcpu::VirtualCpu;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .ok_or("missing argument: usage run_mir_file <program.mir>")?;

    let vcpu = VirtualCpu::new(32);
    let pvpu = PhotonicProcessor::new(8, 8)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    let stats = runtime.run_ir_file(path)?;

    println!("tasks_completed={}", stats.tasks_completed);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
