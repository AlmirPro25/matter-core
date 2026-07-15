//! Tensor Backend - algebra linear nativa em Rust para a linguagem Matter.
//!
//! Ideia central: a conta pesada (multiplicacao de matrizes, softmax, etc.)
//! NAO roda no interpretador de bytecode. Os tensores ficam armazenados aqui,
//! em Rust, e o programa Matter manipula apenas um "handle" (um inteiro).
//!
//! Isso permite treinar modelos muito maiores: o loop O(n^3) do matmul executa
//! em Rust compilado, enquanto o Matter so orquestra as operacoes.
//!
//! Uso em Matter:
//!     let a = tensor.random(64, 32, 0.1)
//!     let b = tensor.random(32, 16, 0.1)
//!     let c = tensor.matmul(a, b)        # 64x16, calculado em Rust
//!     print tensor.get(c, 0, 0)

use matter_backend::{Backend, Value};
use std::collections::HashMap;

#[derive(Clone)]
struct Tensor {
    rows: usize,
    cols: usize,
    data: Vec<f64>, // row-major: data[r * cols + c]
}

pub struct TensorBackend {
    store: HashMap<i64, Tensor>,
    next_id: i64,
    rng: u64,
}

impl TensorBackend {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            next_id: 1,
            rng: 0x2545F4914F6CDD1D, // seed fixa -> resultados reproduziveis
        }
    }

    fn insert(&mut self, t: Tensor) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.store.insert(id, t);
        id
    }

    fn get_t(&self, id: i64) -> Result<&Tensor, String> {
        self.store
            .get(&id)
            .ok_or_else(|| format!("tensor: handle {} nao existe", id))
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

impl Default for TensorBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn arity(op: &str, expected: usize, got: usize) -> Result<(), String> {
    if got != expected {
        Err(format!(
            "tensor.{} espera {} argumentos, recebeu {}",
            op, expected, got
        ))
    } else {
        Ok(())
    }
}

fn ai(args: &[Value], i: usize, op: &str) -> Result<i64, String> {
    args[i]
        .as_int()
        .map_err(|e| format!("tensor.{}: {}", op, e))
}

fn af(args: &[Value], i: usize, op: &str) -> Result<f64, String> {
    args[i]
        .as_float()
        .map_err(|e| format!("tensor.{}: {}", op, e))
}

impl Backend for TensorBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            // ---- Criacao ----
            "zeros" => {
                arity("zeros", 2, args.len())?;
                let r = ai(&args, 0, "zeros")? as usize;
                let c = ai(&args, 1, "zeros")? as usize;
                let id = self.insert(Tensor {
                    rows: r,
                    cols: c,
                    data: vec![0.0; r * c],
                });
                Ok(Value::Int(id))
            }
            "fill" => {
                arity("fill", 3, args.len())?;
                let r = ai(&args, 0, "fill")? as usize;
                let c = ai(&args, 1, "fill")? as usize;
                let v = af(&args, 2, "fill")?;
                let id = self.insert(Tensor {
                    rows: r,
                    cols: c,
                    data: vec![v; r * c],
                });
                Ok(Value::Int(id))
            }
            "random" => {
                // random(rows, cols, scale) -> valores em [-scale, scale]
                arity("random", 3, args.len())?;
                let r = ai(&args, 0, "random")? as usize;
                let c = ai(&args, 1, "random")? as usize;
                let scale = af(&args, 2, "random")?;
                let mut data = Vec::with_capacity(r * c);
                for _ in 0..(r * c) {
                    let u = self.next_f64();
                    data.push((2.0 * u - 1.0) * scale);
                }
                let id = self.insert(Tensor {
                    rows: r,
                    cols: c,
                    data,
                });
                Ok(Value::Int(id))
            }
            "from_list" => {
                // from_list(lista_plana, rows, cols) -> handle (row-major)
                arity("from_list", 3, args.len())?;
                let r = ai(&args, 1, "from_list")? as usize;
                let c = ai(&args, 2, "from_list")? as usize;
                let items = match &args[0] {
                    Value::List(items) => items,
                    _ => {
                        return Err("tensor.from_list: 1o argumento deve ser uma lista".to_string())
                    }
                };
                if items.len() != r * c {
                    return Err(format!(
                        "tensor.from_list: lista tem {} itens mas rows*cols = {}",
                        items.len(),
                        r * c
                    ));
                }
                let mut data = Vec::with_capacity(r * c);
                for it in items.iter() {
                    data.push(
                        it.as_float()
                            .map_err(|e| format!("tensor.from_list: {}", e))?,
                    );
                }
                let id = self.insert(Tensor {
                    rows: r,
                    cols: c,
                    data,
                });
                Ok(Value::Int(id))
            }
            "to_list" => {
                // to_list(handle) -> lista plana row-major
                arity("to_list", 1, args.len())?;
                let t = self.get_t(ai(&args, 0, "to_list")?)?;
                let out: Vec<Value> = t.data.iter().map(|v| Value::Float(*v)).collect();
                Ok(Value::new_list(out))
            }
            "copy" => {
                arity("copy", 1, args.len())?;
                let t = self.get_t(ai(&args, 0, "copy")?)?.clone();
                let id = self.insert(t);
                Ok(Value::Int(id))
            }

            // ---- Forma / acesso ----
            "rows" => {
                arity("rows", 1, args.len())?;
                Ok(Value::Int(self.get_t(ai(&args, 0, "rows")?)?.rows as i64))
            }
            "cols" => {
                arity("cols", 1, args.len())?;
                Ok(Value::Int(self.get_t(ai(&args, 0, "cols")?)?.cols as i64))
            }
            "shape" => {
                arity("shape", 1, args.len())?;
                let t = self.get_t(ai(&args, 0, "shape")?)?;
                Ok(Value::new_list(vec![
                    Value::Int(t.rows as i64),
                    Value::Int(t.cols as i64),
                ]))
            }
            "get" => {
                arity("get", 3, args.len())?;
                let id = ai(&args, 0, "get")?;
                let r = ai(&args, 1, "get")? as usize;
                let c = ai(&args, 2, "get")? as usize;
                let t = self.get_t(id)?;
                if r >= t.rows || c >= t.cols {
                    return Err(format!(
                        "tensor.get: indice ({},{}) fora de {}x{}",
                        r, c, t.rows, t.cols
                    ));
                }
                Ok(Value::Float(t.data[r * t.cols + c]))
            }
            "set" => {
                arity("set", 4, args.len())?;
                let id = ai(&args, 0, "set")?;
                let r = ai(&args, 1, "set")? as usize;
                let c = ai(&args, 2, "set")? as usize;
                let v = af(&args, 3, "set")?;
                let t = self
                    .store
                    .get_mut(&id)
                    .ok_or_else(|| format!("tensor: handle {} nao existe", id))?;
                if r >= t.rows || c >= t.cols {
                    return Err(format!(
                        "tensor.set: indice ({},{}) fora de {}x{}",
                        r, c, t.rows, t.cols
                    ));
                }
                let cols = t.cols;
                t.data[r * cols + c] = v;
                Ok(Value::Unit)
            }
            "row_argmax" => {
                // row_argmax(handle, r) -> indice da coluna de maior valor na linha r
                arity("row_argmax", 2, args.len())?;
                let id = ai(&args, 0, "row_argmax")?;
                let r = ai(&args, 1, "row_argmax")? as usize;
                let t = self.get_t(id)?;
                if r >= t.rows {
                    return Err(format!("tensor.row_argmax: linha {} fora de {}", r, t.rows));
                }
                let base = r * t.cols;
                let mut best = 0usize;
                let mut bv = t.data[base];
                for c in 1..t.cols {
                    if t.data[base + c] > bv {
                        bv = t.data[base + c];
                        best = c;
                    }
                }
                Ok(Value::Int(best as i64))
            }

            // ---- Algebra (a conta pesada, em Rust) ----
            "matmul" => {
                arity("matmul", 2, args.len())?;
                let a = self.get_t(ai(&args, 0, "matmul")?)?;
                let b = self.get_t(ai(&args, 1, "matmul")?)?;
                if a.cols != b.rows {
                    return Err(format!(
                        "tensor.matmul: shapes incompativeis {}x{} * {}x{}",
                        a.rows, a.cols, b.rows, b.cols
                    ));
                }
                let (m, k, n) = (a.rows, a.cols, b.cols);
                let mut data = vec![0.0; m * n];
                for i in 0..m {
                    for p in 0..k {
                        let aip = a.data[i * k + p];
                        if aip == 0.0 {
                            continue;
                        }
                        let brow = p * n;
                        let crow = i * n;
                        for j in 0..n {
                            data[crow + j] += aip * b.data[brow + j];
                        }
                    }
                }
                let id = self.insert(Tensor {
                    rows: m,
                    cols: n,
                    data,
                });
                Ok(Value::Int(id))
            }
            "transpose" => {
                arity("transpose", 1, args.len())?;
                let t = self.get_t(ai(&args, 0, "transpose")?)?;
                let (r, c) = (t.rows, t.cols);
                let mut data = vec![0.0; r * c];
                for i in 0..r {
                    for j in 0..c {
                        data[j * r + i] = t.data[i * c + j];
                    }
                }
                let id = self.insert(Tensor {
                    rows: c,
                    cols: r,
                    data,
                });
                Ok(Value::Int(id))
            }
            "add" => {
                // add(a, b): elementwise. Se b for 1xN, faz broadcast por linha (bias).
                arity("add", 2, args.len())?;
                let a = self.get_t(ai(&args, 0, "add")?)?;
                let b = self.get_t(ai(&args, 1, "add")?)?;
                let out = elementwise(a, b, "add", |x, y| x + y)?;
                let id = self.insert(out);
                Ok(Value::Int(id))
            }
            "sub" => {
                arity("sub", 2, args.len())?;
                let a = self.get_t(ai(&args, 0, "sub")?)?;
                let b = self.get_t(ai(&args, 1, "sub")?)?;
                let out = elementwise(a, b, "sub", |x, y| x - y)?;
                let id = self.insert(out);
                Ok(Value::Int(id))
            }
            "hadamard" => {
                arity("hadamard", 2, args.len())?;
                let a = self.get_t(ai(&args, 0, "hadamard")?)?;
                let b = self.get_t(ai(&args, 1, "hadamard")?)?;
                let out = elementwise(a, b, "hadamard", |x, y| x * y)?;
                let id = self.insert(out);
                Ok(Value::Int(id))
            }
            "scale" => {
                arity("scale", 2, args.len())?;
                let a = self.get_t(ai(&args, 0, "scale")?)?;
                let s = af(&args, 1, "scale")?;
                let data: Vec<f64> = a.data.iter().map(|v| v * s).collect();
                let id = self.insert(Tensor {
                    rows: a.rows,
                    cols: a.cols,
                    data,
                });
                Ok(Value::Int(id))
            }
            "relu" => {
                arity("relu", 1, args.len())?;
                let a = self.get_t(ai(&args, 0, "relu")?)?;
                let data: Vec<f64> = a
                    .data
                    .iter()
                    .map(|v| if *v > 0.0 { *v } else { 0.0 })
                    .collect();
                let id = self.insert(Tensor {
                    rows: a.rows,
                    cols: a.cols,
                    data,
                });
                Ok(Value::Int(id))
            }
            "relu_grad" => {
                // 1.0 onde pre > 0, senao 0.0
                arity("relu_grad", 1, args.len())?;
                let a = self.get_t(ai(&args, 0, "relu_grad")?)?;
                let data: Vec<f64> = a
                    .data
                    .iter()
                    .map(|v| if *v > 0.0 { 1.0 } else { 0.0 })
                    .collect();
                let id = self.insert(Tensor {
                    rows: a.rows,
                    cols: a.cols,
                    data,
                });
                Ok(Value::Int(id))
            }
            "softmax_rows" => {
                // softmax estavel por linha
                arity("softmax_rows", 1, args.len())?;
                let a = self.get_t(ai(&args, 0, "softmax_rows")?)?;
                let (r, c) = (a.rows, a.cols);
                let mut data = vec![0.0; r * c];
                for i in 0..r {
                    let base = i * c;
                    let mut m = a.data[base];
                    for j in 1..c {
                        if a.data[base + j] > m {
                            m = a.data[base + j];
                        }
                    }
                    let mut s = 0.0;
                    for j in 0..c {
                        let e = (a.data[base + j] - m).exp();
                        data[base + j] = e;
                        s += e;
                    }
                    for j in 0..c {
                        data[base + j] /= s;
                    }
                }
                let id = self.insert(Tensor {
                    rows: r,
                    cols: c,
                    data,
                });
                Ok(Value::Int(id))
            }

            // ---- Treino (in-place, evita criar handles novos) ----
            "axpy" => {
                // axpy(a, b, alpha): a = a + alpha*b  (in place). Ex: W -= lr*grad -> axpy(W, grad, -lr)
                arity("axpy", 3, args.len())?;
                let aid = ai(&args, 0, "axpy")?;
                let bid = ai(&args, 1, "axpy")?;
                let alpha = af(&args, 2, "axpy")?;
                let b = self.get_t(bid)?.clone();
                let a = self
                    .store
                    .get_mut(&aid)
                    .ok_or_else(|| format!("tensor: handle {} nao existe", aid))?;
                if a.rows != b.rows || a.cols != b.cols {
                    return Err(format!(
                        "tensor.axpy: shapes incompativeis {}x{} vs {}x{}",
                        a.rows, a.cols, b.rows, b.cols
                    ));
                }
                for i in 0..a.data.len() {
                    a.data[i] += alpha * b.data[i];
                }
                Ok(Value::Unit)
            }
            "sum_rows" => {
                // sum_rows(a) -> 1xC com a soma de cada coluna (util p/ gradiente de bias)
                arity("sum_rows", 1, args.len())?;
                let a = self.get_t(ai(&args, 0, "sum_rows")?)?;
                let mut data = vec![0.0; a.cols];
                for i in 0..a.rows {
                    for j in 0..a.cols {
                        data[j] += a.data[i * a.cols + j];
                    }
                }
                let id = self.insert(Tensor {
                    rows: 1,
                    cols: a.cols,
                    data,
                });
                Ok(Value::Int(id))
            }

            "ce_loss" => {
                // ce_loss(probs, lista_de_alvos) -> media de -ln(prob do alvo) por linha
                arity("ce_loss", 2, args.len())?;
                let p = self.get_t(ai(&args, 0, "ce_loss")?)?;
                let targets = match &args[1] {
                    Value::List(items) => items,
                    _ => {
                        return Err(
                            "tensor.ce_loss: 2o argumento deve ser lista de alvos".to_string()
                        )
                    }
                };
                if targets.len() != p.rows {
                    return Err(format!(
                        "tensor.ce_loss: {} alvos mas {} linhas",
                        targets.len(),
                        p.rows
                    ));
                }
                let mut soma = 0.0;
                for (i, t) in targets.iter().enumerate() {
                    let ti = t.as_int().map_err(|e| format!("tensor.ce_loss: {}", e))? as usize;
                    if ti >= p.cols {
                        return Err(format!("tensor.ce_loss: alvo {} fora de {}", ti, p.cols));
                    }
                    soma -= (p.data[i * p.cols + ti] + 1e-12).ln();
                }
                Ok(Value::Float(soma / p.rows as f64))
            }

            // ---- Gestao de memoria ----
            "free" => {
                arity("free", 1, args.len())?;
                let id = ai(&args, 0, "free")?;
                self.store.remove(&id);
                Ok(Value::Unit)
            }
            "count" => {
                arity("count", 0, args.len())?;
                Ok(Value::Int(self.store.len() as i64))
            }
            "seed" => {
                arity("seed", 1, args.len())?;
                let s = ai(&args, 0, "seed")?;
                self.rng = (s as u64) | 1; // garante nao-zero
                Ok(Value::Unit)
            }

            _ => Err(format!("tensor: metodo desconhecido '{}'", method)),
        }
    }
}

/// Operacao elementwise com broadcast opcional de bias (b com 1 linha).
fn elementwise<F: Fn(f64, f64) -> f64>(
    a: &Tensor,
    b: &Tensor,
    op: &str,
    f: F,
) -> Result<Tensor, String> {
    if a.rows == b.rows && a.cols == b.cols {
        let data: Vec<f64> = a
            .data
            .iter()
            .zip(b.data.iter())
            .map(|(x, y)| f(*x, *y))
            .collect();
        Ok(Tensor {
            rows: a.rows,
            cols: a.cols,
            data,
        })
    } else if b.rows == 1 && b.cols == a.cols {
        // broadcast da linha b por todas as linhas de a
        let mut data = vec![0.0; a.rows * a.cols];
        for i in 0..a.rows {
            for j in 0..a.cols {
                data[i * a.cols + j] = f(a.data[i * a.cols + j], b.data[j]);
            }
        }
        Ok(Tensor {
            rows: a.rows,
            cols: a.cols,
            data,
        })
    } else {
        Err(format!(
            "tensor.{}: shapes incompativeis {}x{} vs {}x{}",
            op, a.rows, a.cols, b.rows, b.cols
        ))
    }
}
