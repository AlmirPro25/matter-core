//! Neural Network Protocol mappings

pub struct SpikeTrain {
    pub sequence: Vec<bool>,
}

impl SpikeTrain {
    pub fn new(sequence: Vec<bool>) -> Self {
        Self { sequence }
    }

    pub fn to_binary_string(&self) -> String {
        self.sequence
            .iter()
            .map(|&b| if b { '1' } else { '0' })
            .collect()
    }
}
