//! Matter Core (lite)
//!
//! Nucleo enxuto da linguagem Matter: compila e roda `.matter` usando apenas
//! compiler + VM + stdlib + backends essenciais. Sem fisica, sem bridges,
//! sem studio. Prova que o coracao da linguagem e independente e leve.

use std::process::exit;

use matter_backend::{AgentBackend, GraphBackend, NetBackend, StoreBackend, ToolBackend};
use matter_bytecode::Bytecode;
use matter_stdlib::{
    AudioBackend, ConsoleBackend, FileBackend, FileIOBackend, HashMapBackend, JsonBackend,
    ListBackend, MapBackend, MathBackend, RandomBackend, StringBackend, TensorBackend, TimeBackend, TypeBackend,
    VecBackend, WorldBackend,
};
use matter_vm::Vm;

/// Monta uma VM com os backends do nucleo (sem fisica nem bridges).
fn build_vm(bytecode: Bytecode) -> Vm {
    let mut vm = Vm::new(bytecode);

    // Backends essenciais de runtime
    vm.register_backend("agent".to_string(), Box::new(AgentBackend::new()));
    vm.register_backend("store".to_string(), Box::new(StoreBackend::new()));
    vm.register_backend("net".to_string(), Box::new(NetBackend::new()));
    vm.register_backend("graph".to_string(), Box::new(GraphBackend::new()));
    vm.register_backend("tool".to_string(), Box::new(ToolBackend::new()));

    // Standard library
    vm.register_backend("math".to_string(), Box::new(MathBackend::new()));
    vm.register_backend("string".to_string(), Box::new(StringBackend::new()));
    vm.register_backend("list".to_string(), Box::new(ListBackend::new()));
    vm.register_backend("time".to_string(), Box::new(TimeBackend::new()));
    vm.register_backend("random".to_string(), Box::new(RandomBackend::new()));
    vm.register_backend("json".to_string(), Box::new(JsonBackend::new()));
    vm.register_backend("world".to_string(), Box::new(WorldBackend::new()));
    vm.register_backend("audio".to_string(), Box::new(AudioBackend::new()));
    vm.register_backend("map".to_string(), Box::new(MapBackend::new()));
    vm.register_backend("type".to_string(), Box::new(TypeBackend::new()));
    vm.register_backend("console".to_string(), Box::new(ConsoleBackend::new()));
    vm.register_backend("file".to_string(), Box::new(FileBackend::new()));
    vm.register_backend("fileio".to_string(), Box::new(FileIOBackend::new()));
    vm.register_backend("Vec".to_string(), Box::new(VecBackend::new()));
    vm.register_backend("HashMap".to_string(), Box::new(HashMapBackend::new()));
    vm.register_backend("tensor".to_string(), Box::new(TensorBackend::new()));

    vm
}

fn read_source(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: nao consegui ler '{}': {}", path, e);
            exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Matter Core (lite)");
    eprintln!("Uso:");
    eprintln!("  mc run   <arquivo.matter>   compila e executa");
    eprintln!("  mc check <arquivo.matter>   compila e valida (efeitos), sem rodar");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        print_usage();
        exit(2);
    }

    let cmd = args[1].as_str();
    let path = args[2].as_str();
    let source = read_source(path);

    match cmd {
        "check" => match matter_compiler::compile_checked(&source) {
            Ok(_) => println!("ok: '{}' compilou e passou no checador de efeitos", path),
            Err(e) => {
                eprintln!("erro de compilacao: {}", e);
                exit(1);
            }
        },
        "run" => {
            let bytecode = match matter_compiler::compile_checked(&source) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("erro de compilacao: {}", e);
                    exit(1);
                }
            };
            let mut vm = build_vm(bytecode);
            if let Err(e) = vm.run() {
                eprintln!("erro de execucao: {}", e);
                exit(1);
            }
        }
        other => {
            eprintln!("comando desconhecido: '{}'", other);
            print_usage();
            exit(2);
        }
    }
}
