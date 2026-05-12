use matter_bus::{BusMessage, CpuCommand, MessageBus};
use matter_display::VirtualMonitor;
use matter_ir::MatterOp;
use matter_photonic_vpu::{PhotonicError, PhotonicInstruction, PhotonicProcessor};
use matter_vcpu::{Instruction as VcpuInstruction, VirtualCpu, VirtualCpuError};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[derive(Debug)]
pub enum SchedulerError {
    Vcpu(VirtualCpuError),
    Pvpu(PhotonicError),
    Thread(String),
    TaskBudgetExceeded { task_id: u64, kind: BudgetKind },
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vcpu(e) => write!(f, "vcpu error: {e}"),
            Self::Pvpu(e) => write!(f, "pvpu error: {e}"),
            Self::Thread(e) => write!(f, "thread error: {e}"),
            Self::TaskBudgetExceeded { task_id, kind } => {
                write!(f, "task budget exceeded: task_id={task_id}, kind={kind}")
            }
        }
    }
}

impl std::error::Error for SchedulerError {}

impl From<VirtualCpuError> for SchedulerError {
    fn from(value: VirtualCpuError) -> Self {
        Self::Vcpu(value)
    }
}

impl From<PhotonicError> for SchedulerError {
    fn from(value: PhotonicError) -> Self {
        Self::Pvpu(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HybridStats {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_budget_exceeded: u64,
    pub frames: u64,
    pub total_cycles: u64,
    pub total_energy: f32,
    pub messages_exchanged: u64,
    pub bus_depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskBudget {
    pub max_frames: Option<u64>,
    pub max_cycles: Option<u64>,
    pub max_energy: Option<f32>,
    pub max_messages: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledTask {
    pub id: u64,
    pub task: HybridTask,
    pub budget: Option<TaskBudget>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskOutcome {
    Completed,
    FailedBudget { kind: BudgetKind },
    FailedError,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskExecutionRecord {
    pub task_id: u64,
    pub outcome: TaskOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetKind {
    Frames,
    Cycles,
    Energy,
    Messages,
}

impl std::fmt::Display for BudgetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frames => write!(f, "frames"),
            Self::Cycles => write!(f, "cycles"),
            Self::Energy => write!(f, "energy"),
            Self::Messages => write!(f, "messages"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HybridTask {
    CpuSteps {
        steps: usize,
    },
    PhotonicRunProgram {
        source: String,
        width: usize,
        height: usize,
    },
    PhotonicInterferenceDemo,
    ProjectPhotonicToDisplay {
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
    },
    IrProgram {
        ops: Vec<MatterOp>,
    },
    Shutdown,
}

pub struct HybridRuntime {
    pub vcpu: VirtualCpu,
    pub pvpu: PhotonicProcessor,
    pub bus: MessageBus,
    pub display: Option<VirtualMonitor>,
    frames: u64,
    total_cycles: u64,
    total_energy: f32,
    messages_exchanged: u64,
    tasks_completed: u64,
    tasks_failed: u64,
    tasks_budget_exceeded: u64,
    shutdown: bool,
    tasks: VecDeque<ScheduledTask>,
    history: Vec<TaskExecutionRecord>,
    history_limit: Option<usize>,
    next_task_id: u64,
}

impl HybridRuntime {
    pub fn new(vcpu: VirtualCpu, pvpu: PhotonicProcessor) -> Self {
        Self {
            vcpu,
            pvpu,
            bus: MessageBus::new(),
            display: None,
            frames: 0,
            total_cycles: 0,
            total_energy: 0.0,
            messages_exchanged: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            tasks_budget_exceeded: 0,
            shutdown: false,
            tasks: VecDeque::new(),
            history: Vec::new(),
            history_limit: None,
            next_task_id: 1,
        }
    }

    pub fn attach_display(&mut self, display: VirtualMonitor) {
        self.display = Some(display);
    }

    pub fn step(&mut self) -> Result<(), SchedulerError> {
        if self.shutdown {
            return Ok(());
        }

        if self.vcpu.running {
            self.vcpu.step()?;
        }

        for message in take_vcpu_messages(&mut self.vcpu) {
            self.bus.send(message);
        }

        self.pvpu.step(&PhotonicInstruction::Nop)?;
        self.bus.send(BusMessage::PixelEnergy {
            x: 0,
            y: 0,
            energy: self.pvpu.total_light_energy(),
        });

        self.process_bus_messages()?;
        self.refresh_stats();
        self.frames = self.frames.saturating_add(1);
        Ok(())
    }

    pub fn run_frames(&mut self, count: usize) -> Result<(), SchedulerError> {
        for _ in 0..count {
            if self.shutdown {
                break;
            }
            self.step()?;
        }
        Ok(())
    }

    pub fn run_threaded_frames(&mut self, count: usize) -> Result<(), SchedulerError> {
        let vcpu_arc = Arc::new(Mutex::new(self.vcpu.clone()));
        let pvpu_arc = Arc::new(Mutex::new(self.pvpu.clone()));

        for _ in 0..count {
            if self.shutdown {
                break;
            }

            let (tx, rx) = mpsc::channel::<BusMessage>();

            let vcpu_clone = Arc::clone(&vcpu_arc);
            let tx_vcpu = tx.clone();
            let vcpu_worker = thread::spawn(move || -> Result<(), SchedulerError> {
                let mut vcpu = vcpu_clone
                    .lock()
                    .map_err(|_| SchedulerError::Thread("vcpu mutex poisoned".to_string()))?;
                if vcpu.running {
                    vcpu.step()?;
                }
                for msg in take_vcpu_messages(&mut vcpu) {
                    tx_vcpu.send(msg).map_err(|_| {
                        SchedulerError::Thread("failed to send vcpu bus message".to_string())
                    })?;
                }
                Ok(())
            });

            let pvpu_clone = Arc::clone(&pvpu_arc);
            let tx_pvpu = tx.clone();
            let pvpu_worker = thread::spawn(move || -> Result<(), SchedulerError> {
                let mut pvpu = pvpu_clone
                    .lock()
                    .map_err(|_| SchedulerError::Thread("pvpu mutex poisoned".to_string()))?;
                pvpu.step(&PhotonicInstruction::Nop)?;
                tx_pvpu
                    .send(BusMessage::PixelEnergy {
                        x: 0,
                        y: 0,
                        energy: pvpu.total_light_energy(),
                    })
                    .map_err(|_| {
                        SchedulerError::Thread("failed to send pvpu bus message".to_string())
                    })?;
                Ok(())
            });

            drop(tx);

            let vcpu_result = vcpu_worker
                .join()
                .map_err(|_| SchedulerError::Thread("vcpu worker panicked".to_string()))?;
            vcpu_result?;

            let pvpu_result = pvpu_worker
                .join()
                .map_err(|_| SchedulerError::Thread("pvpu worker panicked".to_string()))?;
            pvpu_result?;

            for msg in rx.try_iter() {
                self.bus.send(msg);
            }

            {
                let mut pvpu = pvpu_arc
                    .lock()
                    .map_err(|_| SchedulerError::Thread("pvpu mutex poisoned".to_string()))?;
                self.process_bus_messages_with_pvpu(&mut pvpu)?;
            }

            {
                let vcpu = vcpu_arc
                    .lock()
                    .map_err(|_| SchedulerError::Thread("vcpu mutex poisoned".to_string()))?;
                self.vcpu = vcpu.clone();
            }
            {
                let pvpu = pvpu_arc
                    .lock()
                    .map_err(|_| SchedulerError::Thread("pvpu mutex poisoned".to_string()))?;
                self.pvpu = pvpu.clone();
            }

            self.refresh_stats();
            self.frames = self.frames.saturating_add(1);
        }

        Ok(())
    }

    pub fn stats(&self) -> HybridStats {
        HybridStats {
            tasks_completed: self.tasks_completed,
            tasks_failed: self.tasks_failed,
            tasks_budget_exceeded: self.tasks_budget_exceeded,
            frames: self.frames,
            total_cycles: self.total_cycles,
            total_energy: self.total_energy,
            messages_exchanged: self.messages_exchanged,
            bus_depth: self.bus.len(),
        }
    }

    pub fn run_ir_program(&mut self, ops: &[MatterOp]) -> Result<HybridStats, SchedulerError> {
        let mut current_ir_task_id: Option<u64> = None;
        for op in ops {
            match op {
                MatterOp::Cpu(instr) => {
                    self.vcpu.load_program(vec![instr.clone()])?;
                    if self.vcpu.running {
                        self.vcpu.step()?;
                        self.vcpu.running = false;
                    }
                }
                MatterOp::Photonic(instr) => {
                    self.pvpu.step(instr)?;
                }
                MatterOp::Bus(message) => {
                    self.bus.send(message.clone());
                    self.process_bus_messages()?;
                }
                MatterOp::BeginTask => {
                    let id = self.next_task_id;
                    self.next_task_id = self.next_task_id.saturating_add(1);
                    current_ir_task_id = Some(id);
                }
                MatterOp::EndTask => {
                    if let Some(id) = current_ir_task_id.take() {
                        self.tasks_completed = self.tasks_completed.saturating_add(1);
                        self.push_history(TaskExecutionRecord {
                            task_id: id,
                            outcome: TaskOutcome::Completed,
                        });
                    }
                }
                MatterOp::SleepFrames(frames) => {
                    let sleep_frames = match usize::try_from(*frames) {
                        Ok(v) => v,
                        Err(_) => usize::MAX,
                    };
                    self.run_frames(sleep_frames)?;
                }
            }

            self.frames = self.frames.saturating_add(1);
            self.refresh_stats();
        }

        Ok(self.stats())
    }

    pub fn run_ir_source(&mut self, source: &str) -> Result<HybridStats, SchedulerError> {
        let ops = matter_ir::parse_ir_program(source)
            .map_err(|e| SchedulerError::Thread(format!("ir parse error: {e}")))?;
        self.run_ir_program(&ops)
    }

    pub fn run_ir_file<P: AsRef<Path>>(&mut self, path: P) -> Result<HybridStats, SchedulerError> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| SchedulerError::Thread(format!("failed to read ir file: {e}")))?;
        self.run_ir_source(&source)
    }

    pub fn project_photonic_to_display(
        &self,
        display: &mut VirtualMonitor,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
    ) -> Result<(), SchedulerError> {
        display
            .set_pixel_from_photonic(&self.pvpu, src_x, src_y, dst_x, dst_y)
            .map_err(|e| SchedulerError::Thread(format!("display projection error: {e}")))?;
        Ok(())
    }

    pub fn push_task(&mut self, task: HybridTask) {
        let _ = self.push_task_with_budget(task, None);
    }

    pub fn push_task_with_budget(&mut self, task: HybridTask, budget: Option<TaskBudget>) -> u64 {
        let id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        self.tasks.push_back(ScheduledTask { id, task, budget });
        id
    }

    pub fn task_queue_len(&self) -> usize {
        self.tasks.len()
    }

    pub fn task_history(&self) -> &[TaskExecutionRecord] {
        &self.history
    }

    pub fn clear_task_history(&mut self) {
        self.history.clear();
    }

    pub fn set_history_limit(&mut self, limit: Option<usize>) {
        self.history_limit = limit;
        self.enforce_history_limit();
    }

    pub fn task_record(&self, task_id: u64) -> Option<&TaskExecutionRecord> {
        self.history.iter().find(|r| r.task_id == task_id)
    }

    pub fn last_task_record(&self) -> Option<&TaskExecutionRecord> {
        self.history.last()
    }

    pub fn run_next_task(&mut self) -> Result<bool, SchedulerError> {
        self.run_next_task_with_budget()
    }

    pub fn run_next_task_with_budget(&mut self) -> Result<bool, SchedulerError> {
        if self.shutdown {
            return Ok(false);
        }
        let Some(scheduled) = self.tasks.pop_front() else {
            return Ok(false);
        };

        let baseline_frames = self.frames;
        let baseline_cycles = self.total_cycles;
        let baseline_energy = self.total_energy;
        let baseline_messages = self.messages_exchanged;

        let execution_result = match scheduled.task.clone() {
            HybridTask::CpuSteps { steps } => {
                for _ in 0..steps {
                    if self.shutdown {
                        break;
                    }
                    if let Err(e) = self.step() {
                        self.tasks_failed = self.tasks_failed.saturating_add(1);
                        self.push_history(TaskExecutionRecord {
                            task_id: scheduled.id,
                            outcome: TaskOutcome::FailedError,
                        });
                        return Err(e);
                    }
                }
                Ok(())
            }
            HybridTask::PhotonicRunProgram {
                source,
                width,
                height,
            } => {
                let result = (|| -> Result<(), SchedulerError> {
                    let program = matter_photonic_vpu::parse_program(&source)?;
                    let mut processor = PhotonicProcessor::new(width, height)?;
                    processor.run(&program)?;
                    self.pvpu = processor;
                    self.bus.send(BusMessage::PixelEnergy {
                        x: 0,
                        y: 0,
                        energy: self.pvpu.total_light_energy(),
                    });
                    self.process_bus_messages()?;
                    self.refresh_stats();
                    self.frames = self.frames.saturating_add(1);
                    Ok(())
                })();
                if result.is_err() {
                    self.tasks_failed = self.tasks_failed.saturating_add(1);
                    self.push_history(TaskExecutionRecord {
                        task_id: scheduled.id,
                        outcome: TaskOutcome::FailedError,
                    });
                }
                result
            }
            HybridTask::PhotonicInterferenceDemo => {
                let result = (|| -> Result<(), SchedulerError> {
                    self.pvpu.set_pixel(0, 0, 1.0, 0.0)?;
                    self.pvpu.set_pixel(1, 0, 1.0, std::f32::consts::PI)?;
                    self.pvpu.interfere(0, 0, 1, 0, 2, 0)?;
                    self.bus.send(BusMessage::PixelEnergy {
                        x: 2,
                        y: 0,
                        energy: self.pvpu.total_light_energy(),
                    });
                    self.process_bus_messages()?;
                    self.refresh_stats();
                    self.frames = self.frames.saturating_add(1);
                    Ok(())
                })();
                if result.is_err() {
                    self.tasks_failed = self.tasks_failed.saturating_add(1);
                    self.push_history(TaskExecutionRecord {
                        task_id: scheduled.id,
                        outcome: TaskOutcome::FailedError,
                    });
                }
                result
            }
            HybridTask::ProjectPhotonicToDisplay {
                src_x,
                src_y,
                dst_x,
                dst_y,
            } => {
                let result = (|| -> Result<(), SchedulerError> {
                    let intensity = self.pvpu.intensity_at(src_x, src_y).map_err(|e| {
                        SchedulerError::Thread(format!("photonic intensity error: {e}"))
                    })?;
                    let display = self.display.as_mut().ok_or_else(|| {
                        SchedulerError::Thread("display not attached".to_string())
                    })?;
                    display
                        .set_pixel_intensity(dst_x, dst_y, intensity)
                        .map_err(|e| {
                            SchedulerError::Thread(format!("display projection error: {e}"))
                        })?;
                    let _ = display.present().map_err(|e| {
                        SchedulerError::Thread(format!("display present error: {e}"))
                    })?;
                    self.refresh_stats();
                    self.frames = self.frames.saturating_add(1);
                    Ok(())
                })();
                if result.is_err() {
                    self.tasks_failed = self.tasks_failed.saturating_add(1);
                    self.push_history(TaskExecutionRecord {
                        task_id: scheduled.id,
                        outcome: TaskOutcome::FailedError,
                    });
                }
                result
            }
            HybridTask::IrProgram { ops } => {
                let result = self.run_ir_program(&ops).map(|_| ());
                if result.is_err() {
                    self.tasks_failed = self.tasks_failed.saturating_add(1);
                    self.push_history(TaskExecutionRecord {
                        task_id: scheduled.id,
                        outcome: TaskOutcome::FailedError,
                    });
                }
                result
            }
            HybridTask::Shutdown => {
                let result = (|| -> Result<(), SchedulerError> {
                    self.bus.send(BusMessage::Shutdown);
                    self.process_bus_messages()?;
                    self.refresh_stats();
                    Ok(())
                })();
                if result.is_err() {
                    self.tasks_failed = self.tasks_failed.saturating_add(1);
                    self.push_history(TaskExecutionRecord {
                        task_id: scheduled.id,
                        outcome: TaskOutcome::FailedError,
                    });
                }
                result
            }
        };

        execution_result?;

        if let Some(kind) = self.check_budget_exceeded(
            &scheduled,
            baseline_frames,
            baseline_cycles,
            baseline_energy,
            baseline_messages,
        ) {
            self.tasks_failed = self.tasks_failed.saturating_add(1);
            self.tasks_budget_exceeded = self.tasks_budget_exceeded.saturating_add(1);
            self.push_history(TaskExecutionRecord {
                task_id: scheduled.id,
                outcome: TaskOutcome::FailedBudget { kind },
            });
            return Err(SchedulerError::TaskBudgetExceeded {
                task_id: scheduled.id,
                kind,
            });
        }

        self.tasks_completed = self.tasks_completed.saturating_add(1);
        self.push_history(TaskExecutionRecord {
            task_id: scheduled.id,
            outcome: TaskOutcome::Completed,
        });
        Ok(true)
    }

    pub fn run_all_tasks(&mut self) -> Result<(), SchedulerError> {
        while self.run_next_task()? {
            if self.shutdown {
                break;
            }
        }
        Ok(())
    }

    pub fn run_next_task_threaded(&mut self) -> Result<bool, SchedulerError> {
        if self.shutdown {
            return Ok(false);
        }
        let Some(scheduled) = self.tasks.pop_front() else {
            return Ok(false);
        };

        let baseline_frames = self.frames;
        let baseline_cycles = self.total_cycles;
        let baseline_energy = self.total_energy;
        let baseline_messages = self.messages_exchanged;

        let execution_result = match scheduled.task.clone() {
            HybridTask::CpuSteps { steps } => self.run_threaded_frames(steps),
            HybridTask::PhotonicRunProgram {
                source,
                width,
                height,
            } => {
                let program = matter_photonic_vpu::parse_program(&source)?;
                let mut processor = PhotonicProcessor::new(width, height)?;
                processor.run(&program)?;
                self.pvpu = processor;
                self.bus.send(BusMessage::PixelEnergy {
                    x: 0,
                    y: 0,
                    energy: self.pvpu.total_light_energy(),
                });
                self.process_bus_messages()?;
                self.refresh_stats();
                self.frames = self.frames.saturating_add(1);
                Ok(())
            }
            HybridTask::PhotonicInterferenceDemo => {
                self.pvpu.set_pixel(0, 0, 1.0, 0.0)?;
                self.pvpu.set_pixel(1, 0, 1.0, std::f32::consts::PI)?;
                self.pvpu.interfere(0, 0, 1, 0, 2, 0)?;
                self.bus.send(BusMessage::PixelEnergy {
                    x: 2,
                    y: 0,
                    energy: self.pvpu.total_light_energy(),
                });
                self.process_bus_messages()?;
                self.refresh_stats();
                self.frames = self.frames.saturating_add(1);
                Ok(())
            }
            HybridTask::ProjectPhotonicToDisplay {
                src_x,
                src_y,
                dst_x,
                dst_y,
            } => {
                let intensity = self.pvpu.intensity_at(src_x, src_y).map_err(|e| {
                    SchedulerError::Thread(format!("photonic intensity error: {e}"))
                })?;
                let display = self
                    .display
                    .as_mut()
                    .ok_or_else(|| SchedulerError::Thread("display not attached".to_string()))?;
                display
                    .set_pixel_intensity(dst_x, dst_y, intensity)
                    .map_err(|e| {
                        SchedulerError::Thread(format!("display projection error: {e}"))
                    })?;
                let _ = display
                    .present()
                    .map_err(|e| SchedulerError::Thread(format!("display present error: {e}")))?;
                self.refresh_stats();
                self.frames = self.frames.saturating_add(1);
                Ok(())
            }
            HybridTask::IrProgram { ops } => self.run_ir_program(&ops).map(|_| ()),
            HybridTask::Shutdown => {
                self.bus.send(BusMessage::Shutdown);
                self.process_bus_messages()?;
                self.refresh_stats();
                Ok(())
            }
        };

        if let Err(e) = execution_result {
            self.tasks_failed = self.tasks_failed.saturating_add(1);
            self.push_history(TaskExecutionRecord {
                task_id: scheduled.id,
                outcome: TaskOutcome::FailedError,
            });
            return Err(e);
        }

        if let Some(kind) = self.check_budget_exceeded(
            &scheduled,
            baseline_frames,
            baseline_cycles,
            baseline_energy,
            baseline_messages,
        ) {
            self.tasks_failed = self.tasks_failed.saturating_add(1);
            self.tasks_budget_exceeded = self.tasks_budget_exceeded.saturating_add(1);
            self.push_history(TaskExecutionRecord {
                task_id: scheduled.id,
                outcome: TaskOutcome::FailedBudget { kind },
            });
            return Err(SchedulerError::TaskBudgetExceeded {
                task_id: scheduled.id,
                kind,
            });
        }

        self.tasks_completed = self.tasks_completed.saturating_add(1);
        self.push_history(TaskExecutionRecord {
            task_id: scheduled.id,
            outcome: TaskOutcome::Completed,
        });
        Ok(true)
    }

    pub fn run_all_tasks_threaded(&mut self) -> Result<(), SchedulerError> {
        while self.run_next_task_threaded()? {
            if self.shutdown {
                break;
            }
        }
        Ok(())
    }

    fn process_bus_messages(&mut self) -> Result<(), SchedulerError> {
        let mut pvpu = self.pvpu.clone();
        self.process_bus_messages_with_pvpu(&mut pvpu)?;
        self.pvpu = pvpu;
        Ok(())
    }

    fn process_bus_messages_with_pvpu(
        &mut self,
        pvpu: &mut PhotonicProcessor,
    ) -> Result<(), SchedulerError> {
        while let Some(message) = self.bus.receive() {
            self.messages_exchanged = self.messages_exchanged.saturating_add(1);
            match message {
                BusMessage::CpuCommand(cmd) => match cmd {
                    CpuCommand::SetPixel {
                        x,
                        y,
                        amplitude,
                        phase,
                    } => pvpu.set_pixel(x, y, amplitude, phase)?,
                    CpuCommand::Modulate { x, y, factor } => pvpu.modulate(x, y, factor)?,
                },
                BusMessage::PixelEnergy { x, y, energy } => {
                    if energy > 0.1 {
                        if let Ok(intensity) = pvpu.intensity_at(x, y) {
                            self.bus
                                .send(BusMessage::MotionDetected { x, y, intensity });
                        }
                    }
                }
                BusMessage::MotionDetected { .. } => {}
                BusMessage::Shutdown => {
                    self.shutdown = true;
                }
            }
        }
        Ok(())
    }

    fn refresh_stats(&mut self) {
        self.total_cycles = self.vcpu.cycles.saturating_add(self.pvpu.cycles);
        self.total_energy = self.vcpu.energy_consumed as f32 + self.pvpu.energy_used;
    }

    fn check_budget_exceeded(
        &self,
        scheduled: &ScheduledTask,
        baseline_frames: u64,
        baseline_cycles: u64,
        baseline_energy: f32,
        baseline_messages: u64,
    ) -> Option<BudgetKind> {
        let budget = scheduled.budget.as_ref()?;
        let used_frames = self.frames.saturating_sub(baseline_frames);
        let used_cycles = self.total_cycles.saturating_sub(baseline_cycles);
        let used_energy = self.total_energy - baseline_energy;
        let used_messages = self.messages_exchanged.saturating_sub(baseline_messages);

        if let Some(max) = budget.max_frames {
            if used_frames > max {
                return Some(BudgetKind::Frames);
            }
        }
        if let Some(max) = budget.max_cycles {
            if used_cycles > max {
                return Some(BudgetKind::Cycles);
            }
        }
        if let Some(max) = budget.max_energy {
            if used_energy > max {
                return Some(BudgetKind::Energy);
            }
        }
        if let Some(max) = budget.max_messages {
            if used_messages > max {
                return Some(BudgetKind::Messages);
            }
        }
        None
    }

    fn push_history(&mut self, record: TaskExecutionRecord) {
        self.history.push(record);
        self.enforce_history_limit();
    }

    fn enforce_history_limit(&mut self) {
        if let Some(limit) = self.history_limit {
            if self.history.len() > limit {
                let remove_count = self.history.len() - limit;
                self.history.drain(0..remove_count);
            }
        }
    }
}

fn take_vcpu_messages(vcpu: &mut VirtualCpu) -> Vec<BusMessage> {
    if vcpu.memory.len() < 5 {
        return Vec::new();
    }

    let opcode = vcpu.memory[0];
    if opcode == 0 {
        return Vec::new();
    }

    let msg = match opcode {
        1 => Some(BusMessage::CpuCommand(CpuCommand::SetPixel {
            x: vcpu.memory[1].max(0) as usize,
            y: vcpu.memory[2].max(0) as usize,
            amplitude: vcpu.memory[3] as f32 / 1000.0,
            phase: vcpu.memory[4] as f32 / 1000.0,
        })),
        2 => Some(BusMessage::CpuCommand(CpuCommand::Modulate {
            x: vcpu.memory[1].max(0) as usize,
            y: vcpu.memory[2].max(0) as usize,
            factor: vcpu.memory[3] as f32 / 1000.0,
        })),
        9 => Some(BusMessage::Shutdown),
        _ => None,
    };

    vcpu.memory[0] = 0;
    msg.into_iter().collect()
}

pub fn demo_vcpu_program() -> Vec<VcpuInstruction> {
    vec![
        VcpuInstruction::LoadConst { reg: 0, value: 1 },
        VcpuInstruction::Store { addr: 0, reg: 0 },
        VcpuInstruction::LoadConst { reg: 1, value: 4 },
        VcpuInstruction::Store { addr: 1, reg: 1 },
        VcpuInstruction::LoadConst { reg: 2, value: 4 },
        VcpuInstruction::Store { addr: 2, reg: 2 },
        VcpuInstruction::LoadConst { reg: 3, value: 900 },
        VcpuInstruction::Store { addr: 3, reg: 3 },
        VcpuInstruction::LoadConst { reg: 4, value: 0 },
        VcpuInstruction::Store { addr: 4, reg: 4 },
        VcpuInstruction::Jump { target: 0 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_display::VirtualMonitor;
    use matter_ir::MatterOp;

    #[test]
    fn runtime_troca_mensagens_e_acumula_metricas() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.run_frames(10).expect("runtime should run");
        let stats = runtime.stats();

        assert_eq!(stats.frames, 10);
        assert!(stats.total_cycles > 0);
        assert!(stats.total_energy > 0.0);
        assert!(stats.messages_exchanged > 0);
    }

    #[test]
    fn threaded_runtime_runs_frames() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime
            .run_threaded_frames(20)
            .expect("threaded runtime should run");

        assert_eq!(runtime.stats().frames, 20);
    }

    #[test]
    fn messages_are_exchanged() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime
            .run_threaded_frames(10)
            .expect("threaded runtime should run");

        assert!(runtime.stats().messages_exchanged > 0);
    }

    #[test]
    fn stats_increase() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime
            .run_threaded_frames(10)
            .expect("threaded runtime should run");
        let stats = runtime.stats();

        assert!(stats.total_cycles > 0);
        assert!(stats.total_energy > 0.0);
    }

    #[test]
    fn push_task_increases_queue() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let before = runtime.task_queue_len();
        runtime.push_task(HybridTask::CpuSteps { steps: 2 });
        assert_eq!(runtime.task_queue_len(), before + 1);
    }

    #[test]
    fn task_without_budget_runs() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 2 });
        runtime.run_all_tasks().expect("tasks should run");
        assert_eq!(runtime.stats().tasks_completed, 1);
    }

    #[test]
    fn task_with_cycle_budget_passes() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let budget = TaskBudget {
            max_frames: None,
            max_cycles: Some(5),
            max_energy: None,
            max_messages: None,
        };
        runtime.push_task_with_budget(HybridTask::CpuSteps { steps: 1 }, Some(budget));
        runtime.run_all_tasks().expect("task should pass budget");
        assert_eq!(runtime.stats().tasks_completed, 1);
    }

    #[test]
    fn task_with_cycle_budget_exceeded_returns_error() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let budget = TaskBudget {
            max_frames: None,
            max_cycles: Some(1),
            max_energy: None,
            max_messages: None,
        };
        let id = runtime.push_task_with_budget(HybridTask::CpuSteps { steps: 2 }, Some(budget));
        let err = runtime.run_all_tasks().expect_err("should exceed budget");
        assert_eq!(
            err.to_string(),
            SchedulerError::TaskBudgetExceeded {
                task_id: id,
                kind: BudgetKind::Cycles
            }
            .to_string()
        );
    }

    #[test]
    fn failed_task_updates_stats() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let budget = TaskBudget {
            max_frames: Some(0),
            max_cycles: Some(0),
            max_energy: Some(0.0),
            max_messages: Some(0),
        };
        runtime.push_task_with_budget(HybridTask::CpuSteps { steps: 1 }, Some(budget));
        let _ = runtime.run_all_tasks();
        let stats = runtime.stats();
        assert_eq!(stats.tasks_failed, 1);
        assert_eq!(stats.tasks_budget_exceeded, 1);
    }

    #[test]
    fn run_all_tasks_stops_on_budget_error() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let failing_budget = TaskBudget {
            max_frames: None,
            max_cycles: Some(1),
            max_energy: None,
            max_messages: None,
        };
        runtime.push_task_with_budget(HybridTask::CpuSteps { steps: 5 }, Some(failing_budget));
        runtime.push_task(HybridTask::PhotonicInterferenceDemo);

        let _ = runtime.run_all_tasks();
        let stats = runtime.stats();
        assert_eq!(stats.tasks_completed, 0);
        assert_eq!(runtime.task_queue_len(), 1);
    }

    #[test]
    fn run_next_task_executes() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 3 });
        let executed = runtime.run_next_task().expect("task should run");
        assert!(executed);
        assert_eq!(runtime.stats().tasks_completed, 1);
    }

    #[test]
    fn run_all_tasks_completes() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 5 });
        runtime.push_task(HybridTask::PhotonicInterferenceDemo);
        runtime.push_task(HybridTask::Shutdown);
        runtime.run_all_tasks().expect("all tasks should run");

        let stats = runtime.stats();
        assert_eq!(stats.tasks_completed, 3);
        assert_eq!(runtime.task_queue_len(), 0);
    }

    #[test]
    fn photonic_task_generates_energy() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let source = "\
SET_PIXEL 0 0 1.0 0.0
SET_PIXEL 1 0 1.0 0.0
INTERFERE 0 0 1 0 2 0
HALT
"
        .to_string();

        runtime.push_task(HybridTask::PhotonicRunProgram {
            source,
            width: 8,
            height: 8,
        });
        runtime.run_all_tasks().expect("all tasks should run");
        assert!(runtime.stats().total_energy > 0.0);
    }

    #[test]
    fn run_all_tasks_threaded_completes() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 10 });
        runtime.push_task(HybridTask::PhotonicInterferenceDemo);
        runtime.push_task(HybridTask::Shutdown);
        runtime
            .run_all_tasks_threaded()
            .expect("threaded tasks should run");

        let stats = runtime.stats();
        assert_eq!(stats.tasks_completed, 3);
        assert!(stats.frames >= 11);
    }

    #[test]
    fn shutdown_stops_following_tasks() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 3 });
        runtime.push_task(HybridTask::Shutdown);
        runtime.push_task(HybridTask::CpuSteps { steps: 50 });
        runtime
            .run_all_tasks_threaded()
            .expect("threaded tasks should run");

        let stats = runtime.stats();
        assert_eq!(stats.tasks_completed, 2);
        assert!(runtime.task_queue_len() >= 1);
    }

    #[test]
    fn completed_tasks_are_recorded_in_history() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.run_all_tasks().expect("tasks should run");

        assert_eq!(runtime.task_history().len(), 1);
        assert_eq!(runtime.task_history()[0].outcome, TaskOutcome::Completed);
    }

    #[test]
    fn budget_failures_are_recorded_in_history() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task_with_budget(
            HybridTask::CpuSteps { steps: 2 },
            Some(TaskBudget {
                max_frames: None,
                max_cycles: Some(1),
                max_energy: None,
                max_messages: None,
            }),
        );

        let _ = runtime.run_all_tasks();
        assert_eq!(runtime.task_history().len(), 1);
        assert!(matches!(
            runtime.task_history()[0].outcome,
            TaskOutcome::FailedBudget {
                kind: BudgetKind::Cycles
            }
        ));
    }

    #[test]
    fn task_record_returns_specific_task_status() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let id = runtime.push_task_with_budget(
            HybridTask::CpuSteps { steps: 2 },
            Some(TaskBudget {
                max_frames: None,
                max_cycles: Some(1),
                max_energy: None,
                max_messages: None,
            }),
        );
        let _ = runtime.run_all_tasks();

        let record = runtime.task_record(id).expect("record should exist");
        assert!(matches!(
            record.outcome,
            TaskOutcome::FailedBudget {
                kind: BudgetKind::Cycles
            }
        ));
    }

    #[test]
    fn last_task_record_returns_latest_entry() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.push_task(HybridTask::PhotonicInterferenceDemo);
        runtime.run_all_tasks().expect("tasks should run");

        let last = runtime
            .last_task_record()
            .expect("last record should exist");
        assert_eq!(last.task_id, 2);
        assert_eq!(last.outcome, TaskOutcome::Completed);
    }

    #[test]
    fn clear_task_history_empties_records() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.run_all_tasks().expect("tasks should run");
        assert_eq!(runtime.task_history().len(), 1);

        runtime.clear_task_history();
        assert_eq!(runtime.task_history().len(), 0);
    }

    #[test]
    fn history_limit_keeps_latest_records() {
        let mut vcpu = VirtualCpu::new(16);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);
        runtime.set_history_limit(Some(2));

        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.push_task(HybridTask::CpuSteps { steps: 1 });
        runtime.run_all_tasks().expect("tasks should run");

        assert_eq!(runtime.task_history().len(), 2);
        assert_eq!(runtime.task_history()[0].task_id, 2);
        assert_eq!(runtime.task_history()[1].task_id, 3);
    }

    #[test]
    fn ir_cpu_op_executes() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![MatterOp::Cpu(VcpuInstruction::LoadConst {
            reg: 0,
            value: 42,
        })];
        let _ = runtime.run_ir_program(&ops).expect("ir should run");
        assert_eq!(runtime.vcpu.registers[0], 42);
    }

    #[test]
    fn ir_photonic_op_executes() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![MatterOp::Photonic(PhotonicInstruction::SetPixel {
            x: 0,
            y: 0,
            amplitude: 1.0,
            phase: 0.0,
        })];
        let _ = runtime.run_ir_program(&ops).expect("ir should run");
        assert!(runtime.pvpu.total_light_energy() > 0.0);
    }

    #[test]
    fn ir_bus_op_sends_message() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![MatterOp::Bus(BusMessage::PixelEnergy {
            x: 0,
            y: 0,
            energy: 1.0,
        })];
        let _ = runtime.run_ir_program(&ops).expect("ir should run");
        assert!(runtime.stats().messages_exchanged > 0);
    }

    #[test]
    fn ir_mixed_program_updates_stats() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![
            MatterOp::BeginTask,
            MatterOp::Cpu(VcpuInstruction::LoadConst { reg: 0, value: 7 }),
            MatterOp::Photonic(PhotonicInstruction::SetPixel {
                x: 0,
                y: 0,
                amplitude: 1.0,
                phase: 0.0,
            }),
            MatterOp::Bus(BusMessage::PixelEnergy {
                x: 0,
                y: 0,
                energy: 1.0,
            }),
            MatterOp::EndTask,
        ];
        let stats = runtime.run_ir_program(&ops).expect("ir should run");
        assert!(stats.frames > 0);
        assert!(stats.total_cycles > 0);
        assert!(stats.total_energy > 0.0);
        assert!(stats.messages_exchanged > 0);
        assert!(stats.tasks_completed > 0);
    }

    #[test]
    fn ir_budget_exceeded_returns_error() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![
            MatterOp::Cpu(VcpuInstruction::LoadConst { reg: 0, value: 1 }),
            MatterOp::Cpu(VcpuInstruction::LoadConst { reg: 1, value: 1 }),
        ];
        runtime.push_task_with_budget(
            HybridTask::IrProgram { ops },
            Some(TaskBudget {
                max_frames: None,
                max_cycles: Some(1),
                max_energy: None,
                max_messages: None,
            }),
        );
        let err = runtime.run_all_tasks().expect_err("must exceed budget");
        assert!(matches!(
            err,
            SchedulerError::TaskBudgetExceeded {
                kind: BudgetKind::Cycles,
                ..
            }
        ));
    }

    #[test]
    fn ir_history_records_program() {
        let mut vcpu = VirtualCpu::new(8);
        vcpu.load_program(demo_vcpu_program())
            .expect("load should work");
        let pvpu = PhotonicProcessor::new(4, 4).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let ops = vec![MatterOp::BeginTask, MatterOp::EndTask];
        let _ = runtime.run_ir_program(&ops).expect("ir should run");
        assert!(!runtime.task_history().is_empty());
        assert!(matches!(
            runtime.task_history()[0].outcome,
            TaskOutcome::Completed
        ));
    }

    #[test]
    fn run_ir_source_executes() {
        let vcpu = VirtualCpu::new(16);
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let src = "BEGIN_TASK\nCPU_LOAD_CONST r0 1\nEND_TASK\n";
        let stats = runtime.run_ir_source(src).expect("ir source should run");
        assert_eq!(stats.tasks_completed, 1);
    }

    #[test]
    fn run_ir_file_executes() {
        let vcpu = VirtualCpu::new(16);
        let pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("demo.mir");
        let stats = runtime.run_ir_file(path).expect("ir file should run");
        assert_eq!(stats.tasks_completed, 1);
    }

    #[test]
    fn project_photonic_to_display_sets_pixel() {
        let vcpu = VirtualCpu::new(16);
        let mut pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        pvpu.set_pixel(0, 0, 1.0, 0.0)
            .expect("set_pixel should work");
        let runtime = HybridRuntime::new(vcpu, pvpu);

        let mut display = VirtualMonitor::new(8, 8).expect("display should be created");
        display.power_on();
        runtime
            .project_photonic_to_display(&mut display, 0, 0, 1, 1)
            .expect("projection should work");

        let px = display.get_pixel(1, 1).expect("pixel should exist");
        assert_eq!(px.r, 255);
        assert_eq!(px.g, 255);
        assert_eq!(px.b, 255);
    }

    #[test]
    fn display_projection_task_runs() {
        let vcpu = VirtualCpu::new(16);
        let mut pvpu = PhotonicProcessor::new(8, 8).expect("pvpu should be created");
        pvpu.set_pixel(0, 0, 1.0, 0.0)
            .expect("set_pixel should work");
        let mut runtime = HybridRuntime::new(vcpu, pvpu);

        let mut display = VirtualMonitor::new(8, 8).expect("display should be created");
        display.power_on();
        runtime.attach_display(display);

        runtime.push_task(HybridTask::ProjectPhotonicToDisplay {
            src_x: 0,
            src_y: 0,
            dst_x: 1,
            dst_y: 1,
        });
        runtime.run_all_tasks().expect("task should run");

        let display = runtime
            .display
            .as_ref()
            .expect("display should remain attached");
        let px = display.get_pixel(1, 1).expect("pixel should exist");
        assert_eq!(px.r, 255);
    }
}
