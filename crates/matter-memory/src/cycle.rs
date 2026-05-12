//! Cycle detector for reference counting memory management.
//!
//! This module implements a mark-and-sweep cycle detector that works
//! alongside reference counting to detect and collect circular references.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Unique identifier for tracked objects
type ObjectId = usize;

/// Global object ID counter
static NEXT_OBJECT_ID: AtomicUsize = AtomicUsize::new(1);

/// Generate a unique object ID
pub fn next_object_id() -> ObjectId {
    NEXT_OBJECT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Trait for objects that can be tracked for cycles
pub trait Traceable {
    /// Get the unique ID of this object
    fn object_id(&self) -> ObjectId;

    /// Get the IDs of objects this object references
    fn references(&self) -> Vec<ObjectId>;

    /// Check if this object is still alive (has strong references)
    fn is_alive(&self) -> bool;
}

/// Graph node representing a tracked object
#[derive(Debug, Clone)]
struct GraphNode {
    id: ObjectId,
    references: Vec<ObjectId>,
    is_alive: bool,
}

/// Cycle detector using mark-and-sweep algorithm
pub struct CycleDetector {
    /// Graph of tracked objects
    graph: Arc<Mutex<HashMap<ObjectId, GraphNode>>>,

    /// Collection threshold (number of allocations before collection)
    threshold: usize,

    /// Number of allocations since last collection
    allocations_since_collection: AtomicUsize,

    /// Statistics
    collections_run: AtomicUsize,
    cycles_detected: AtomicUsize,
    objects_collected: AtomicUsize,
}

impl CycleDetector {
    /// Create a new cycle detector with default threshold (1000)
    pub fn new() -> Self {
        Self::with_threshold(1000)
    }

    /// Create a new cycle detector with custom threshold
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            graph: Arc::new(Mutex::new(HashMap::new())),
            threshold,
            allocations_since_collection: AtomicUsize::new(0),
            collections_run: AtomicUsize::new(0),
            cycles_detected: AtomicUsize::new(0),
            objects_collected: AtomicUsize::new(0),
        }
    }

    /// Track a new object
    pub fn track<T: Traceable>(&self, object: &T) {
        let node = GraphNode {
            id: object.object_id(),
            references: object.references(),
            is_alive: object.is_alive(),
        };

        if let Ok(mut graph) = self.graph.lock() {
            graph.insert(node.id, node);
        }

        // Increment allocation counter
        let count = self
            .allocations_since_collection
            .fetch_add(1, Ordering::Relaxed);

        // Check if we should run collection
        if count >= self.threshold {
            self.collect();
        }
    }

    /// Update an existing tracked object
    pub fn update<T: Traceable>(&self, object: &T) {
        if let Ok(mut graph) = self.graph.lock() {
            if let Some(node) = graph.get_mut(&object.object_id()) {
                node.references = object.references();
                node.is_alive = object.is_alive();
            }
        }
    }

    /// Remove a tracked object
    pub fn untrack(&self, id: ObjectId) {
        if let Ok(mut graph) = self.graph.lock() {
            graph.remove(&id);
        }
    }

    /// Run cycle detection and collection
    pub fn collect(&self) -> CycleDetectionResult {
        self.allocations_since_collection
            .store(0, Ordering::Relaxed);
        self.collections_run.fetch_add(1, Ordering::Relaxed);

        let graph = match self.graph.lock() {
            Ok(g) => g,
            Err(_) => return CycleDetectionResult::default(),
        };

        // Phase 1: Mark - Find all reachable objects from alive roots
        let mut reachable = HashSet::new();
        let mut to_visit = Vec::new();

        // Start with all alive objects as roots
        for (id, node) in graph.iter() {
            if node.is_alive {
                to_visit.push(*id);
            }
        }

        // Mark phase: DFS from roots
        while let Some(id) = to_visit.pop() {
            if reachable.insert(id) {
                if let Some(node) = graph.get(&id) {
                    for &ref_id in &node.references {
                        if !reachable.contains(&ref_id) {
                            to_visit.push(ref_id);
                        }
                    }
                }
            }
        }

        // Phase 2: Sweep - Find unreachable objects (cycles)
        let mut cycles = Vec::new();
        let mut unreachable_objects = Vec::new();

        for (id, node) in graph.iter() {
            if !reachable.contains(id) && !node.is_alive {
                unreachable_objects.push(*id);

                // Check if this object is part of a cycle
                if self.is_in_cycle(*id, &graph) {
                    cycles.push(*id);
                }
            }
        }

        // Update statistics
        if !cycles.is_empty() {
            self.cycles_detected.fetch_add(1, Ordering::Relaxed);
        }
        self.objects_collected
            .fetch_add(unreachable_objects.len(), Ordering::Relaxed);

        CycleDetectionResult {
            cycles_found: cycles.len(),
            objects_collected: unreachable_objects.len(),
            cycle_objects: cycles,
        }
    }

    /// Check if an object is part of a cycle
    fn is_in_cycle(&self, start_id: ObjectId, graph: &HashMap<ObjectId, GraphNode>) -> bool {
        let mut visited = HashSet::new();
        let mut path = HashSet::new();

        self.has_cycle_dfs(start_id, graph, &mut visited, &mut path)
    }

    /// DFS helper to detect cycles
    fn has_cycle_dfs(
        &self,
        id: ObjectId,
        graph: &HashMap<ObjectId, GraphNode>,
        visited: &mut HashSet<ObjectId>,
        path: &mut HashSet<ObjectId>,
    ) -> bool {
        if path.contains(&id) {
            return true; // Cycle detected
        }

        if visited.contains(&id) {
            return false; // Already processed
        }

        visited.insert(id);
        path.insert(id);

        if let Some(node) = graph.get(&id) {
            for &ref_id in &node.references {
                if self.has_cycle_dfs(ref_id, graph, visited, path) {
                    return true;
                }
            }
        }

        path.remove(&id);
        false
    }

    /// Force immediate collection
    pub fn force_collect(&self) -> CycleDetectionResult {
        self.collect()
    }

    /// Get statistics
    pub fn stats(&self) -> CycleDetectorStats {
        let graph = match self.graph.lock() {
            Ok(graph) => graph,
            Err(poisoned) => poisoned.into_inner(),
        };

        CycleDetectorStats {
            tracked_objects: graph.len(),
            collections_run: self.collections_run.load(Ordering::Relaxed),
            cycles_detected: self.cycles_detected.load(Ordering::Relaxed),
            objects_collected: self.objects_collected.load(Ordering::Relaxed),
            threshold: self.threshold,
        }
    }

    /// Set collection threshold
    pub fn set_threshold(&mut self, threshold: usize) {
        self.threshold = threshold;
    }

    /// Clear all tracked objects
    pub fn clear(&self) {
        if let Ok(mut graph) = self.graph.lock() {
            graph.clear();
        }
        self.allocations_since_collection
            .store(0, Ordering::Relaxed);
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of cycle detection
#[derive(Debug, Clone, Default)]
pub struct CycleDetectionResult {
    /// Number of cycles found
    pub cycles_found: usize,

    /// Number of objects collected
    pub objects_collected: usize,

    /// IDs of objects in cycles
    pub cycle_objects: Vec<ObjectId>,
}

/// Statistics for cycle detector
#[derive(Debug, Clone)]
pub struct CycleDetectorStats {
    /// Number of tracked objects
    pub tracked_objects: usize,

    /// Number of collections run
    pub collections_run: usize,

    /// Number of cycles detected
    pub cycles_detected: usize,

    /// Number of objects collected
    pub objects_collected: usize,

    /// Collection threshold
    pub threshold: usize,
}

impl std::fmt::Display for CycleDetectorStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cycle Detector Statistics:")?;
        writeln!(f, "  Tracked objects:    {}", self.tracked_objects)?;
        writeln!(f, "  Collections run:    {}", self.collections_run)?;
        writeln!(f, "  Cycles detected:    {}", self.cycles_detected)?;
        writeln!(f, "  Objects collected:  {}", self.objects_collected)?;
        writeln!(f, "  Threshold:          {}", self.threshold)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock traceable object for testing
    struct MockObject {
        id: ObjectId,
        refs: Vec<ObjectId>,
        alive: bool,
    }

    impl Traceable for MockObject {
        fn object_id(&self) -> ObjectId {
            self.id
        }

        fn references(&self) -> Vec<ObjectId> {
            self.refs.clone()
        }

        fn is_alive(&self) -> bool {
            self.alive
        }
    }

    #[test]
    fn test_cycle_detector_creation() {
        let detector = CycleDetector::new();
        let stats = detector.stats();
        assert_eq!(stats.tracked_objects, 0);
        assert_eq!(stats.collections_run, 0);
    }

    #[test]
    fn test_track_object() {
        let detector = CycleDetector::new();
        let obj = MockObject {
            id: 1,
            refs: vec![],
            alive: true,
        };

        detector.track(&obj);
        let stats = detector.stats();
        assert_eq!(stats.tracked_objects, 1);
    }

    #[test]
    fn test_untrack_object() {
        let detector = CycleDetector::new();
        let obj = MockObject {
            id: 1,
            refs: vec![],
            alive: true,
        };

        detector.track(&obj);
        detector.untrack(1);
        let stats = detector.stats();
        assert_eq!(stats.tracked_objects, 0);
    }

    #[test]
    fn test_simple_cycle_detection() {
        let detector = CycleDetector::with_threshold(10);

        // Create cycle: 1 -> 2 -> 1
        let obj1 = MockObject {
            id: 1,
            refs: vec![2],
            alive: false,
        };
        let obj2 = MockObject {
            id: 2,
            refs: vec![1],
            alive: false,
        };

        detector.track(&obj1);
        detector.track(&obj2);

        let result = detector.force_collect();
        assert!(result.cycles_found > 0);
        assert!(result.objects_collected > 0);
    }

    #[test]
    fn test_no_cycle_with_alive_root() {
        let detector = CycleDetector::with_threshold(10);

        // Create cycle but with alive root: 1 -> 2 -> 1
        let obj1 = MockObject {
            id: 1,
            refs: vec![2],
            alive: true, // Alive root
        };
        let obj2 = MockObject {
            id: 2,
            refs: vec![1],
            alive: false,
        };

        detector.track(&obj1);
        detector.track(&obj2);

        let result = detector.force_collect();
        assert_eq!(result.objects_collected, 0); // Nothing collected (reachable from alive root)
    }

    #[test]
    fn test_complex_cycle() {
        let detector = CycleDetector::with_threshold(10);

        // Create complex cycle: 1 -> 2 -> 3 -> 1
        let obj1 = MockObject {
            id: 1,
            refs: vec![2],
            alive: false,
        };
        let obj2 = MockObject {
            id: 2,
            refs: vec![3],
            alive: false,
        };
        let obj3 = MockObject {
            id: 3,
            refs: vec![1],
            alive: false,
        };

        detector.track(&obj1);
        detector.track(&obj2);
        detector.track(&obj3);

        let result = detector.force_collect();
        assert!(result.cycles_found > 0);
        assert_eq!(result.objects_collected, 3);
    }

    #[test]
    fn test_threshold_triggers_collection() {
        let detector = CycleDetector::with_threshold(2); // Lower threshold

        // Track 3 objects to trigger collection (threshold is 2, so 3rd triggers)
        for i in 1..=3 {
            let obj = MockObject {
                id: i,
                refs: vec![],
                alive: false,
            };
            detector.track(&obj);
        }

        let stats = detector.stats();
        // After tracking 3 objects with threshold 2, collection should have run
        assert!(
            stats.collections_run >= 1,
            "Expected at least 1 collection, got {}",
            stats.collections_run
        );
    }

    #[test]
    fn test_clear() {
        let detector = CycleDetector::new();

        for i in 1..=5 {
            let obj = MockObject {
                id: i,
                refs: vec![],
                alive: true,
            };
            detector.track(&obj);
        }

        detector.clear();
        let stats = detector.stats();
        assert_eq!(stats.tracked_objects, 0);
    }

    #[test]
    fn test_stats_display() {
        let detector = CycleDetector::new();
        let stats = detector.stats();
        let display = format!("{}", stats);
        assert!(display.contains("Cycle Detector Statistics"));
        assert!(display.contains("Tracked objects"));
    }

    #[test]
    fn test_update_object() {
        let detector = CycleDetector::new();

        let obj = MockObject {
            id: 1,
            refs: vec![],
            alive: true,
        };
        detector.track(&obj);

        let updated_obj = MockObject {
            id: 1,
            refs: vec![2, 3],
            alive: true,
        };
        detector.update(&updated_obj);

        // Object should still be tracked
        let stats = detector.stats();
        assert_eq!(stats.tracked_objects, 1);
    }
}
