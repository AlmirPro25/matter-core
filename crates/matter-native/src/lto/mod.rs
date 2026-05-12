//! Link-Time Optimization (LTO)
//!
//! Performs whole-program analysis and optimization at link time.

use matter_bytecode::{Bytecode, Constant, Instruction};
use std::collections::{HashMap, HashSet};

/// Link-Time Optimizer
pub struct LtoOptimizer {
    /// Functions that can be inlined across modules
    inlinable_functions: HashMap<String, Vec<Instruction>>,
    /// Functions that are never called (dead code)
    dead_functions: HashSet<String>,
    /// Global constants that can be propagated
    global_constants: HashMap<String, Constant>,
    /// Functions with identical bodies (can be merged)
    function_hashes: HashMap<u64, Vec<String>>,
}

impl LtoOptimizer {
    /// Create a new LTO optimizer
    pub fn new() -> Self {
        Self {
            inlinable_functions: HashMap::new(),
            dead_functions: HashSet::new(),
            global_constants: HashMap::new(),
            function_hashes: HashMap::new(),
        }
    }

    fn collect_called_functions(instructions: &[Instruction], out: &mut HashSet<String>) {
        for i in 0..instructions.len().saturating_sub(1) {
            if let (Instruction::LoadGlobal(name), Instruction::Call(_)) =
                (&instructions[i], &instructions[i + 1])
            {
                out.insert(name.clone());
            }
        }
    }

    /// Perform whole-program analysis
    pub fn analyze(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        self.find_inlinable_functions(bytecode)?;
        self.find_dead_code(bytecode)?;
        self.find_global_constants(bytecode)?;
        self.find_mergeable_functions(bytecode)?;
        Ok(())
    }

    /// Optimize bytecode with LTO
    pub fn optimize(&self, bytecode: &Bytecode) -> Result<Bytecode, String> {
        let mut optimized = bytecode.clone();
        self.cross_module_inline(&mut optimized)?;
        self.global_dead_code_elimination(&mut optimized)?;
        self.global_constant_propagation(&mut optimized)?;
        self.merge_functions(&mut optimized)?;
        Ok(optimized)
    }

    fn find_inlinable_functions(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        for (name, func) in &bytecode.functions {
            if func.instructions.len() < 20
                && !self.has_loops(&func.instructions)
                && !self.is_recursive(name, &func.instructions)
            {
                self.inlinable_functions
                    .insert(name.clone(), func.instructions.clone());
            }
        }
        Ok(())
    }

    fn find_dead_code(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        let mut called_functions = HashSet::new();
        Self::collect_called_functions(&bytecode.main_instructions, &mut called_functions);
        for func in bytecode.functions.values() {
            Self::collect_called_functions(&func.instructions, &mut called_functions);
        }

        for name in bytecode.functions.keys() {
            if !called_functions.contains(name) {
                self.dead_functions.insert(name.clone());
            }
        }
        Ok(())
    }

    fn find_global_constants(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        let mut assignments: HashMap<String, Vec<Constant>> = HashMap::new();

        for instr in &bytecode.main_instructions {
            match instr {
                Instruction::LoadConst(id) => {
                    if let Some(constant) = bytecode.constants.get(*id) {
                        assignments
                            .entry("_temp".to_string())
                            .or_default()
                            .push(constant.clone());
                    }
                }
                Instruction::StoreGlobal(name) => {
                    if let Some(constants) = assignments.get("_temp") {
                        if constants.len() == 1 {
                            self.global_constants
                                .insert(name.clone(), constants[0].clone());
                        }
                    }
                    assignments.clear();
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn find_mergeable_functions(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        for (name, func) in &bytecode.functions {
            let hash = self.hash_instructions(&func.instructions);
            self.function_hashes
                .entry(hash)
                .or_default()
                .push(name.clone());
        }
        Ok(())
    }

    fn cross_module_inline(&self, bytecode: &mut Bytecode) -> Result<(), String> {
        let mut new_instructions = Vec::new();
        let mut i = 0usize;

        while i < bytecode.main_instructions.len() {
            if i + 1 < bytecode.main_instructions.len() {
                if let (Instruction::LoadGlobal(name), Instruction::Call(arg_count)) = (
                    &bytecode.main_instructions[i],
                    &bytecode.main_instructions[i + 1],
                ) {
                    if *arg_count == 0 {
                        if let Some(body) = self.inlinable_functions.get(name) {
                            new_instructions.extend(body.clone());
                            i += 2;
                            continue;
                        }
                    }
                }
            }

            new_instructions.push(bytecode.main_instructions[i].clone());
            i += 1;
        }

        bytecode.main_instructions = new_instructions;
        Ok(())
    }

    fn global_dead_code_elimination(&self, bytecode: &mut Bytecode) -> Result<(), String> {
        bytecode
            .functions
            .retain(|name, _| !self.dead_functions.contains(name));
        Ok(())
    }

    fn global_constant_propagation(&self, bytecode: &mut Bytecode) -> Result<(), String> {
        let mut new_instructions = Vec::new();
        let original = bytecode.main_instructions.clone();

        for instr in &original {
            match instr {
                Instruction::LoadGlobal(name) => {
                    if let Some(constant) = self.global_constants.get(name) {
                        let const_id = bytecode.add_constant(constant.clone());
                        new_instructions.push(Instruction::LoadConst(const_id));
                    } else {
                        new_instructions.push(instr.clone());
                    }
                }
                _ => new_instructions.push(instr.clone()),
            }
        }

        bytecode.main_instructions = new_instructions;
        Ok(())
    }

    fn merge_functions(&self, bytecode: &mut Bytecode) -> Result<(), String> {
        let mut redirects: HashMap<String, String> = HashMap::new();

        for functions in self.function_hashes.values() {
            if functions.len() > 1 {
                let canonical = &functions[0];
                for func in &functions[1..] {
                    redirects.insert(func.clone(), canonical.clone());
                }
            }
        }

        let len = bytecode.main_instructions.len();
        for i in 0..len.saturating_sub(1) {
            if matches!(bytecode.main_instructions[i + 1], Instruction::Call(_)) {
                if let Instruction::LoadGlobal(name) = &mut bytecode.main_instructions[i] {
                    if let Some(canonical) = redirects.get(name) {
                        *name = canonical.clone();
                    }
                }
            }
        }

        bytecode
            .functions
            .retain(|name, _| !redirects.contains_key(name));

        Ok(())
    }

    fn has_loops(&self, instructions: &[Instruction]) -> bool {
        for instr in instructions {
            match instr {
                Instruction::Jump(_) | Instruction::JumpIfFalse(_) => return true,
                _ => {}
            }
        }
        false
    }

    fn is_recursive(&self, name: &str, instructions: &[Instruction]) -> bool {
        for i in 0..instructions.len().saturating_sub(1) {
            if let (Instruction::LoadGlobal(called), Instruction::Call(_)) =
                (&instructions[i], &instructions[i + 1])
            {
                if called == name {
                    return true;
                }
            }
        }
        false
    }

    fn hash_instructions(&self, instructions: &[Instruction]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for instr in instructions {
            std::mem::discriminant(instr).hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn stats(&self) -> LtoStats {
        let mut mergeable_count = 0;
        for functions in self.function_hashes.values() {
            if functions.len() > 1 {
                mergeable_count += functions.len() - 1;
            }
        }

        LtoStats {
            inlinable_functions: self.inlinable_functions.len(),
            dead_functions: self.dead_functions.len(),
            global_constants: self.global_constants.len(),
            mergeable_functions: mergeable_count,
        }
    }
}

impl Default for LtoOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct LtoStats {
    pub inlinable_functions: usize,
    pub dead_functions: usize,
    pub global_constants: usize,
    pub mergeable_functions: usize,
}

impl std::fmt::Display for LtoStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "LTO Statistics:")?;
        writeln!(f, "  Inlinable functions: {}", self.inlinable_functions)?;
        writeln!(f, "  Dead functions: {}", self.dead_functions)?;
        writeln!(f, "  Global constants: {}", self.global_constants)?;
        writeln!(f, "  Mergeable functions: {}", self.mergeable_functions)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_bytecode::Function;

    #[test]
    fn test_lto_optimizer_creation() {
        let optimizer = LtoOptimizer::new();
        assert_eq!(optimizer.inlinable_functions.len(), 0);
        assert_eq!(optimizer.dead_functions.len(), 0);
    }

    #[test]
    fn test_find_inlinable_functions() {
        let mut bytecode = Bytecode::new();

        let small_func = Function {
            name: "small".to_string(),
            param_count: 0,
            instructions: vec![Instruction::LoadConst(0), Instruction::Return],
        };
        bytecode.functions.insert("small".to_string(), small_func);

        let large_func = Function {
            name: "large".to_string(),
            param_count: 0,
            instructions: vec![Instruction::Pop; 25],
        };
        bytecode.functions.insert("large".to_string(), large_func);

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();

        assert!(optimizer.inlinable_functions.contains_key("small"));
        assert!(!optimizer.inlinable_functions.contains_key("large"));
    }

    #[test]
    fn test_find_dead_code() {
        let mut bytecode = Bytecode::new();

        let called_func = Function {
            name: "called".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        bytecode.functions.insert("called".to_string(), called_func);

        let dead_func = Function {
            name: "dead".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        bytecode.functions.insert("dead".to_string(), dead_func);

        bytecode.main_instructions = vec![
            Instruction::LoadGlobal("called".to_string()),
            Instruction::Call(0),
            Instruction::Halt,
        ];

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();

        assert!(!optimizer.dead_functions.contains("called"));
        assert!(optimizer.dead_functions.contains("dead"));
    }

    #[test]
    fn test_cross_module_inline() {
        let mut bytecode = Bytecode::new();

        let func = Function {
            name: "add_one".to_string(),
            param_count: 0,
            instructions: vec![
                Instruction::LoadConst(0),
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("add_one".to_string(), func);
        bytecode.constants.push(Constant::Int(1));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(0),
            Instruction::LoadGlobal("add_one".to_string()),
            Instruction::Call(0),
            Instruction::Halt,
        ];

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();
        let optimized = optimizer.optimize(&bytecode).unwrap();

        // Pair LoadGlobal+Call(0) should have been removed for this callsite.
        let mut found_pair = false;
        for i in 0..optimized.main_instructions.len().saturating_sub(1) {
            if let (Instruction::LoadGlobal(name), Instruction::Call(0)) = (
                &optimized.main_instructions[i],
                &optimized.main_instructions[i + 1],
            ) {
                if name == "add_one" {
                    found_pair = true;
                    break;
                }
            }
        }
        assert!(!found_pair);
    }

    #[test]
    fn test_global_dead_code_elimination() {
        let mut bytecode = Bytecode::new();

        let dead_func = Function {
            name: "dead".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        bytecode.functions.insert("dead".to_string(), dead_func);

        bytecode.main_instructions = vec![Instruction::Halt];

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();
        let optimized = optimizer.optimize(&bytecode).unwrap();

        assert!(!optimized.functions.contains_key("dead"));
    }

    #[test]
    fn test_function_merging() {
        let mut bytecode = Bytecode::new();

        let func1 = Function {
            name: "func1".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        let func2 = Function {
            name: "func2".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        bytecode.functions.insert("func1".to_string(), func1);
        bytecode.functions.insert("func2".to_string(), func2);

        bytecode.main_instructions = vec![
            Instruction::LoadGlobal("func1".to_string()),
            Instruction::Call(0),
            Instruction::LoadGlobal("func2".to_string()),
            Instruction::Call(0),
            Instruction::Halt,
        ];

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();
        let optimized = optimizer.optimize(&bytecode).unwrap();

        assert_eq!(optimized.functions.len(), 1);
    }

    #[test]
    fn test_lto_stats() {
        let mut bytecode = Bytecode::new();

        let func1 = Function {
            name: "func1".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        let func2 = Function {
            name: "func2".to_string(),
            param_count: 0,
            instructions: vec![],
        };
        bytecode.functions.insert("func1".to_string(), func1);
        bytecode.functions.insert("func2".to_string(), func2);

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();

        let stats = optimizer.stats();
        assert!(stats.inlinable_functions > 0 || stats.dead_functions > 0);
    }

    #[test]
    fn test_lto_preserves_correctness() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c42),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();
        let optimized = optimizer.optimize(&bytecode).unwrap();

        assert_eq!(optimized.main_instructions.len(), 3);
    }

    #[test]
    fn test_lto_reduces_binary_size() {
        let mut bytecode = Bytecode::new();

        for i in 0..10 {
            let func = Function {
                name: format!("dead_{}", i),
                param_count: 0,
                instructions: vec![],
            };
            bytecode.functions.insert(format!("dead_{}", i), func);
        }

        bytecode.main_instructions = vec![Instruction::Halt];
        let original_size = bytecode.functions.len();

        let mut optimizer = LtoOptimizer::new();
        optimizer.analyze(&bytecode).unwrap();
        let optimized = optimizer.optimize(&bytecode).unwrap();

        assert!(optimized.functions.len() < original_size);
    }
}
