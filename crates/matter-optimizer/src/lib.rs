//! Matter Bytecode Optimizer
//! Otimizações de bytecode para melhorar performance

use matter_bytecode::{Bytecode, Constant, Instruction};
use std::collections::HashMap;

/// Trait para optimization passes
pub trait OptimizationPass {
    fn name(&self) -> &str;
    fn optimize(&self, bytecode: &mut Bytecode) -> bool;
}

/// Otimizador de bytecode com múltiplos passes
pub struct BytecodeOptimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
    _level: OptimizationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,    // -O0
    Basic,   // -O1
    Default, // -O2
    Max,     // -O3
}

impl BytecodeOptimizer {
    pub fn new(level: OptimizationLevel) -> Self {
        let mut passes: Vec<Box<dyn OptimizationPass>> = Vec::new();

        match level {
            OptimizationLevel::None => {}
            OptimizationLevel::Basic => {
                passes.push(Box::new(ConstantFoldingPass));
                passes.push(Box::new(DeadCodeEliminationPass));
            }
            OptimizationLevel::Default => {
                passes.push(Box::new(ConstantFoldingPass));
                passes.push(Box::new(DeadCodeEliminationPass));
                passes.push(Box::new(PeepholeOptimizationPass));
            }
            OptimizationLevel::Max => {
                passes.push(Box::new(ConstantFoldingPass));
                passes.push(Box::new(DeadCodeEliminationPass));
                passes.push(Box::new(PeepholeOptimizationPass));
                passes.push(Box::new(JumpOptimizationPass));
            }
        }

        Self {
            passes,
            _level: level,
        }
    }

    pub fn optimize(&self, bytecode: &mut Bytecode) -> OptimizationStats {
        let mut stats = OptimizationStats::default();
        let original_size = bytecode.main_instructions.len();

        for pass in &self.passes {
            let changed = pass.optimize(bytecode);
            if changed {
                stats.passes_applied += 1;
            }
        }

        stats.original_size = original_size;
        stats.optimized_size = bytecode.main_instructions.len();
        stats.reduction_percent = if original_size > 0 {
            ((original_size - stats.optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        stats
    }
}

#[derive(Debug, Default)]
pub struct OptimizationStats {
    pub passes_applied: usize,
    pub original_size: usize,
    pub optimized_size: usize,
    pub reduction_percent: f64,
}

/// Pass 1: Constant Folding
/// Avalia expressões constantes em compile-time
pub struct ConstantFoldingPass;

impl OptimizationPass for ConstantFoldingPass {
    fn name(&self) -> &str {
        "constant-folding"
    }

    fn optimize(&self, bytecode: &mut Bytecode) -> bool {
        let mut changed = false;
        let mut new_instructions = Vec::new();
        let mut stack: Vec<Constant> = Vec::new();

        for inst in &bytecode.main_instructions {
            match inst {
                Instruction::LoadConst(id) => {
                    stack.push(bytecode.constants[*id].clone());
                    new_instructions.push(inst.clone());
                }
                Instruction::Add => {
                    if stack.len() >= 2 {
                        let b = stack.pop();
                        let a = stack.pop();
                        if let (Some(Constant::Int(bval)), Some(Constant::Int(aval))) = (b, a) {
                            let result = aval + bval;

                            // Remove last 2 LoadConst + Add
                            new_instructions.pop();
                            new_instructions.pop();

                            // Add result as new constant
                            let result_const = Constant::Int(result);
                            let const_id = if let Some(pos) =
                                bytecode.constants.iter().position(|c| c == &result_const)
                            {
                                pos
                            } else {
                                let id = bytecode.constants.len();
                                bytecode.constants.push(result_const.clone());
                                id
                            };

                            new_instructions.push(Instruction::LoadConst(const_id));
                            stack.push(result_const);
                            changed = true;
                            continue;
                        }
                    }
                    stack.clear();
                    new_instructions.push(inst.clone());
                }
                Instruction::Sub => {
                    if stack.len() >= 2 {
                        let b = stack.pop();
                        let a = stack.pop();
                        if let (Some(Constant::Int(bval)), Some(Constant::Int(aval))) = (b, a) {
                            let result = aval - bval;

                            new_instructions.pop();
                            new_instructions.pop();

                            let result_const = Constant::Int(result);
                            let const_id = if let Some(pos) =
                                bytecode.constants.iter().position(|c| c == &result_const)
                            {
                                pos
                            } else {
                                let id = bytecode.constants.len();
                                bytecode.constants.push(result_const.clone());
                                id
                            };

                            new_instructions.push(Instruction::LoadConst(const_id));
                            stack.push(result_const);
                            changed = true;
                            continue;
                        }
                    }
                    stack.clear();
                    new_instructions.push(inst.clone());
                }
                Instruction::Mul => {
                    if stack.len() >= 2 {
                        let b = stack.pop();
                        let a = stack.pop();
                        if let (Some(Constant::Int(bval)), Some(Constant::Int(aval))) = (b, a) {
                            let result = aval * bval;

                            new_instructions.pop();
                            new_instructions.pop();

                            let result_const = Constant::Int(result);
                            let const_id = if let Some(pos) =
                                bytecode.constants.iter().position(|c| c == &result_const)
                            {
                                pos
                            } else {
                                let id = bytecode.constants.len();
                                bytecode.constants.push(result_const.clone());
                                id
                            };

                            new_instructions.push(Instruction::LoadConst(const_id));
                            stack.push(result_const);
                            changed = true;
                            continue;
                        }
                    }
                    stack.clear();
                    new_instructions.push(inst.clone());
                }
                Instruction::Div => {
                    if stack.len() >= 2 {
                        let b = stack.pop();
                        let a = stack.pop();
                        if let (Some(Constant::Int(bval)), Some(Constant::Int(aval))) = (b, a) {
                            if bval != 0 {
                                let result = aval / bval;

                                new_instructions.pop();
                                new_instructions.pop();

                                let result_const = Constant::Int(result);
                                let const_id = if let Some(pos) =
                                    bytecode.constants.iter().position(|c| c == &result_const)
                                {
                                    pos
                                } else {
                                    let id = bytecode.constants.len();
                                    bytecode.constants.push(result_const.clone());
                                    id
                                };

                                new_instructions.push(Instruction::LoadConst(const_id));
                                stack.push(result_const);
                                changed = true;
                                continue;
                            }
                        }
                    }
                    stack.clear();
                    new_instructions.push(inst.clone());
                }
                _ => {
                    stack.clear();
                    new_instructions.push(inst.clone());
                }
            }
        }

        if changed {
            bytecode.main_instructions = new_instructions;
        }
        changed
    }
}

/// Pass 2: Dead Code Elimination
/// Remove código não alcançável
pub struct DeadCodeEliminationPass;

impl OptimizationPass for DeadCodeEliminationPass {
    fn name(&self) -> &str {
        "dead-code-elimination"
    }

    fn optimize(&self, bytecode: &mut Bytecode) -> bool {
        let mut reachable = vec![false; bytecode.main_instructions.len()];
        let mut worklist = vec![0];

        // Mark reachable instructions
        while let Some(ip) = worklist.pop() {
            if ip >= bytecode.main_instructions.len() || reachable[ip] {
                continue;
            }

            reachable[ip] = true;

            match &bytecode.main_instructions[ip] {
                Instruction::Jump(target) => {
                    worklist.push(*target);
                }
                Instruction::JumpIfFalse(target) => {
                    worklist.push(*target);
                    worklist.push(ip + 1);
                }
                Instruction::Return | Instruction::Halt => {}
                _ => {
                    worklist.push(ip + 1);
                }
            }
        }

        // Remove unreachable code
        let original_len = bytecode.main_instructions.len();
        let mut new_instructions = Vec::new();
        let mut old_to_new: HashMap<usize, usize> = HashMap::new();

        for (old_ip, inst) in bytecode.main_instructions.iter().enumerate() {
            if reachable[old_ip] {
                old_to_new.insert(old_ip, new_instructions.len());
                new_instructions.push(inst.clone());
            }
        }

        // Fix jump targets
        for inst in &mut new_instructions {
            match inst {
                Instruction::Jump(target) => {
                    if let Some(new_target) = old_to_new.get(target) {
                        *target = *new_target;
                    }
                }
                Instruction::JumpIfFalse(target) => {
                    if let Some(new_target) = old_to_new.get(target) {
                        *target = *new_target;
                    }
                }
                _ => {}
            }
        }

        let changed = new_instructions.len() < original_len;
        if changed {
            bytecode.main_instructions = new_instructions;
        }
        changed
    }
}

/// Pass 3: Peephole Optimization
/// Otimizações locais de padrões
pub struct PeepholeOptimizationPass;

impl OptimizationPass for PeepholeOptimizationPass {
    fn name(&self) -> &str {
        "peephole-optimization"
    }

    fn optimize(&self, bytecode: &mut Bytecode) -> bool {
        let mut changed = false;
        let mut new_instructions = Vec::new();
        let mut i = 0;

        while i < bytecode.main_instructions.len() {
            let inst = &bytecode.main_instructions[i];

            // Pattern: LoadGlobal(x) + StoreGlobal(x) = nop
            if i + 1 < bytecode.main_instructions.len() {
                if let Instruction::LoadGlobal(name1) = inst {
                    if let Instruction::StoreGlobal(name2) = &bytecode.main_instructions[i + 1] {
                        if name1 == name2 {
                            // Skip both instructions
                            i += 2;
                            changed = true;
                            continue;
                        }
                    }
                }
            }

            // Pattern: Pop + LoadConst = LoadConst (remove useless pop)
            if i + 1 < bytecode.main_instructions.len()
                && matches!(inst, Instruction::Pop)
                && matches!(bytecode.main_instructions[i + 1], Instruction::LoadConst(_))
            {
                // Skip pop
                i += 1;
                changed = true;
                continue;
            }

            new_instructions.push(inst.clone());
            i += 1;
        }

        if changed {
            bytecode.main_instructions = new_instructions;
        }
        changed
    }
}

/// Pass 4: Jump Optimization
/// Otimiza jumps desnecessários
pub struct JumpOptimizationPass;

impl OptimizationPass for JumpOptimizationPass {
    fn name(&self) -> &str {
        "jump-optimization"
    }

    fn optimize(&self, bytecode: &mut Bytecode) -> bool {
        let mut changed = false;
        let mut new_instructions = Vec::new();

        for (ip, inst) in bytecode.main_instructions.iter().enumerate() {
            match inst {
                // Jump to next instruction = nop
                Instruction::Jump(target) if *target == ip + 1 => {
                    changed = true;
                    continue;
                }
                _ => {
                    new_instructions.push(inst.clone());
                }
            }
        }

        if changed {
            bytecode.main_instructions = new_instructions;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Add,
            Instruction::Halt,
        ];

        let optimizer = BytecodeOptimizer::new(OptimizationLevel::Basic);
        let stats = optimizer.optimize(&mut bytecode);

        assert!(stats.passes_applied > 0);
        assert_eq!(bytecode.main_instructions.len(), 2); // LoadConst(30) + Halt
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));

        bytecode.main_instructions = vec![
            Instruction::Jump(2),
            Instruction::LoadConst(c1), // Dead code
            Instruction::Halt,
        ];

        let optimizer = BytecodeOptimizer::new(OptimizationLevel::Basic);
        optimizer.optimize(&mut bytecode);

        assert_eq!(bytecode.main_instructions.len(), 2); // Jump + Halt
    }

    #[test]
    fn test_peephole_optimization() {
        let mut bytecode = Bytecode::new();

        bytecode.main_instructions = vec![
            Instruction::LoadGlobal("x".to_string()),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::Halt,
        ];

        let optimizer = BytecodeOptimizer::new(OptimizationLevel::Default);
        optimizer.optimize(&mut bytecode);

        assert_eq!(bytecode.main_instructions.len(), 1); // Only Halt
    }

    #[test]
    fn test_optimization_levels() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Add,
            Instruction::Halt,
        ];

        // O0 - No optimization
        let mut bc0 = bytecode.clone();
        let optimizer0 = BytecodeOptimizer::new(OptimizationLevel::None);
        optimizer0.optimize(&mut bc0);
        assert_eq!(bc0.main_instructions.len(), 4);

        // O1 - Basic optimization
        let mut bc1 = bytecode.clone();
        let optimizer1 = BytecodeOptimizer::new(OptimizationLevel::Basic);
        optimizer1.optimize(&mut bc1);
        assert_eq!(bc1.main_instructions.len(), 2);
    }
}
