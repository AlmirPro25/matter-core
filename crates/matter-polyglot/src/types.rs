//! Conversão de tipos entre Matter e outras linguagens

use matter_backend::Value;

pub trait TypeConverter {
    fn to_matter(&self, foreign_value: &dyn std::any::Any) -> Result<Value, String>;
    fn decode_matter(&self, matter_value: &Value) -> Result<Box<dyn std::any::Any>, String>;
}

/// Mapeamento de tipos entre linguagens
pub struct TypeMapping {
    pub matter_type: &'static str,
    pub python_type: &'static str,
    pub javascript_type: &'static str,
    pub rust_type: &'static str,
    pub go_type: &'static str,
    pub java_type: &'static str,
}

pub const TYPE_MAPPINGS: &[TypeMapping] = &[
    TypeMapping {
        matter_type: "int",
        python_type: "int",
        javascript_type: "number",
        rust_type: "i64",
        go_type: "int64",
        java_type: "long",
    },
    TypeMapping {
        matter_type: "float",
        python_type: "float",
        javascript_type: "number",
        rust_type: "f64",
        go_type: "float64",
        java_type: "double",
    },
    TypeMapping {
        matter_type: "bool",
        python_type: "bool",
        javascript_type: "boolean",
        rust_type: "bool",
        go_type: "bool",
        java_type: "boolean",
    },
    TypeMapping {
        matter_type: "string",
        python_type: "str",
        javascript_type: "string",
        rust_type: "String",
        go_type: "string",
        java_type: "String",
    },
    TypeMapping {
        matter_type: "list",
        python_type: "list",
        javascript_type: "Array",
        rust_type: "Vec<T>",
        go_type: "[]T",
        java_type: "ArrayList<T>",
    },
    TypeMapping {
        matter_type: "map",
        python_type: "dict",
        javascript_type: "Object",
        rust_type: "HashMap<K,V>",
        go_type: "map[K]V",
        java_type: "HashMap<K,V>",
    },
];

pub fn get_type_mapping(matter_type: &str) -> Option<&'static TypeMapping> {
    TYPE_MAPPINGS.iter().find(|m| m.matter_type == matter_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mappings() {
        let int_mapping = get_type_mapping("int").unwrap();
        assert_eq!(int_mapping.python_type, "int");
        assert_eq!(int_mapping.javascript_type, "number");
        assert_eq!(int_mapping.rust_type, "i64");

        let list_mapping = get_type_mapping("list").unwrap();
        assert_eq!(list_mapping.python_type, "list");
        assert_eq!(list_mapping.javascript_type, "Array");
    }
}
