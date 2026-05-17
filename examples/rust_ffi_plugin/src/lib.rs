use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn add(args: *const c_char) -> *mut c_char {
    ffi_entry(args, |values| {
        let left = arg_int(values, 0)?;
        let right = arg_int(values, 1)?;
        Ok(json!({ "type": "int", "value": left + right }))
    })
}

#[no_mangle]
pub extern "C" fn describe(args: *const c_char) -> *mut c_char {
    ffi_entry(args, |values| {
        let name = arg_string(values, 0)?;
        Ok(json!({
            "type": "string",
            "value": format!("hello from Rust FFI, {}", name)
        }))
    })
}

#[no_mangle]
pub extern "C" fn stats(args: *const c_char) -> *mut c_char {
    ffi_entry(args, |values| {
        let items = values
            .first()
            .and_then(|value| value.get("value"))
            .and_then(Value::as_array)
            .ok_or_else(|| "stats expects one list argument".to_string())?;

        let mut count = 0_i64;
        let mut total = 0_i64;
        for item in items {
            total += item
                .get("value")
                .and_then(Value::as_i64)
                .ok_or_else(|| "stats list must contain int values".to_string())?;
            count += 1;
        }

        Ok(json!({
            "type": "map",
            "value": {
                "count": { "type": "int", "value": count },
                "total": { "type": "int", "value": total }
            }
        }))
    })
}

#[no_mangle]
pub extern "C" fn fail(_args: *const c_char) -> *mut c_char {
    into_c_string(json!({
        "type": "error",
        "message": "intentional Rust FFI plugin failure"
    }))
}

#[no_mangle]
pub extern "C" fn matter_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

fn ffi_entry(
    args: *const c_char,
    handler: impl FnOnce(&[Value]) -> Result<Value, String>,
) -> *mut c_char {
    let result = read_args(args).and_then(|values| handler(&values));
    match result {
        Ok(value) => into_c_string(value),
        Err(message) => into_c_string(json!({ "type": "error", "message": message })),
    }
}

fn read_args(args: *const c_char) -> Result<Vec<Value>, String> {
    if args.is_null() {
        return Err("args pointer is null".to_string());
    }

    let source = unsafe { CStr::from_ptr(args) }
        .to_str()
        .map_err(|error| format!("args are not UTF-8: {}", error))?;
    let values: Value =
        serde_json::from_str(source).map_err(|error| format!("args are not JSON: {}", error))?;
    values
        .as_array()
        .cloned()
        .ok_or_else(|| "args must be a JSON array".to_string())
}

fn arg_int(values: &[Value], index: usize) -> Result<i64, String> {
    values
        .get(index)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("argument {} must be an int", index))
}

fn arg_string(values: &[Value], index: usize) -> Result<String, String> {
    values
        .get(index)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("argument {} must be a string", index))
}

fn into_c_string(value: Value) -> *mut c_char {
    let payload = serde_json::to_string(&value).unwrap_or_else(|error| {
        format!(
            "{{\"type\":\"error\",\"message\":\"failed to encode plugin result: {}\"}}",
            escape_json_string(&error.to_string())
        )
    });
    CString::new(payload)
        .unwrap_or_else(|_| CString::new("{\"type\":\"error\",\"message\":\"nul byte\"}").unwrap())
        .into_raw()
}

fn escape_json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn add_returns_integer_result() {
        let output = call_export(
            add,
            r#"[{"type":"int","value":41},{"type":"int","value":1}]"#,
        );
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["type"], "int");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn describe_returns_string_result() {
        let output = call_export(describe, r#"[{"type":"string","value":"Matter"}]"#);
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["type"], "string");
        assert_eq!(value["value"], "hello from Rust FFI, Matter");
    }

    #[test]
    fn stats_returns_map_result() {
        let output = call_export(
            stats,
            r#"[{"type":"list","value":[{"type":"int","value":10},{"type":"int","value":20},{"type":"int","value":12}]}]"#,
        );
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["type"], "map");
        assert_eq!(value["value"]["count"]["value"], 3);
        assert_eq!(value["value"]["total"]["value"], 42);
    }

    #[test]
    fn fail_returns_formal_error() {
        let output = call_export(fail, "[]");
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["type"], "error");
        assert_eq!(value["message"], "intentional Rust FFI plugin failure");
    }

    #[test]
    fn invalid_args_are_formal_errors() {
        let output = call_export(add, r#"[{"type":"int","value":1}]"#);
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["type"], "error");
        assert!(value["message"]
            .as_str()
            .unwrap_or_default()
            .contains("argument 1"));
    }

    fn call_export(
        export: extern "C" fn(*const std::os::raw::c_char) -> *mut std::os::raw::c_char,
        args: &str,
    ) -> String {
        let input = CString::new(args).unwrap();
        let output = export(input.as_ptr());
        assert!(!output.is_null());

        let text = unsafe { CStr::from_ptr(output) }
            .to_str()
            .unwrap()
            .to_string();
        matter_free_string(output);
        text
    }
}
