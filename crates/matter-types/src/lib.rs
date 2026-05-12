//! Matter Type System
//!
//! Sistema de tipos gradual que permite começar dinâmico e adicionar tipos progressivamente.
//!
//! ## Funcionalidades
//!
//! - Tipos opcionais (int, string, bool, etc)
//! - Inferência de tipos inteligente
//! - Tipos nullable (int?)
//! - Tipos não-nullable (string!)
//! - Tipos genéricos (<T>)
//! - Union types (int | string)
//! - Type aliases
//!
//! ## Exemplo
//!
//! ```matter
//! // Dinâmico (padrão)
//! let x = 42;
//!
//! // Tipado explícito
//! let age: int = 25;
//!
//! // Nullable
//! let maybe: int? = null;
//!
//! // Não-nullable
//! let required: string! = "value";
//!
//! // Genérico
//! fn identity<T>(value: T) -> T {
//!     return value;
//! }
//! ```

use std::collections::HashMap;
use std::fmt;

/// Type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Any type (dynamic)
    Any,

    /// Unit type (void)
    Unit,

    /// Integer type
    Int,

    /// Float type
    Float,

    /// Boolean type
    Bool,

    /// String type
    String,

    /// List type
    List(Box<Type>),

    /// Map type
    Map(Box<Type>, Box<Type>),

    /// Struct type
    Struct(String, Vec<(String, Type)>),

    /// Function type
    Function(Vec<Type>, Box<Type>),

    /// Nullable type
    Nullable(Box<Type>),

    /// Non-nullable type
    NonNullable(Box<Type>),

    /// Union type
    Union(Vec<Type>),

    /// Generic type parameter
    Generic(String),

    /// Type alias
    Alias(String),
}

impl Type {
    /// Check if type is nullable
    pub fn is_nullable(&self) -> bool {
        matches!(self, Type::Nullable(_) | Type::Any)
    }

    /// Check if type is non-nullable
    pub fn is_non_nullable(&self) -> bool {
        matches!(self, Type::NonNullable(_))
    }

    /// Get inner type if nullable
    pub fn inner_type(&self) -> Option<&Type> {
        match self {
            Type::Nullable(inner) | Type::NonNullable(inner) => Some(inner),
            _ => None,
        }
    }

    /// Check if this type is compatible with another
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            // Any is compatible with everything
            (Type::Any, _) | (_, Type::Any) => true,

            // Same types are compatible
            (a, b) if a == b => true,

            // Nullable compatibility
            (Type::Nullable(a), b) => a.is_compatible_with(b),
            (a, Type::Nullable(b)) => a.is_compatible_with(b),

            // Non-nullable compatibility
            (Type::NonNullable(a), b) => a.is_compatible_with(b),
            (a, Type::NonNullable(b)) => a.is_compatible_with(b),

            // Union types
            (Type::Union(types), other) => types.iter().any(|t| t.is_compatible_with(other)),
            (other, Type::Union(types)) => types.iter().any(|t| other.is_compatible_with(t)),

            // Lists
            (Type::List(a), Type::List(b)) => a.is_compatible_with(b),

            // Maps
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                k1.is_compatible_with(k2) && v1.is_compatible_with(v2)
            }

            // Functions
            (Type::Function(args1, ret1), Type::Function(args2, ret2)) => {
                args1.len() == args2.len()
                    && args1
                        .iter()
                        .zip(args2.iter())
                        .all(|(a, b)| a.is_compatible_with(b))
                    && ret1.is_compatible_with(ret2)
            }

            _ => false,
        }
    }

    /// Infer type from value
    pub fn infer_from_literal(literal: &str) -> Type {
        // Try to parse as int
        if literal.parse::<i64>().is_ok() {
            return Type::Int;
        }

        // Try to parse as float
        if literal.parse::<f64>().is_ok() {
            return Type::Float;
        }

        // Check for boolean
        if literal == "true" || literal == "false" {
            return Type::Bool;
        }

        // Check for string (quoted)
        if literal.starts_with('"') && literal.ends_with('"') {
            return Type::String;
        }

        // Default to Any
        Type::Any
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Any => write!(f, "any"),
            Type::Unit => write!(f, "unit"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::List(inner) => write!(f, "list<{}>", inner),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Struct(name, _) => write!(f, "{}", name),
            Type::Function(args, ret) => {
                write!(f, "fn(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Nullable(inner) => write!(f, "{}?", inner),
            Type::NonNullable(inner) => write!(f, "{}!", inner),
            Type::Union(types) => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            Type::Generic(name) => write!(f, "{}", name),
            Type::Alias(name) => write!(f, "{}", name),
        }
    }
}

/// Type environment for type checking
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Variable types
    variables: HashMap<String, Type>,

    /// Function types
    functions: HashMap<String, Type>,

    /// Type aliases
    aliases: HashMap<String, Type>,

    /// Generic type parameters
    generics: HashMap<String, Type>,
}

impl TypeEnv {
    /// Create a new type environment
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            aliases: HashMap::new(),
            generics: HashMap::new(),
        }
    }

    /// Add a variable type
    pub fn add_variable(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }

    /// Get a variable type
    pub fn get_variable(&self, name: &str) -> Option<&Type> {
        self.variables.get(name)
    }

    /// Add a function type
    pub fn add_function(&mut self, name: String, ty: Type) {
        self.functions.insert(name, ty);
    }

    /// Get a function type
    pub fn get_function(&self, name: &str) -> Option<&Type> {
        self.functions.get(name)
    }

    /// Add a type alias
    pub fn add_alias(&mut self, name: String, ty: Type) {
        self.aliases.insert(name, ty);
    }

    /// Get a type alias
    pub fn get_alias(&self, name: &str) -> Option<&Type> {
        self.aliases.get(name)
    }

    /// Add a generic type parameter
    pub fn add_generic(&mut self, name: String, ty: Type) {
        self.generics.insert(name, ty);
    }

    /// Get a generic type parameter
    pub fn get_generic(&self, name: &str) -> Option<&Type> {
        self.generics.get(name)
    }

    /// Resolve a type (follow aliases)
    pub fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Alias(name) => {
                if let Some(resolved) = self.get_alias(name) {
                    self.resolve_type(resolved)
                } else {
                    ty.clone()
                }
            }
            Type::Generic(name) => {
                if let Some(resolved) = self.get_generic(name) {
                    self.resolve_type(resolved)
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<String>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
        }
    }

    /// Check if a value matches a type
    pub fn check_type(&mut self, value_type: &Type, expected_type: &Type) -> bool {
        let value_type = self.env.resolve_type(value_type);
        let expected_type = self.env.resolve_type(expected_type);

        if !value_type.is_compatible_with(&expected_type) {
            self.errors.push(format!(
                "Type mismatch: expected {}, got {}",
                expected_type, value_type
            ));
            return false;
        }

        true
    }

    /// Get type errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Check if there are errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get type environment
    pub fn env(&self) -> &TypeEnv {
        &self.env
    }

    /// Get mutable type environment
    pub fn env_mut(&mut self) -> &mut TypeEnv {
        &mut self.env
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatibility() {
        assert!(Type::Int.is_compatible_with(&Type::Int));
        assert!(Type::Any.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&Type::Any));
        assert!(!Type::Int.is_compatible_with(&Type::String));
    }

    #[test]
    fn test_nullable_types() {
        let nullable_int = Type::Nullable(Box::new(Type::Int));
        assert!(nullable_int.is_nullable());
        assert!(nullable_int.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&nullable_int));
    }

    #[test]
    fn test_union_types() {
        let union = Type::Union(vec![Type::Int, Type::String]);
        assert!(union.is_compatible_with(&Type::Int));
        assert!(union.is_compatible_with(&Type::String));
        assert!(!union.is_compatible_with(&Type::Bool));
    }

    #[test]
    fn test_type_inference() {
        assert_eq!(Type::infer_from_literal("42"), Type::Int);
        assert_eq!(Type::infer_from_literal("3.14"), Type::Float);
        assert_eq!(Type::infer_from_literal("true"), Type::Bool);
        assert_eq!(Type::infer_from_literal("\"hello\""), Type::String);
    }

    #[test]
    fn test_type_checker() {
        let mut checker = TypeChecker::new();
        assert!(checker.check_type(&Type::Int, &Type::Int));
        assert!(!checker.check_type(&Type::Int, &Type::String));
        assert!(checker.has_errors());
    }

    #[test]
    fn test_type_env() {
        let mut env = TypeEnv::new();
        env.add_variable("x".to_string(), Type::Int);
        assert_eq!(env.get_variable("x"), Some(&Type::Int));

        env.add_alias("MyInt".to_string(), Type::Int);
        let resolved = env.resolve_type(&Type::Alias("MyInt".to_string()));
        assert_eq!(resolved, Type::Int);
    }
}
