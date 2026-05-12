use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::{demo_vcpu_program, HybridRuntime, HybridTask, TaskBudget};
use matter_vcpu::VirtualCpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vcpu = VirtualCpu::new(32);
    vcpu.load_program(demo_vcpu_program())?;

    let pvpu = PhotonicProcessor::new(16, 16)?;
    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    runtime.push_task(HybridTask::CpuSteps { steps: 5 });
    runtime.push_task_with_budget(
        HybridTask::CpuSteps { steps: 2 },
        Some(TaskBudget {
            max_frames: Some(3),
            max_cycles: Some(10),
            max_energy: None,
            max_messages: None,
        }),
    );
    runtime.push_task(HybridTask::PhotonicInterferenceDemo);

    runtime.run_all_tasks_threaded()?;
    let stats = runtime.stats();

    println!("tasks_completed={}", stats.tasks_completed);
    println!("tasks_failed={}", stats.tasks_failed);
    println!("tasks_budget_exceeded={}", stats.tasks_budget_exceeded);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
