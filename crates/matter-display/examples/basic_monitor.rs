use matter_display::{Color, VirtualMonitor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut monitor = VirtualMonitor::new(8, 8)?;
    monitor.power_on();

    monitor.set_pixel(
        0,
        0,
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
    )?;
    monitor.set_pixel(
        1,
        1,
        Color {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        },
    )?;
    monitor.set_pixel(
        2,
        2,
        Color {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
    )?;

    let stats = monitor.present()?;

    println!("frames_presented={}", stats.frames_presented);
    println!("energy_used={}", stats.energy_used);

    Ok(())
}
