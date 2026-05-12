use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum CpuCommand {
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum BusMessage {
    PixelEnergy { x: usize, y: usize, energy: f32 },
    MotionDetected { x: usize, y: usize, intensity: f32 },
    CpuCommand(CpuCommand),
    Shutdown,
}

#[derive(Debug, Default, Clone)]
pub struct MessageBus {
    queue: VecDeque<BusMessage>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn send(&mut self, message: BusMessage) {
        self.queue.push_back(message);
    }

    pub fn receive(&mut self) -> Option<BusMessage> {
        self.queue.pop_front()
    }

    pub fn drain(&mut self) -> Vec<BusMessage> {
        self.queue.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_receive_and_drain_work() {
        let mut bus = MessageBus::new();
        bus.send(BusMessage::Shutdown);
        bus.send(BusMessage::MotionDetected {
            x: 1,
            y: 2,
            intensity: 0.8,
        });

        assert_eq!(bus.len(), 2);
        assert_eq!(bus.receive(), Some(BusMessage::Shutdown));

        let rest = bus.drain();
        assert_eq!(rest.len(), 1);
        assert!(bus.is_empty());
    }
}
