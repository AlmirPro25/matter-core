use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::{demo_vcpu_program, HybridRuntime};
use matter_vcpu::VirtualCpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vcpu = VirtualCpu::new(32);
    vcpu.load_program(demo_vcpu_program())?;

    let pvpu = PhotonicProcessor::new(32, 32)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    runtime.run_frames(100)?;
    let stats = runtime.stats();

    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
