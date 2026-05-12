use matter_ir::parse_ir_program;
use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::HybridRuntime;
use matter_vcpu::VirtualCpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vcpu = VirtualCpu::new(16);
    vcpu.load_program(vec![])?;
    let pvpu = PhotonicProcessor::new(8, 8)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    let source = "
BEGIN_TASK
CPU_LOAD_CONST r0 10
CPU_LOAD_CONST r1 20
CPU_ADD r2 r0 r1
CPU_PRINT r2
PHOTONIC_SET_PIXEL 0 0 1.0 0.0
BUS_SHUTDOWN
END_TASK
";

    let ops = parse_ir_program(source)?;
    let stats = runtime.run_ir_program(&ops)?;

    println!("tasks_completed={}", stats.tasks_completed);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
