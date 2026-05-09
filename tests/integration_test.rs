/// Integration tests for Matter Core
/// Testes end-to-end do pipeline completo

use matter_ast::Program;
use matter_bytecode::{Bytecode, BytecodeBuilder};
use matter_parser::Parser;
use matter_runtime::Runtime;
use std::fs;
use std::path::Path;

/// Helper para executar código Matter e capturar output
fn run_matter_code(source: &str) -> Result<Vec<String>, String> {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|e| e.to_string())?;

    let builder = BytecodeBuilder::new();
    let bytecode = builder
        .build_checked(&program)
        .map_err(|e| e.to_string())?;

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    runtime.run()?;

    Ok(runtime.take_output())
}

/// Helper para compilar e executar bytecode
fn compile_and_run(source: &str) -> Result<Vec<String>, String> {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|e| e.to_string())?;

    let builder = BytecodeBuilder::new();
    let bytecode = builder
        .build_checked(&program)
        .map_err(|e| e.to_string())?;

    // Serialize and deserialize to test round-trip
    let mut buffer = Vec::new();
    bytecode
        .serialize(&mut buffer)
        .map_err(|e| e.to_string())?;

    let bytecode = Bytecode::deserialize(&mut buffer.as_slice()).map_err(|e| e.to_string())?;

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    runtime.run()?;

    Ok(runtime.take_output())
}

#[test]
fn test_hello_world() {
    let source = r#"
        let x = 10
        let y = 20
        let z = x + y
        print z
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["30"]);
}

#[test]
fn test_conditionals() {
    let source = r#"
        let x = 15
        if x > 10 {
            print "greater"
        }
        if x < 10 {
            print "less"
        } else {
            print "not less"
        }
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["greater", "not less"]);
}

#[test]
fn test_functions() {
    let source = r#"
        fn add(a, b) {
            return a + b
        }
        
        fn multiply(x, y) {
            return x * y
        }
        
        let r1 = add(10, 20)
        let r2 = multiply(5, 6)
        print r1
        print r2
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["30", "30"]);
}

#[test]
fn test_recursion() {
    let source = r#"
        fn factorial(n) {
            if n <= 1 {
                return 1
            }
            return n * factorial(n - 1)
        }
        
        print factorial(5)
        print factorial(6)
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["120", "720"]);
}

#[test]
fn test_while_loop() {
    let source = r#"
        let counter = 0
        while counter < 3 {
            print counter
            set counter = counter + 1
        }
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn test_loop_with_break() {
    let source = r#"
        let i = 0
        loop {
            if i >= 3 {
                break
            }
            print i
            set i = i + 1
        }
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["0", "1", "2"]);
}

#[test]
fn test_loop_with_continue() {
    let source = r#"
        let i = 0
        while i < 5 {
            set i = i + 1
            if i == 3 {
                continue
            }
            print i
        }
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["1", "2", "4", "5"]);
}

#[test]
fn test_nested_scopes() {
    let source = r#"
        let x = 10
        if true {
            let x = 20
            print x
        }
        print x
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["20", "10"]);
}

#[test]
fn test_lists() {
    let source = r#"
        let nums = [1, 2, 3]
        print nums
        nums.push(4)
        print nums
        print nums.len()
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["[1, 2, 3]", "[1, 2, 3, 4]", "4"]);
}

#[test]
fn test_list_indexing() {
    let source = r#"
        let nums = [10, 20, 30]
        print nums[0]
        print nums[1]
        print nums[2]
        set nums[1] = 99
        print nums[1]
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["10", "20", "30", "99"]);
}

#[test]
fn test_maps() {
    let source = r#"
        let user = { "name": "Alice", "age": 30 }
        print user
        print user["name"]
        print user.has("age")
        print user.has("email")
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(
        output,
        vec![
            "{age: 30, name: Alice}",
            "Alice",
            "true",
            "false"
        ]
    );
}

#[test]
fn test_structs() {
    let source = r#"
        struct User {
            name: string,
            age: int
        }
        
        let user = User { name: "Bob", age: 25 }
        print user.name
        print user.age
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["Bob", "25"]);
}

#[test]
fn test_for_loop() {
    let source = r#"
        let nums = [1, 2, 3]
        for num in nums {
            print num
        }
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["1", "2", "3"]);
}

#[test]
fn test_bytecode_equivalence() {
    let source = r#"
        fn fib(n) {
            if n <= 1 {
                return n
            }
            return fib(n - 1) + fib(n - 2)
        }
        
        print fib(7)
    "#;

    let direct_output = run_matter_code(source).expect("Failed to run directly");
    let bytecode_output = compile_and_run(source).expect("Failed to run from bytecode");

    assert_eq!(direct_output, bytecode_output);
    assert_eq!(direct_output, vec!["13"]);
}

#[test]
fn test_complex_expressions() {
    let source = r#"
        let a = 10
        let b = 20
        let c = 30
        
        let result = (a + b) * c - (a * b)
        print result
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["700"]); // (10 + 20) * 30 - (10 * 20) = 900 - 200 = 700
}

#[test]
fn test_nested_function_calls() {
    let source = r#"
        fn add(a, b) {
            return a + b
        }
        
        fn double(x) {
            return x * 2
        }
        
        let result = double(add(5, 10))
        print result
    "#;

    let output = run_matter_code(source).expect("Failed to run");
    assert_eq!(output, vec!["30"]);
}

#[test]
fn test_error_undefined_variable() {
    let source = r#"
        print x
    "#;

    let result = run_matter_code(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined variable"));
}

#[test]
fn test_error_undefined_function() {
    let source = r#"
        let x = unknown_function(10)
    "#;

    let result = run_matter_code(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown function"));
}

#[test]
fn test_error_wrong_arity() {
    let source = r#"
        fn add(a, b) {
            return a + b
        }
        
        let x = add(10)
    "#;

    let result = run_matter_code(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expects 2 argument"));
}

#[test]
fn test_error_break_outside_loop() {
    let source = r#"
        break
    "#;

    let result = run_matter_code(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("outside of a loop"));
}

#[test]
fn test_error_return_outside_function() {
    let source = r#"
        return 10
    "#;

    let result = run_matter_code(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("outside of a function"));
}

#[test]
fn test_all_examples() {
    let examples_dir = Path::new("examples");
    if !examples_dir.exists() {
        return; // Skip if examples directory doesn't exist
    }

    let test_files = vec![
        "hello.matter",
        "simple.matter",
        "test_functions.matter",
        "test_recursion.matter",
        "test_loops.matter",
        "test_lists.matter",
        "test_maps.matter",
        "test_structs.matter",
    ];

    for file in test_files {
        let path = examples_dir.join(file);
        if !path.exists() {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read {}", file));

        let result = run_matter_code(&source);
        assert!(
            result.is_ok(),
            "Example {} failed: {:?}",
            file,
            result.err()
        );
    }
}
