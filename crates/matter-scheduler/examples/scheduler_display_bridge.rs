use matter_display::VirtualMonitor;
use matter_photonic_vpu::PhotonicProcessor;
use matter_scheduler::HybridRuntime;
use matter_vcpu::VirtualCpu;
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vcpu = VirtualCpu::new(32);
    let mut pvpu = PhotonicProcessor::new(8, 8)?;
    pvpu.set_pixel(0, 0, 1.0, 0.0)?;
    pvpu.set_pixel(1, 0, 1.0, PI)?;
    pvpu.interfere(0, 0, 1, 0, 2, 0)?;

    let runtime = HybridRuntime::new(vcpu, pvpu);

    let mut display = VirtualMonitor::new(8, 8)?;
    display.power_on();

    runtime.project_photonic_to_display(&mut display, 2, 0, 2, 0)?;
    let d_stats = display.present()?;
    let r_stats = runtime.stats();
    let px = display.get_pixel(2, 0)?;

    println!("pixel_gray={}", px.r);
    println!("display_frames={}", d_stats.frames_presented);
    println!("display_energy={}", d_stats.energy_used);
    println!("runtime_frames={}", r_stats.frames);
    println!("runtime_cycles={}", r_stats.total_cycles);
    println!("runtime_energy={}", r_stats.total_energy);

    Ok(())
}
