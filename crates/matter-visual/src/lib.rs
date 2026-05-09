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

#[derive(Debug, Clone)]
pub struct VisualCameraSpec {
    pub x: i64,
    pub y: i64,
    pub zoom: i64,
}

#[derive(Debug, Clone)]
pub struct VisualInputBinding {
    pub key: String,
    pub target: String,
    pub event: String,
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
    camera: Option<VisualCameraSpec>,
    inputs: Vec<VisualInputBinding>,
    theme: HashMap<String, String>,
    scenes: Vec<String>,
    current_scene: Option<String>,
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
            camera: None,
            inputs: Vec::new(),
            theme: HashMap::new(),
            scenes: Vec::new(),
            current_scene: None,
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

    pub fn export_canvas(&self, path: &str) -> Result<(), VisualError> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
            }
        }

        fs::write(path, render_pxl_canvas(self))
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
        let region_name = region.name.clone();
        self.regions.insert(region_name.clone(), region);
        if let Some(scene) = &self.current_scene {
            self.properties
                .entry(region_name)
                .or_default()
                .entry("scene".to_string())
                .or_insert_with(|| Value::String(scene.clone()));
        }
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
            "scene" => {
                if args.len() != 1 {
                    return Err(format!("visual.scene expects 1 argument, got {}", args.len()));
                }
                let name = args[0]
                    .as_string()
                    .map_err(|_| "visual.scene expects string name".to_string())?;
                if !self.scenes.iter().any(|scene| scene == &name) {
                    self.scenes.push(name.clone());
                }
                self.current_scene = Some(name);
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
            "state" => {
                if args.len() != 2 {
                    return Err(format!("visual.state expects 2 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.state expects string target".to_string())?;
                let state = args[1]
                    .as_string()
                    .map_err(|_| "visual.state expects string state".to_string())?;
                self.set_property(&target, "state", Value::String(state))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "layer" => {
                if args.len() != 2 {
                    return Err(format!("visual.layer expects 2 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.layer expects string target".to_string())?;
                let layer = args[1]
                    .as_int()
                    .map_err(|_| "visual.layer expects int layer".to_string())?;
                self.set_property(&target, "layer", Value::Int(layer))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "camera" => {
                if args.len() != 3 {
                    return Err(format!("visual.camera expects 3 arguments, got {}", args.len()));
                }
                let x = args[0]
                    .as_int()
                    .map_err(|_| "visual.camera expects int x".to_string())?;
                let y = args[1]
                    .as_int()
                    .map_err(|_| "visual.camera expects int y".to_string())?;
                let zoom = args[2]
                    .as_int()
                    .map_err(|_| "visual.camera expects int zoom".to_string())?;
                if zoom <= 0 {
                    return Err("visual.camera zoom must be greater than 0".to_string());
                }
                self.camera = Some(VisualCameraSpec { x, y, zoom });
                Ok(Value::Unit)
            }
            "input" => {
                if args.len() != 3 {
                    return Err(format!("visual.input expects 3 arguments, got {}", args.len()));
                }
                let key = args[0]
                    .as_string()
                    .map_err(|_| "visual.input expects string key".to_string())?;
                let target = args[1]
                    .as_string()
                    .map_err(|_| "visual.input expects string target".to_string())?;
                let event = args[2]
                    .as_string()
                    .map_err(|_| "visual.input expects string event".to_string())?;
                self.inputs.push(VisualInputBinding { key, target, event });
                Ok(Value::Unit)
            }
            "motion" => {
                if args.len() != 3 {
                    return Err(format!("visual.motion expects 3 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.motion expects string target".to_string())?;
                let kind = args[1]
                    .as_string()
                    .map_err(|_| "visual.motion expects string kind".to_string())?;
                let speed = args[2]
                    .as_int()
                    .map_err(|_| "visual.motion expects int speed".to_string())?;
                if speed <= 0 {
                    return Err("visual.motion speed must be greater than 0".to_string());
                }
                self.set_property(&target, "motion", Value::String(kind))
                    .map_err(|e| e.to_string())?;
                self.set_property(&target, "motionSpeed", Value::Int(speed))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "sprite" => {
                if args.len() != 3 {
                    return Err(format!("visual.sprite expects 3 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.sprite expects string target".to_string())?;
                let source = args[1]
                    .as_string()
                    .map_err(|_| "visual.sprite expects string source".to_string())?;
                let fit = args[2]
                    .as_string()
                    .map_err(|_| "visual.sprite expects string fit".to_string())?;
                self.set_property(&target, "sprite", Value::String(source))
                    .map_err(|e| e.to_string())?;
                self.set_property(&target, "spriteFit", Value::String(fit))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "text" => {
                if args.len() != 2 {
                    return Err(format!("visual.text expects 2 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.text expects string target".to_string())?;
                let text = args[1]
                    .as_string()
                    .map_err(|_| "visual.text expects string text".to_string())?;
                self.set_property(&target, "text", Value::String(text))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "visible" => {
                if args.len() != 2 {
                    return Err(format!("visual.visible expects 2 arguments, got {}", args.len()));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.visible expects string target".to_string())?;
                let visible = args[1]
                    .as_bool()
                    .map_err(|_| "visual.visible expects bool visible".to_string())?;
                self.set_property(&target, "visible", Value::Bool(visible))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "theme" => {
                if args.len() != 2 {
                    return Err(format!("visual.theme expects 2 arguments, got {}", args.len()));
                }
                let key = args[0]
                    .as_string()
                    .map_err(|_| "visual.theme expects string key".to_string())?;
                let value = args[1]
                    .as_string()
                    .map_err(|_| "visual.theme expects string value".to_string())?;
                self.theme.insert(key, value);
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
            "canvas" => {
                if args.len() != 1 {
                    return Err(format!("visual.canvas expects 1 argument, got {}", args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.canvas expects string path".to_string())?;
                self.export_canvas(&path).map_err(|e| e.to_string())?;
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
    let input_json = backend
        .inputs
        .iter()
        .map(render_input)
        .collect::<Vec<_>>()
        .join(",");
    let theme_json = string_map_json(&backend.theme);
    let scenes = if backend.scenes.is_empty() {
        vec!["main".to_string()]
    } else {
        backend.scenes.clone()
    };
    let scene_json = string_array_json(&scenes);
    let active_scene = backend
        .current_scene
        .clone()
        .unwrap_or_else(|| scenes.first().cloned().unwrap_or_else(|| "main".to_string()));

    let camera_json = backend
        .camera
        .as_ref()
        .map(render_camera)
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"format\":\"PXL\",\"version\":1,\"surfaces\":[{}],\"regions\":[{}],\"pulses\":[{}],\"camera\":{},\"inputs\":[{}],\"theme\":{},\"scenes\":{},\"activeScene\":\"{}\",\"loaded\":{},\"apps\":{}}}",
        surface_json,
        region_json,
        pulse_json,
        camera_json,
        input_json,
        theme_json,
        scene_json,
        json_escape(&active_scene),
        loaded_json,
        app_json
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

fn render_camera(camera: &VisualCameraSpec) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"zoom\":{}}}",
        camera.x, camera.y, camera.zoom
    )
}

fn render_input(input: &VisualInputBinding) -> String {
    format!(
        "{{\"key\":\"{}\",\"target\":\"{}\",\"event\":\"{}\"}}",
        json_escape(&input.key),
        json_escape(&input.target),
        json_escape(&input.event)
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

fn string_map_json(values: &HashMap<String, String>) -> String {
    let mut items: Vec<(&String, &String)> = values.iter().collect();
    items.sort_by(|left, right| left.0.cmp(right.0));
    format!(
        "{{{}}}",
        items
            .into_iter()
            .map(|(key, value)| format!("\"{}\":\"{}\"", json_escape(key), json_escape(value)))
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

fn render_pxl_canvas(backend: &TraceVisualBackend) -> String {
    let pxl = render_pxl_document(backend);
    // Use string concatenation to avoid format! escaping issues
    let html_template = r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>PXL Canvas Engine</title><style>body{margin:0;font-family:Arial,sans-serif;background:#f3f6fb;color:#111827}main{min-height:100vh;display:grid;grid-template-columns:minmax(0,1fr) 280px;gap:18px;padding:20px;box-sizing:border-box}.stage{display:grid;place-items:center;background:white;border:1px solid #d8e1ee;border-radius:8px;min-height:calc(100vh - 40px);overflow:auto}canvas{background:#f8fafc;border:1px solid #cbd5e1;box-shadow:0 16px 38px rgba(15,23,42,.12);max-width:100%;height:auto}aside{background:#111827;color:#e2e8f0;border-radius:8px;padding:14px;display:flex;flex-direction:column;gap:12px;min-height:0}h1{font-size:16px;margin:0;color:white}.meta,.bindings{font-size:12px;color:#cbd5e1;line-height:1.45}.bindings div{display:flex;justify-content:space-between;gap:8px;border-top:1px solid rgba(226,232,240,.16);padding-top:6px;margin-top:6px}.bindings button{border:1px solid #93c5fd;background:#e0f2fe;color:#0f172a;border-radius:5px;padding:3px 7px;cursor:pointer}kbd{background:#e2e8f0;color:#111827;border-radius:4px;padding:1px 5px;font-size:11px}button.trace{border:1px solid #93c5fd;background:#dbeafe;color:#0f172a;border-radius:6px;padding:8px 10px;font-weight:700;cursor:pointer}.event-log{display:flex;flex-direction:column;gap:6px;overflow:auto;font-size:12px}.event-log div{border-top:1px solid rgba(226,232,240,.16);padding-top:6px}@media (max-width: 760px){main{grid-template-columns:1fr}.stage{min-height:60vh}}</style></head><body><main><section class="stage"><canvas id="pxl-canvas" width="720" height="520"></canvas></section><aside><h1>PXL Canvas Engine</h1><div class="meta" id="pxl-meta">Loading PXL scene</div><section class="bindings" id="scene-list"></section><section class="bindings" id="input-bindings"></section><button class="trace" id="export-trace" type="button">Export trace</button><section class="event-log" id="event-log"><div>Click a PXL region</div></section></aside></main><script>const pxl="#;
    let html_end = r#";const theme=pxl.theme||{};const events=[];const canvas=document.getElementById('pxl-canvas');const ctx=canvas.getContext('2d');const meta=document.getElementById('pxl-meta');const log=document.getElementById('event-log');const bindings=document.getElementById('input-bindings');const traceButton=document.getElementById('export-trace');function themeValue(key,fallback){return theme[key]||fallback;}document.body.style.background=themeValue('page','#f3f6fb');document.querySelector('aside').style.background=themeValue('panel','#111827');traceButton.style.background=themeValue('button','#dbeafe');const surface=pxl.surfaces[0]||{name:'empty',width:720,height:520};const camera=pxl.camera||{x:0,y:0,zoom:100};const pulseTargets=new Set((pxl.pulses||[]).map((pulse)=>pulse.target));let selected=null;const zoomScale=Math.max(1,camera.zoom||100)/100;let scale=Math.min(960/Math.max(1,surface.width),640/Math.max(1,surface.height),1);canvas.width=Math.max(1,Math.round(surface.width*scale));canvas.height=Math.max(1,Math.round(surface.height*scale));meta.textContent=surface.name+' '+surface.width+'x'+surface.height+' regions='+(pxl.regions||[]).length+' inputs='+(pxl.inputs||[]).length+' camera='+camera.x+','+camera.y+' zoom='+camera.zoom;bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';function prop(region,key){return region.properties&&region.properties[key]!==undefined?region.properties[key]:region[key];}const spriteImages=new Map();(pxl.regions||[]).forEach((region)=>{const source=prop(region,'sprite');if(source){const image=new Image();image.src=source;spriteImages.set(region.name,image);}});function regionVisible(region){return prop(region,'visible')!==false;}function layerValue(region){const layer=Number(prop(region,'layer')||0);return Number.isFinite(layer)?layer:0;}function sortedRegions(){return [...(pxl.regions||[])].filter(regionVisible).sort((left,right)=>layerValue(left)-layerValue(right)||left.name.localeCompare(right.name));}function eventName(region){return prop(region,'event')||prop(region,'behavior')||'tap';}function regionState(region){return prop(region,'state')||'idle';}function regionText(region){return prop(region,'text')||region.name;}function textSize(region){const size=Number(prop(region,'textSize')||13);return Number.isFinite(size)?size:13;}function fillFor(region,time){const state=regionState(region);const pulsing=pulseTargets.has(region.name);if(state==='active')return themeValue('activeFill','rgba(5,150,105,.28)');if(state==='disabled')return themeValue('disabledFill','rgba(100,116,139,.24)');if(state==='error')return themeValue('errorFill','rgba(185,28,28,.30)');if(pulsing){const alpha=.18+Math.sin(time/180)*.08;return 'rgba(220,38,38,'+alpha.toFixed(3)+')';}return themeValue('regionFill','rgba(37,99,235,.18)');}function strokeFor(region){const state=regionState(region);if(region.name===selected)return themeValue('selected','#f59e0b');if(state==='active')return themeValue('active','#059669');if(state==='disabled')return themeValue('disabled','#64748b');if(state==='error')return themeValue('error','#b91c1c');if(pulseTargets.has(region.name))return themeValue('pulse','#dc2626');return themeValue('accent','#2563eb');}function motionOffset(region,time){const kind=prop(region,'motion')||'none';const speed=Math.max(1,Number(prop(region,'motionSpeed')||1000));const phase=time/speed*Math.PI*2;if(kind==='float')return {x:0,y:Math.sin(phase)*6,s:1};if(kind==='shake')return {x:Math.sin(phase*8)*3,y:0,s:1};if(kind==='breathe')return {x:0,y:0,s:1+Math.sin(phase)*.035};return {x:0,y:0,s:1};}function drawSprite(region,x,y,w,h){const image=spriteImages.get(region.name);if(!image||!image.complete||image.naturalWidth===0)return false;const fit=prop(region,'spriteFit')||'stretch';let dx=x,dy=y,dw=w,dh=h;if(fit==='contain'){const ratio=Math.min(w/image.naturalWidth,h/image.naturalHeight);dw=image.naturalWidth*ratio;dh=image.naturalHeight*ratio;dx=x+(w-dw)/2;dy=y+(h-dh)/2;}ctx.save();ctx.clip();ctx.drawImage(image,dx,dy,dw,dh);ctx.restore();return true;}function recordEvent(entry){const event={time:new Date().toISOString(),...entry};events.push(event);const line=document.createElement('div');line.textContent=Object.entries(event).filter(([key])=>key!=='time').map(([key,value])=>key+'='+value).join(' ');log.appendChild(line);log.scrollTop=log.scrollHeight;}function downloadTrace(){const blob=new Blob([JSON.stringify({format:'PXL_TRACE',version:1,events},null,2)],{type:'application/json'});const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download='pxl-trace.json';link.click();URL.revokeObjectURL(url);}function drawRegion(region,time){if(!regionVisible(region))return;const motion=motionOffset(region,time);const x=(region.x-camera.x)*scale*zoomScale+motion.x;const y=(region.y-camera.y)*scale*zoomScale+motion.y;const w=Math.max(1,region.w*scale*zoomScale)*motion.s;const h=Math.max(1,region.h*scale*zoomScale)*motion.s;ctx.fillStyle=fillFor(region,time);ctx.strokeStyle=strokeFor(region);ctx.lineWidth=region.name===selected?4:2;ctx.beginPath();ctx.roundRect(x,y,w,h,8);ctx.fill();drawSprite(region,x,y,w,h);ctx.stroke();ctx.fillStyle=themeValue('text','#0f172a');ctx.font='700 '+textSize(region)+'px Arial';ctx.textBaseline='top';ctx.fillText(regionText(region),x+8,y+8,Math.max(10,w-16));ctx.font='11px Arial';ctx.fillStyle=themeValue('mutedText','#334155');ctx.fillText('id='+region.name+' z='+layerValue(region)+' '+regionState(region)+' / '+eventName(region),x+8,y+textSize(region)+14,Math.max(10,w-16));}function draw(time){ctx.clearRect(0,0,canvas.width,canvas.height);ctx.fillStyle=themeValue('surface','#f8fafc');ctx.fillRect(0,0,canvas.width,canvas.height);ctx.strokeStyle=themeValue('surfaceBorder','#d8e1ee');ctx.lineWidth=1;ctx.strokeRect(.5,.5,canvas.width-1,canvas.height-1);sortedRegions().forEach((region)=>drawRegion(region,time));requestAnimationFrame(draw);}function hit(clientX,clientY){const rect=canvas.getBoundingClientRect();const x=((clientX-rect.left)*(canvas.width/rect.width))/(scale*zoomScale)+camera.x;const y=((clientY-rect.top)*(canvas.height/rect.height))/(scale*zoomScale)+camera.y;return sortedRegions().reverse().find((region)=>x>=region.x&&x<=region.x+region.w&&y>=region.y&&y<=region.y+region.h);}canvas.addEventListener('click',(event)=>{const region=hit(event.clientX,event.clientY);if(!region)return;selected=region.name;recordEvent({type:'pointer',target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)});});window.addEventListener('keydown',(event)=>{const binding=(pxl.inputs||[]).find((input)=>input.key.toLowerCase()===event.key.toLowerCase());if(!binding)return;const target=(pxl.regions||[]).find((region)=>region.name===binding.target);if(target&&!regionVisible(target))return;selected=binding.target;recordEvent({type:'keyboard',key:binding.key,target:binding.target,event:binding.event});});traceButton.addEventListener('click',downloadTrace);requestAnimationFrame(draw);</script></body></html>"#;

    let mut html = format!("{}{}{}", html_template, pxl, html_end);
    html = html.replace(
        "const bindings=document.getElementById('input-bindings');const traceButton=document.getElementById('export-trace');",
        "const bindings=document.getElementById('input-bindings');const sceneList=document.getElementById('scene-list');const traceButton=document.getElementById('export-trace');",
    );
    html = html.replace(
        "let selected=null;const zoomScale",
        "let selected=null;let activeScene=pxl.activeScene||(pxl.scenes&&pxl.scenes[0])||'main';const zoomScale",
    );
    html = html.replace(
        "bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';",
        "function renderSceneList(){sceneList.innerHTML=(pxl.scenes||['main']).map((scene)=>'<div><button type=\"button\" data-scene=\"'+scene+'\">'+scene+'</button><span>'+((scene===activeScene)?'active':'')+'</span></div>').join('');sceneList.querySelectorAll('[data-scene]').forEach((node)=>node.addEventListener('click',()=>{activeScene=node.dataset.scene;selected=null;renderSceneList();recordEvent({type:'scene',scene:activeScene,event:'scene_change'});}));}renderSceneList();bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';",
    );
    html = html.replace(
        "function regionVisible(region){return prop(region,'visible')!==false;}",
        "function regionScene(region){return prop(region,'scene')||'main';}function regionVisible(region){return prop(region,'visible')!==false&&regionScene(region)===activeScene;}",
    );
    html
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
            let state = region_state(properties);
            let event = region_event(region, properties);
            let is_pulsing = backend.pulses.iter().any(|target| target == &region.name);
            let class_name = region_class(is_pulsing, state.as_deref());
            content.push_str(&format!(
                "<button class=\"{}\" type=\"button\" data-region=\"{}\" data-event=\"{}\" style=\"left:{}px;top:{}px;width:{}px;height:{}px\"><strong>{}</strong><small>{}</small></button>",
                class_name,
                html_escape(&region.name),
                html_escape(&event),
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
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>PXL Preview</title><style>body{{margin:0;font-family:Arial,sans-serif;background:#eef2f7;color:#111827}}main{{padding:24px;display:grid;gap:20px}}.surface{{background:white;border:1px solid #d8e1ee;border-radius:8px;overflow:hidden;box-shadow:0 8px 24px rgba(15,23,42,.08)}}header{{display:flex;justify-content:space-between;padding:12px 14px;background:#111827;color:white;font-weight:700}}header span{{font-weight:400;color:#cbd5e1}}.canvas{{position:relative;margin:18px;background:#f8fafc;border:1px solid #cbd5e1;overflow:hidden}}.region{{position:absolute;box-sizing:border-box;border:2px solid #2563eb;background:rgba(37,99,235,.14);border-radius:6px;padding:6px;color:#0f172a;display:flex;flex-direction:column;gap:2px;overflow:hidden;text-align:left;cursor:pointer;font:inherit}}.region small{{font-size:11px;color:#334155}}.region.selected{{outline:3px solid #f59e0b;outline-offset:2px}}.pulse{{border-color:#dc2626;background:rgba(220,38,38,.14)}}.state-active{{border-color:#059669;background:rgba(5,150,105,.16)}}.state-disabled{{border-color:#64748b;background:rgba(100,116,139,.16);opacity:.72}}.state-error{{border-color:#b91c1c;background:rgba(185,28,28,.18)}}.empty{{padding:24px;background:white;border-radius:8px}}.event-log{{background:#0f172a;color:#e2e8f0;border-radius:8px;padding:14px;font-size:13px;min-height:64px}}.event-log strong{{display:block;color:white;margin-bottom:8px}}.event-log div{{padding:3px 0;border-top:1px solid rgba(226,232,240,.14)}}</style></head><body><main>{}<section class=\"event-log\"><strong>Preview events</strong><div>Click a PXL region</div></section></main><script>document.querySelectorAll('[data-region]').forEach((node)=>node.addEventListener('click',()=>{{document.querySelectorAll('[data-region]').forEach((item)=>item.classList.remove('selected'));node.classList.add('selected');const log=document.querySelector('.event-log');const line=document.createElement('div');line.textContent='region='+node.dataset.region+' event='+node.dataset.event;log.appendChild(line);}}));</script></body></html>",
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
        if let Some(Value::String(state)) = properties.get("state") {
            return format!("state: {}", state);
        }
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

fn region_state(properties: Option<&HashMap<String, Value>>) -> Option<String> {
    properties.and_then(|properties| match properties.get("state") {
        Some(Value::String(state)) => Some(state.clone()),
        _ => None,
    })
}

fn region_event(region: &VisualRegionSpec, properties: Option<&HashMap<String, Value>>) -> String {
    if let Some(properties) = properties {
        if let Some(Value::String(event)) = properties.get("event") {
            return event.clone();
        }
        if let Some(Value::String(behavior)) = properties.get("behavior") {
            return behavior.clone();
        }
    }
    region
        .behavior
        .clone()
        .unwrap_or_else(|| "tap".to_string())
}

fn region_class(is_pulsing: bool, state: Option<&str>) -> String {
    let mut classes = vec!["region".to_string()];
    if is_pulsing {
        classes.push("pulse".to_string());
    }
    if let Some(state) = state {
        classes.push(format!("state-{}", css_class_token(state)));
    }
    classes.join(" ")
}

fn css_class_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
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
        assert!(html.contains("data-region=\"checkout\""));
        assert!(html.contains("Preview events"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_state_is_exported_and_previewed() {
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
                "state",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("active".to_string()),
                ],
            )
            .unwrap();

        let snapshot = backend.call("snapshot", vec![]).unwrap().as_string().unwrap();
        assert!(snapshot.contains("\"state\":\"active\""));

        let path = std::env::temp_dir().join("matter_visual_state_preview_test.html");
        backend
            .call("preview", vec![Value::String(path.display().to_string())])
            .unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("state-active"));
        assert!(html.contains("state: active"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_canvas_writes_engine_html() {
        let mut backend = TraceVisualBackend::new();
        let path = std::env::temp_dir().join("matter_visual_canvas_test.html");
        backend
            .call(
                "surface",
                vec![
                    Value::String("main".to_string()),
                    Value::Int(960),
                    Value::Int(540),
                ],
            )
            .unwrap();
        backend
            .call("scene", vec![Value::String("home".to_string())])
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
                    Value::String("event".to_string()),
                    Value::String("checkout_tap".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "state",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("active".to_string()),
                ],
            )
            .unwrap();
        backend
            .call("pulse", vec![Value::String("checkout".to_string())])
            .unwrap();
        backend
            .call(
                "layer",
                vec![Value::String("checkout".to_string()), Value::Int(4)],
            )
            .unwrap();
        backend
            .call("camera", vec![Value::Int(20), Value::Int(40), Value::Int(125)])
            .unwrap();
        backend
            .call(
                "input",
                vec![
                    Value::String("Enter".to_string()),
                    Value::String("checkout".to_string()),
                    Value::String("checkout_submit".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "theme",
                vec![
                    Value::String("accent".to_string()),
                    Value::String("#0f766e".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "motion",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("breathe".to_string()),
                    Value::Int(1200),
                ],
            )
            .unwrap();
        backend
            .call(
                "sprite",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("assets/checkout.png".to_string()),
                    Value::String("contain".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "text",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("Buy now".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::String("checkout".to_string()),
                    Value::String("textSize".to_string()),
                    Value::Int(16),
                ],
            )
            .unwrap();
        backend
            .call(
                "visible",
                vec![Value::String("checkout".to_string()), Value::Bool(false)],
            )
            .unwrap();

        let result = backend
            .call("canvas", vec![Value::String(path.display().to_string())])
            .unwrap();

        assert_eq!(result, Value::String(path.display().to_string()));
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("PXL Canvas Engine"));
        assert!(html.contains("<canvas id=\"pxl-canvas\""));
        assert!(html.contains("requestAnimationFrame"));
        assert!(html.contains("checkout_tap"));
        assert!(html.contains("\"camera\":{\"x\":20,\"y\":40,\"zoom\":125}"));
        assert!(html.contains("\"layer\":4"));
        assert!(html.contains("\"scenes\":[\"home\"]"));
        assert!(html.contains("\"activeScene\":\"home\""));
        assert!(html.contains("\"scene\":\"home\""));
        assert!(html.contains("scene-list"));
        assert!(html.contains("activeScene"));
        assert!(html.contains("regionScene"));
        assert!(html.contains("\"inputs\":[{\"key\":\"Enter\",\"target\":\"checkout\",\"event\":\"checkout_submit\"}]"));
        assert!(html.contains("\"theme\":{\"accent\":\"#0f766e\"}"));
        assert!(html.contains("themeValue"));
        assert!(html.contains("\"motion\":\"breathe\""));
        assert!(html.contains("\"motionSpeed\":1200"));
        assert!(html.contains("\"sprite\":\"assets/checkout.png\""));
        assert!(html.contains("\"spriteFit\":\"contain\""));
        assert!(html.contains("\"text\":\"Buy now\""));
        assert!(html.contains("\"textSize\":16"));
        assert!(html.contains("\"visible\":false"));
        assert!(html.contains("regionVisible"));
        assert!(html.contains("filter(regionVisible)"));
        assert!(html.contains("regionText"));
        assert!(html.contains("textSize"));
        assert!(html.contains("spriteImages"));
        assert!(html.contains("drawSprite"));
        assert!(html.contains("motionOffset"));
        assert!(html.contains("input-bindings"));
        assert!(html.contains("keydown"));
        assert!(html.contains("PXL_TRACE"));
        assert!(html.contains("export-trace"));
        assert!(html.contains("recordEvent"));
        assert!(html.contains("downloadTrace"));
        assert!(html.contains("sortedRegions"));
        assert!(html.contains("zoomScale"));
        assert!(html.contains("\"state\":\"active\""));
        let _ = fs::remove_file(path);
    }
}
