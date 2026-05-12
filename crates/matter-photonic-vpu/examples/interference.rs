use matter_photonic_vpu::{PhotonicInstruction, PhotonicProcessor};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut processor = PhotonicProcessor::new(4, 4)?;

    let program = vec![
        PhotonicInstruction::SetPixel {
            x: 0,
            y: 0,
            amplitude: 1.0,
            phase: 0.0,
        },
        PhotonicInstruction::SetPixel {
            x: 1,
            y: 0,
            amplitude: 1.0,
            phase: PI,
        },
        PhotonicInstruction::Interfere {
            ax: 0,
            ay: 0,
            bx: 1,
            by: 0,
            out_x: 2,
            out_y: 0,
        },
        PhotonicInstruction::PrintIntensity { x: 2, y: 0 },
        PhotonicInstruction::Halt,
    ];

    processor.run(&program)?;

    println!(
        "cycles={} energy_used={}",
        processor.cycles, processor.energy_used
    );
    Ok(())
}
