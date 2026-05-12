use matter_ir::parse_matter_program;
use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::HybridRuntime;
use matter_vcpu::VirtualCpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matter_source = "
begin
cpu load_const r0 10
cpu load_const r1 20
cpu add r2 r0 r1
cpu print r2
sleep 2
end
";

    let ops = parse_matter_program(matter_source)?;

    let vcpu = VirtualCpu::new(16);
    let pvpu = PhotonicProcessor::new(8, 8)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    let stats = runtime.run_ir_program(&ops)?;

    println!("tasks_completed={}", stats.tasks_completed);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
