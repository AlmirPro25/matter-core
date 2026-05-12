use matter_backend::{Backend, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyMode {
    Eco,
    Balanced,
    Performance,
    Adaptive,
    Critical,
}

#[derive(Debug, Clone)]
pub struct EnergySnapshot {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub io_usage: f64,
    pub network_usage: f64,
    pub battery_level: f64,
    pub temperature: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct EnergyCost {
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub io_cost: f64,
    pub network_cost: f64,
    pub backend_cost: f64,
    pub estimated_total: f64,
}

#[derive(Debug, Clone)]
pub struct EnergyScore {
    pub cost: f64,
    pub value: f64,
    pub efficiency: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub struct EnergyPolicy {
    pub mode: EnergyMode,
    pub max_cpu: f64,
    pub max_memory: f64,
    pub battery_aware: bool,
    pub prefer_cache: bool,
    pub allow_defer: bool,
}

impl Default for EnergyPolicy {
    fn default() -> Self {
        Self {
            mode: EnergyMode::Balanced,
            max_cpu: 85.0,
            max_memory: 85.0,
            battery_aware: true,
            prefer_cache: true,
            allow_defer: true,
        }
    }
}

pub trait EnergyMonitor: Send {
    fn snapshot(&self) -> EnergySnapshot;
}

pub struct DefaultEnergyMonitor;

impl Default for DefaultEnergyMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultEnergyMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl EnergyMonitor for DefaultEnergyMonitor {
    fn snapshot(&self) -> EnergySnapshot {
        EnergySnapshot {
            cpu_usage: 35.0,
            memory_usage: 42.0,
            io_usage: 15.0,
            network_usage: 18.0,
            battery_level: 76.0,
            temperature: 56.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

pub struct EnergyEstimator;

impl EnergyEstimator {
    pub fn estimate_from_snapshot(snapshot: &EnergySnapshot, backend_cost: f64) -> EnergyCost {
        let cpu_cost = snapshot.cpu_usage * 0.5;
        let memory_cost = snapshot.memory_usage * 0.25;
        let io_cost = snapshot.io_usage * 0.15;
        let network_cost = snapshot.network_usage * 0.2;
        let estimated_total = cpu_cost + memory_cost + io_cost + network_cost + backend_cost;

        EnergyCost {
            cpu_cost,
            memory_cost,
            io_cost,
            network_cost,
            backend_cost,
            estimated_total,
        }
    }

    pub fn estimate_backend_call(backend: &str) -> f64 {
        match backend {
            "agent" | "visual" | "net" | "tool" => 12.0,
            "graph" | "store" => 8.0,
            "math" | "string" | "list" | "time" | "random" | "json" => 2.0,
            "energy" => 1.0,
            _ => 4.0,
        }
    }

    pub fn estimate_instruction_cost(op: &str) -> f64 {
        match op {
            "backend_call" => 10.0,
            "call" => 4.0,
            "load_store" => 1.0,
            "arith" => 1.5,
            "control" => 2.0,
            _ => 1.0,
        }
    }
}

pub struct EnergyDecision;

impl EnergyDecision {
    pub fn score(cost: &EnergyCost, value: f64) -> EnergyScore {
        let efficiency = if cost.estimated_total <= 0.0 {
            value
        } else {
            value / cost.estimated_total
        };

        let recommendation = if efficiency > 2.0 {
            "keep".to_string()
        } else if efficiency > 1.0 {
            "optimize".to_string()
        } else {
            "defer_or_cache".to_string()
        };

        EnergyScore {
            cost: cost.estimated_total,
            value,
            efficiency,
            recommendation,
        }
    }

    pub fn should_defer(policy: &EnergyPolicy, snapshot: &EnergySnapshot) -> bool {
        policy.allow_defer
            && ((policy.battery_aware && snapshot.battery_level < 20.0)
                || snapshot.cpu_usage > policy.max_cpu
                || snapshot.memory_usage > policy.max_memory)
    }
}

pub struct EnergyRuntime {
    pub policy: EnergyPolicy,
    monitor: Box<dyn EnergyMonitor>,
    cache: HashMap<String, Value>,
}

impl Default for EnergyRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyRuntime {
    pub fn new() -> Self {
        Self {
            policy: EnergyPolicy::default(),
            monitor: Box::new(DefaultEnergyMonitor::new()),
            cache: HashMap::new(),
        }
    }

    pub fn with_monitor(monitor: Box<dyn EnergyMonitor>) -> Self {
        Self {
            policy: EnergyPolicy::default(),
            monitor,
            cache: HashMap::new(),
        }
    }

    pub fn snapshot(&self) -> EnergySnapshot {
        self.monitor.snapshot()
    }

    pub fn set_mode(&mut self, mode: EnergyMode) {
        self.policy.mode = mode;
    }

    pub fn mode_name(&self) -> &'static str {
        match self.policy.mode {
            EnergyMode::Eco => "eco",
            EnergyMode::Balanced => "balanced",
            EnergyMode::Performance => "performance",
            EnergyMode::Adaptive => "adaptive",
            EnergyMode::Critical => "critical",
        }
    }

    pub fn score_named(&self, name: &str) -> EnergyScore {
        let snapshot = self.snapshot();
        let backend_cost = EnergyEstimator::estimate_backend_call(name);
        let cost = EnergyEstimator::estimate_from_snapshot(&snapshot, backend_cost);
        EnergyDecision::score(&cost, 100.0)
    }

    pub fn estimate_named(&self, name: &str) -> EnergyCost {
        let snapshot = self.snapshot();
        let backend_cost = EnergyEstimator::estimate_backend_call(name);
        EnergyEstimator::estimate_from_snapshot(&snapshot, backend_cost)
    }

    pub fn defer_named(&self, _name: &str) -> bool {
        let snapshot = self.snapshot();
        EnergyDecision::should_defer(&self.policy, &snapshot)
    }

    pub fn task_priority(&self, base_priority: i64) -> i64 {
        match self.policy.mode {
            EnergyMode::Eco => (base_priority - 2).max(1),
            EnergyMode::Balanced => base_priority,
            EnergyMode::Performance => base_priority + 1,
            EnergyMode::Adaptive => base_priority,
            EnergyMode::Critical => (base_priority - 3).max(1),
        }
    }

    pub fn configure(&mut self, key: &str, value: Value) -> Result<Value, String> {
        match key {
            "battery_aware" => {
                self.policy.battery_aware = value.as_bool()?;
                Ok(Value::Unit)
            }
            "prefer_cache" => {
                self.policy.prefer_cache = value.as_bool()?;
                Ok(Value::Unit)
            }
            "allow_defer" => {
                self.policy.allow_defer = value.as_bool()?;
                Ok(Value::Unit)
            }
            _ => Err(format!("energy.configure unknown key '{}'", key)),
        }
    }
}

pub struct EnergyBackend {
    runtime: EnergyRuntime,
}

impl Default for EnergyBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyBackend {
    pub fn new() -> Self {
        Self {
            runtime: EnergyRuntime::new(),
        }
    }

    fn mode_from_arg(arg: &str) -> Result<EnergyMode, String> {
        match arg.to_ascii_lowercase().as_str() {
            "eco" => Ok(EnergyMode::Eco),
            "balanced" => Ok(EnergyMode::Balanced),
            "performance" => Ok(EnergyMode::Performance),
            "adaptive" => Ok(EnergyMode::Adaptive),
            "critical" => Ok(EnergyMode::Critical),
            _ => Err(format!("unknown energy mode '{}'", arg)),
        }
    }
}

impl Backend for EnergyBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "cpu" => Ok(Value::Int(self.runtime.snapshot().cpu_usage as i64)),
            "memory" => Ok(Value::Int(self.runtime.snapshot().memory_usage as i64)),
            "mode" => Ok(Value::new_string(self.runtime.mode_name().to_string())),
            "set_mode" => {
                if args.len() != 1 {
                    return Err("energy.set_mode expects 1 arg".to_string());
                }
                let mode = Self::mode_from_arg(&args[0].as_string()?)?;
                self.runtime.set_mode(mode);
                Ok(Value::Unit)
            }
            "configure" => {
                if args.len() != 2 {
                    return Err("energy.configure expects 2 args".to_string());
                }
                let key = args[0].as_string()?;
                self.runtime.configure(&key, args[1].clone())
            }
            "profile" => {
                if args.len() != 1 {
                    return Err("energy.profile expects 1 arg".to_string());
                }
                let Value::Map(entries) = &args[0] else {
                    return Err("energy.profile expects a map".to_string());
                };

                if let Some(mode_value) = entries.get("mode") {
                    let mode = Self::mode_from_arg(&mode_value.as_string()?)?;
                    self.runtime.set_mode(mode);
                }
                if let Some(battery_aware) = entries.get("battery_aware") {
                    self.runtime
                        .configure("battery_aware", battery_aware.clone())?;
                }
                if let Some(prefer_cache) = entries.get("prefer_cache") {
                    self.runtime
                        .configure("prefer_cache", prefer_cache.clone())?;
                }
                if let Some(allow_defer) = entries.get("allow_defer") {
                    self.runtime.configure("allow_defer", allow_defer.clone())?;
                }
                Ok(Value::Unit)
            }
            "score" => {
                if args.len() != 1 {
                    return Err("energy.score expects 1 arg".to_string());
                }
                let name = args[0].as_string()?;
                let score = self.runtime.score_named(&name);
                let mut map = HashMap::new();
                map.insert("cost".to_string(), Value::Int(score.cost as i64));
                map.insert("value".to_string(), Value::Int(score.value as i64));
                map.insert(
                    "efficiency".to_string(),
                    Value::Int(score.efficiency as i64),
                );
                map.insert(
                    "recommendation".to_string(),
                    Value::new_string(score.recommendation),
                );
                Ok(Value::new_map(map))
            }
            "estimate" => {
                if args.len() != 1 {
                    return Err("energy.estimate expects 1 arg".to_string());
                }
                let name = args[0].as_string()?;
                let cost = self.runtime.estimate_named(&name);
                let mut map = HashMap::new();
                map.insert("cpu_cost".to_string(), Value::Int(cost.cpu_cost as i64));
                map.insert(
                    "memory_cost".to_string(),
                    Value::Int(cost.memory_cost as i64),
                );
                map.insert("io_cost".to_string(), Value::Int(cost.io_cost as i64));
                map.insert(
                    "network_cost".to_string(),
                    Value::Int(cost.network_cost as i64),
                );
                map.insert(
                    "backend_cost".to_string(),
                    Value::Int(cost.backend_cost as i64),
                );
                map.insert(
                    "estimated_total".to_string(),
                    Value::Int(cost.estimated_total as i64),
                );
                Ok(Value::new_map(map))
            }
            "defer" => {
                if args.len() != 1 {
                    return Err("energy.defer expects 1 arg".to_string());
                }
                let name = args[0].as_string()?;
                Ok(Value::Bool(self.runtime.defer_named(&name)))
            }
            "cache" => {
                if args.len() != 2 {
                    return Err("energy.cache expects 2 args".to_string());
                }
                let key = args[0].as_string()?;
                self.runtime.cache.insert(key, args[1].clone());
                Ok(Value::Unit)
            }
            "reuse" => {
                if args.len() != 1 {
                    return Err("energy.reuse expects 1 arg".to_string());
                }
                let key = args[0].as_string()?;
                Ok(self.runtime.cache.get(&key).cloned().unwrap_or(Value::Unit))
            }
            _ => Err(format!(
                "Backend call failed [context:backend=energy,method={}]: unknown method",
                method
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_score_computes_efficiency() {
        let cost = EnergyCost {
            cpu_cost: 10.0,
            memory_cost: 5.0,
            io_cost: 2.0,
            network_cost: 3.0,
            backend_cost: 8.0,
            estimated_total: 28.0,
        };
        let score = EnergyDecision::score(&cost, 56.0);
        assert!(score.efficiency >= 2.0);
    }

    #[test]
    fn energy_policy_defers_on_low_battery() {
        let policy = EnergyPolicy::default();
        let snapshot = EnergySnapshot {
            cpu_usage: 20.0,
            memory_usage: 30.0,
            io_usage: 10.0,
            network_usage: 10.0,
            battery_level: 10.0,
            temperature: 45.0,
            timestamp: 0,
        };
        assert!(EnergyDecision::should_defer(&policy, &snapshot));
    }

    #[test]
    fn energy_backend_cpu_returns_int() {
        let mut backend = EnergyBackend::new();
        let value = backend.call("cpu", vec![]).unwrap();
        assert!(matches!(value, Value::Int(_)));
    }

    #[test]
    fn eco_mode_reduces_task_priority() {
        let mut runtime = EnergyRuntime::new();
        runtime.set_mode(EnergyMode::Eco);
        assert!(runtime.task_priority(5) < 5);
    }

    #[test]
    fn energy_profile_applies_mode_and_flags() {
        let mut backend = EnergyBackend::new();
        let mut profile = HashMap::new();
        profile.insert(
            "mode".to_string(),
            Value::new_string("adaptive".to_string()),
        );
        profile.insert("battery_aware".to_string(), Value::Bool(true));
        profile.insert("prefer_cache".to_string(), Value::Bool(true));
        backend
            .call("profile", vec![Value::new_map(profile)])
            .unwrap();

        let mode = backend.call("mode", vec![]).unwrap().as_string().unwrap();
        assert_eq!(mode, "adaptive");
    }

    #[test]
    fn expensive_backends_cost_more_than_local_backends() {
        let agent_cost = EnergyEstimator::estimate_backend_call("agent");
        let visual_cost = EnergyEstimator::estimate_backend_call("visual");
        let net_cost = EnergyEstimator::estimate_backend_call("net");
        let tool_cost = EnergyEstimator::estimate_backend_call("tool");
        let math_cost = EnergyEstimator::estimate_backend_call("math");

        assert!(agent_cost > math_cost);
        assert!(visual_cost > math_cost);
        assert!(net_cost > math_cost);
        assert!(tool_cost > math_cost);
    }
}
