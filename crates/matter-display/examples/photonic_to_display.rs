use matter_display::VirtualMonitor;
use matter_photonic_vpu::PhotonicProcessor;
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pvpu = PhotonicProcessor::new(4, 4)?;
    pvpu.set_pixel(0, 0, 1.0, 0.0)?;
    pvpu.set_pixel(1, 0, 1.0, PI)?;
    pvpu.interfere(0, 0, 1, 0, 2, 0)?;
    let intensity = pvpu.intensity_at(2, 0)?;

    let mut monitor = VirtualMonitor::new(8, 8)?;
    monitor.power_on();
    monitor.set_pixel_intensity(2, 0, intensity)?;
    let stats = monitor.present()?;
    let pixel = monitor.get_pixel(2, 0)?;

    println!("photonic_intensity={intensity:.6}");
    println!("display_pixel_r={}", pixel.r);
    println!("frames_presented={}", stats.frames_presented);
    println!("energy_used={}", stats.energy_used);

    Ok(())
}
