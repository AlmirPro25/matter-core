//! # Matter Leak Detector
//!
//! Automatic memory leak detection:
//! - Allocation tracking
//! - Reference cycle detection
//! - Leak reporting with suggestions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Allocation information
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub ptr: usize,
    pub size: usize,
    pub timestamp: Instant,
    pub stack_trace: Vec<String>,
    pub source_location: Option<SourceLocation>,
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Memory leak
#[derive(Debug, Clone)]
pub struct Leak {
    pub allocation: AllocationInfo,
    pub age: std::time::Duration,
    pub suggestion: String,
}

/// Reference cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceCycle {
    pub objects: Vec<usize>,
    pub suggestion: String,
}

/// Leak detector
pub struct LeakDetector {
    allocations: HashMap<usize, AllocationInfo>,
    references: HashMap<usize, Vec<usize>>,
    threshold_age: std::time::Duration,
}

impl LeakDetector {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            references: HashMap::new(),
            threshold_age: std::time::Duration::from_secs(60),
        }
    }

    pub fn with_threshold(threshold: std::time::Duration) -> Self {
        Self {
            allocations: HashMap::new(),
            references: HashMap::new(),
            threshold_age: threshold,
        }
    }

    pub fn track_allocation(&mut self, info: AllocationInfo) {
        self.allocations.insert(info.ptr, info);
    }

    pub fn track_deallocation(&mut self, ptr: usize) {
        self.allocations.remove(&ptr);
        self.references.remove(&ptr);
    }

    pub fn track_reference(&mut self, from: usize, to: usize) {
        self.references
            .entry(from)
            .or_insert_with(Vec::new)
            .push(to);
    }

    pub fn detect_leaks(&self) -> Vec<Leak> {
        let now = Instant::now();
        let mut leaks = Vec::new();

        for (_, info) in &self.allocations {
            let age = now - info.timestamp;

            if age > self.threshold_age {
                let suggestion = self.generate_leak_suggestion(info);
                leaks.push(Leak {
                    allocation: info.clone(),
                    age,
                    suggestion,
                });
            }
        }

        // Sort by age (oldest first)
        leaks.sort_by(|a, b| b.age.cmp(&a.age));
        leaks
    }

    pub fn detect_cycles(&self) -> Vec<ReferenceCycle> {
        let mut cycles = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for &ptr in self.allocations.keys() {
            if visited.contains(&ptr) {
                continue;
            }

            if let Some(cycle) = self.find_cycle(ptr, &mut visited) {
                let suggestion = self.generate_cycle_suggestion(&cycle);
                cycles.push(ReferenceCycle {
                    objects: cycle,
                    suggestion,
                });
            }
        }

        cycles
    }

    fn find_cycle(
        &self,
        start: usize,
        visited: &mut std::collections::HashSet<usize>,
    ) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        let mut current = start;
        let mut path_set = std::collections::HashSet::new();

        loop {
            if path_set.contains(&current) {
                // Found a cycle
                let cycle_start = path.iter().position(|&p| p == current).unwrap();
                return Some(path[cycle_start..].to_vec());
            }

            visited.insert(current);
            path.push(current);
            path_set.insert(current);

            // Follow first reference
            if let Some(refs) = self.references.get(&current) {
                if let Some(&next) = refs.first() {
                    current = next;
                    continue;
                }
            }

            // No more references
            break;
        }

        None
    }

    fn generate_leak_suggestion(&self, info: &AllocationInfo) -> String {
        if let Some(loc) = &info.source_location {
            format!(
                "Memory leak detected at {}:{}:{}. Consider adding explicit deallocation or using RAII pattern.",
                loc.file, loc.line, loc.column
            )
        } else {
            format!(
                "Memory leak detected (size: {} bytes). Consider adding explicit deallocation or using RAII pattern.",
                info.size
            )
        }
    }

    fn generate_cycle_suggestion(&self, cycle: &[usize]) -> String {
        format!(
            "Reference cycle detected involving {} objects. Consider using weak references to break the cycle.",
            cycle.len()
        )
    }

    pub fn report(&self) -> LeakReport {
        let leaks = self.detect_leaks();
        let cycles = self.detect_cycles();

        let total_leaked = leaks.iter().map(|l| l.allocation.size).sum();
        let total_allocations = self.allocations.len();

        LeakReport {
            total_allocations,
            leak_count: leaks.len(),
            cycle_count: cycles.len(),
            total_leaked_bytes: total_leaked,
            leaks,
            cycles,
        }
    }
}

impl Default for LeakDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Leak detection report
#[derive(Debug, Clone)]
pub struct LeakReport {
    pub total_allocations: usize,
    pub leak_count: usize,
    pub cycle_count: usize,
    pub total_leaked_bytes: usize,
    pub leaks: Vec<Leak>,
    pub cycles: Vec<ReferenceCycle>,
}

impl LeakReport {
    pub fn has_issues(&self) -> bool {
        self.leak_count > 0 || self.cycle_count > 0
    }

    pub fn print_summary(&self) {
        println!("=== Leak Detection Report ===");
        println!("Total allocations: {}", self.total_allocations);
        println!("Leaks detected: {}", self.leak_count);
        println!("Cycles detected: {}", self.cycle_count);
        println!("Total leaked: {} bytes", self.total_leaked_bytes);

        if !self.leaks.is_empty() {
            println!("\n=== Leaks ===");
            for (i, leak) in self.leaks.iter().enumerate() {
                println!(
                    "{}. {} bytes (age: {:?})",
                    i + 1,
                    leak.allocation.size,
                    leak.age
                );
                println!("   {}", leak.suggestion);
            }
        }

        if !self.cycles.is_empty() {
            println!("\n=== Reference Cycles ===");
            for (i, cycle) in self.cycles.iter().enumerate() {
                println!("{}. {} objects in cycle", i + 1, cycle.objects.len());
                println!("   {}", cycle.suggestion);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leak_detection() {
        let mut detector = LeakDetector::with_threshold(std::time::Duration::from_millis(10));

        detector.track_allocation(AllocationInfo {
            ptr: 0x1000,
            size: 1024,
            timestamp: Instant::now() - std::time::Duration::from_secs(120),
            stack_trace: vec!["main".to_string()],
            source_location: Some(SourceLocation {
                file: "test.matter".to_string(),
                line: 42,
                column: 10,
            }),
        });

        let leaks = detector.detect_leaks();
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].allocation.size, 1024);
    }

    #[test]
    fn test_cycle_detection() {
        let mut detector = LeakDetector::new();

        // Create a cycle: A -> B -> C -> A
        detector.track_allocation(AllocationInfo {
            ptr: 0x1000,
            size: 100,
            timestamp: Instant::now(),
            stack_trace: vec![],
            source_location: None,
        });

        detector.track_allocation(AllocationInfo {
            ptr: 0x2000,
            size: 100,
            timestamp: Instant::now(),
            stack_trace: vec![],
            source_location: None,
        });

        detector.track_allocation(AllocationInfo {
            ptr: 0x3000,
            size: 100,
            timestamp: Instant::now(),
            stack_trace: vec![],
            source_location: None,
        });

        detector.track_reference(0x1000, 0x2000);
        detector.track_reference(0x2000, 0x3000);
        detector.track_reference(0x3000, 0x1000);

        let cycles = detector.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].objects.len(), 3);
    }

    #[test]
    fn test_leak_report() {
        let mut detector = LeakDetector::with_threshold(std::time::Duration::from_millis(10));

        detector.track_allocation(AllocationInfo {
            ptr: 0x1000,
            size: 1024,
            timestamp: Instant::now() - std::time::Duration::from_secs(120),
            stack_trace: vec![],
            source_location: None,
        });

        let report = detector.report();
        assert!(report.has_issues());
        assert_eq!(report.leak_count, 1);
        assert_eq!(report.total_leaked_bytes, 1024);
    }
}
