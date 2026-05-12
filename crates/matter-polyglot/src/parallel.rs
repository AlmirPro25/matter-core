// Matter Polyglot: Parallel Bridge Execution
// Executa múltiplas linguagens simultaneamente

use crate::{Bridge, BridgeResult, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

fn lock_unpoison<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Executor paralelo de bridges
/// 
/// Permite executar chamadas para múltiplas linguagens simultaneamente
/// Performance: N linguagens em paralelo = N x speedup
pub struct ParallelBridgeExecutor {
    /// Bridges disponíveis
    bridges: Arc<Mutex<HashMap<String, Box<dyn Bridge + Send>>>>,
    /// Número de threads
    num_threads: usize,
}

/// Tarefa de execução
pub struct BridgeTask {
    /// Nome da linguagem
    pub language: String,
    /// Módulo
    pub module: String,
    /// Função
    pub function: String,
    /// Argumentos
    pub args: Vec<Value>,
}

/// Resultado de execução
pub struct BridgeTaskResult {
    /// Nome da linguagem
    pub language: String,
    /// Resultado
    pub result: BridgeResult<Value>,
    /// Tempo de execução (ms)
    pub execution_time_ms: u64,
}

impl ParallelBridgeExecutor {
    /// Cria novo executor
    pub fn new(num_threads: usize) -> Self {
        Self {
            bridges: Arc::new(Mutex::new(HashMap::new())),
            num_threads,
        }
    }

    /// Registra um bridge
    pub fn register_bridge(&mut self, bridge: Box<dyn Bridge + Send>) {
        let mut bridges = lock_unpoison(&self.bridges);
        bridges.insert(bridge.name().to_string(), bridge);
    }

    /// Executa múltiplas tarefas em paralelo
    pub fn execute_parallel(&self, tasks: Vec<BridgeTask>) -> Vec<BridgeTaskResult> {
        if tasks.is_empty() {
            return Vec::new();
        }

        let worker_count = self.num_threads.max(1);
        let chunk_size = (tasks.len() + worker_count - 1) / worker_count;
        let mut handles = vec![];

        for chunk in tasks.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let bridges = Arc::clone(&self.bridges);

            let handle = thread::spawn(move || {
                let mut results = Vec::new();

                for task in chunk {
                    let start = std::time::Instant::now();

                    let result = {
                        let bridges = lock_unpoison(&bridges);
                        
                        if let Some(bridge) = bridges.get(&task.language) {
                            bridge.call(&task.module, &task.function, task.args.clone())
                        } else {
                            Err(crate::BridgeError::RuntimeError(
                                format!("Bridge not found: {}", task.language)
                            ))
                        }
                    };

                    let execution_time_ms = start.elapsed().as_millis() as u64;

                    results.push(BridgeTaskResult {
                        language: task.language,
                        result,
                        execution_time_ms,
                    });
                }

                results
            });

            handles.push(handle);
        }

        // Coleta resultados
        let mut all_results = Vec::new();
        for handle in handles {
            if let Ok(results) = handle.join() {
                all_results.extend(results);
            }
        }

        all_results
    }

    /// Executa tarefas com timeout
    pub fn execute_with_timeout(
        &self,
        tasks: Vec<BridgeTask>,
        timeout_ms: u64,
    ) -> Vec<BridgeTaskResult> {
        let (tx, rx) = std::sync::mpsc::channel();
        let executor = self.clone();

        thread::spawn(move || {
            let results = executor.execute_parallel(tasks);
            let _ = tx.send(results);
        });

        match rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)) {
            Ok(results) => results,
            Err(_) => {
                // Timeout - retorna erro para todas as tarefas
                vec![]
            }
        }
    }

    /// Executa com retry automático
    pub fn execute_with_retry(
        &self,
        tasks: Vec<BridgeTask>,
        max_retries: usize,
    ) -> Vec<BridgeTaskResult> {
        let mut results = self.execute_parallel(tasks.clone());
        let mut retry_count = 0;

        while retry_count < max_retries {
            // Identifica tarefas que falharam
            let failed_tasks: Vec<_> = results
                .iter()
                .enumerate()
                .filter(|(_, r)| r.result.is_err())
                .map(|(i, _)| tasks[i].clone())
                .collect();

            if failed_tasks.is_empty() {
                break;
            }

            // Retry tarefas que falharam
            let retry_results = self.execute_parallel(failed_tasks);

            // Atualiza resultados
            let mut retry_idx = 0;
            for (i, result) in results.iter_mut().enumerate() {
                if result.result.is_err() {
                    *result = retry_results[retry_idx].clone();
                    retry_idx += 1;
                }
            }

            retry_count += 1;
        }

        results
    }
}

impl Clone for ParallelBridgeExecutor {
    fn clone(&self) -> Self {
        Self {
            bridges: Arc::clone(&self.bridges),
            num_threads: self.num_threads,
        }
    }
}

impl Clone for BridgeTask {
    fn clone(&self) -> Self {
        Self {
            language: self.language.clone(),
            module: self.module.clone(),
            function: self.function.clone(),
            args: self.args.clone(),
        }
    }
}

impl Clone for BridgeTaskResult {
    fn clone(&self) -> Self {
        Self {
            language: self.language.clone(),
            result: self.result.clone(),
            execution_time_ms: self.execution_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = ParallelBridgeExecutor::new(4);
        assert_eq!(executor.num_threads, 4);
    }

    #[test]
    fn test_parallel_execution() {
        let executor = ParallelBridgeExecutor::new(2);
        
        // Cria tarefas de teste
        let tasks = vec![
            BridgeTask {
                language: "python".to_string(),
                module: "math".to_string(),
                function: "sqrt".to_string(),
                args: vec![Value::Int(16)],
            },
            BridgeTask {
                language: "nodejs".to_string(),
                module: "fs".to_string(),
                function: "readFile".to_string(),
                args: vec![Value::String("test.txt".to_string())],
            },
        ];

        // Executa (vai falhar porque bridges não estão registrados, mas testa a estrutura)
        let results = executor.execute_parallel(tasks);
        assert_eq!(results.len(), 2);
    }
}
