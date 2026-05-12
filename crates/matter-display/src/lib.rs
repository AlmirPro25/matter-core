#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayStats {
    pub powered: bool,
    pub width: usize,
    pub height: usize,
    pub frames_presented: u64,
    pub energy_used: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualMonitor {
    pub powered: bool,
    pub width: usize,
    pub height: usize,
    pub framebuffer: Vec<Color>,
    pub frames_presented: u64,
    pub energy_used: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayError {
    InvalidSize,
    PoweredOff,
    OutOfBounds,
    SourceError,
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "invalid monitor size"),
            Self::PoweredOff => write!(f, "monitor is powered off"),
            Self::OutOfBounds => write!(f, "pixel out of bounds"),
            Self::SourceError => write!(f, "invalid source data"),
        }
    }
}

impl std::error::Error for DisplayError {}

const CLEAR_ENERGY_PER_PIXEL: f32 = 0.0001;
const PRESENT_ENERGY_PER_PIXEL: f32 = 0.0005;

impl VirtualMonitor {
    pub fn new(width: usize, height: usize) -> Result<Self, DisplayError> {
        if width == 0 || height == 0 {
            return Err(DisplayError::InvalidSize);
        }

        let len = width.checked_mul(height).ok_or(DisplayError::InvalidSize)?;
        Ok(Self {
            powered: false,
            width,
            height,
            framebuffer: vec![
                Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                };
                len
            ],
            frames_presented: 0,
            energy_used: 0.0,
        })
    }

    pub fn power_on(&mut self) {
        self.powered = true;
    }

    pub fn power_off(&mut self) {
        self.powered = false;
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), DisplayError> {
        self.ensure_powered()?;
        let idx = self.idx(x, y)?;
        self.framebuffer[idx] = color;
        Ok(())
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Result<Color, DisplayError> {
        self.ensure_powered()?;
        let idx = self.idx(x, y)?;
        Ok(self.framebuffer[idx])
    }

    pub fn clear(&mut self, color: Color) -> Result<(), DisplayError> {
        self.ensure_powered()?;
        for pixel in &mut self.framebuffer {
            *pixel = color;
        }
        self.energy_used += (self.width * self.height) as f32 * CLEAR_ENERGY_PER_PIXEL;
        Ok(())
    }

    pub fn set_pixel_intensity(
        &mut self,
        x: usize,
        y: usize,
        intensity: f32,
    ) -> Result<(), DisplayError> {
        self.ensure_powered()?;
        let clamped = intensity.clamp(0.0, 1.0);
        let byte = (clamped * 255.0).round() as u8;
        self.set_pixel(
            x,
            y,
            Color {
                r: byte,
                g: byte,
                b: byte,
                a: 255,
            },
        )
    }

    pub fn present(&mut self) -> Result<DisplayStats, DisplayError> {
        self.ensure_powered()?;
        self.frames_presented = self.frames_presented.saturating_add(1);
        self.energy_used += (self.width * self.height) as f32 * PRESENT_ENERGY_PER_PIXEL;
        Ok(self.stats())
    }

    pub fn set_pixel_from_photonic(
        &mut self,
        photonic: &matter_photonic_vpu::PhotonicProcessor,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
    ) -> Result<(), DisplayError> {
        let intensity = photonic
            .intensity_at(src_x, src_y)
            .map_err(|_| DisplayError::SourceError)?;
        self.set_pixel_intensity(dst_x, dst_y, intensity)
    }

    pub fn stats(&self) -> DisplayStats {
        DisplayStats {
            powered: self.powered,
            width: self.width,
            height: self.height,
            frames_presented: self.frames_presented,
            energy_used: self.energy_used,
        }
    }

    fn ensure_powered(&self) -> Result<(), DisplayError> {
        if !self.powered {
            return Err(DisplayError::PoweredOff);
        }
        Ok(())
    }

    fn idx(&self, x: usize, y: usize) -> Result<usize, DisplayError> {
        if x >= self.width || y >= self.height {
            return Err(DisplayError::OutOfBounds);
        }
        Ok(y * self.width + x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_cria_com_tamanho_valido() {
        let m = VirtualMonitor::new(8, 8).expect("monitor should be created");
        assert_eq!(m.width, 8);
        assert_eq!(m.height, 8);
        assert_eq!(m.framebuffer.len(), 64);
    }

    #[test]
    fn tamanho_invalido_retorna_erro() {
        let err = VirtualMonitor::new(0, 8).expect_err("must fail");
        assert_eq!(err, DisplayError::InvalidSize);
    }

    #[test]
    fn nao_desenha_desligado() {
        let mut m = VirtualMonitor::new(8, 8).expect("monitor should be created");
        let err = m
            .set_pixel(
                0,
                0,
                Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            )
            .expect_err("must fail");
        assert_eq!(err, DisplayError::PoweredOff);
    }

    #[test]
    fn power_on_permite_set_pixel() {
        let mut m = VirtualMonitor::new(8, 8).expect("monitor should be created");
        m.power_on();
        let c = Color {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        m.set_pixel(1, 2, c).expect("set_pixel should work");
        let got = m.get_pixel(1, 2).expect("get_pixel should work");
        assert_eq!(got, c);
    }

    #[test]
    fn set_pixel_out_of_bounds_retorna_erro() {
        let mut m = VirtualMonitor::new(8, 8).expect("monitor should be created");
        m.power_on();
        let err = m
            .set_pixel(
                9,
                1,
                Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            )
            .expect_err("must fail");
        assert_eq!(err, DisplayError::OutOfBounds);
    }

    #[test]
    fn clear_preenche_framebuffer() {
        let mut m = VirtualMonitor::new(4, 4).expect("monitor should be created");
        m.power_on();
        let color = Color {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        };
        m.clear(color).expect("clear should work");
        assert!(m.framebuffer.iter().all(|p| *p == color));
    }

    #[test]
    fn present_incrementa_frames_e_energia() {
        let mut m = VirtualMonitor::new(4, 4).expect("monitor should be created");
        m.power_on();
        let before = m.energy_used;
        let s = m.present().expect("present should work");
        assert_eq!(s.frames_presented, 1);
        assert!(s.energy_used > before);
    }

    #[test]
    fn set_pixel_intensity_mapeia_para_grayscale() {
        let mut m = VirtualMonitor::new(2, 2).expect("monitor should be created");
        m.power_on();
        m.set_pixel_intensity(0, 0, 0.5)
            .expect("intensity should map");
        let c = m.get_pixel(0, 0).expect("pixel should exist");
        assert_eq!(c.r, c.g);
        assert_eq!(c.g, c.b);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn set_pixel_from_photonic_reads_intensity() {
        let mut photonic =
            matter_photonic_vpu::PhotonicProcessor::new(4, 4).expect("photonic should be created");
        photonic
            .set_pixel(1, 1, 1.0, 0.0)
            .expect("set_pixel should work");

        let mut display = VirtualMonitor::new(4, 4).expect("display should be created");
        display.power_on();
        display
            .set_pixel_from_photonic(&photonic, 1, 1, 2, 2)
            .expect("copy should work");
        let c = display.get_pixel(2, 2).expect("pixel should exist");
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }
}
