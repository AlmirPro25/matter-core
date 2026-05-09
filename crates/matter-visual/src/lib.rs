/// Visual Backend for Matter Core
/// Integração com PVM/PXL como backend visual desacoplado
///
/// Arquitetura:
/// - Matter Core permanece linguagem geral
/// - PVM/PXL é um backend/plugin visual
/// - Visual é um target, não uma dependência core

use matter_backend::{Backend, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VisualSurfaceSpec {
    pub name: String,
    pub width: i64,
    pub height: i64,
}

/// Especificação de uma região visual (PXL)
#[derive(Debug, Clone)]
pub struct VisualRegionSpec {
    pub name: String,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub semantic: Option<String>,
    pub behavior: Option<String>,
    pub material: Option<String>,
    pub energy: Option<f64>,
}

/// Erros do sistema visual
#[derive(Debug)]
pub enum VisualError {
    InvalidArgument(String),
    RuntimeError(String),
    PvmNotAvailable,
}

impl std::fmt::Display for VisualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisualError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            VisualError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            VisualError::PvmNotAvailable => write!(f, "PVM runtime not available"),
        }
    }
}

impl std::error::Error for VisualError {}

/// Trait para runtime visual (contrato para PVM futuro)
pub trait VisualRuntime {
    fn run_app(&mut self, name: &str) -> Result<(), VisualError>;
    fn load_pvmbc(&mut self, path: &str) -> Result<(), VisualError>;
    fn create_surface(&mut self, name: &str, width: i64, height: i64) -> Result<(), VisualError>;
    fn create_region(&mut self, region: VisualRegionSpec) -> Result<(), VisualError>;
    fn pulse(&mut self, target: &str) -> Result<(), VisualError>;
    fn set_property(&mut self, target: &str, key: &str, value: Value) -> Result<(), VisualError>;
}

/// Backend visual com trace/mock (implementação inicial)
/// Depois será substituído por PvmVisualBackend real
pub struct TraceVisualBackend {
    surfaces: HashMap<String, VisualSurfaceSpec>,
    regions: HashMap<String, VisualRegionSpec>,
    properties: HashMap<String, HashMap<String, Value>>,
    pulses: Vec<String>,
    loaded: Vec<String>,
    apps: Vec<String>,
    stdout_enabled: bool,
}

impl TraceVisualBackend {
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
            regions: HashMap::new(),
            properties: HashMap::new(),
            pulses: Vec::new(),
            loaded: Vec::new(),
            apps: Vec::new(),
            stdout_enabled: true,
        }
    }

    pub fn new_silent() -> Self {
        let mut backend = Self::new();
        backend.stdout_enabled = false;
        backend
    }

    pub fn pxl_snapshot(&self) -> String {
        render_pxl_document(self)
    }

    pub fn export_pxl(&self, path: &str) -> Result<(), VisualError> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
            }
        }

        fs::write(path, self.pxl_snapshot())
            .map_err(|error| VisualError::RuntimeError(error.to_string()))
    }

    pub fn export_preview(&self, path: &str) -> Result<(), VisualError> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
            }
        }

        fs::write(path, render_pxl_preview(self))
            .map_err(|error| VisualError::RuntimeError(error.to_string()))
    }
}

impl Default for TraceVisualBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualRuntime for TraceVisualBackend {
    fn run_app(&mut self, name: &str) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!("[VISUAL] run {}", name);
        }
        self.apps.push(name.to_string());
        Ok(())
    }

    fn load_pvmbc(&mut self, path: &str) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!("[VISUAL] load {}", path);
        }
        self.loaded.push(path.to_string());
        Ok(())
    }

    fn create_surface(&mut self, name: &str, width: i64, height: i64) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!("[VISUAL] surface {} {}x{}", name, width, height);
        }
        self.surfaces.insert(
            name.to_string(),
            VisualSurfaceSpec {
                name: name.to_string(),
                width,
                height,
            },
        );
        Ok(())
    }

    fn create_region(&mut self, region: VisualRegionSpec) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!(
                "[VISUAL] region {} x={} y={} w={} h={}",
                region.name, region.x, region.y, region.w, region.h
            );
            if let Some(ref semantic) = region.semantic {
                println!("  semantic: {}", semantic);
            }
            if let Some(ref behavior) = region.behavior {
                println!("  behavior: {}", behavior);
            }
            if let Some(ref material) = region.material {
                println!("  material: {}", material);
            }
            if let Some(energy) = region.energy {
                println!("  energy: {}", energy);
            }
        }
        self.regions.insert(region.name.clone(), region);
        Ok(())
    }

    fn pulse(&mut self, target: &str) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!("[VISUAL] pulse {}", target);
        }
        self.pulses.push(target.to_string());
        Ok(())
    }

    fn set_property(&mut self, target: &str, key: &str, value: Value) -> Result<(), VisualError> {
        if self.stdout_enabled {
            println!("[VISUAL] set {} {} = {}", target, key, value.to_display_string());
        }
        self.properties
            .entry(target.to_string())
            .or_default()
            .insert(key.to_string(), value);
        Ok(())
    }
}

impl Backend for TraceVisualBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "run" => {
                if args.len() != 1 {
                    return Err(format!("visual.run expects 1 argument, got {}", args.len()));
                }
                let name = args[0].as_string()
                    .map_err(|_| "visual.run expects string argument".to_string())?;
                self.run_app(&name)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "load" => {
                if args.len() != 1 {
                    return Err(format!("visual.load expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string()
                    .map_err(|_| "visual.load expects string argument".to_string())?;
                self.load_pvmbc(&path)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "surface" => {
                if args.len() != 3 {
                    return Err(format!("visual.surface expects 3 arguments, got {}", args.len()));
                }
                let name = args[0].as_string()
                    .map_err(|_| "visual.surface expects string name".to_string())?;
                let width = args[1].as_int()
                    .map_err(|_| "visual.surface expects int width".to_string())?;
                let height = args[2].as_int()
                    .map_err(|_| "visual.surface expects int height".to_string())?;
                self.create_surface(&name, width, height)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "region" => {
                // Forma simples: visual.region(name, x, y, w, h)
                if args.len() == 5 {
                    let name = args[0].as_string()
                        .map_err(|_| "visual.region expects string name".to_string())?;
                    let x = args[1].as_int()
                        .map_err(|_| "visual.region expects int x".to_string())?;
                    let y = args[2].as_int()
                        .map_err(|_| "visual.region expects int y".to_string())?;
                    let w = args[3].as_int()
                        .map_err(|_| "visual.region expects int w".to_string())?;
                    let h = args[4].as_int()
                        .map_err(|_| "visual.region expects int h".to_string())?;

                    let region = VisualRegionSpec {
                        name,
                        x,
                        y,
                        w,
                        h,
                        semantic: None,
                        behavior: None,
                        material: None,
                        energy: None,
                    };

                    self.create_region(region)
                        .map_err(|e| e.to_string())?;
                    Ok(Value::Unit)
                }
                // Forma com map (futuro): visual.region(name, {x: 100, y: 200, ...})
                else if args.len() == 2 {
                    let name = args[0].as_string()
                        .map_err(|_| "visual.region expects string name".to_string())?;

                    // Extrair propriedades do map
                    if let Value::Map(ref props) = args[1] {
                        let x = props.get("x")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'x' property".to_string())?;
                        let y = props.get("y")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'y' property".to_string())?;
                        let w = props.get("w")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'w' property".to_string())?;
                        let h = props.get("h")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'h' property".to_string())?;

                        let semantic = props.get("semantic")
                            .and_then(|v| v.as_string().ok());
                        let behavior = props.get("behavior")
                            .and_then(|v| v.as_string().ok());
                        let material = props.get("material")
                            .and_then(|v| v.as_string().ok());
                        let energy = props.get("energy")
                            .and_then(|v| v.as_int().ok())
                            .map(|i| i as f64);

                        let region = VisualRegionSpec {
                            name,
                            x,
                            y,
                            w,
                            h,
                            semantic,
                            behavior,
                            material,
                            energy,
                        };

                        self.create_region(region)
                            .map_err(|e| e.to_string())?;
                        Ok(Value::Unit)
                    } else {
                        Err("visual.region expects map as second argument".to_string())
                    }
                } else {
                    Err(format!("visual.region expects 2 or 5 arguments, got {}", args.len()))
                }
            }
            "pulse" => {
                if args.len() != 1 {
                    return Err(format!("visual.pulse expects 1 argument, got {}", args.len()));
                }
                let target = args[0].as_string()
                    .map_err(|_| "visual.pulse expects string argument".to_string())?;
                self.pulse(&target)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "set" => {
                if args.len() != 3 {
                    return Err(format!("visual.set expects 3 arguments, got {}", args.len()));
                }
                let target = args[0].as_string()
                    .map_err(|_| "visual.set expects string target".to_string())?;
                let key = args[1].as_string()
                    .map_err(|_| "visual.set expects string key".to_string())?;
                let value = args[2].clone();
                self.set_property(&target, &key, value)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "snapshot" => {
                if !args.is_empty() {
                    return Err(format!("visual.snapshot expects 0 arguments, got {}", args.len()));
                }
                Ok(Value::String(self.pxl_snapshot()))
            }
            "export" => {
                if args.len() != 1 {
                    return Err(format!("visual.export expects 1 argument, got {}", args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.export expects string path".to_string())?;
                self.export_pxl(&path).map_err(|e| e.to_string())?;
                Ok(Value::String(path))
            }
            "preview" => {
                if args.len() != 1 {
                    return Err(format!("visual.preview expects 1 argument, got {}", args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.preview expects string path".to_string())?;
                self.export_preview(&path).map_err(|e| e.to_string())?;
                Ok(Value::String(path))
            }
            _ => Err(format!("Unknown visual method: {}", method)),
        }
    }
}

fn render_pxl_document(backend: &TraceVisualBackend) -> String {
    let mut surfaces: Vec<&VisualSurfaceSpec> = backend.surfaces.values().collect();
    surfaces.sort_by(|left, right| left.name.cmp(&right.name));
    let surface_json = surfaces
        .into_iter()
        .map(render_surface)
        .collect::<Vec<_>>()
        .join(",");

    let mut regions: Vec<&VisualRegionSpec> = backend.regions.values().collect();
    regions.sort_by(|left, right| left.name.cmp(&right.name));
    let region_json = regions
        .into_iter()
        .map(|region| render_region(region, backend.properties.get(&region.name)))
        .collect::<Vec<_>>()
        .join(",");

    let pulse_json = backend
        .pulses
        .iter()
        .map(|target| format!("{{\"target\":\"{}\"}}", json_escape(target)))
        .collect::<Vec<_>>()
        .join(",");
    let loaded_json = string_array_json(&backend.loaded);
    let app_json = string_array_json(&backend.apps);

    format!(
        "{{\"format\":\"PXL\",\"version\":1,\"surfaces\":[{}],\"regions\":[{}],\"pulses\":[{}],\"loaded\":{},\"apps\":{}}}",
        surface_json, region_json, pulse_json, loaded_json, app_json
    )
}

fn render_surface(surface: &VisualSurfaceSpec) -> String {
    format!(
        "{{\"name\":\"{}\",\"width\":{},\"height\":{}}}",
        json_escape(&surface.name),
        surface.width,
        surface.height
    )
}

fn render_region(region: &VisualRegionSpec, properties: Option<&HashMap<String, Value>>) -> String {
    let mut fields = vec![
        format!("\"name\":\"{}\"", json_escape(&region.name)),
        format!("\"x\":{}", region.x),
        format!("\"y\":{}", region.y),
        format!("\"w\":{}", region.w),
        format!("\"h\":{}", region.h),
    ];

    if let Some(semantic) = &region.semantic {
        fields.push(format!("\"semantic\":\"{}\"", json_escape(semantic)));
    }
    if let Some(behavior) = &region.behavior {
        fields.push(format!("\"behavior\":\"{}\"", json_escape(behavior)));
    }
    if let Some(material) = &region.material {
        fields.push(format!("\"material\":\"{}\"", json_escape(material)));
    }
    if let Some(energy) = region.energy {
        fields.push(format!("\"energy\":{}", energy));
    }
    if let Some(properties) = properties {
        fields.push(format!("\"properties\":{}", value_map_json(properties)));
    }

    format!("{{{}}}", fields.join(","))
}

fn string_array_json(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn value_map_json(values: &HashMap<String, Value>) -> String {
    let mut items: Vec<(&String, &Value)> = values.iter().collect();
    items.sort_by(|left, right| left.0.cmp(right.0));
    format!(
        "{{{}}}",
        items
            .into_iter()
            .map(|(key, value)| format!("\"{}\":{}", json_escape(key), value_json(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn value_json(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => format!("\"{}\"", json_escape(value)),
        Value::Unit => "null".to_string(),
        Value::Function(name) => format!("\"<fn {}>\"", json_escape(name)),
        Value::List(values) => format!(
            "[{}]",
            values.iter().map(value_json).collect::<Vec<_>>().join(",")
        ),
        Value::Map(values) => value_map_json(values),
        Value::Struct { type_name, fields } => {
            let mut values = fields.clone();
            values.insert("__type".to_string(), Value::String(type_name.clone()));
            value_map_json(&values)
        }
    }
}

fn render_pxl_preview(backend: &TraceVisualBackend) -> String {
    let mut surfaces: Vec<&VisualSurfaceSpec> = backend.surfaces.values().collect();
    surfaces.sort_by(|left, right| left.name.cmp(&right.name));

    let mut content = String::new();
    for surface in surfaces {
        let scale = preview_scale(surface.width, surface.height);
        let width = (surface.width as f64 * scale).round() as i64;
        let height = (surface.height as f64 * scale).round() as i64;
        content.push_str(&format!(
            "<section class=\"surface\"><header>{} <span>{}x{}</span></header><div class=\"canvas\" style=\"width:{}px;height:{}px\">",
            html_escape(&surface.name),
            surface.width,
            surface.height,
            width.max(1),
            height.max(1)
        ));

        let mut regions: Vec<&VisualRegionSpec> = backend.regions.values().collect();
        regions.sort_by(|left, right| left.name.cmp(&right.name));
        for region in regions {
            let properties = backend.properties.get(&region.name);
            let label = region_label(region, properties);
            let is_pulsing = backend.pulses.iter().any(|target| target == &region.name);
            let class_name = if is_pulsing { "region pulse" } else { "region" };
            content.push_str(&format!(
                "<div class=\"{}\" style=\"left:{}px;top:{}px;width:{}px;height:{}px\"><strong>{}</strong><small>{}</small></div>",
                class_name,
                (region.x as f64 * scale).round() as i64,
                (region.y as f64 * scale).round() as i64,
                (region.w as f64 * scale).round().max(1.0) as i64,
                (region.h as f64 * scale).round().max(1.0) as i64,
                html_escape(&region.name),
                html_escape(&label)
            ));
        }

        content.push_str("</div></section>");
    }

    if content.is_empty() {
        content.push_str("<p class=\"empty\">No PXL surfaces recorded.</p>");
    }

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>PXL Preview</title><style>body{{margin:0;font-family:Arial,sans-serif;background:#eef2f7;color:#111827}}main{{padding:24px;display:grid;gap:20px}}.surface{{background:white;border:1px solid #d8e1ee;border-radius:8px;overflow:hidden;box-shadow:0 8px 24px rgba(15,23,42,.08)}}header{{display:flex;justify-content:space-between;padding:12px 14px;background:#111827;color:white;font-weight:700}}header span{{font-weight:400;color:#cbd5e1}}.canvas{{position:relative;margin:18px;background:#f8fafc;border:1px solid #cbd5e1;overflow:hidden}}.region{{position:absolute;box-sizing:border-box;border:2px solid #2563eb;background:rgba(37,99,235,.14);border-radius:6px;padding:6px;color:#0f172a;display:flex;flex-direction:column;gap:2px;overflow:hidden}}.region small{{font-size:11px;color:#334155}}.pulse{{border-color:#dc2626;background:rgba(220,38,38,.14)}}.empty{{padding:24px;background:white;border-radius:8px}}</style></head><body><main>{}</main></body></html>",
        content
    )
}

fn preview_scale(width: i64, height: i64) -> f64 {
    let max_width = 720.0;
    let max_height = 520.0;
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    (max_width / width).min(max_height / height).min(1.0)
}

fn region_label(region: &VisualRegionSpec, properties: Option<&HashMap<String, Value>>) -> String {
    if let Some(properties) = properties {
        if let Some(Value::String(semantic)) = properties.get("semantic") {
            return semantic.clone();
        }
        if let Some(Value::String(material)) = properties.get("material") {
            return material.clone();
        }
    }
    region
        .semantic
        .clone()
        .or_else(|| region.behavior.clone())
        .or_else(|| region.material.clone())
        .unwrap_or_else(|| "region".to_string())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Placeholder para PvmVisualBackend futuro
/// Este será implementado quando integrarmos o PVM real
pub struct PvmVisualBackend {
    // Aqui virão as estruturas do PVM real
    // pvm_runtime: PvmRuntime,
    // pvmbc_loader: PvmbcLoader,
    // etc.
}

impl PvmVisualBackend {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }
}

// Quando o PVM estiver pronto, implementaremos:
// impl VisualRuntime for PvmVisualBackend { ... }
// impl Backend for PvmVisualBackend { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_visual_run() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call("run", vec![Value::String("pizzaria".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_surface() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call(
            "surface",
            vec![
                Value::String("main".to_string()),
                Value::Int(1080),
                Value::Int(1920),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_region_simple() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call(
            "region",
            vec![
                Value::String("checkout".to_string()),
                Value::Int(100),
                Value::Int(200),
                Value::Int(300),
                Value::Int(80),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_pulse() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call("pulse", vec![Value::String("checkout".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_set() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call(
            "set",
            vec![
                Value::String("checkout".to_string()),
                Value::String("energy".to_string()),
                Value::Int(80),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_visual_region_with_map() {
        let mut backend = TraceVisualBackend::new();
        let mut props = HashMap::new();
        props.insert("x".to_string(), Value::Int(100));
        props.insert("y".to_string(), Value::Int(200));
        props.insert("w".to_string(), Value::Int(300));
        props.insert("h".to_string(), Value::Int(80));
        props.insert("semantic".to_string(), Value::String("action_button".to_string()));
        props.insert("behavior".to_string(), Value::String("pulse".to_string()));
        props.insert("energy".to_string(), Value::Int(1));

        let result = backend.call(
            "region",
            vec![Value::String("checkout".to_string()), Value::Map(props)],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_visual_snapshot_exports_pxl_document() {
        let mut backend = TraceVisualBackend::new();
        backend
            .call(
                "surface",
                vec![
                    Value::String("main".to_string()),
                    Value::Int(800),
                    Value::Int(600),
                ],
            )
            .unwrap();
        backend
            .call(
                "region",
                vec![
                    Value::String("button".to_string()),
                    Value::Int(10),
                    Value::Int(20),
                    Value::Int(120),
                    Value::Int(40),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::String("button".to_string()),
                    Value::String("material".to_string()),
                    Value::String("glass".to_string()),
                ],
            )
            .unwrap();
        backend
            .call("pulse", vec![Value::String("button".to_string())])
            .unwrap();

        let snapshot = backend.call("snapshot", vec![]).unwrap().as_string().unwrap();
        assert!(snapshot.contains("\"format\":\"PXL\""));
        assert!(snapshot.contains("\"surfaces\""));
        assert!(snapshot.contains("\"regions\""));
        assert!(snapshot.contains("\"material\":\"glass\""));
        assert!(snapshot.contains("\"target\":\"button\""));
    }

    #[test]
    fn test_visual_export_writes_pxl_document() {
        let mut backend = TraceVisualBackend::new();
        let path = std::env::temp_dir().join("matter_visual_export_test.pxl.json");
        backend
            .call(
                "surface",
                vec![
                    Value::String("main".to_string()),
                    Value::Int(320),
                    Value::Int(240),
                ],
            )
            .unwrap();

        let result = backend
            .call("export", vec![Value::String(path.display().to_string())])
            .unwrap();

        assert_eq!(result, Value::String(path.display().to_string()));
        let exported = fs::read_to_string(&path).unwrap();
        assert!(exported.contains("\"format\":\"PXL\""));
        assert!(exported.contains("\"main\""));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_preview_writes_html() {
        let mut backend = TraceVisualBackend::new();
        let path = std::env::temp_dir().join("matter_visual_preview_test.html");
        backend
            .call(
                "surface",
                vec![
                    Value::String("main".to_string()),
                    Value::Int(800),
                    Value::Int(600),
                ],
            )
            .unwrap();
        backend
            .call(
                "region",
                vec![
                    Value::String("checkout".to_string()),
                    Value::Int(120),
                    Value::Int(220),
                    Value::Int(260),
                    Value::Int(80),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("semantic".to_string()),
                    Value::String("primary_action".to_string()),
                ],
            )
            .unwrap();

        let result = backend
            .call("preview", vec![Value::String(path.display().to_string())])
            .unwrap();

        assert_eq!(result, Value::String(path.display().to_string()));
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("PXL Preview"));
        assert!(html.contains("checkout"));
        assert!(html.contains("primary_action"));
        let _ = fs::remove_file(path);
    }
}
