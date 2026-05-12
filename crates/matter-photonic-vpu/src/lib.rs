#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotonicPixel {
    pub amplitude: f32,
    pub phase: f32,
    pub wavelength_nm: f32,
    pub energy: f32,
}

impl Default for PhotonicPixel {
    fn default() -> Self {
        Self {
            amplitude: 0.0,
            phase: 0.0,
            wavelength_nm: 532.0,
            energy: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotonicGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<PhotonicPixel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotonicProcessor {
    pub grid: PhotonicGrid,
    pub cycles: u64,
    pub energy_used: f32,
    halted: bool,
}

const AMPLITUDE_EPSILON: f32 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub enum PhotonicInstruction {
    Nop,
    SetPixel {
        x: usize,
        y: usize,
        amplitude: f32,
        phase: f32,
    },
    Modulate {
        x: usize,
        y: usize,
        factor: f32,
    },
    Interfere {
        ax: usize,
        ay: usize,
        bx: usize,
        by: usize,
        out_x: usize,
        out_y: usize,
    },
    Threshold {
        threshold: f32,
    },
    Normalize,
    PrintIntensity {
        x: usize,
        y: usize,
    },
    Halt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhotonicError {
    OutOfBounds,
    InvalidAmplitude,
    InvalidPhase,
    InvalidGridSize,
    Halted,
    ParseError { line: usize, message: String },
    UnknownInstruction { line: usize, instruction: String },
    InvalidNumber { line: usize, value: String },
}

impl std::fmt::Display for PhotonicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds => write!(f, "pixel coordinates out of bounds"),
            Self::InvalidAmplitude => write!(f, "invalid amplitude"),
            Self::InvalidPhase => write!(f, "invalid phase"),
            Self::InvalidGridSize => write!(f, "invalid grid size"),
            Self::Halted => write!(f, "processor halted"),
            Self::ParseError { line, message } => {
                write!(f, "parse error at line {line}: {message}")
            }
            Self::UnknownInstruction { line, instruction } => {
                write!(f, "unknown instruction at line {line}: {instruction}")
            }
            Self::InvalidNumber { line, value } => {
                write!(f, "invalid number at line {line}: {value}")
            }
        }
    }
}

impl std::error::Error for PhotonicError {}

pub fn parse_program(source: &str) -> Result<Vec<PhotonicInstruction>, PhotonicError> {
    let mut out = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let line = i + 1;
        let clean = strip_comments(raw).trim();
        if clean.is_empty() {
            continue;
        }
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let op = parts[0];
        let instr = match op {
            "NOP" => {
                expect_arity(&parts, 1, line)?;
                PhotonicInstruction::Nop
            }
            "SET_PIXEL" => {
                expect_arity(&parts, 5, line)?;
                PhotonicInstruction::SetPixel {
                    x: parse_usize(parts[1], line)?,
                    y: parse_usize(parts[2], line)?,
                    amplitude: parse_f32(parts[3], line)?,
                    phase: parse_f32(parts[4], line)?,
                }
            }
            "MODULATE" => {
                expect_arity(&parts, 4, line)?;
                PhotonicInstruction::Modulate {
                    x: parse_usize(parts[1], line)?,
                    y: parse_usize(parts[2], line)?,
                    factor: parse_f32(parts[3], line)?,
                }
            }
            "INTERFERE" => {
                expect_arity(&parts, 7, line)?;
                PhotonicInstruction::Interfere {
                    ax: parse_usize(parts[1], line)?,
                    ay: parse_usize(parts[2], line)?,
                    bx: parse_usize(parts[3], line)?,
                    by: parse_usize(parts[4], line)?,
                    out_x: parse_usize(parts[5], line)?,
                    out_y: parse_usize(parts[6], line)?,
                }
            }
            "THRESHOLD" => {
                expect_arity(&parts, 2, line)?;
                PhotonicInstruction::Threshold {
                    threshold: parse_f32(parts[1], line)?,
                }
            }
            "NORMALIZE" => {
                expect_arity(&parts, 1, line)?;
                PhotonicInstruction::Normalize
            }
            "PRINT_INTENSITY" => {
                expect_arity(&parts, 3, line)?;
                PhotonicInstruction::PrintIntensity {
                    x: parse_usize(parts[1], line)?,
                    y: parse_usize(parts[2], line)?,
                }
            }
            "HALT" => {
                expect_arity(&parts, 1, line)?;
                PhotonicInstruction::Halt
            }
            _ => {
                return Err(PhotonicError::UnknownInstruction {
                    line,
                    instruction: op.to_string(),
                });
            }
        };
        out.push(instr);
    }
    Ok(out)
}

pub fn run_program_text(
    width: usize,
    height: usize,
    source: &str,
) -> Result<PhotonicProcessor, PhotonicError> {
    let program = parse_program(source)?;
    let mut processor = PhotonicProcessor::new(width, height)?;
    processor.run(&program)?;
    Ok(processor)
}

fn strip_comments(line: &str) -> &str {
    let hash = line.find('#');
    let semicolon = line.find(';');
    let end = match (hash, semicolon) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => line.len(),
    };
    &line[..end]
}

fn expect_arity(parts: &[&str], expected: usize, line: usize) -> Result<(), PhotonicError> {
    if parts.len() != expected {
        return Err(PhotonicError::ParseError {
            line,
            message: format!("expected {expected} tokens, got {}", parts.len()),
        });
    }
    Ok(())
}

fn parse_usize(token: &str, line: usize) -> Result<usize, PhotonicError> {
    token
        .parse::<usize>()
        .map_err(|_| PhotonicError::InvalidNumber {
            line,
            value: token.to_string(),
        })
}

fn parse_f32(token: &str, line: usize) -> Result<f32, PhotonicError> {
    if token.eq_ignore_ascii_case("PI") {
        return Ok(std::f32::consts::PI);
    }
    if token.eq_ignore_ascii_case("TAU") {
        return Ok(std::f32::consts::TAU);
    }
    token
        .parse::<f32>()
        .map_err(|_| PhotonicError::InvalidNumber {
            line,
            value: token.to_string(),
        })
}

impl PhotonicProcessor {
    pub fn new(width: usize, height: usize) -> Result<Self, PhotonicError> {
        if width == 0 || height == 0 {
            return Err(PhotonicError::InvalidGridSize);
        }
        let len = width
            .checked_mul(height)
            .ok_or(PhotonicError::InvalidGridSize)?;

        Ok(Self {
            grid: PhotonicGrid {
                width,
                height,
                cells: vec![PhotonicPixel::default(); len],
            },
            cycles: 0,
            energy_used: 0.0,
            halted: false,
        })
    }

    pub fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        amplitude: f32,
        phase: f32,
    ) -> Result<(), PhotonicError> {
        if !amplitude.is_finite() || amplitude < 0.0 {
            return Err(PhotonicError::InvalidAmplitude);
        }
        if !phase.is_finite() {
            return Err(PhotonicError::InvalidPhase);
        }

        let idx = self.idx(x, y)?;
        let pixel = &mut self.grid.cells[idx];
        pixel.amplitude = amplitude;
        pixel.phase = phase;
        pixel.energy = amplitude * amplitude;
        Ok(())
    }

    pub fn modulate(&mut self, x: usize, y: usize, factor: f32) -> Result<(), PhotonicError> {
        if !factor.is_finite() {
            return Err(PhotonicError::InvalidAmplitude);
        }
        let idx = self.idx(x, y)?;
        let current = self.grid.cells[idx].amplitude;
        let new_amplitude = current * factor;
        if !new_amplitude.is_finite() || new_amplitude < 0.0 {
            return Err(PhotonicError::InvalidAmplitude);
        }
        self.grid.cells[idx].amplitude = new_amplitude;
        self.grid.cells[idx].energy = new_amplitude * new_amplitude;
        Ok(())
    }

    pub fn interfere(
        &mut self,
        ax: usize,
        ay: usize,
        bx: usize,
        by: usize,
        out_x: usize,
        out_y: usize,
    ) -> Result<(), PhotonicError> {
        let a = self.grid.cells[self.idx(ax, ay)?];
        let b = self.grid.cells[self.idx(bx, by)?];

        let a_real = a.amplitude * a.phase.cos();
        let a_imag = a.amplitude * a.phase.sin();
        let b_real = b.amplitude * b.phase.cos();
        let b_imag = b.amplitude * b.phase.sin();

        let real = a_real + b_real;
        let imag = a_imag + b_imag;

        let mut amplitude = (real * real + imag * imag).sqrt();
        let mut phase = imag.atan2(real);
        if amplitude.abs() < AMPLITUDE_EPSILON {
            amplitude = 0.0;
            phase = 0.0;
        }

        self.set_pixel(out_x, out_y, amplitude, phase)
    }

    pub fn threshold(&mut self, threshold: f32) -> Result<(), PhotonicError> {
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(PhotonicError::InvalidAmplitude);
        }

        for pixel in &mut self.grid.cells {
            if pixel.amplitude < threshold {
                pixel.amplitude = 0.0;
                pixel.energy = 0.0;
            }
        }
        Ok(())
    }

    pub fn normalize(&mut self) -> Result<(), PhotonicError> {
        let mut max_amplitude = 0.0f32;
        for p in &self.grid.cells {
            if p.amplitude > max_amplitude {
                max_amplitude = p.amplitude;
            }
        }

        if max_amplitude <= 0.0 {
            return Ok(());
        }

        for pixel in &mut self.grid.cells {
            pixel.amplitude /= max_amplitude;
            pixel.energy = pixel.amplitude * pixel.amplitude;
        }
        Ok(())
    }

    pub fn total_light_energy(&self) -> f32 {
        self.grid.cells.iter().map(|p| p.energy).sum()
    }

    pub fn intensity_at(&self, x: usize, y: usize) -> Result<f32, PhotonicError> {
        let idx = self.idx(x, y)?;
        let amplitude = self.grid.cells[idx].amplitude;
        Ok(amplitude * amplitude)
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn step(&mut self, instruction: &PhotonicInstruction) -> Result<(), PhotonicError> {
        if self.halted {
            return Err(PhotonicError::Halted);
        }

        let cost = match instruction {
            PhotonicInstruction::Nop => 1,
            PhotonicInstruction::SetPixel {
                x,
                y,
                amplitude,
                phase,
            } => {
                self.set_pixel(*x, *y, *amplitude, *phase)?;
                1
            }
            PhotonicInstruction::Modulate { x, y, factor } => {
                self.modulate(*x, *y, *factor)?;
                1
            }
            PhotonicInstruction::Interfere {
                ax,
                ay,
                bx,
                by,
                out_x,
                out_y,
            } => {
                self.interfere(*ax, *ay, *bx, *by, *out_x, *out_y)?;
                3
            }
            PhotonicInstruction::Threshold { threshold } => {
                self.threshold(*threshold)?;
                self.grid.width as u64 * self.grid.height as u64
            }
            PhotonicInstruction::Normalize => {
                self.normalize()?;
                self.grid.width as u64 * self.grid.height as u64
            }
            PhotonicInstruction::PrintIntensity { x, y } => {
                let idx = self.idx(*x, *y)?;
                let intensity = self.grid.cells[idx].amplitude * self.grid.cells[idx].amplitude;
                println!("{}", intensity);
                1
            }
            PhotonicInstruction::Halt => {
                self.halted = true;
                1
            }
        };

        self.cycles = self
            .cycles
            .checked_add(cost)
            .ok_or(PhotonicError::InvalidGridSize)?;
        self.energy_used = self.total_light_energy();
        Ok(())
    }

    pub fn run(&mut self, program: &[PhotonicInstruction]) -> Result<(), PhotonicError> {
        for instruction in program {
            self.step(instruction)?;
            if self.halted {
                break;
            }
        }
        Ok(())
    }

    fn idx(&self, x: usize, y: usize) -> Result<usize, PhotonicError> {
        if x >= self.grid.width || y >= self.grid.height {
            return Err(PhotonicError::OutOfBounds);
        }
        Ok(y * self.grid.width + x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn criar_grid_valido() {
        let p = PhotonicProcessor::new(4, 4).expect("processor should be created");
        assert_eq!(p.grid.width, 4);
        assert_eq!(p.grid.height, 4);
        assert_eq!(p.grid.cells.len(), 16);
    }

    #[test]
    fn grid_invalido_retorna_erro() {
        let err = PhotonicProcessor::new(0, 4).expect_err("must fail");
        assert_eq!(err, PhotonicError::InvalidGridSize);
    }

    #[test]
    fn set_pixel_altera_celula() {
        let mut p = PhotonicProcessor::new(2, 2).expect("processor should be created");
        p.set_pixel(1, 1, 0.5, 0.25)
            .expect("set_pixel should succeed");
        let idx = 3;
        assert_eq!(p.grid.cells[idx].amplitude, 0.5);
        assert_eq!(p.grid.cells[idx].phase, 0.25);
        assert!((p.grid.cells[idx].energy - 0.25).abs() < 1e-6);
    }

    #[test]
    fn out_of_bounds_retorna_erro() {
        let mut p = PhotonicProcessor::new(2, 2).expect("processor should be created");
        let err = p.set_pixel(2, 0, 1.0, 0.0).expect_err("must fail");
        assert_eq!(err, PhotonicError::OutOfBounds);
    }

    #[test]
    fn modulate_altera_amplitude() {
        let mut p = PhotonicProcessor::new(2, 2).expect("processor should be created");
        p.set_pixel(0, 0, 0.5, 0.0).expect("set should work");
        p.modulate(0, 0, 2.0).expect("modulate should work");
        assert!((p.grid.cells[0].amplitude - 1.0).abs() < 1e-6);
    }

    #[test]
    fn interferencia_construtiva_fase_igual_aumenta_amplitude() {
        let mut p = PhotonicProcessor::new(3, 3).expect("processor should be created");
        p.set_pixel(0, 0, 1.0, 0.0).expect("set A should work");
        p.set_pixel(1, 0, 1.0, 0.0).expect("set B should work");
        p.interfere(0, 0, 1, 0, 2, 0)
            .expect("interfere should work");
        let idx = 2;
        assert!((p.grid.cells[idx].amplitude - 2.0).abs() < 1e-5);
    }

    #[test]
    fn interferencia_destrutiva_fases_opostas_cancela_amplitude() {
        let mut p = PhotonicProcessor::new(3, 3).expect("processor should be created");
        p.set_pixel(0, 0, 1.0, 0.0).expect("set A should work");
        p.set_pixel(1, 0, 1.0, PI).expect("set B should work");
        p.interfere(0, 0, 1, 0, 2, 0)
            .expect("interfere should work");
        let idx = 2;
        assert_eq!(p.grid.cells[idx].amplitude, 0.0);
        assert_eq!(p.grid.cells[idx].energy, 0.0);
        assert_eq!(p.grid.cells[idx].phase, 0.0);
    }

    #[test]
    fn threshold_zera_pixels_abaixo_do_limite() {
        let mut p = PhotonicProcessor::new(2, 1).expect("processor should be created");
        p.set_pixel(0, 0, 0.2, 0.0).expect("set should work");
        p.set_pixel(1, 0, 0.8, 0.0).expect("set should work");
        p.threshold(0.5).expect("threshold should work");
        assert_eq!(p.grid.cells[0].amplitude, 0.0);
        assert_eq!(p.grid.cells[1].amplitude, 0.8);
    }

    #[test]
    fn normalize_mantem_amplitude_maxima_em_um() {
        let mut p = PhotonicProcessor::new(2, 1).expect("processor should be created");
        p.set_pixel(0, 0, 0.5, 0.0).expect("set should work");
        p.set_pixel(1, 0, 2.0, 0.0).expect("set should work");
        p.normalize().expect("normalize should work");
        assert!((p.grid.cells[1].amplitude - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ciclos_aumentam_corretamente() {
        let mut p = PhotonicProcessor::new(2, 2).expect("processor should be created");
        let program = vec![
            PhotonicInstruction::Nop,
            PhotonicInstruction::SetPixel {
                x: 0,
                y: 0,
                amplitude: 1.0,
                phase: 0.0,
            },
            PhotonicInstruction::Interfere {
                ax: 0,
                ay: 0,
                bx: 0,
                by: 0,
                out_x: 1,
                out_y: 1,
            },
            PhotonicInstruction::Threshold { threshold: 0.5 },
            PhotonicInstruction::Normalize,
            PhotonicInstruction::Halt,
        ];
        p.run(&program).expect("run should succeed");
        // Nop(1) + Set(1) + Interfere(3) + Threshold(4) + Normalize(4) + Halt(1) = 14
        assert_eq!(p.cycles, 14);
    }

    #[test]
    fn parse_set_pixel() {
        let parsed = parse_program("SET_PIXEL 1 2 0.5 1.57").expect("parse should succeed");
        assert_eq!(
            parsed,
            vec![PhotonicInstruction::SetPixel {
                x: 1,
                y: 2,
                amplitude: 0.5,
                phase: 1.57
            }]
        );
    }

    #[test]
    fn parse_unknown_instruction_error() {
        let err = parse_program("FOO 1 2").expect_err("must fail");
        assert_eq!(
            err,
            PhotonicError::UnknownInstruction {
                line: 1,
                instruction: "FOO".to_string()
            }
        );
    }

    #[test]
    fn run_program_text_executes() {
        let src = "\
SET_PIXEL 0 0 1.0 0.0
SET_PIXEL 1 0 1.0 3.1415927
INTERFERE 0 0 1 0 2 0
HALT
";
        let p = run_program_text(4, 4, src).expect("run should succeed");
        let idx = 2;
        assert!(p.grid.cells[idx].amplitude.abs() < 1e-4);
    }

    #[test]
    fn parse_e_executa_arquivo_interference_pvpu() {
        let src = include_str!("../examples/interference.pvpu");
        let p = run_program_text(4, 4, src).expect("run should succeed");
        let intensity = p.intensity_at(2, 0).expect("intensity should exist");
        assert!(intensity < 1e-4);
    }

    #[test]
    fn parse_aceita_constantes_pi_e_tau() {
        let parsed = parse_program("SET_PIXEL 0 0 1.0 PI\nSET_PIXEL 1 0 1.0 TAU\nHALT")
            .expect("parse should succeed");
        match &parsed[0] {
            PhotonicInstruction::SetPixel { phase, .. } => {
                assert!((*phase - std::f32::consts::PI).abs() < 1e-6);
            }
            _ => panic!("expected SET_PIXEL"),
        }
        match &parsed[1] {
            PhotonicInstruction::SetPixel { phase, .. } => {
                assert!((*phase - std::f32::consts::TAU).abs() < 1e-6);
            }
            _ => panic!("expected SET_PIXEL"),
        }
    }

    #[test]
    fn step_apos_halt_retorna_erro_halted() {
        let mut p = PhotonicProcessor::new(2, 2).expect("processor should be created");
        p.step(&PhotonicInstruction::Halt)
            .expect("halt should work");
        assert!(p.is_halted());
        let err = p
            .step(&PhotonicInstruction::Nop)
            .expect_err("must fail after halt");
        assert_eq!(err, PhotonicError::Halted);
    }
}
