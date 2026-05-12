use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::{demo_vcpu_program, HybridRuntime, HybridTask};
use matter_vcpu::VirtualCpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vcpu = VirtualCpu::new(32);
    vcpu.load_program(demo_vcpu_program())?;

    let pvpu = PhotonicProcessor::new(16, 16)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    runtime.push_task(HybridTask::CpuSteps { steps: 20 });
    runtime.push_task(HybridTask::PhotonicInterferenceDemo);
    runtime.push_task(HybridTask::PhotonicRunProgram {
        source: "SET_PIXEL 0 0 1.0 0.0\nSET_PIXEL 1 0 1.0 0.0\nINTERFERE 0 0 1 0 2 0\nHALT\n"
            .to_string(),
        width: 16,
        height: 16,
    });

    runtime.run_all_tasks_threaded()?;
    let stats = runtime.stats();

    println!("tasks_completed={}", stats.tasks_completed);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
