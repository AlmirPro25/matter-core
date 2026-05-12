//! Backend contracts for Matter
//! Define interfaces para backends (visual, agent, terminal, etc)

use matter_agent_protocol::{
    AgentFrame, AgentHandoffPacket, AgentId, AgentRole, AgentSession, MergeStrategy, TaskState,
};
use matter_memory::Rc;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

/// Value type with reference-counted heap allocations
///
/// Stack values (Int, Bool, Unit) are stored directly for performance.
/// Heap values (String, List, Map, Function, Struct) use Rc for:
/// - Cheap cloning (O(1) atomic increment)
/// - Shared ownership
/// - Automatic deallocation
/// - Cycle detection ready
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Stack values - cheap to copy
    Int(i64),
    Float(f64),
    Bool(bool),
    Unit,

    // Heap values - use Rc for shared ownership
    String(Rc<String>),
    Function(Rc<String>),
    List(Rc<Vec<Value>>),
    Map(Rc<HashMap<String, Value>>),
    Struct {
        type_name: Rc<String>,
        fields: Rc<HashMap<String, Value>>,
    },
}

impl Value {
    /// Create a new String value
    pub fn new_string(s: String) -> Self {
        Value::String(Rc::new(s))
    }

    /// Create a new Function value
    pub fn new_function(name: String) -> Self {
        Value::Function(Rc::new(name))
    }

    /// Create a new List value
    pub fn new_list(elements: Vec<Value>) -> Self {
        Value::List(Rc::new(elements))
    }

    /// Create a new Map value
    pub fn new_map(entries: HashMap<String, Value>) -> Self {
        Value::Map(Rc::new(entries))
    }

    /// Create a new Struct value
    pub fn new_struct(type_name: String, fields: HashMap<String, Value>) -> Self {
        Value::Struct {
            type_name: Rc::new(type_name),
            fields: Rc::new(fields),
        }
    }

    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(n) => Ok(*n),
            Value::Float(f) => Ok(*f as i64),
            _ => Err(type_error("int", self)),
        }
    }

    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(n) => Ok(*n as f64),
            _ => Err(type_error("float", self)),
        }
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(type_error("bool", self)),
        }
    }

    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok((**s).clone()),
            _ => Err(type_error("string", self)),
        }
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                let s = f.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::String(s) => (**s).clone(),
            Value::Unit => "()".to_string(),
            Value::Function(name) => format!("<fn {}>", **name),
            Value::List(elements) => {
                let items: Vec<String> = elements.iter().map(|v| v.to_display_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Map(entries) => {
                let mut items: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value.to_display_string()))
                    .collect();
                items.sort();
                format!("{{{}}}", items.join(", "))
            }
            Value::Struct { type_name, fields } => {
                let mut items: Vec<String> = fields
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value.to_display_string()))
                    .collect();
                items.sort();
                format!("{} {{{}}}", **type_name, items.join(", "))
            }
        }
    }
}

pub fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::Bool(_) => "bool",
        Value::Unit => "unit",
        Value::String(_) => "string",
        Value::Function(_) => "function",
        Value::List(_) => "list",
        Value::Map(_) => "map",
        Value::Struct { .. } => "struct",
    }
}

pub fn type_error(expected: &str, value: &Value) -> String {
    format!(
        "Type check failed [context:expected={},actual={}]",
        expected,
        value_type_name(value)
    )
}

pub trait Backend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String>;
}

fn backend_unknown_method_error(backend: &str, method: &str) -> String {
    format!(
        "Backend call failed [context:backend={},method={}]: unknown method",
        backend, method
    )
}

fn backend_arity_error(op: &str, expected: usize, got: usize) -> String {
    format!(
        "Backend call failed [context:op={},expected_args={},actual_args={}]: invalid arity",
        op, expected, got
    )
}

/// Mock backend para trace/debug
pub struct TraceBackend {
    name: String,
}

impl TraceBackend {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Backend for TraceBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        print!("[BACKEND:{}::{}](", self.name, method);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{}", arg.to_display_string());
        }
        println!(")");

        Ok(Value::Unit)
    }
}

/// Agent backend (mock por enquanto)
pub struct AgentBackend;

impl AgentBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for AgentBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "say" => {
                if let Some(arg) = args.first() {
                    println!("[AGENT] {}", arg.to_display_string());
                }
                Ok(Value::Unit)
            }
            _ => Err(backend_unknown_method_error("agent", method)),
        }
    }
}

/// Visual backend (mock por enquanto)
pub struct VisualBackend;

impl VisualBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VisualBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for VisualBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "run" => {
                if let Some(arg) = args.first() {
                    println!("[VISUAL] Running app: {}", arg.to_display_string());
                }
                Ok(Value::Unit)
            }
            _ => Err(backend_unknown_method_error("visual", method)),
        }
    }
}

/// SVG chart backend for reports, dashboards, and API-generated visual output.
pub struct GraphBackend;

impl GraphBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GraphBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for GraphBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "bar" => {
                let (title, labels, values) = chart_args("graph.bar", args)?;
                Ok(Value::new_string(render_bar_chart(
                    &title, &labels, &values,
                )))
            }
            "line" => {
                let (title, labels, values) = chart_args("graph.line", args)?;
                Ok(Value::new_string(render_line_chart(
                    &title, &labels, &values,
                )))
            }
            "pie" => {
                let (title, labels, values) = chart_args("graph.pie", args)?;
                Ok(Value::new_string(render_pie_chart(
                    &title, &labels, &values,
                )))
            }
            "stats" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("graph.stats", 1, args.len()));
                }
                let values = value_list_ints("graph.stats values", &args[0])?;
                Ok(chart_stats(&values))
            }
            "table" => {
                let (title, labels, values) = chart_args("graph.table", args)?;
                Ok(Value::new_string(render_table(&title, &labels, &values)))
            }
            "save" => {
                if args.len() != 5 {
                    return Err(backend_arity_error("graph.save", 5, args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("graph.save: {}", e))?;
                let kind = args[1]
                    .as_string()
                    .map_err(|e| format!("graph.save: {}", e))?;
                let title = args[2]
                    .as_string()
                    .map_err(|e| format!("graph.save: {}", e))?;
                let labels = value_list_strings("graph.save labels", &args[3])?;
                let values = value_list_ints("graph.save values", &args[4])?;
                validate_chart_data("graph.save", &labels, &values)?;

                let svg = match kind.as_str() {
                    "bar" => render_bar_chart(&title, &labels, &values),
                    "line" => render_line_chart(&title, &labels, &values),
                    "pie" => render_pie_chart(&title, &labels, &values),
                    _ => return Err(format!("graph.save: unknown chart kind '{}'", kind)),
                };

                if let Some(parent) = std::path::Path::new(&path).parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("graph.save could not create directory: {}", e))?;
                    }
                }
                fs::write(&path, svg)
                    .map_err(|e| format!("graph.save could not write '{}': {}", path, e))?;
                Ok(Value::new_string(path))
            }
            "dashboard" => {
                if args.len() != 3 {
                    return Err(backend_arity_error("graph.dashboard", 3, args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("graph.dashboard: {}", e))?;
                let title = args[1]
                    .as_string()
                    .map_err(|e| format!("graph.dashboard: {}", e))?;
                let charts = dashboard_charts(&args[2])?;
                let html = render_dashboard(&title, &charts);

                if let Some(parent) = std::path::Path::new(&path).parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent).map_err(|e| {
                            format!("graph.dashboard could not create directory: {}", e)
                        })?;
                    }
                }
                fs::write(&path, html)
                    .map_err(|e| format!("graph.dashboard could not write '{}': {}", path, e))?;
                Ok(Value::new_string(path))
            }
            _ => Err(backend_unknown_method_error("graph", method)),
        }
    }
}

struct ChartSpec {
    kind: String,
    title: String,
    labels: Vec<String>,
    values: Vec<i64>,
}

fn chart_args(name: &str, args: Vec<Value>) -> Result<(String, Vec<String>, Vec<i64>), String> {
    if args.len() != 3 {
        return Err(backend_arity_error(name, 3, args.len()));
    }
    let title = args[0]
        .as_string()
        .map_err(|e| format!("{}: {}", name, e))?;
    let labels = value_list_strings(&format!("{} labels", name), &args[1])?;
    let values = value_list_ints(&format!("{} values", name), &args[2])?;
    validate_chart_data(name, &labels, &values)?;
    Ok((title, labels, values))
}

fn dashboard_charts(value: &Value) -> Result<Vec<ChartSpec>, String> {
    let items = match value {
        Value::List(items) => items,
        _ => return Err("graph.dashboard charts must be a list".to_string()),
    };
    if items.is_empty() {
        return Err("graph.dashboard requires at least one chart".to_string());
    }

    let mut charts = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let map = match item {
            Value::Map(map) => map,
            _ => return Err(format!("graph.dashboard chart {} must be a map", index)),
        };
        let kind = required_string(map, "kind", index)?;
        let title = required_string(map, "title", index)?;
        let labels_value = map
            .get("labels")
            .ok_or_else(|| format!("graph.dashboard chart {} missing labels", index))?;
        let values_value = map
            .get("values")
            .ok_or_else(|| format!("graph.dashboard chart {} missing values", index))?;
        let labels = value_list_strings("graph.dashboard labels", labels_value)?;
        let values = value_list_ints("graph.dashboard values", values_value)?;
        validate_chart_data("graph.dashboard", &labels, &values)?;
        if !matches!(kind.as_str(), "bar" | "line" | "pie") {
            return Err(format!(
                "graph.dashboard chart {} has unknown kind '{}'",
                index, kind
            ));
        }
        charts.push(ChartSpec {
            kind,
            title,
            labels,
            values,
        });
    }
    Ok(charts)
}

fn required_string(
    map: &HashMap<String, Value>,
    key: &str,
    index: usize,
) -> Result<String, String> {
    map.get(key)
        .ok_or_else(|| format!("graph.dashboard chart {} missing {}", index, key))?
        .as_string()
        .map_err(|e| format!("graph.dashboard chart {} {}: {}", index, key, e))
}

fn value_list_strings(context: &str, value: &Value) -> Result<Vec<String>, String> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|item| item.as_string().map_err(|e| format!("{}: {}", context, e)))
            .collect(),
        _ => Err(format!("{} must be a list", context)),
    }
}

fn value_list_ints(context: &str, value: &Value) -> Result<Vec<i64>, String> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|item| item.as_int().map_err(|e| format!("{}: {}", context, e)))
            .collect(),
        _ => Err(format!("{} must be a list", context)),
    }
}

fn validate_chart_data(name: &str, labels: &[String], values: &[i64]) -> Result<(), String> {
    if labels.is_empty() {
        return Err(format!("{} requires at least one data point", name));
    }
    if labels.len() != values.len() {
        return Err(format!(
            "{} requires labels and values with the same length",
            name
        ));
    }
    Ok(())
}

fn chart_stats(values: &[i64]) -> Value {
    let mut stats = HashMap::new();
    stats.insert("count".to_string(), Value::Int(values.len() as i64));

    if values.is_empty() {
        stats.insert("total".to_string(), Value::Int(0));
        stats.insert("min".to_string(), Value::Unit);
        stats.insert("max".to_string(), Value::Unit);
        stats.insert("average".to_string(), Value::Unit);
        return Value::new_map(stats);
    }

    let total: i64 = values.iter().sum();
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    stats.insert("total".to_string(), Value::Int(total));
    stats.insert("min".to_string(), Value::Int(min));
    stats.insert("max".to_string(), Value::Int(max));
    stats.insert(
        "average".to_string(),
        Value::Int(total / values.len() as i64),
    );
    Value::new_map(stats)
}

fn render_bar_chart(title: &str, labels: &[String], values: &[i64]) -> String {
    let width = 720i64;
    let height = 420i64;
    let left = 64i64;
    let top = 64i64;
    let chart_w = 600i64;
    let chart_h = 280i64;
    let max = values
        .iter()
        .map(|value| value.abs())
        .max()
        .unwrap_or(1)
        .max(1);
    let step = (chart_w / labels.len() as i64).max(28);
    let bar_w = (step - 12).max(12);

    let mut body = String::new();
    for (index, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        let magnitude = value.abs();
        let bar_h = (magnitude * chart_h / max).max(if magnitude == 0 { 0 } else { 2 });
        let x = left + index as i64 * step + 6;
        let y = top + chart_h - bar_h;
        body.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#2563eb\" rx=\"3\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#111827\">{}</text><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#4b5563\">{}</text>",
            x,
            y,
            bar_w,
            bar_h,
            x + bar_w / 2,
            y - 8,
            value,
            x + bar_w / 2,
            top + chart_h + 24,
            escape_svg(label)
        ));
    }

    chart_shell(width, height, title, &body)
}

fn render_line_chart(title: &str, labels: &[String], values: &[i64]) -> String {
    let width = 720i64;
    let height = 420i64;
    let left = 64i64;
    let top = 64i64;
    let chart_w = 600i64;
    let chart_h = 280i64;
    let max = values.iter().copied().max().unwrap_or(1).max(1);
    let min = values.iter().copied().min().unwrap_or(0).min(0);
    let range = (max - min).max(1);
    let denom = (values.len() as i64 - 1).max(1);

    let mut points = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let x = left + index as i64 * chart_w / denom;
        let y = top + chart_h - ((*value - min) * chart_h / range);
        points.push((x, y, value));
    }

    let path = points
        .iter()
        .enumerate()
        .map(|(index, (x, y, _))| {
            if index == 0 {
                format!("M {} {}", x, y)
            } else {
                format!("L {} {}", x, y)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut body = format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"#059669\" stroke-width=\"4\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
        path
    );
    for (index, (x, y, value)) in points.iter().enumerate() {
        body.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#059669\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#111827\">{}</text><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#4b5563\">{}</text>",
            x,
            y,
            x,
            y - 12,
            value,
            x,
            top + chart_h + 24,
            escape_svg(&labels[index])
        ));
    }

    chart_shell(width, height, title, &body)
}

fn render_pie_chart(title: &str, labels: &[String], values: &[i64]) -> String {
    let width = 720i64;
    let height = 420i64;
    let cx = 250.0f64;
    let cy = 220.0f64;
    let radius = 120.0f64;
    let total: i64 = values.iter().map(|value| value.abs()).sum::<i64>().max(1);
    let colors = [
        "#2563eb", "#059669", "#dc2626", "#d97706", "#7c3aed", "#0891b2",
    ];
    let mut start = -std::f64::consts::FRAC_PI_2;
    let mut body = String::new();

    for (index, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        let slice = value.abs() as f64 / total as f64;
        let end = start + slice * std::f64::consts::TAU;
        let large_arc = if end - start > std::f64::consts::PI {
            1
        } else {
            0
        };
        let x1 = cx + radius * start.cos();
        let y1 = cy + radius * start.sin();
        let x2 = cx + radius * end.cos();
        let y2 = cy + radius * end.sin();
        let color = colors[index % colors.len()];
        body.push_str(&format!(
            "<path d=\"M {} {} L {:.2} {:.2} A {} {} 0 {} 1 {:.2} {:.2} Z\" fill=\"{}\"/>",
            cx, cy, x1, y1, radius, radius, large_arc, x2, y2, color
        ));
        body.push_str(&format!(
            "<rect x=\"470\" y=\"{}\" width=\"14\" height=\"14\" fill=\"{}\"/><text x=\"492\" y=\"{}\" font-size=\"13\" fill=\"#111827\">{} ({})</text>",
            128 + index as i64 * 28,
            color,
            140 + index as i64 * 28,
            escape_svg(label),
            value
        ));
        start = end;
    }

    chart_shell(width, height, title, &body)
}

fn render_chart_by_kind(chart: &ChartSpec) -> String {
    match chart.kind.as_str() {
        "bar" => render_bar_chart(&chart.title, &chart.labels, &chart.values),
        "line" => render_line_chart(&chart.title, &chart.labels, &chart.values),
        "pie" => render_pie_chart(&chart.title, &chart.labels, &chart.values),
        _ => String::new(),
    }
}

fn render_dashboard(title: &str, charts: &[ChartSpec]) -> String {
    let mut cards = String::new();
    for chart in charts {
        cards.push_str("<section class=\"chart-card\">");
        cards.push_str(&render_chart_by_kind(chart));
        cards.push_str("</section>");
    }

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>body{{margin:0;font-family:Arial,sans-serif;background:#eef2f7;color:#111827}}header{{padding:28px 32px;background:#111827;color:white}}main{{display:grid;grid-template-columns:repeat(auto-fit,minmax(340px,1fr));gap:18px;padding:24px}}.chart-card{{background:white;border:1px solid #dbe3ef;border-radius:8px;overflow:hidden;box-shadow:0 8px 24px rgba(15,23,42,.08)}}svg{{display:block;width:100%;height:auto}}</style></head><body><header><h1>{}</h1></header><main>{}</main></body></html>",
        escape_svg(title),
        escape_svg(title),
        cards
    )
}

fn render_table(title: &str, labels: &[String], values: &[i64]) -> String {
    let mut rows = String::new();
    for (label, value) in labels.iter().zip(values.iter()) {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            escape_svg(label),
            value
        ));
    }

    format!(
        "<table><caption>{}</caption><thead><tr><th>label</th><th>value</th></tr></thead><tbody>{}</tbody></table>",
        escape_svg(title),
        rows
    )
}

fn chart_shell(width: i64, height: i64, title: &str, body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\"/><text x=\"36\" y=\"38\" font-size=\"22\" font-family=\"Arial, sans-serif\" font-weight=\"700\" fill=\"#111827\">{}</text><line x1=\"64\" y1=\"344\" x2=\"664\" y2=\"344\" stroke=\"#cbd5e1\"/><line x1=\"64\" y1=\"64\" x2=\"64\" y2=\"344\" stroke=\"#cbd5e1\"/>{}</svg>",
        width,
        height,
        width,
        height,
        escape_svg(title),
        body
    )
}

fn escape_svg(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// HTTP network backend.
pub struct NetBackend {
    timeout: Duration,
}

impl NetBackend {
    pub fn new() -> Self {
        let timeout_ms = env::var("MATTER_NET_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(10_000);

        Self {
            timeout: Duration::from_millis(timeout_ms),
        }
    }
}

impl Default for NetBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NetBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "get" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("net.get", 1, args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.get: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                Ok(Value::new_string(response.body))
            }
            "status" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("net.status", 1, args.len()));
                }
                let url = args[0]
                    .as_string()
                    .map_err(|e| format!("net.status: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                Ok(Value::Int(response.status as i64))
            }
            "ok" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("net.ok", 1, args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.ok: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                let ok = (200..300).contains(&response.status);
                Ok(Value::Bool(ok))
            }
            "post" => {
                if args.len() != 2 {
                    return Err(backend_arity_error("net.post", 2, args.len()));
                }
                let url = args[0]
                    .as_string()
                    .map_err(|e| format!("net.post: {}", e))?;
                let body = args[1]
                    .as_string()
                    .map_err(|e| format!("net.post: {}", e))?;
                let response = http_request("POST", &url, &body, self.timeout)?;
                Ok(Value::new_string(response.body))
            }
            _ => Err(backend_unknown_method_error("net", method)),
        }
    }
}

struct HttpResponse {
    status: u16,
    body: String,
}

fn http_request(
    method: &str,
    url: &str,
    body: &str,
    timeout: Duration,
) -> Result<HttpResponse, String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let mut stream = TcpStream::connect(&address).map_err(|e| {
        format!(
            "net.{} failed to connect to '{}': {}",
            method.to_lowercase(),
            address,
            e
        )
    })?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| format!("net could not set read timeout: {}", e))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| format!("net could not set write timeout: {}", e))?;

    let request = if method == "POST" {
        format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: matter-core/0.1\r\nConnection: close\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            parsed.path,
            parsed.host,
            body.len(),
            body
        )
    } else {
        format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: matter-core/0.1\r\nConnection: close\r\n\r\n",
            parsed.path, parsed.host
        )
    };

    stream.write_all(request.as_bytes()).map_err(|e| {
        format!(
            "net.{} failed to write request: {}",
            method.to_lowercase(),
            e
        )
    })?;

    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| {
        format!(
            "net.{} failed to read response: {}",
            method.to_lowercase(),
            e
        )
    })?;

    parse_http_response(&response)
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "net currently supports only http:// URLs".to_string())?;
    let (authority, path) = if let Some((authority, path)) = rest.split_once('/') {
        (authority, format!("/{}", path))
    } else {
        (rest, "/".to_string())
    };

    if authority.is_empty() {
        return Err("net URL is missing host".to_string());
    }

    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        let port = port
            .parse::<u16>()
            .map_err(|_| format!("net URL has invalid port '{}'", port))?;
        (host.to_string(), port)
    } else {
        (authority.to_string(), 80)
    };

    Ok(ParsedHttpUrl { host, port, path })
}

fn parse_http_response(response: &str) -> Result<HttpResponse, String> {
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "net received invalid HTTP response".to_string())?;
    let status_line = headers
        .lines()
        .next()
        .ok_or_else(|| "net received empty HTTP response".to_string())?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "net response is missing status code".to_string())?
        .parse::<u16>()
        .map_err(|_| "net response has invalid status code".to_string())?;

    Ok(HttpResponse {
        status,
        body: body.to_string(),
    })
}

/// File-backed key/value store.
pub struct StoreBackend {
    path: PathBuf,
    values: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct ToolSpec {
    name: String,
    description: String,
    expensive: bool,
}

/// Tool-calling backend that lets Matter orchestrate IA-style tool requests.
pub struct ToolBackend {
    tools: HashMap<String, ToolSpec>,
}

impl ToolBackend {
    pub fn new() -> Self {
        let mut backend = Self {
            tools: HashMap::new(),
        };
        backend.seed_defaults();
        backend
    }

    fn seed_defaults(&mut self) {
        self.tools.insert(
            "agent.backend".to_string(),
            ToolSpec {
                name: "agent.backend".to_string(),
                description: "Bridge to agent/backend operations".to_string(),
                expensive: true,
            },
        );
        self.tools.insert(
            "visual.render".to_string(),
            ToolSpec {
                name: "visual.render".to_string(),
                description: "Render visual artifacts and UI scenes".to_string(),
                expensive: true,
            },
        );
        self.tools.insert(
            "net.fetch".to_string(),
            ToolSpec {
                name: "net.fetch".to_string(),
                description: "Network fetch operation".to_string(),
                expensive: true,
            },
        );
    }

    fn spec_to_value(spec: &ToolSpec) -> Value {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::new_string(spec.name.clone()));
        map.insert(
            "description".to_string(),
            Value::new_string(spec.description.clone()),
        );
        map.insert("expensive".to_string(), Value::Bool(spec.expensive));
        Value::new_map(map)
    }

    fn classify_tool(&self, name: &str) -> Value {
        let expensive = self
            .tools
            .get(name)
            .map(|tool| tool.expensive)
            .unwrap_or(true);
        let mut map = HashMap::new();
        map.insert("tool".to_string(), Value::new_string(name.to_string()));
        map.insert("expensive".to_string(), Value::Bool(expensive));
        map.insert(
            "recommendation".to_string(),
            Value::new_string(if expensive {
                "defer_or_cache".to_string()
            } else {
                "execute_now".to_string()
            }),
        );
        Value::new_map(map)
    }

    fn invoke_json(&self, name: &str, payload: Value) -> Value {
        let expensive = self
            .tools
            .get(name)
            .map(|tool| tool.expensive)
            .unwrap_or(true);
        let mut map = HashMap::new();
        map.insert("tool".to_string(), Value::new_string(name.to_string()));
        map.insert("status".to_string(), Value::new_string("ok".to_string()));
        map.insert(
            "format".to_string(),
            Value::new_string("MTR_TOOL_1".to_string()),
        );
        map.insert("expensive".to_string(), Value::Bool(expensive));
        map.insert(
            "energy_hint".to_string(),
            Value::new_string(if expensive {
                "high_cost_virtual_op".to_string()
            } else {
                "low_cost_virtual_op".to_string()
            }),
        );
        map.insert("payload".to_string(), payload);
        Value::new_map(map)
    }

    fn parse_agent_role(value: &str) -> AgentRole {
        match value.to_ascii_lowercase().as_str() {
            "planner" => AgentRole::Planner,
            "reviewer" => AgentRole::Reviewer,
            "runtime" => AgentRole::Runtime,
            "userproxy" | "user_proxy" | "user-proxy" => AgentRole::UserProxy,
            _ => AgentRole::Worker,
        }
    }

    fn parse_task_state(value: &str) -> TaskState {
        match value.to_ascii_lowercase().as_str() {
            "in_progress" | "inprogress" => TaskState::InProgress,
            "blocked" => TaskState::Blocked,
            "ready_for_review" | "readyforreview" => TaskState::ReadyForReview,
            "completed" => TaskState::Completed,
            _ => TaskState::Proposed,
        }
    }

    fn map_get_string(entries: &HashMap<String, Value>, key: &str) -> Result<String, String> {
        match entries.get(key) {
            Some(value) => value.as_string(),
            None => Ok(String::new()),
        }
    }

    fn map_get_string_list(
        entries: &HashMap<String, Value>,
        key: &str,
    ) -> Result<Vec<String>, String> {
        match entries.get(key) {
            Some(Value::List(items)) => items.iter().map(|item| item.as_string()).collect(),
            Some(Value::Unit) | None => Ok(Vec::new()),
            Some(_) => Err(format!(
                "tool.invoke_frame expects '{}' as list<string>",
                key
            )),
        }
    }

    fn invoke_frame(&self, frame_payload: &HashMap<String, Value>) -> Result<Value, String> {
        let from_name = Self::map_get_string(frame_payload, "from_name")?;
        let from_role = Self::parse_agent_role(&Self::map_get_string(frame_payload, "from_role")?);
        let to_name = Self::map_get_string(frame_payload, "to_name")?;
        let to_role = Self::parse_agent_role(&Self::map_get_string(frame_payload, "to_role")?);
        let task_id = Self::map_get_string(frame_payload, "task_id")?;
        let state = Self::parse_task_state(&Self::map_get_string(frame_payload, "state")?);

        let mut frame = AgentFrame::new(
            AgentId::new(
                if from_name.is_empty() {
                    "planner"
                } else {
                    from_name.as_str()
                },
                from_role,
            ),
            AgentId::new(
                if to_name.is_empty() {
                    "worker"
                } else {
                    to_name.as_str()
                },
                to_role,
            ),
            if task_id.is_empty() {
                "tool-task"
            } else {
                task_id.as_str()
            },
        )
        .with_state(state)
        .with_goal(Self::map_get_string(frame_payload, "goal")?)
        .with_summary(Self::map_get_string(frame_payload, "summary")?)
        .with_next_action(Self::map_get_string(frame_payload, "next_action")?);

        for fact in Self::map_get_string_list(frame_payload, "facts")? {
            frame = frame.add_fact(fact);
        }
        for blocker in Self::map_get_string_list(frame_payload, "blockers")? {
            frame = frame.add_blocker(blocker);
        }
        for request in Self::map_get_string_list(frame_payload, "requests")? {
            frame = frame.add_request(request);
        }

        let response = frame.response_from_receiver();
        let session = AgentSession::new("tool-session")
            .add_frame(frame.clone())
            .add_response(response.clone());
        let packet = session.handoff_packet(8);
        let wire = packet.to_wire();

        let mut out = HashMap::new();
        out.insert("status".to_string(), Value::new_string("ok".to_string()));
        out.insert(
            "protocol".to_string(),
            Value::new_string("matter-agent-protocol".to_string()),
        );
        out.insert(
            "readiness".to_string(),
            Value::new_string(format!("{:?}", frame.readiness()).to_ascii_lowercase()),
        );
        out.insert(
            "response_kind".to_string(),
            Value::new_string(format!("{:?}", response.kind).to_ascii_lowercase()),
        );
        out.insert(
            "next_action".to_string(),
            Value::new_string(response.next_action),
        );
        out.insert("summary".to_string(), Value::new_string(response.summary));
        out.insert("wire".to_string(), Value::new_string(wire));
        out.insert("actionable".to_string(), Value::Bool(frame.is_actionable()));
        Ok(Value::new_map(out))
    }

    fn from_wire(&self, wire: &str) -> Result<Value, String> {
        let packet = AgentHandoffPacket::from_wire(wire)
            .map_err(|error| format!("tool.from_wire parse error: {}", error))?;
        let mut out = HashMap::new();
        out.insert("packet_id".to_string(), Value::new_string(packet.packet_id));
        out.insert(
            "session_id".to_string(),
            Value::new_string(packet.context.session_id),
        );
        out.insert(
            "latest_task_id".to_string(),
            Value::new_string(packet.context.latest_task_id),
        );
        out.insert(
            "next_action".to_string(),
            Value::new_string(packet.context.next_action),
        );
        out.insert("terminal".to_string(), Value::Bool(packet.context.terminal));
        out.insert(
            "event_count".to_string(),
            Value::Int(packet.context.event_count as i64),
        );
        out.insert(
            "last_response_kind".to_string(),
            Value::new_string(
                packet
                    .context
                    .last_response_kind
                    .map(|kind| format!("{:?}", kind).to_ascii_lowercase())
                    .unwrap_or_else(|| "none".to_string()),
            ),
        );
        Ok(Value::new_map(out))
    }

    fn parse_merge_strategy(value: &str) -> MergeStrategy {
        match value.to_ascii_lowercase().as_str() {
            "prefer_terminal" | "terminal" => MergeStrategy::PreferTerminal,
            "prefer_blocked" | "blocked" => MergeStrategy::PreferBlocked,
            _ => MergeStrategy::PreferLatest,
        }
    }

    fn merge_wire(
        &self,
        left: &str,
        right: &str,
        strategy: MergeStrategy,
    ) -> Result<Value, String> {
        let left_packet = AgentHandoffPacket::from_wire(left)
            .map_err(|error| format!("tool.merge_wire left parse error: {}", error))?;
        let right_packet = AgentHandoffPacket::from_wire(right)
            .map_err(|error| format!("tool.merge_wire right parse error: {}", error))?;
        let mut out = HashMap::new();
        match left_packet.merge_with_strategy(&right_packet, strategy) {
            Ok(merged) => {
                let merged_wire = merged.to_wire_with_strategy(strategy);
                out.insert("status".to_string(), Value::new_string("ok".to_string()));
                out.insert(
                    "packet_id".to_string(),
                    Value::new_string(merged.packet_id.clone()),
                );
                out.insert(
                    "lineage_depth".to_string(),
                    Value::Int(merged.lineage_depth as i64),
                );
                out.insert(
                    "next_action".to_string(),
                    Value::new_string(merged.context.next_action.clone()),
                );
                out.insert("wire".to_string(), Value::new_string(merged_wire));
            }
            Err(error) => {
                // Controlled fallback: return preferred valid wire instead of hard failure.
                let preferred = match strategy {
                    MergeStrategy::PreferBlocked => right_packet,
                    _ => left_packet,
                };
                out.insert(
                    "status".to_string(),
                    Value::new_string("degraded".to_string()),
                );
                out.insert("error".to_string(), Value::new_string(error));
                out.insert(
                    "packet_id".to_string(),
                    Value::new_string(preferred.packet_id.clone()),
                );
                out.insert(
                    "lineage_depth".to_string(),
                    Value::Int(preferred.lineage_depth as i64),
                );
                out.insert(
                    "next_action".to_string(),
                    Value::new_string(preferred.context.next_action.clone()),
                );
                out.insert(
                    "wire".to_string(),
                    Value::new_string(preferred.to_wire_with_strategy(strategy)),
                );
            }
        }
        Ok(Value::new_map(out))
    }

    fn validate_wire(&self, wire: &str) -> Value {
        match AgentHandoffPacket::from_wire(wire) {
            Ok(packet) => {
                let mut out = HashMap::new();
                out.insert("ok".to_string(), Value::Bool(true));
                out.insert("packet_id".to_string(), Value::new_string(packet.packet_id));
                out.insert(
                    "lineage_depth".to_string(),
                    Value::Int(packet.lineage_depth as i64),
                );
                Value::new_map(out)
            }
            Err(error) => {
                let mut out = HashMap::new();
                out.insert("ok".to_string(), Value::Bool(false));
                out.insert("error".to_string(), Value::new_string(error));
                Value::new_map(out)
            }
        }
    }
}

impl Default for ToolBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for ToolBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "list" => {
                if !args.is_empty() {
                    return Err(backend_arity_error("tool.list", 0, args.len()));
                }
                let mut names: Vec<String> = self.tools.keys().cloned().collect();
                names.sort();
                Ok(Value::new_list(
                    names.into_iter().map(Value::new_string).collect(),
                ))
            }
            "describe" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("tool.describe", 1, args.len()));
                }
                let name = args[0].as_string()?;
                if let Some(spec) = self.tools.get(&name) {
                    Ok(Self::spec_to_value(spec))
                } else {
                    Ok(Value::Unit)
                }
            }
            "register" => {
                if args.len() != 3 {
                    return Err(backend_arity_error("tool.register", 3, args.len()));
                }
                let name = args[0].as_string()?;
                let description = args[1].as_string()?;
                let expensive = args[2].as_bool()?;
                self.tools.insert(
                    name.clone(),
                    ToolSpec {
                        name,
                        description,
                        expensive,
                    },
                );
                Ok(Value::Unit)
            }
            "call" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "Backend call failed [context:op=tool.call,expected_args=1..2,actual_args={}]: invalid arity",
                        args.len()
                    ));
                }
                let tool_name = args[0].as_string()?;
                let payload = args.get(1).cloned().unwrap_or(Value::Unit);
                let Some(spec) = self.tools.get(&tool_name) else {
                    return Err(format!("tool.call unknown tool '{}'", tool_name));
                };

                let mut map = HashMap::new();
                map.insert("tool".to_string(), Value::new_string(tool_name));
                map.insert("status".to_string(), Value::new_string("ok".to_string()));
                map.insert("expensive".to_string(), Value::Bool(spec.expensive));
                map.insert("payload".to_string(), payload);
                Ok(Value::new_map(map))
            }
            "classify" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("tool.classify", 1, args.len()));
                }
                let tool_name = args[0].as_string()?;
                Ok(self.classify_tool(&tool_name))
            }
            "invoke_json" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "Backend call failed [context:op=tool.invoke_json,expected_args=1..2,actual_args={}]: invalid arity",
                        args.len()
                    ));
                }
                let tool_name = args[0].as_string()?;
                let payload = args.get(1).cloned().unwrap_or(Value::Unit);
                Ok(self.invoke_json(&tool_name, payload))
            }
            "invoke_frame" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("tool.invoke_frame", 1, args.len()));
                }
                let Value::Map(entries) = &args[0] else {
                    return Err("tool.invoke_frame expects a map payload".to_string());
                };
                self.invoke_frame(entries)
            }
            "from_wire" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("tool.from_wire", 1, args.len()));
                }
                let wire = args[0].as_string()?;
                self.from_wire(&wire)
            }
            "merge_wire" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(format!(
                        "Backend call failed [context:op=tool.merge_wire,expected_args=2..3,actual_args={}]: invalid arity",
                        args.len()
                    ));
                }
                let left = args[0].as_string()?;
                let right = args[1].as_string()?;
                let strategy = if args.len() == 3 {
                    Self::parse_merge_strategy(&args[2].as_string()?)
                } else {
                    MergeStrategy::PreferLatest
                };
                self.merge_wire(&left, &right, strategy)
            }
            "validate_wire" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("tool.validate_wire", 1, args.len()));
                }
                let wire = args[0].as_string()?;
                Ok(self.validate_wire(&wire))
            }
            _ => Err(backend_unknown_method_error("tool", method)),
        }
    }
}

impl StoreBackend {
    pub fn new() -> Self {
        let path = env::var("MATTER_STORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".matter_store.json"));
        Self::with_path(path)
    }

    pub fn with_path(path: PathBuf) -> Self {
        let values = load_store_file(&path).unwrap_or_default();
        Self { path, values }
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("could not create store directory: {}", e))?;
            }
        }

        let mut object = serde_json::Map::new();
        for (key, value) in &self.values {
            object.insert(key.clone(), encode_value(value));
        }

        let json = serde_json::to_string_pretty(&serde_json::Value::Object(object))
            .map_err(|e| format!("could not encode store: {}", e))?;
        fs::write(&self.path, json)
            .map_err(|e| format!("could not write store '{}': {}", self.path.display(), e))
    }
}

impl Default for StoreBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for StoreBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "set" => {
                if args.len() != 2 {
                    return Err(backend_arity_error("store.set", 2, args.len()));
                }
                let key = args[0].as_string()?;
                self.values.insert(key, args[1].clone());
                self.save()?;
                Ok(Value::Unit)
            }
            "get" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("store.get", 1, args.len()));
                }
                let key = args[0].as_string()?;
                Ok(self.values.get(&key).cloned().unwrap_or(Value::Unit))
            }
            "has" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("store.has", 1, args.len()));
                }
                let key = args[0].as_string()?;
                Ok(Value::Bool(self.values.contains_key(&key)))
            }
            "delete" => {
                if args.len() != 1 {
                    return Err(backend_arity_error("store.delete", 1, args.len()));
                }
                let key = args[0].as_string()?;
                let existed = self.values.remove(&key).is_some();
                self.save()?;
                Ok(Value::Bool(existed))
            }
            "clear" => {
                if !args.is_empty() {
                    return Err(backend_arity_error("store.clear", 0, args.len()));
                }
                self.values.clear();
                self.save()?;
                Ok(Value::Unit)
            }
            "list" => {
                if !args.is_empty() {
                    return Err(backend_arity_error("store.list", 0, args.len()));
                }
                let mut keys: Vec<String> = self.values.keys().cloned().collect();
                keys.sort();
                Ok(Value::new_list(
                    keys.into_iter().map(Value::new_string).collect(),
                ))
            }
            _ => Err(backend_unknown_method_error("store", method)),
        }
    }
}

fn load_store_file(path: &PathBuf) -> Result<HashMap<String, Value>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let source = fs::read_to_string(path)
        .map_err(|e| format!("could not read store '{}': {}", path.display(), e))?;
    if source.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let json: serde_json::Value = serde_json::from_str(&source)
        .map_err(|e| format!("could not parse store '{}': {}", path.display(), e))?;
    let object = json
        .as_object()
        .ok_or_else(|| "store root must be a JSON object".to_string())?;

    let mut values = HashMap::new();
    for (key, value) in object {
        values.insert(key.clone(), decode_value(value)?);
    }
    Ok(values)
}

fn encode_value(value: &Value) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    match value {
        Value::Int(n) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("int".to_string()),
            );
            object.insert("value".to_string(), serde_json::Value::Number((*n).into()));
        }
        Value::Float(f) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("float".to_string()),
            );
            if let Some(num) = serde_json::Number::from_f64(*f) {
                object.insert("value".to_string(), serde_json::Value::Number(num));
            } else {
                object.insert("value".to_string(), serde_json::Value::Null);
            }
        }
        Value::Bool(b) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("bool".to_string()),
            );
            object.insert("value".to_string(), serde_json::Value::Bool(*b));
        }
        Value::String(s) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::String((**s).clone()),
            );
        }
        Value::Unit => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("unit".to_string()),
            );
        }
        Value::List(elements) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("list".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::Array(elements.iter().map(encode_value).collect()),
            );
        }
        Value::Map(entries) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("map".to_string()),
            );
            let mut values = serde_json::Map::new();
            for (key, value) in entries.iter() {
                values.insert(key.clone(), encode_value(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Struct { type_name, fields } => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("struct".to_string()),
            );
            object.insert(
                "name".to_string(),
                serde_json::Value::String((**type_name).clone()),
            );
            let mut values = serde_json::Map::new();
            for (key, value) in fields.iter() {
                values.insert(key.clone(), encode_value(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Function(name) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("function".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::String((**name).clone()),
            );
        }
    }
    serde_json::Value::Object(object)
}

fn decode_value(json: &serde_json::Value) -> Result<Value, String> {
    let object = json
        .as_object()
        .ok_or_else(|| "stored value must be an object".to_string())?;
    let value_type = object
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "stored value is missing type".to_string())?;

    match value_type {
        "int" => Ok(Value::Int(
            object
                .get("value")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "stored int is invalid".to_string())?,
        )),
        "float" => Ok(Value::Float(
            object
                .get("value")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "stored float is invalid".to_string())?,
        )),
        "bool" => Ok(Value::Bool(
            object
                .get("value")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "stored bool is invalid".to_string())?,
        )),
        "string" => Ok(Value::new_string(
            object
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "stored string is invalid".to_string())?
                .to_string(),
        )),
        "unit" => Ok(Value::Unit),
        "list" => {
            let values = object
                .get("value")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "stored list is invalid".to_string())?;
            values
                .iter()
                .map(decode_value)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::new_list)
        }
        "map" => {
            let values = object
                .get("value")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "stored map is invalid".to_string())?;
            let mut map = HashMap::new();
            for (key, value) in values {
                map.insert(key.clone(), decode_value(value)?);
            }
            Ok(Value::new_map(map))
        }
        "struct" => {
            let type_name = object
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "stored struct is missing name".to_string())?
                .to_string();
            let values = object
                .get("value")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "stored struct fields are invalid".to_string())?;
            let mut fields = HashMap::new();
            for (key, value) in values {
                fields.insert(key.clone(), decode_value(value)?);
            }
            Ok(Value::new_struct(type_name, fields))
        }
        "function" => Ok(Value::new_function(
            object
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "stored function is invalid".to_string())?
                .to_string(),
        )),
        _ => Err(format!("unknown stored value type '{}'", value_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_bar_returns_svg() {
        let mut graph = GraphBackend::new();
        let result = graph
            .call(
                "bar",
                vec![
                    Value::new_string("Vendas".to_string()),
                    Value::new_list(vec![
                        Value::new_string("Jan".to_string()),
                        Value::new_string("Fev".to_string()),
                    ]),
                    Value::new_list(vec![Value::Int(10), Value::Int(24)]),
                ],
            )
            .unwrap();

        let svg = result.as_string().unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Vendas"));
        assert!(svg.contains("Jan"));
    }

    #[test]
    fn graph_rejects_mismatched_lengths() {
        let mut graph = GraphBackend::new();
        let error = graph
            .call(
                "line",
                vec![
                    Value::new_string("Uso".to_string()),
                    Value::new_list(vec![Value::new_string("A".to_string())]),
                    Value::new_list(vec![Value::Int(1), Value::Int(2)]),
                ],
            )
            .unwrap_err();

        assert!(error.contains("same length"));
    }

    #[test]
    fn graph_dashboard_writes_html() {
        let mut graph = GraphBackend::new();
        let path = std::env::temp_dir().join("matter_graph_dashboard_test.html");
        let mut chart = HashMap::new();
        chart.insert("kind".to_string(), Value::new_string("bar".to_string()));
        chart.insert(
            "title".to_string(),
            Value::new_string("Receita".to_string()),
        );
        chart.insert(
            "labels".to_string(),
            Value::new_list(vec![
                Value::new_string("Jan".to_string()),
                Value::new_string("Fev".to_string()),
            ]),
        );
        chart.insert(
            "values".to_string(),
            Value::new_list(vec![Value::Int(10), Value::Int(20)]),
        );

        let result = graph
            .call(
                "dashboard",
                vec![
                    Value::new_string(path.display().to_string()),
                    Value::new_string("Painel".to_string()),
                    Value::new_list(vec![Value::new_map(chart)]),
                ],
            )
            .unwrap();

        assert_eq!(result, Value::new_string(path.display().to_string()));
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("Painel"));
        assert!(html.contains("<svg"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn graph_stats_returns_summary_map() {
        let mut graph = GraphBackend::new();
        let result = graph
            .call(
                "stats",
                vec![Value::new_list(vec![
                    Value::Int(12),
                    Value::Int(20),
                    Value::Int(16),
                    Value::Int(28),
                ])],
            )
            .unwrap();

        let Value::Map(stats) = result else {
            unreachable!("expected stats map");
        };
        assert_eq!(stats.get("count"), Some(&Value::Int(4)));
        assert_eq!(stats.get("total"), Some(&Value::Int(76)));
        assert_eq!(stats.get("min"), Some(&Value::Int(12)));
        assert_eq!(stats.get("max"), Some(&Value::Int(28)));
        assert_eq!(stats.get("average"), Some(&Value::Int(19)));
    }

    #[test]
    fn graph_table_returns_html_table() {
        let mut graph = GraphBackend::new();
        let result = graph
            .call(
                "table",
                vec![
                    Value::new_string("Resumo".to_string()),
                    Value::new_list(vec![
                        Value::new_string("Jan".to_string()),
                        Value::new_string("Fev".to_string()),
                    ]),
                    Value::new_list(vec![Value::Int(10), Value::Int(20)]),
                ],
            )
            .unwrap();

        let html = result.as_string().unwrap();
        assert!(html.contains("<table>"));
        assert!(html.contains("<caption>Resumo</caption>"));
        assert!(html.contains("<td>Jan</td><td>10</td>"));
    }

    #[test]
    fn unknown_method_errors_use_context_contract() {
        let mut agent = AgentBackend::new();
        let agent_err = agent.call("noop", vec![]).unwrap_err();
        assert!(agent_err.starts_with("Backend call failed"));
        assert!(agent_err.contains("[context:backend=agent,method=noop]"));

        let mut visual = VisualBackend::new();
        let visual_err = visual.call("noop", vec![]).unwrap_err();
        assert!(visual_err.contains("[context:backend=visual,method=noop]"));

        let mut graph = GraphBackend::new();
        let graph_err = graph.call("noop", vec![]).unwrap_err();
        assert!(graph_err.contains("[context:backend=graph,method=noop]"));

        let mut net = NetBackend::new();
        let net_err = net.call("noop", vec![]).unwrap_err();
        assert!(net_err.contains("[context:backend=net,method=noop]"));

        let path = std::env::temp_dir().join("matter_store_unknown_method_test.json");
        let mut store = StoreBackend::with_path(path.clone());
        let store_err = store.call("noop", vec![]).unwrap_err();
        assert!(store_err.contains("[context:backend=store,method=noop]"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn value_type_errors_use_context_contract() {
        let int_err = Value::Bool(true).as_int().unwrap_err();
        assert!(int_err.starts_with("Type check failed"));
        assert!(int_err.contains("[context:expected=int,actual=bool]"));

        let bool_err = Value::Int(7).as_bool().unwrap_err();
        assert!(bool_err.contains("[context:expected=bool,actual=int]"));

        let string_err = Value::Unit.as_string().unwrap_err();
        assert!(string_err.contains("[context:expected=string,actual=unit]"));
    }

    #[test]
    fn net_and_store_arity_errors_use_context_contract() {
        let mut net = NetBackend::new();
        let net_err = net.call("get", vec![]).unwrap_err();
        assert!(net_err.starts_with("Backend call failed"));
        assert!(net_err.contains("[context:op=net.get,expected_args=1,actual_args=0]"));

        let path = std::env::temp_dir().join("matter_store_arity_contract_test.json");
        let mut store = StoreBackend::with_path(path.clone());
        let store_err = store.call("clear", vec![Value::Int(1)]).unwrap_err();
        assert!(store_err.contains("[context:op=store.clear,expected_args=0,actual_args=1]"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn graph_arity_errors_use_context_contract() {
        let mut graph = GraphBackend::new();
        let err = graph.call("stats", vec![]).unwrap_err();
        assert!(err.starts_with("Backend call failed"));
        assert!(err.contains("[context:op=graph.stats,expected_args=1,actual_args=0]"));
    }

    #[test]
    fn tool_backend_can_register_list_describe_and_call() {
        let mut backend = ToolBackend::new();
        backend
            .call(
                "register",
                vec![
                    Value::new_string("local.summarize".to_string()),
                    Value::new_string("Local text summarization".to_string()),
                    Value::Bool(false),
                ],
            )
            .unwrap();

        let list = backend.call("list", vec![]).unwrap();
        let Value::List(names) = list else {
            panic!("tool.list should return a list");
        };
        assert!(names.iter().any(
            |name| matches!(name, Value::String(value) if value.as_str() == "local.summarize")
        ));

        let described = backend
            .call(
                "describe",
                vec![Value::new_string("local.summarize".to_string())],
            )
            .unwrap();
        let Value::Map(spec) = described else {
            panic!("tool.describe should return map");
        };
        assert_eq!(
            spec.get("expensive"),
            Some(&Value::Bool(false)),
            "registered tool should preserve expensive flag"
        );

        let result = backend
            .call(
                "call",
                vec![
                    Value::new_string("local.summarize".to_string()),
                    Value::new_string("hello".to_string()),
                ],
            )
            .unwrap();
        let Value::Map(out) = result else {
            panic!("tool.call should return map");
        };
        assert_eq!(
            out.get("status"),
            Some(&Value::new_string("ok".to_string()))
        );
    }

    #[test]
    fn tool_backend_classify_and_invoke_json_include_energy_hints() {
        let mut backend = ToolBackend::new();
        let class = backend
            .call(
                "classify",
                vec![Value::new_string("visual.render".to_string())],
            )
            .unwrap();
        let Value::Map(class_map) = class else {
            panic!("tool.classify should return map");
        };
        assert_eq!(class_map.get("expensive"), Some(&Value::Bool(true)));

        let invoked = backend
            .call(
                "invoke_json",
                vec![
                    Value::new_string("visual.render".to_string()),
                    Value::new_string("{\"scene\":\"hero\"}".to_string()),
                ],
            )
            .unwrap();
        let Value::Map(invoke_map) = invoked else {
            panic!("tool.invoke_json should return map");
        };
        assert_eq!(
            invoke_map.get("format"),
            Some(&Value::new_string("MTR_TOOL_1".to_string()))
        );
        assert_eq!(
            invoke_map.get("energy_hint"),
            Some(&Value::new_string("high_cost_virtual_op".to_string()))
        );
    }

    #[test]
    fn tool_backend_invoke_frame_and_from_wire_round_trip() {
        let mut backend = ToolBackend::new();

        let mut payload = HashMap::new();
        payload.insert(
            "from_name".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "from_role".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "to_name".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "to_role".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "task_id".to_string(),
            Value::new_string("task-42".to_string()),
        );
        payload.insert(
            "state".to_string(),
            Value::new_string("proposed".to_string()),
        );
        payload.insert(
            "goal".to_string(),
            Value::new_string("Integrate tool-calling protocol".to_string()),
        );
        payload.insert(
            "summary".to_string(),
            Value::new_string("Frame is complete and actionable".to_string()),
        );
        payload.insert(
            "next_action".to_string(),
            Value::new_string("execute".to_string()),
        );
        payload.insert(
            "facts".to_string(),
            Value::new_list(vec![Value::new_string("energy-aware".to_string())]),
        );

        let invoked = backend
            .call("invoke_frame", vec![Value::new_map(payload)])
            .unwrap();
        let Value::Map(invoke_map) = invoked else {
            panic!("tool.invoke_frame should return map");
        };
        assert_eq!(
            invoke_map.get("status"),
            Some(&Value::new_string("ok".to_string()))
        );
        assert_eq!(invoke_map.get("actionable"), Some(&Value::Bool(true)));

        let wire = invoke_map
            .get("wire")
            .expect("wire should exist")
            .as_string()
            .expect("wire should be string");

        let decoded = backend
            .call("from_wire", vec![Value::new_string(wire)])
            .unwrap();
        let Value::Map(decoded_map) = decoded else {
            panic!("tool.from_wire should return map");
        };
        assert_eq!(
            decoded_map.get("latest_task_id"),
            Some(&Value::new_string("task-42".to_string()))
        );
        assert_eq!(decoded_map.get("terminal"), Some(&Value::Bool(false)));
    }

    #[test]
    fn tool_backend_merge_and_validate_wire() {
        let mut backend = ToolBackend::new();

        let mut payload_a = HashMap::new();
        payload_a.insert(
            "from_name".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload_a.insert(
            "from_role".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload_a.insert(
            "to_name".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload_a.insert(
            "to_role".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload_a.insert(
            "task_id".to_string(),
            Value::new_string("merge-task".to_string()),
        );
        payload_a.insert(
            "state".to_string(),
            Value::new_string("proposed".to_string()),
        );
        payload_a.insert(
            "goal".to_string(),
            Value::new_string("merge packets".to_string()),
        );
        payload_a.insert(
            "summary".to_string(),
            Value::new_string("first frame".to_string()),
        );
        payload_a.insert(
            "next_action".to_string(),
            Value::new_string("execute".to_string()),
        );

        let mut payload_b = payload_a.clone();
        payload_b.insert(
            "state".to_string(),
            Value::new_string("blocked".to_string()),
        );
        payload_b.insert(
            "blockers".to_string(),
            Value::new_list(vec![Value::new_string("missing_api_key".to_string())]),
        );
        payload_b.insert(
            "summary".to_string(),
            Value::new_string("second frame blocked".to_string()),
        );

        let wire_a = match backend
            .call("invoke_frame", vec![Value::new_map(payload_a)])
            .unwrap()
        {
            Value::Map(map) => map.get("wire").unwrap().as_string().unwrap(),
            _ => panic!("invoke_frame result expected map"),
        };
        let wire_b = match backend
            .call("invoke_frame", vec![Value::new_map(payload_b)])
            .unwrap()
        {
            Value::Map(map) => map.get("wire").unwrap().as_string().unwrap(),
            _ => panic!("invoke_frame result expected map"),
        };

        let merged = backend
            .call(
                "merge_wire",
                vec![
                    Value::new_string(wire_a.clone()),
                    Value::new_string(wire_b.clone()),
                    Value::new_string("prefer_blocked".to_string()),
                ],
            )
            .unwrap();
        let Value::Map(merged_map) = merged else {
            panic!("merge_wire should return map");
        };
        let merged_wire = merged_map.get("wire").unwrap().as_string().unwrap();

        let valid = backend
            .call("validate_wire", vec![Value::new_string(merged_wire)])
            .unwrap();
        let Value::Map(valid_map) = valid else {
            panic!("validate_wire should return map");
        };
        assert_eq!(valid_map.get("ok"), Some(&Value::Bool(true)));
    }
}
