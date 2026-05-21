//! Microelectrode Array (MEA) interface

pub struct HighDensityMEA {
    pub rows: usize,
    pub cols: usize,
    pub sampling_rate_hz: f32,
}

impl HighDensityMEA {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            sampling_rate_hz: 20_000.0, // 20kHz sampling
        }
    }
}
