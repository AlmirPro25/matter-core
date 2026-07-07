#!/usr/bin/env python3
import re

# Ler o arquivo
with open(r'crates\matter-visual\src\lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Substituir construções de Value (mas não pattern matches)
# Padrão: Value::String(...) seguido de ) ou , mas NÃO seguido de =>
content = re.sub(r'Value::String\(([^)]+)\)(?!\s*=>)', r'Value::new_string(\1)', content)
content = re.sub(r'Value::List\(([^)]+)\)(?!\s*=>)', r'Value::new_list(\1)', content)
content = re.sub(r'Value::Map\(([^)]+)\)(?!\s*=>)', r'Value::new_map(\1)', content)

# Corrigir derefs em pattern matches que retornam String
# region_state
content = re.sub(
    r'Some\(Value::String\(state\)\) => Some\(state\.clone\(\)\)',
    r'Some(Value::String(state)) => Some((**state).clone())',
    content
)

# region_label - format!
content = re.sub(
    r'if let Some\(Value::String\(state\)\) = properties\.get\("state"\) \{\s+return format!\("state: \{\}", state\);',
    r'if let Some(Value::String(state)) = properties.get("state") {\n            return format!("state: {}", **state);',
    content
)

# region_label - semantic.clone()
content = re.sub(
    r'if let Some\(Value::String\(semantic\)\) = properties\.get\("semantic"\) \{\s+return semantic\.clone\(\);',
    r'if let Some(Value::String(semantic)) = properties.get("semantic") {\n            return (**semantic).clone();',
    content
)

# region_label - material.clone()
content = re.sub(
    r'if let Some\(Value::String\(material\)\) = properties\.get\("material"\) \{\s+return material\.clone\(\);',
    r'if let Some(Value::String(material)) = properties.get("material") {\n            return (**material).clone();',
    content
)

# region_event - event.clone()
content = re.sub(
    r'if let Some\(Value::String\(event\)\) = properties\.get\("event"\) \{\s+return event\.clone\(\);',
    r'if let Some(Value::String(event)) = properties.get("event") {\n            return (**event).clone();',
    content
)

# region_event - behavior.clone()
content = re.sub(
    r'if let Some\(Value::String\(behavior\)\) = properties\.get\("behavior"\) \{\s+return behavior\.clone\(\);',
    r'if let Some(Value::String(behavior)) = properties.get("behavior") {\n            return (**behavior).clone();',
    content
)

# Corrigir value_json - Struct
content = re.sub(
    r'Value::Struct \{ type_name, fields \} => \{\s+let mut values = fields\.clone\(\);\s+values\.insert\("__type"\.to_string\(\), Value::new_string\(type_name\.clone\(\)\)\);',
    r'Value::Struct { type_name, fields } => {\n            let mut values = (**fields).clone();\n            values.insert("__type".to_string(), Value::new_string((**type_name).clone()));',
    content
)

# Escrever o arquivo
with open(r'crates\matter-visual\src\lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Correções aplicadas com sucesso!")
