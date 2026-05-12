use matter_bus::BusMessage;
use matter_ir::MatterOp;
use matter_photonic_vpu::{PhotonicInstruction, PhotonicProcessor};
use matter_scheduler::HybridRuntime;
use matter_vcpu::{Instruction as VcpuInstruction, VirtualCpu};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vcpu = VirtualCpu::new(16);
    vcpu.load_program(vec![VcpuInstruction::Nop])?;
    let pvpu = PhotonicProcessor::new(8, 8)?;

    let mut runtime = HybridRuntime::new(vcpu, pvpu);

    let ops = vec![
        MatterOp::BeginTask,
        MatterOp::Cpu(VcpuInstruction::LoadConst { reg: 0, value: 10 }),
        MatterOp::Photonic(PhotonicInstruction::SetPixel {
            x: 0,
            y: 0,
            amplitude: 1.0,
            phase: 0.0,
        }),
        MatterOp::Bus(BusMessage::PixelEnergy {
            x: 0,
            y: 0,
            energy: 1.0,
        }),
        MatterOp::EndTask,
    ];

    let stats = runtime.run_ir_program(&ops)?;

    println!("tasks_completed={}", stats.tasks_completed);
    println!("frames={}", stats.frames);
    println!("cycles={}", stats.total_cycles);
    println!("energy={}", stats.total_energy);
    println!("messages={}", stats.messages_exchanged);

    Ok(())
}
