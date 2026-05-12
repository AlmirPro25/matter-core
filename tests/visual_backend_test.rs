//! Integration tests for Visual Backend
//! Tests the PVM/PXL integration with Matter Core

use matter_bytecode::BytecodeBuilder;
use matter_parser::Parser;
use matter_runtime::Runtime;

/// Helper para executar código Matter visual
fn run_visual_code(source: &str) -> Result<(), String> {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|e| e.to_string())?;

    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).map_err(|e| e.to_string())?;

    let mut runtime = Runtime::new(bytecode);
    runtime.run()?;

    Ok(())
}

#[test]
fn test_visual_basic_commands() {
    let source = r#"
        visual.surface("main", 1080, 1920)
        visual.region("checkout", 100, 200, 300, 80)
        visual.pulse("checkout")
        visual.run("pizzaria")
    "#;

    assert!(run_visual_code(source).is_ok());
}

#[test]
fn test_visual_with_events() {
    let source = r#"
        on boot {
            visual.run("pizzaria")
            visual.surface("main", 1080, 1920)
        }
    "#;

    let mut parser = Parser::from_source(source);
    let program = parser.parse().unwrap();
    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).unwrap();
    let mut runtime = Runtime::new(bytecode);

    // Execute main
    assert!(runtime.run().is_ok());

    // Emit boot event
    assert!(runtime.emit_event("boot").is_ok());
}

#[test]
fn test_visual_set_properties() {
    let source = r#"
        visual.region("button", 100, 100, 200, 50)
        visual.set("button", "energy", 100)
        visual.set("button", "material", "glass")
    "#;

    assert!(run_visual_code(source).is_ok());
}

#[test]
fn test_visual_load_pvmbc() {
    let source = r#"
        visual.load("apps/pizzaria.pvmbc")
        visual.run("pizzaria")
    "#;

    assert!(run_visual_code(source).is_ok());
}

#[test]
fn test_visual_complex_workflow() {
    let source = r#"
        on boot {
            visual.surface("main", 1080, 1920)
            visual.region("menu", 50, 50, 400, 60)
            visual.region("checkout", 100, 200, 300, 80)
            visual.set("checkout", "energy", 100)
        }
        
        on tap {
            visual.pulse("checkout")
            visual.set("checkout", "energy", 80)
        }
    "#;

    let mut parser = Parser::from_source(source);
    let program = parser.parse().unwrap();
    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).unwrap();
    let mut runtime = Runtime::new(bytecode);

    // Execute main
    assert!(runtime.run().is_ok());

    // Emit boot event
    assert!(runtime.emit_event("boot").is_ok());

    // Emit tap event
    assert!(runtime.emit_event("tap").is_ok());
}

#[test]
fn test_visual_bytecode_serialization() {
    let source = r#"
        visual.surface("main", 1080, 1920)
        visual.pulse("main")
    "#;

    // Parse and build bytecode
    let mut parser = Parser::from_source(source);
    let program = parser.parse().unwrap();
    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).unwrap();

    // Serialize and deserialize
    let mut buffer = Vec::new();
    assert!(bytecode.serialize(&mut buffer).is_ok());

    let bytecode_deserialized = matter_bytecode::Bytecode::deserialize(&mut buffer.as_slice());
    assert!(bytecode_deserialized.is_ok());

    // Execute deserialized bytecode
    let mut runtime = Runtime::new(bytecode_deserialized.unwrap());
    assert!(runtime.run().is_ok());
}
