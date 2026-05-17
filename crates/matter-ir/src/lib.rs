use matter_bus::BusMessage;
use matter_bus::CpuCommand;
use matter_photonic_vpu::PhotonicInstruction;
use matter_vcpu::Instruction as VcpuInstruction;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum MatterOp {
    Cpu(VcpuInstruction),
    Photonic(PhotonicInstruction),
    Bus(BusMessage),
    BeginTask,
    EndTask,
    SleepFrames(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrError {
    ParseError { line: usize, message: String },
    UnknownInstruction { line: usize, instruction: String },
    InvalidNumber { line: usize, value: String },
}

impl std::fmt::Display for IrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

impl std::error::Error for IrError {}

pub fn parse_ir_program(source: &str) -> Result<Vec<MatterOp>, IrError> {
    let mut ops = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line = idx + 1;
        let clean = strip_comments(raw_line).trim();
        if clean.is_empty() {
            continue;
        }
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let op = parts[0];
        let parsed = match op {
            "BEGIN_TASK" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::BeginTask
            }
            "END_TASK" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::EndTask
            }
            "SLEEP_FRAMES" => {
                expect_arity(&parts, 2, line)?;
                MatterOp::SleepFrames(parse_u64(parts[1], line)?)
            }
            "BUS_SHUTDOWN" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::Bus(BusMessage::Shutdown)
            }
            "BUS_CPU_COMMAND" => {
                expect_arity(&parts, 2, line)?;
                let cmd = parts[1];
                let message = match cmd {
                    "hello" => BusMessage::CpuCommand(CpuCommand::SetPixel {
                        x: 0,
                        y: 0,
                        amplitude: 1.0,
                        phase: 0.0,
                    }),
                    "done" => BusMessage::CpuCommand(CpuCommand::Modulate {
                        x: 0,
                        y: 0,
                        factor: 1.0,
                    }),
                    _ => {
                        return Err(IrError::ParseError {
                            line,
                            message: format!("unsupported BUS_CPU_COMMAND: {cmd}"),
                        });
                    }
                };
                MatterOp::Bus(message)
            }
            "CPU_NOP" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::Cpu(VcpuInstruction::Nop)
            }
            "CPU_LOAD_CONST" => {
                expect_arity(&parts, 3, line)?;
                MatterOp::Cpu(VcpuInstruction::LoadConst {
                    reg: parse_reg(parts[1], line)?,
                    value: parse_i64(parts[2], line)?,
                })
            }
            "CPU_ADD" => {
                expect_arity(&parts, 4, line)?;
                MatterOp::Cpu(VcpuInstruction::Add {
                    dst: parse_reg(parts[1], line)?,
                    a: parse_reg(parts[2], line)?,
                    b: parse_reg(parts[3], line)?,
                })
            }
            "CPU_PRINT" => {
                expect_arity(&parts, 2, line)?;
                MatterOp::Cpu(VcpuInstruction::Print {
                    reg: parse_reg(parts[1], line)?,
                })
            }
            "CPU_HALT" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::Cpu(VcpuInstruction::Halt)
            }
            "PHOTONIC_NOP" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::Photonic(PhotonicInstruction::Nop)
            }
            "PHOTONIC_SET_PIXEL" => {
                expect_arity(&parts, 5, line)?;
                MatterOp::Photonic(PhotonicInstruction::SetPixel {
                    x: parse_usize(parts[1], line)?,
                    y: parse_usize(parts[2], line)?,
                    amplitude: parse_f32(parts[3], line)?,
                    phase: parse_f32(parts[4], line)?,
                })
            }
            "PHOTONIC_INTERFERE" => {
                expect_arity(&parts, 7, line)?;
                MatterOp::Photonic(PhotonicInstruction::Interfere {
                    ax: parse_usize(parts[1], line)?,
                    ay: parse_usize(parts[2], line)?,
                    bx: parse_usize(parts[3], line)?,
                    by: parse_usize(parts[4], line)?,
                    out_x: parse_usize(parts[5], line)?,
                    out_y: parse_usize(parts[6], line)?,
                })
            }
            "PHOTONIC_PRINT_INTENSITY" => {
                expect_arity(&parts, 3, line)?;
                MatterOp::Photonic(PhotonicInstruction::PrintIntensity {
                    x: parse_usize(parts[1], line)?,
                    y: parse_usize(parts[2], line)?,
                })
            }
            "PHOTONIC_HALT" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::Photonic(PhotonicInstruction::Halt)
            }
            _ => {
                return Err(IrError::UnknownInstruction {
                    line,
                    instruction: op.to_string(),
                });
            }
        };
        ops.push(parsed);
    }
    Ok(ops)
}

pub fn parse_ir_file<P: AsRef<Path>>(path: P) -> Result<Vec<MatterOp>, IrError> {
    let source = std::fs::read_to_string(path).map_err(|e| IrError::ParseError {
        line: 0,
        message: format!("failed to read ir file: {e}"),
    })?;
    parse_ir_program(&source)
}

pub fn parse_matter_program(source: &str) -> Result<Vec<MatterOp>, IrError> {
    let mut ops = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line = idx + 1;
        let clean = strip_comments(raw_line).trim();
        if clean.is_empty() {
            continue;
        }
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let head = parts[0];
        let parsed = match head {
            "begin" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::BeginTask
            }
            "end" => {
                expect_arity(&parts, 1, line)?;
                MatterOp::EndTask
            }
            "sleep" => {
                expect_arity(&parts, 2, line)?;
                MatterOp::SleepFrames(parse_u64(parts[1], line)?)
            }
            "bus" => {
                expect_arity(&parts, 2, line)?;
                match parts[1] {
                    "shutdown" => MatterOp::Bus(BusMessage::Shutdown),
                    other => {
                        return Err(IrError::ParseError {
                            line,
                            message: format!("unsupported bus op: {other}"),
                        });
                    }
                }
            }
            "cpu" => {
                if parts.len() < 2 {
                    return Err(IrError::ParseError {
                        line,
                        message: "missing cpu operation".to_string(),
                    });
                }
                match parts[1] {
                    "nop" => {
                        expect_arity(&parts, 2, line)?;
                        MatterOp::Cpu(VcpuInstruction::Nop)
                    }
                    "load_const" => {
                        expect_arity(&parts, 4, line)?;
                        MatterOp::Cpu(VcpuInstruction::LoadConst {
                            reg: parse_reg(parts[2], line)?,
                            value: parse_i64(parts[3], line)?,
                        })
                    }
                    "add" => {
                        expect_arity(&parts, 5, line)?;
                        MatterOp::Cpu(VcpuInstruction::Add {
                            dst: parse_reg(parts[2], line)?,
                            a: parse_reg(parts[3], line)?,
                            b: parse_reg(parts[4], line)?,
                        })
                    }
                    "print" => {
                        expect_arity(&parts, 3, line)?;
                        MatterOp::Cpu(VcpuInstruction::Print {
                            reg: parse_reg(parts[2], line)?,
                        })
                    }
                    "halt" => {
                        expect_arity(&parts, 2, line)?;
                        MatterOp::Cpu(VcpuInstruction::Halt)
                    }
                    other => {
                        return Err(IrError::ParseError {
                            line,
                            message: format!("unsupported cpu op: {other}"),
                        });
                    }
                }
            }
            "photonic" => {
                if parts.len() < 2 {
                    return Err(IrError::ParseError {
                        line,
                        message: "missing photonic operation".to_string(),
                    });
                }
                match parts[1] {
                    "nop" => {
                        expect_arity(&parts, 2, line)?;
                        MatterOp::Photonic(PhotonicInstruction::Nop)
                    }
                    "set_pixel" => {
                        expect_arity(&parts, 6, line)?;
                        MatterOp::Photonic(PhotonicInstruction::SetPixel {
                            x: parse_usize(parts[2], line)?,
                            y: parse_usize(parts[3], line)?,
                            amplitude: parse_f32(parts[4], line)?,
                            phase: parse_f32(parts[5], line)?,
                        })
                    }
                    "interfere" => {
                        expect_arity(&parts, 8, line)?;
                        MatterOp::Photonic(PhotonicInstruction::Interfere {
                            ax: parse_usize(parts[2], line)?,
                            ay: parse_usize(parts[3], line)?,
                            bx: parse_usize(parts[4], line)?,
                            by: parse_usize(parts[5], line)?,
                            out_x: parse_usize(parts[6], line)?,
                            out_y: parse_usize(parts[7], line)?,
                        })
                    }
                    "print_intensity" => {
                        expect_arity(&parts, 4, line)?;
                        MatterOp::Photonic(PhotonicInstruction::PrintIntensity {
                            x: parse_usize(parts[2], line)?,
                            y: parse_usize(parts[3], line)?,
                        })
                    }
                    "halt" => {
                        expect_arity(&parts, 2, line)?;
                        MatterOp::Photonic(PhotonicInstruction::Halt)
                    }
                    other => {
                        return Err(IrError::ParseError {
                            line,
                            message: format!("unsupported photonic op: {other}"),
                        });
                    }
                }
            }
            _ => {
                return Err(IrError::UnknownInstruction {
                    line,
                    instruction: head.to_string(),
                });
            }
        };
        ops.push(parsed);
    }
    Ok(ops)
}

pub fn parse_matter_file<P: AsRef<Path>>(path: P) -> Result<Vec<MatterOp>, IrError> {
    let source = std::fs::read_to_string(path).map_err(|e| IrError::ParseError {
        line: 0,
        message: format!("failed to read matter file: {e}"),
    })?;
    parse_matter_program(&source)
}

pub fn emit_ir_program(ops: &[MatterOp]) -> String {
    let mut out = String::new();
    for op in ops {
        let line = match op {
            MatterOp::BeginTask => "BEGIN_TASK".to_string(),
            MatterOp::EndTask => "END_TASK".to_string(),
            MatterOp::SleepFrames(n) => format!("SLEEP_FRAMES {n}"),
            MatterOp::Bus(BusMessage::Shutdown) => "BUS_SHUTDOWN".to_string(),
            MatterOp::Bus(BusMessage::CpuCommand(CpuCommand::SetPixel { .. })) => {
                "BUS_CPU_COMMAND hello".to_string()
            }
            MatterOp::Bus(BusMessage::CpuCommand(CpuCommand::Modulate { .. })) => {
                "BUS_CPU_COMMAND done".to_string()
            }
            MatterOp::Bus(BusMessage::PixelEnergy { x, y, energy }) => {
                format!("BUS_PIXEL_ENERGY {x} {y} {energy}")
            }
            MatterOp::Bus(BusMessage::MotionDetected { x, y, intensity }) => {
                format!("BUS_MOTION_DETECTED {x} {y} {intensity}")
            }
            MatterOp::Cpu(VcpuInstruction::Nop) => "CPU_NOP".to_string(),
            MatterOp::Cpu(VcpuInstruction::LoadConst { reg, value }) => {
                format!("CPU_LOAD_CONST r{reg} {value}")
            }
            MatterOp::Cpu(VcpuInstruction::Add { dst, a, b }) => {
                format!("CPU_ADD r{dst} r{a} r{b}")
            }
            MatterOp::Cpu(VcpuInstruction::Sub { dst, a, b }) => {
                format!("CPU_SUB r{dst} r{a} r{b}")
            }
            MatterOp::Cpu(VcpuInstruction::Mul { dst, a, b }) => {
                format!("CPU_MUL r{dst} r{a} r{b}")
            }
            MatterOp::Cpu(VcpuInstruction::Div { dst, a, b }) => {
                format!("CPU_DIV r{dst} r{a} r{b}")
            }
            MatterOp::Cpu(VcpuInstruction::Print { reg }) => format!("CPU_PRINT r{reg}"),
            MatterOp::Cpu(VcpuInstruction::Halt) => "CPU_HALT".to_string(),
            MatterOp::Cpu(_) => "CPU_UNSUPPORTED_OP".to_string(),
            MatterOp::Photonic(PhotonicInstruction::Nop) => "PHOTONIC_NOP".to_string(),
            MatterOp::Photonic(PhotonicInstruction::SetPixel {
                x,
                y,
                amplitude,
                phase,
            }) => {
                format!("PHOTONIC_SET_PIXEL {x} {y} {amplitude} {phase}")
            }
            MatterOp::Photonic(PhotonicInstruction::Interfere {
                ax,
                ay,
                bx,
                by,
                out_x,
                out_y,
            }) => {
                format!("PHOTONIC_INTERFERE {ax} {ay} {bx} {by} {out_x} {out_y}")
            }
            MatterOp::Photonic(PhotonicInstruction::PrintIntensity { x, y }) => {
                format!("PHOTONIC_PRINT_INTENSITY {x} {y}")
            }
            MatterOp::Photonic(PhotonicInstruction::Halt) => "PHOTONIC_HALT".to_string(),
            MatterOp::Photonic(PhotonicInstruction::Modulate { x, y, factor }) => {
                format!("PHOTONIC_MODULATE {x} {y} {factor}")
            }
            MatterOp::Photonic(PhotonicInstruction::Threshold { threshold }) => {
                format!("PHOTONIC_THRESHOLD {threshold}")
            }
            MatterOp::Photonic(PhotonicInstruction::Normalize) => "PHOTONIC_NORMALIZE".to_string(),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
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

fn expect_arity(parts: &[&str], expected: usize, line: usize) -> Result<(), IrError> {
    if parts.len() != expected {
        return Err(IrError::ParseError {
            line,
            message: format!("expected {expected} tokens, got {}", parts.len()),
        });
    }
    Ok(())
}

fn parse_u64(token: &str, line: usize) -> Result<u64, IrError> {
    token.parse::<u64>().map_err(|_| IrError::InvalidNumber {
        line,
        value: token.to_string(),
    })
}

fn parse_i64(token: &str, line: usize) -> Result<i64, IrError> {
    token.parse::<i64>().map_err(|_| IrError::InvalidNumber {
        line,
        value: token.to_string(),
    })
}

fn parse_usize(token: &str, line: usize) -> Result<usize, IrError> {
    token.parse::<usize>().map_err(|_| IrError::InvalidNumber {
        line,
        value: token.to_string(),
    })
}

fn parse_f32(token: &str, line: usize) -> Result<f32, IrError> {
    token.parse::<f32>().map_err(|_| IrError::InvalidNumber {
        line,
        value: token.to_string(),
    })
}

fn parse_reg(token: &str, line: usize) -> Result<usize, IrError> {
    if token.len() != 2 || !token.starts_with('r') {
        return Err(IrError::ParseError {
            line,
            message: format!("invalid register: {token}"),
        });
    }
    let idx = token[1..]
        .parse::<usize>()
        .map_err(|_| IrError::InvalidNumber {
            line,
            value: token.to_string(),
        })?;
    if idx > 7 {
        return Err(IrError::ParseError {
            line,
            message: format!("register out of range: {token}"),
        });
    }
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ir_ops() {
        let ops = [
            MatterOp::BeginTask,
            MatterOp::SleepFrames(1),
            MatterOp::EndTask,
        ];
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn parse_begin_end_task() {
        let ops = parse_ir_program("BEGIN_TASK\nEND_TASK").expect("parse should succeed");
        assert_eq!(ops, vec![MatterOp::BeginTask, MatterOp::EndTask]);
    }

    #[test]
    fn parse_cpu_load_const() {
        let ops = parse_ir_program("CPU_LOAD_CONST r0 10").expect("parse should succeed");
        assert_eq!(
            ops,
            vec![MatterOp::Cpu(VcpuInstruction::LoadConst {
                reg: 0,
                value: 10
            })]
        );
    }

    #[test]
    fn parse_photonic_set_pixel() {
        let ops = parse_ir_program("PHOTONIC_SET_PIXEL 0 0 1.0 0.0").expect("parse should succeed");
        assert_eq!(
            ops,
            vec![MatterOp::Photonic(PhotonicInstruction::SetPixel {
                x: 0,
                y: 0,
                amplitude: 1.0,
                phase: 0.0
            })]
        );
    }

    #[test]
    fn parse_bus_shutdown() {
        let ops = parse_ir_program("BUS_SHUTDOWN").expect("parse should succeed");
        assert_eq!(ops, vec![MatterOp::Bus(BusMessage::Shutdown)]);
    }

    #[test]
    fn parse_comments_and_empty_lines() {
        let src = "\n# a\nCPU_NOP\n; b\n\nEND_TASK # c\n";
        let ops = parse_ir_program(src).expect("parse should succeed");
        assert_eq!(
            ops,
            vec![MatterOp::Cpu(VcpuInstruction::Nop), MatterOp::EndTask]
        );
    }

    #[test]
    fn parse_unknown_instruction_returns_error() {
        let err = parse_ir_program("UNKNOWN_OP").expect_err("should fail");
        assert_eq!(
            err,
            IrError::UnknownInstruction {
                line: 1,
                instruction: "UNKNOWN_OP".to_string()
            }
        );
    }

    #[test]
    fn parse_matter_program_cpu_flow() {
        let src = "\
begin
cpu load_const r0 10
cpu load_const r1 20
cpu add r2 r0 r1
cpu print r2
end
";
        let ops = parse_matter_program(src).expect("parse should succeed");
        assert_eq!(ops.len(), 6);
        assert_eq!(ops[0], MatterOp::BeginTask);
        assert_eq!(ops[5], MatterOp::EndTask);
    }

    #[test]
    fn parse_matter_program_unknown_instruction() {
        let err = parse_matter_program("foo bar").expect_err("should fail");
        assert_eq!(
            err,
            IrError::UnknownInstruction {
                line: 1,
                instruction: "foo".to_string()
            }
        );
    }

    #[test]
    fn parse_ir_file_reads_demo() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("matter-scheduler")
            .join("examples")
            .join("demo.mir");
        let ops = parse_ir_file(path).expect("parse file should succeed");
        assert!(!ops.is_empty());
    }

    #[test]
    fn parse_matter_file_reads_demo() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("matter-scheduler")
            .join("examples")
            .join("demo.matter");
        let ops = parse_matter_file(path).expect("parse file should succeed");
        assert!(!ops.is_empty());
    }

    #[test]
    fn emit_ir_program_contains_expected_lines() {
        let ops = vec![
            MatterOp::BeginTask,
            MatterOp::Cpu(VcpuInstruction::LoadConst { reg: 0, value: 10 }),
            MatterOp::EndTask,
        ];
        let txt = emit_ir_program(&ops);
        assert!(txt.contains("BEGIN_TASK"));
        assert!(txt.contains("CPU_LOAD_CONST r0 10"));
        assert!(txt.contains("END_TASK"));
    }
}
