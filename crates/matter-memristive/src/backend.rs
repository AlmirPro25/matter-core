use crate::CrossbarArray;
use matter_backend::{Backend, Value};

pub struct MemristiveBackend {
    crossbar: CrossbarArray,
}

impl MemristiveBackend {
    pub fn new() -> Self {
        Self {
            crossbar: CrossbarArray::new(8, 8),
        }
    }
}

impl Default for MemristiveBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MemristiveBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "write" => {
                if args.len() < 4 {
                    return Err("memristive.write: expected 4 arguments (row: int, col: int, voltage: float, duration: float)".to_string());
                }
                let row = args[0].as_int()? as usize;
                let col = args[1].as_int()? as usize;
                let voltage = args[2].as_float()?;
                let duration = args[3].as_float()?;

                if row >= self.crossbar.rows || col >= self.crossbar.cols {
                    return Err(format!(
                        "memristive.write: index out of bounds (rows: {}, cols: {})",
                        self.crossbar.rows, self.crossbar.cols
                    ));
                }

                self.crossbar.memristors[row][col].apply_voltage(voltage, duration);
                Ok(Value::Unit)
            }
            "read" => {
                if args.len() < 3 {
                    return Err("memristive.read: expected 3 arguments (row: int, col: int, voltage: float)".to_string());
                }
                let row = args[0].as_int()? as usize;
                let col = args[1].as_int()? as usize;
                let voltage = args[2].as_float()?;

                if row >= self.crossbar.rows || col >= self.crossbar.cols {
                    return Err(format!(
                        "memristive.read: index out of bounds (rows: {}, cols: {})",
                        self.crossbar.rows, self.crossbar.cols
                    ));
                }

                let current = self.crossbar.memristors[row][col].read_current(voltage);
                Ok(Value::Float(current))
            }
            "resistance" => {
                if args.len() < 2 {
                    return Err(
                        "memristive.resistance: expected 2 arguments (row: int, col: int)"
                            .to_string(),
                    );
                }
                let row = args[0].as_int()? as usize;
                let col = args[1].as_int()? as usize;

                if row >= self.crossbar.rows || col >= self.crossbar.cols {
                    return Err(format!(
                        "memristive.resistance: index out of bounds (rows: {}, cols: {})",
                        self.crossbar.rows, self.crossbar.cols
                    ));
                }

                let resistance = self.crossbar.memristors[row][col].resistance;
                Ok(Value::Float(resistance))
            }
            _ => Err(format!("Unknown memristive method: {}", method)),
        }
    }
}
