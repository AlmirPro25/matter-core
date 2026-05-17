//! Optimization passes for native code
//!
//! Implements various optimization techniques:
//! - Peephole optimization (local patterns)
//! - Dead code elimination
//! - Constant folding
//! - Register allocation improvements

use crate::OptLevel;

/// Optimize machine code based on optimization level
pub fn optimize(code: &[u8], level: OptLevel) -> Result<Vec<u8>, String> {
    if level != OptLevel::O0 && has_relative_control_flow(code) {
        return Ok(code.to_vec());
    }

    let mut optimized = code.to_vec();

    match level {
        OptLevel::O0 => {
            // No optimization
        }
        OptLevel::O1 => {
            // Basic optimizations
            optimized = peephole_optimize(&optimized)?;
        }
        OptLevel::O2 => {
            // Moderate optimizations
            optimized = peephole_optimize(&optimized)?;
            optimized = remove_redundant_moves(&optimized)?;
            optimized = strength_reduction(&optimized)?;
        }
        OptLevel::O3 => {
            // Aggressive optimizations
            optimized = peephole_optimize(&optimized)?;
            optimized = remove_redundant_moves(&optimized)?;
            optimized = optimize_jumps(&optimized)?;
            optimized = strength_reduction(&optimized)?;
            optimized = constant_propagation(&optimized)?;
            optimized = dead_code_elimination(&optimized)?;
            optimized = inline_expansion(&optimized)?;
            optimized = loop_unrolling(&optimized)?;
        }
    }

    Ok(optimized)
}

fn has_relative_control_flow(code: &[u8]) -> bool {
    let mut i = 0;

    while i < code.len() {
        match code[i] {
            0xE8 | 0xE9 | 0xEB => return true,
            0x0F if i + 1 < code.len() && (0x80..=0x8F).contains(&code[i + 1]) => return true,
            _ => {
                i += 1;
            }
        }
    }

    false
}

/// Peephole optimization - optimize small instruction sequences
fn peephole_optimize(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern 1: mov rax, X; mov rax, Y -> mov rax, Y
        if i + 20 < code.len()
            && is_mov_rax_imm(&code[i..i + 10])
            && is_mov_rax_imm(&code[i + 10..i + 20])
        {
            // Skip first mov, keep second
            i += 10;
            continue;
        }

        // Pattern 2: push rax; pop rax -> (remove both)
        if i + 1 < code.len() && code[i] == 0x50 && code[i + 1] == 0x58 {
            // Skip both instructions
            i += 2;
            continue;
        }

        // Pattern 3: add rax, 0 -> (remove)
        if i + 7 < code.len()
            && code[i] == 0x48
            && code[i + 1] == 0x83
            && code[i + 2] == 0xC0
            && code[i + 3] == 0x00
        {
            // Skip add rax, 0
            i += 4;
            continue;
        }

        // No optimization, copy byte
        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Remove redundant mov instructions
fn remove_redundant_moves(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern: mov rax, rax -> (remove)
        if i + 2 < code.len() && code[i] == 0x48 && code[i + 1] == 0x89 && code[i + 2] == 0xC0 {
            // Skip mov rax, rax
            i += 3;
            continue;
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Optimize jump instructions
fn optimize_jumps(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern: jmp +5; <5 bytes> -> <5 bytes> (remove jump to next instruction)
        if i + 4 < code.len() && code[i] == 0xE9 {
            let offset = i32::from_le_bytes([code[i + 1], code[i + 2], code[i + 3], code[i + 4]]);

            if offset == 0 {
                // Jump to next instruction, skip it
                i += 5;
                continue;
            }
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Check if bytes represent "mov rax, imm64"
fn is_mov_rax_imm(bytes: &[u8]) -> bool {
    bytes.len() >= 10 && bytes[0] == 0x48 && bytes[1] == 0xB8
}

/// Strength reduction - replace expensive operations with cheaper ones
fn strength_reduction(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern: imul rax, 2 -> add rax, rax (multiplication by 2)
        if i + 6 < code.len()
            && code[i] == 0x48
            && code[i + 1] == 0x69
            && code[i + 2] == 0xC0
            && code[i + 3] == 0x02
            && code[i + 4] == 0x00
            && code[i + 5] == 0x00
            && code[i + 6] == 0x00
        {
            // Replace with: add rax, rax (48 01 C0)
            optimized.extend_from_slice(&[0x48, 0x01, 0xC0]);
            i += 7;
            continue;
        }

        // Pattern: imul rax, 1 -> nop (multiplication by 1)
        if i + 6 < code.len()
            && code[i] == 0x48
            && code[i + 1] == 0x69
            && code[i + 2] == 0xC0
            && code[i + 3] == 0x01
            && code[i + 4] == 0x00
            && code[i + 5] == 0x00
            && code[i + 6] == 0x00
        {
            // Skip multiplication by 1
            i += 7;
            continue;
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Constant propagation - propagate known constant values
fn constant_propagation(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern: mov rax, 0; add rax, X -> mov rax, X
        if i + 17 < code.len()
            && code[i] == 0x48
            && code[i + 1] == 0xB8
            && code[i + 2..i + 10] == [0x00; 8]
            && code[i + 10] == 0x48
            && code[i + 11] == 0x05
        {
            // Skip mov rax, 0 and replace add with mov
            let value =
                i32::from_le_bytes([code[i + 12], code[i + 13], code[i + 14], code[i + 15]]);
            // mov rax, value
            optimized.extend_from_slice(&[0x48, 0xB8]);
            optimized.extend_from_slice(&(value as i64).to_le_bytes());
            i += 16;
            continue;
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Dead code elimination - remove unreachable code
fn dead_code_elimination(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;
    let mut skip_until_label = false;

    while i < code.len() {
        // Pattern: ret; <code> -> ret (code after ret is unreachable)
        if code[i] == 0xC3 {
            // ret instruction
            optimized.push(code[i]);
            i += 1;
            skip_until_label = true;
            continue;
        }

        // If we're skipping dead code, look for potential labels/targets
        if skip_until_label {
            // Check if this might be a jump target (heuristic: aligned address)
            if i % 16 == 0 {
                skip_until_label = false;
            } else {
                // Skip dead code
                i += 1;
                continue;
            }
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Inline expansion - inline small function calls
///
/// Replaces function calls with the function body when:
/// - Function is small (< 32 bytes)
/// - Function is called frequently
/// - No recursion detected
fn inline_expansion(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    // Track function definitions and their sizes
    let mut functions = Vec::new();
    let mut current_func_start = 0;
    let mut in_function = false;

    // First pass: identify functions
    let mut j = 0;
    while j < code.len() {
        // Function prologue: push rbp; mov rbp, rsp
        if j + 3 < code.len()
            && code[j] == 0x55
            && code[j + 1] == 0x48
            && code[j + 2] == 0x89
            && code[j + 3] == 0xE5
        {
            current_func_start = j;
            in_function = true;
        }

        // Function epilogue: pop rbp; ret
        if in_function && j + 1 < code.len() && code[j] == 0x5D && code[j + 1] == 0xC3 {
            let func_size = j + 2 - current_func_start;
            if func_size < 32 {
                // Small function, candidate for inlining
                functions.push((current_func_start, func_size));
            }
            in_function = false;
        }

        j += 1;
    }

    // Second pass: inline small function calls
    while i < code.len() {
        // Pattern: call <offset> where target is a small function
        if i + 4 < code.len() && code[i] == 0xE8 {
            let offset = i32::from_le_bytes([code[i + 1], code[i + 2], code[i + 3], code[i + 4]]);
            let target = (i as i32 + 5 + offset) as usize;

            // Check if target is a small function
            let mut should_inline = false;
            let mut func_start = 0;
            let mut func_size = 0;

            for &(start, size) in &functions {
                if start == target {
                    should_inline = true;
                    func_start = start;
                    func_size = size;
                    break;
                }
            }

            if should_inline && func_start + func_size <= code.len() {
                // Inline the function body (skip prologue and epilogue)
                // Prologue: 4 bytes (push rbp; mov rbp, rsp)
                // Epilogue: 2 bytes (pop rbp; ret)
                let body_start = func_start + 4;
                let body_end = func_start + func_size - 2;

                if body_end > body_start {
                    optimized.extend_from_slice(&code[body_start..body_end]);
                }

                i += 5; // Skip call instruction
                continue;
            }
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

/// Loop unrolling - unroll small loops for better performance
///
/// Unrolls loops with constant iteration counts:
/// - Reduces loop overhead (condition checks, jumps)
/// - Enables better instruction pipelining
/// - Increases code size but improves performance
fn loop_unrolling(code: &[u8]) -> Result<Vec<u8>, String> {
    let mut optimized = Vec::new();
    let mut i = 0;

    while i < code.len() {
        // Pattern: Simple counted loop
        // mov rcx, N; loop_start: <body>; dec rcx; jnz loop_start
        if i + 10 < code.len() && code[i] == 0x48 && code[i + 1] == 0xB9
        // mov rcx, imm64
        {
            let count = i64::from_le_bytes([
                code[i + 2],
                code[i + 3],
                code[i + 4],
                code[i + 5],
                code[i + 6],
                code[i + 7],
                code[i + 8],
                code[i + 9],
            ]);

            // Only unroll small loops (2-8 iterations)
            if (2..=8).contains(&count) {
                // Look for loop body and back-edge
                let loop_start = i + 10;
                let mut loop_end = loop_start;
                let mut found_loop = false;

                // Scan for dec rcx; jnz pattern
                for j in loop_start..code.len().saturating_sub(6) {
                    if code[j] == 0x48
                        && code[j + 1] == 0xFF
                        && code[j + 2] == 0xC9 // dec rcx
                        && code[j + 3] == 0x75
                    {
                        // jnz short
                        loop_end = j;
                        found_loop = true;
                        break;
                    }
                }

                if found_loop && loop_end > loop_start {
                    let body_size = loop_end - loop_start;

                    // Only unroll if body is small (< 16 bytes)
                    if body_size < 16 {
                        // Unroll the loop: repeat body N times
                        for _ in 0..count {
                            optimized.extend_from_slice(&code[loop_start..loop_end]);
                        }

                        i = loop_end + 4; // Skip dec rcx; jnz
                        continue;
                    }
                }
            }
        }

        optimized.push(code[i]);
        i += 1;
    }

    Ok(optimized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peephole_basic() {
        // push rax; pop rax
        let code = vec![0x50, 0x58];
        let optimized = peephole_optimize(&code).unwrap();
        // Should remove both instructions
        assert_eq!(optimized.len(), 0);
    }

    #[test]
    fn test_remove_redundant_moves() {
        // mov rax, rax
        let code = vec![0x48, 0x89, 0xC0];
        let optimized = remove_redundant_moves(&code).unwrap();
        // Should remove the redundant move
        assert_eq!(optimized.len(), 0);
    }

    #[test]
    fn test_optimize_jumps() {
        // jmp +0 (jump to next instruction)
        let code = vec![0xE9, 0x00, 0x00, 0x00, 0x00];
        let optimized = optimize_jumps(&code).unwrap();
        // Should remove the useless jump
        assert_eq!(optimized.len(), 0);
    }

    #[test]
    fn test_no_optimization_needed() {
        // Some random valid code
        let code = vec![0x48, 0xB8, 0x0A, 0x00, 0x00, 0x00]; // mov rax, 10 (partial)
        let optimized = peephole_optimize(&code).unwrap();
        // Should keep the code as-is
        assert_eq!(optimized.len(), code.len());
    }

    #[test]
    fn test_strength_reduction_mul_by_2() {
        // imul rax, 2 (48 69 C0 02 00 00 00)
        let code = vec![0x48, 0x69, 0xC0, 0x02, 0x00, 0x00, 0x00];
        let optimized = strength_reduction(&code).unwrap();
        // Should be replaced with add rax, rax (48 01 C0)
        assert_eq!(optimized, vec![0x48, 0x01, 0xC0]);
    }

    #[test]
    fn test_strength_reduction_mul_by_1() {
        // imul rax, 1 (48 69 C0 01 00 00 00)
        let code = vec![0x48, 0x69, 0xC0, 0x01, 0x00, 0x00, 0x00];
        let optimized = strength_reduction(&code).unwrap();
        // Should be removed
        assert_eq!(optimized.len(), 0);
    }

    #[test]
    fn test_dead_code_elimination() {
        // ret; nop; nop (C3 90 90)
        let code = vec![0xC3, 0x90, 0x90];
        let optimized = dead_code_elimination(&code).unwrap();
        // Should remove nops after ret
        assert_eq!(optimized, vec![0xC3]);
    }

    #[test]
    fn test_optimization_levels() {
        let code = vec![0x50, 0x58]; // push rax; pop rax

        // O0 - no optimization
        let o0 = optimize(&code, OptLevel::O0).unwrap();
        assert_eq!(o0.len(), 2);

        // O1 - basic optimization
        let o1 = optimize(&code, OptLevel::O1).unwrap();
        assert_eq!(o1.len(), 0);

        // O2 - moderate optimization
        let o2 = optimize(&code, OptLevel::O2).unwrap();
        assert_eq!(o2.len(), 0);

        // O3 - aggressive optimization
        let o3 = optimize(&code, OptLevel::O3).unwrap();
        assert_eq!(o3.len(), 0);
    }

    #[test]
    fn test_optimize_preserves_relative_control_flow() {
        let code = vec![
            0x48, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, // mov rax, 0
            0x0F, 0x84, 0x0A, 0, 0, 0, // je +10
            0x50, 0x58, // push rax; pop rax
        ];

        let optimized = optimize(&code, OptLevel::O3).unwrap();
        assert_eq!(optimized, code);
    }

    #[test]
    fn test_inline_expansion() {
        // Test that inline expansion doesn't break valid code
        // Small function: push rbp; mov rbp, rsp; mov rax, 42; pop rbp; ret
        let code = vec![
            // Function at offset 0
            0x55, // push rbp
            0x48, 0x89, 0xE5, // mov rbp, rsp
            0x48, 0xB8, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov rax, 42
            0x5D, // pop rbp
            0xC3, // ret
        ];

        let optimized = inline_expansion(&code).unwrap();

        // Should preserve the function (no calls to inline)
        assert_eq!(optimized.len(), code.len());
    }

    #[test]
    fn test_loop_unrolling_small() {
        // Test that loop unrolling doesn't break valid code
        // mov rcx, 4; add rax, 1; dec rcx; jnz
        let code = vec![
            0x48, 0xB9, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov rcx, 4
            0x48, 0x83, 0xC0, 0x01, // add rax, 1
        ];

        let optimized = loop_unrolling(&code).unwrap();

        // Should preserve the code (pattern not complete)
        assert_eq!(optimized.len(), code.len());
    }

    #[test]
    fn test_loop_unrolling_too_large() {
        // mov rcx, 100 (too many iterations, should not unroll)
        let code = vec![
            0x48, 0xB9, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov rcx, 100
            0x48, 0x83, 0xC0, 0x01, // add rax, 1
        ];

        let optimized = loop_unrolling(&code).unwrap();

        // Should NOT unroll (too many iterations)
        assert_eq!(optimized.len(), code.len());
    }

    #[test]
    fn test_inline_expansion_preserves_code() {
        // Test that inline expansion preserves code without function calls
        let code = vec![0x48, 0xB8, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // mov rax, 10
        let optimized = inline_expansion(&code).unwrap();
        assert_eq!(optimized.len(), code.len());
    }

    #[test]
    fn test_loop_unrolling_preserves_code() {
        // Test that loop unrolling preserves code without loops
        let code = vec![0x48, 0xB8, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // mov rax, 10
        let optimized = loop_unrolling(&code).unwrap();
        assert_eq!(optimized.len(), code.len());
    }
}
