// Matter Smart Type Inference
// Cross-language type inference with automatic conversion
#![allow(clippy::result_large_err)]

use matter_error::MatterError;
use matter_types::Type;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type Result<T> = std::result::Result<T, MatterError>;

/// Smart type inference engine
pub struct SmartInference {
    /// Type constraint graph
    constraints: DiGraph<TypeNode, TypeConstraint>,
    /// Variable to node mapping
    variables: HashMap<String, NodeIndex>,
    /// Language-specific type mappings
    language_types: HashMap<String, LanguageTypeSystem>,
}

/// Type node in constraint graph
#[derive(Debug, Clone)]
pub struct TypeNode {
    pub name: String,
    pub inferred_type: Option<Type>,
    pub language: Option<String>,
    pub confidence: f64,
}

/// Type constraint between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeConstraint {
    /// Must be equal
    Equal,
    /// Must be subtype
    Subtype,
    /// Must be convertible
    Convertible,
    /// Cross-language conversion
    CrossLanguage { from: String, to: String },
}

/// Language-specific type system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageTypeSystem {
    pub language: String,
    pub type_mappings: HashMap<String, Vec<String>>,
    pub conversion_cost: HashMap<(String, String), f64>,
}

impl SmartInference {
    /// Create a new smart inference engine
    pub fn new() -> Self {
        let mut engine = Self {
            constraints: DiGraph::new(),
            variables: HashMap::new(),
            language_types: HashMap::new(),
        };

        // Initialize language type systems
        engine.init_python_types();
        engine.init_nodejs_types();
        engine.init_rust_types();
        engine.init_go_types();
        engine.init_java_types();

        engine
    }

    /// Add a variable to the inference graph
    pub fn add_variable(&mut self, name: &str, language: Option<&str>) -> NodeIndex {
        if let Some(&idx) = self.variables.get(name) {
            return idx;
        }

        let node = TypeNode {
            name: name.to_string(),
            inferred_type: None,
            language: language.map(|s| s.to_string()),
            confidence: 0.0,
        };

        let idx = self.constraints.add_node(node);
        self.variables.insert(name.to_string(), idx);
        idx
    }

    /// Add a type constraint
    pub fn add_constraint(&mut self, from: NodeIndex, to: NodeIndex, constraint: TypeConstraint) {
        self.constraints.add_edge(from, to, constraint);
    }

    /// Infer types across languages
    pub fn infer_types(&mut self) -> Result<HashMap<String, Type>> {
        // Iterative constraint solving
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            // Propagate type information through constraints
            for edge in self.constraints.edge_indices() {
                if let Some((from, to)) = self.constraints.edge_endpoints(edge) {
                    let constraint = self.constraints[edge].clone();

                    if self.propagate_constraint(from, to, &constraint)? {
                        changed = true;
                    }
                }
            }
        }

        // Extract inferred types
        let mut result = HashMap::new();
        for (name, &idx) in &self.variables {
            if let Some(ref typ) = self.constraints[idx].inferred_type {
                result.insert(name.clone(), typ.clone());
            }
        }

        Ok(result)
    }

    /// Propagate constraint between nodes
    fn propagate_constraint(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        constraint: &TypeConstraint,
    ) -> Result<bool> {
        let from_type = self.constraints[from].inferred_type.clone();
        let to_type = self.constraints[to].inferred_type.clone();

        match constraint {
            TypeConstraint::Equal => {
                if let Some(typ) = from_type.or(to_type.clone()) {
                    if self.constraints[from].inferred_type.is_none() {
                        self.constraints[from].inferred_type = Some(typ.clone());
                        self.constraints[from].confidence = 1.0;
                        return Ok(true);
                    }
                    if self.constraints[to].inferred_type.is_none() {
                        self.constraints[to].inferred_type = Some(typ);
                        self.constraints[to].confidence = 1.0;
                        return Ok(true);
                    }
                }
            }
            TypeConstraint::Convertible => {
                if let (Some(from_t), None) = (&from_type, &to_type) {
                    self.constraints[to].inferred_type = Some(from_t.clone());
                    self.constraints[to].confidence = 0.8;
                    return Ok(true);
                }
            }
            TypeConstraint::CrossLanguage {
                from: from_lang,
                to: to_lang,
            } => {
                if let Some(from_t) = &from_type {
                    if let Some(converted) = self.convert_type(from_t, from_lang, to_lang)? {
                        if self.constraints[to].inferred_type.is_none() {
                            self.constraints[to].inferred_type = Some(converted);
                            self.constraints[to].confidence = 0.9;
                            return Ok(true);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(false)
    }

    /// Convert type between languages
    fn convert_type(&self, typ: &Type, from_lang: &str, to_lang: &str) -> Result<Option<Type>> {
        // Get language type systems
        let from_system = self.language_types.get(from_lang);
        let to_system = self.language_types.get(to_lang);

        if from_system.is_none() || to_system.is_none() {
            return Ok(None);
        }

        // Simple type conversions
        let converted = match typ {
            Type::Int => Some(Type::Int),
            Type::Float => Some(Type::Float),
            Type::String => Some(Type::String),
            Type::Bool => Some(Type::Bool),
            Type::List(inner) => self
                .convert_type(inner, from_lang, to_lang)?
                .map(|converted_inner| Type::List(Box::new(converted_inner))),
            _ => Some(typ.clone()),
        };

        Ok(converted)
    }

    /// Initialize Python type system
    fn init_python_types(&mut self) {
        let mut mappings = HashMap::new();
        mappings.insert("int".to_string(), vec!["Number".to_string()]);
        mappings.insert("float".to_string(), vec!["Number".to_string()]);
        mappings.insert("str".to_string(), vec!["String".to_string()]);
        mappings.insert("bool".to_string(), vec!["Bool".to_string()]);
        mappings.insert("list".to_string(), vec!["Array".to_string()]);
        mappings.insert("dict".to_string(), vec!["Object".to_string()]);

        self.language_types.insert(
            "python".to_string(),
            LanguageTypeSystem {
                language: "python".to_string(),
                type_mappings: mappings,
                conversion_cost: HashMap::new(),
            },
        );
    }

    /// Initialize Node.js type system
    fn init_nodejs_types(&mut self) {
        let mut mappings = HashMap::new();
        mappings.insert("number".to_string(), vec!["Number".to_string()]);
        mappings.insert("string".to_string(), vec!["String".to_string()]);
        mappings.insert("boolean".to_string(), vec!["Bool".to_string()]);
        mappings.insert("Array".to_string(), vec!["Array".to_string()]);
        mappings.insert("Object".to_string(), vec!["Object".to_string()]);

        self.language_types.insert(
            "nodejs".to_string(),
            LanguageTypeSystem {
                language: "nodejs".to_string(),
                type_mappings: mappings,
                conversion_cost: HashMap::new(),
            },
        );
    }

    /// Initialize Rust type system
    fn init_rust_types(&mut self) {
        let mut mappings = HashMap::new();
        mappings.insert("i32".to_string(), vec!["Number".to_string()]);
        mappings.insert("i64".to_string(), vec!["Number".to_string()]);
        mappings.insert("f64".to_string(), vec!["Number".to_string()]);
        mappings.insert("String".to_string(), vec!["String".to_string()]);
        mappings.insert("bool".to_string(), vec!["Bool".to_string()]);
        mappings.insert("Vec".to_string(), vec!["Array".to_string()]);

        self.language_types.insert(
            "rust".to_string(),
            LanguageTypeSystem {
                language: "rust".to_string(),
                type_mappings: mappings,
                conversion_cost: HashMap::new(),
            },
        );
    }

    /// Initialize Go type system
    fn init_go_types(&mut self) {
        let mut mappings = HashMap::new();
        mappings.insert("int".to_string(), vec!["Number".to_string()]);
        mappings.insert("float64".to_string(), vec!["Number".to_string()]);
        mappings.insert("string".to_string(), vec!["String".to_string()]);
        mappings.insert("bool".to_string(), vec!["Bool".to_string()]);
        mappings.insert("[]".to_string(), vec!["Array".to_string()]);
        mappings.insert("map".to_string(), vec!["Object".to_string()]);

        self.language_types.insert(
            "go".to_string(),
            LanguageTypeSystem {
                language: "go".to_string(),
                type_mappings: mappings,
                conversion_cost: HashMap::new(),
            },
        );
    }

    /// Initialize Java type system
    fn init_java_types(&mut self) {
        let mut mappings = HashMap::new();
        mappings.insert("int".to_string(), vec!["Number".to_string()]);
        mappings.insert("double".to_string(), vec!["Number".to_string()]);
        mappings.insert("String".to_string(), vec!["String".to_string()]);
        mappings.insert("boolean".to_string(), vec!["Bool".to_string()]);
        mappings.insert("ArrayList".to_string(), vec!["Array".to_string()]);
        mappings.insert("HashMap".to_string(), vec!["Object".to_string()]);

        self.language_types.insert(
            "java".to_string(),
            LanguageTypeSystem {
                language: "java".to_string(),
                type_mappings: mappings,
                conversion_cost: HashMap::new(),
            },
        );
    }

    /// Get optimal conversion path between languages
    pub fn get_conversion_path(&self, from_lang: &str, to_lang: &str) -> Vec<String> {
        // Direct conversion if possible
        if from_lang == to_lang {
            return vec![from_lang.to_string()];
        }

        // For now, direct conversion
        vec![from_lang.to_string(), to_lang.to_string()]
    }
}

impl Default for SmartInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_inference_creation() {
        let inference = SmartInference::new();
        assert_eq!(inference.language_types.len(), 5);
    }

    #[test]
    fn test_add_variable() {
        let mut inference = SmartInference::new();
        let idx = inference.add_variable("x", Some("python"));
        assert!(inference.variables.contains_key("x"));
        assert_eq!(inference.constraints[idx].name, "x");
    }

    #[test]
    fn test_type_conversion() {
        let inference = SmartInference::new();
        let typ = Type::Float;
        let converted = inference.convert_type(&typ, "python", "nodejs").unwrap();
        assert!(converted.is_some());
        assert_eq!(converted.unwrap(), Type::Float);
    }
}
