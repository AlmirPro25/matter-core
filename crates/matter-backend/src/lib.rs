/// Backend contracts for Matter
/// Define interfaces para backends (visual, agent, terminal, etc)

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Unit,
    Function(String),
    // Sprint 4: Data Model
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
}

impl Value {
    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(n) => Ok(*n),
            _ => Err("Expected int".to_string()),
        }
    }
    
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err("Expected bool".to_string()),
        }
    }
    
    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err("Expected string".to_string()),
        }
    }
    
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::String(s) => s.clone(),
            Value::Unit => "()".to_string(),
            Value::Function(name) => format!("<fn {}>", name),
            Value::List(elements) => {
                let items: Vec<String> = elements.iter()
                    .map(|v| v.to_display_string())
                    .collect();
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
                format!("{} {{{}}}", type_name, items.join(", "))
            }
        }
    }
}

pub trait Backend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String>;
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
            _ => Err(format!("Unknown agent method: {}", method)),
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
            _ => Err(format!("Unknown visual method: {}", method)),
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
                Ok(Value::String(render_bar_chart(&title, &labels, &values)))
            }
            "line" => {
                let (title, labels, values) = chart_args("graph.line", args)?;
                Ok(Value::String(render_line_chart(&title, &labels, &values)))
            }
            "pie" => {
                let (title, labels, values) = chart_args("graph.pie", args)?;
                Ok(Value::String(render_pie_chart(&title, &labels, &values)))
            }
            "stats" => {
                if args.len() != 1 {
                    return Err(format!("graph.stats expects 1 argument, got {}", args.len()));
                }
                let values = value_list_ints("graph.stats values", &args[0])?;
                Ok(chart_stats(&values))
            }
            "save" => {
                if args.len() != 5 {
                    return Err(format!("graph.save expects 5 arguments, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("graph.save: {}", e))?;
                let kind = args[1].as_string().map_err(|e| format!("graph.save: {}", e))?;
                let title = args[2].as_string().map_err(|e| format!("graph.save: {}", e))?;
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
                Ok(Value::String(path))
            }
            "dashboard" => {
                if args.len() != 3 {
                    return Err(format!("graph.dashboard expects 3 arguments, got {}", args.len()));
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
                Ok(Value::String(path))
            }
            _ => Err(format!("Unknown graph method: {}", method)),
        }
    }
}

struct ChartSpec {
    kind: String,
    title: String,
    labels: Vec<String>,
    values: Vec<i64>,
}

fn chart_args(
    name: &str,
    args: Vec<Value>,
) -> Result<(String, Vec<String>, Vec<i64>), String> {
    if args.len() != 3 {
        return Err(format!("{} expects 3 arguments, got {}", name, args.len()));
    }
    let title = args[0].as_string().map_err(|e| format!("{}: {}", name, e))?;
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
            _ => {
                return Err(format!(
                    "graph.dashboard chart {} must be a map",
                    index
                ))
            }
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
        return Value::Map(stats);
    }

    let total: i64 = values.iter().sum();
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    stats.insert("total".to_string(), Value::Int(total));
    stats.insert("min".to_string(), Value::Int(min));
    stats.insert("max".to_string(), Value::Int(max));
    stats.insert("average".to_string(), Value::Int(total / values.len() as i64));
    Value::Map(stats)
}

fn render_bar_chart(title: &str, labels: &[String], values: &[i64]) -> String {
    let width = 720i64;
    let height = 420i64;
    let left = 64i64;
    let top = 64i64;
    let chart_w = 600i64;
    let chart_h = 280i64;
    let max = values.iter().map(|value| value.abs()).max().unwrap_or(1).max(1);
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
    let colors = ["#2563eb", "#059669", "#dc2626", "#d97706", "#7c3aed", "#0891b2"];
    let mut start = -std::f64::consts::FRAC_PI_2;
    let mut body = String::new();

    for (index, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        let slice = value.abs() as f64 / total as f64;
        let end = start + slice * std::f64::consts::TAU;
        let large_arc = if end - start > std::f64::consts::PI { 1 } else { 0 };
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
                    return Err(format!("net.get expects 1 argument, got {}", args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.get: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                Ok(Value::String(response.body))
            }
            "status" => {
                if args.len() != 1 {
                    return Err(format!("net.status expects 1 argument, got {}", args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.status: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                Ok(Value::Int(response.status as i64))
            }
            "ok" => {
                if args.len() != 1 {
                    return Err(format!("net.ok expects 1 argument, got {}", args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.ok: {}", e))?;
                let response = http_request("GET", &url, "", self.timeout)?;
                let ok = (200..300).contains(&response.status);
                Ok(Value::Bool(ok))
            }
            "post" => {
                if args.len() != 2 {
                    return Err(format!("net.post expects 2 arguments, got {}", args.len()));
                }
                let url = args[0].as_string().map_err(|e| format!("net.post: {}", e))?;
                let body = args[1].as_string().map_err(|e| format!("net.post: {}", e))?;
                let response = http_request("POST", &url, &body, self.timeout)?;
                Ok(Value::String(response.body))
            }
            _ => Err(format!("Unknown net method: {}", method)),
        }
    }
}

struct HttpResponse {
    status: u16,
    body: String,
}

fn http_request(method: &str, url: &str, body: &str, timeout: Duration) -> Result<HttpResponse, String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let mut stream = TcpStream::connect(&address)
        .map_err(|e| format!("net.{} failed to connect to '{}': {}", method.to_lowercase(), address, e))?;
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
            body.as_bytes().len(),
            body
        )
    } else {
        format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: matter-core/0.1\r\nConnection: close\r\n\r\n",
            parsed.path, parsed.host
        )
    };

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("net.{} failed to write request: {}", method.to_lowercase(), e))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("net.{} failed to read response: {}", method.to_lowercase(), e))?;

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
        fs::write(&self.path, json).map_err(|e| {
            format!("could not write store '{}': {}", self.path.display(), e)
        })
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
                    return Err("store.set expects 2 arguments".to_string());
                }
                let key = args[0].as_string()?;
                self.values.insert(key, args[1].clone());
                self.save()?;
                Ok(Value::Unit)
            }
            "get" => {
                if args.len() != 1 {
                    return Err("store.get expects 1 argument".to_string());
                }
                let key = args[0].as_string()?;
                Ok(self.values.get(&key).cloned().unwrap_or(Value::Unit))
            }
            "has" => {
                if args.len() != 1 {
                    return Err("store.has expects 1 argument".to_string());
                }
                let key = args[0].as_string()?;
                Ok(Value::Bool(self.values.contains_key(&key)))
            }
            "delete" => {
                if args.len() != 1 {
                    return Err("store.delete expects 1 argument".to_string());
                }
                let key = args[0].as_string()?;
                let existed = self.values.remove(&key).is_some();
                self.save()?;
                Ok(Value::Bool(existed))
            }
            "clear" => {
                if !args.is_empty() {
                    return Err("store.clear expects 0 arguments".to_string());
                }
                self.values.clear();
                self.save()?;
                Ok(Value::Unit)
            }
            "list" => {
                if !args.is_empty() {
                    return Err("store.list expects 0 arguments".to_string());
                }
                let mut keys: Vec<String> = self.values.keys().cloned().collect();
                keys.sort();
                Ok(Value::List(keys.into_iter().map(Value::String).collect()))
            }
            _ => Err(format!("Unknown store method: {}", method)),
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
            object.insert("type".to_string(), serde_json::Value::String("int".to_string()));
            object.insert("value".to_string(), serde_json::Value::Number((*n).into()));
        }
        Value::Bool(b) => {
            object.insert("type".to_string(), serde_json::Value::String("bool".to_string()));
            object.insert("value".to_string(), serde_json::Value::Bool(*b));
        }
        Value::String(s) => {
            object.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            object.insert("value".to_string(), serde_json::Value::String(s.clone()));
        }
        Value::Unit => {
            object.insert("type".to_string(), serde_json::Value::String("unit".to_string()));
        }
        Value::List(elements) => {
            object.insert("type".to_string(), serde_json::Value::String("list".to_string()));
            object.insert(
                "value".to_string(),
                serde_json::Value::Array(elements.iter().map(encode_value).collect()),
            );
        }
        Value::Map(entries) => {
            object.insert("type".to_string(), serde_json::Value::String("map".to_string()));
            let mut values = serde_json::Map::new();
            for (key, value) in entries {
                values.insert(key.clone(), encode_value(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Struct { type_name, fields } => {
            object.insert("type".to_string(), serde_json::Value::String("struct".to_string()));
            object.insert(
                "name".to_string(),
                serde_json::Value::String(type_name.clone()),
            );
            let mut values = serde_json::Map::new();
            for (key, value) in fields {
                values.insert(key.clone(), encode_value(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Function(name) => {
            object.insert("type".to_string(), serde_json::Value::String("function".to_string()));
            object.insert("value".to_string(), serde_json::Value::String(name.clone()));
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
        "bool" => Ok(Value::Bool(
            object
                .get("value")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "stored bool is invalid".to_string())?,
        )),
        "string" => Ok(Value::String(
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
            values.iter().map(decode_value).collect::<Result<Vec<_>, _>>().map(Value::List)
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
            Ok(Value::Map(map))
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
            Ok(Value::Struct { type_name, fields })
        }
        "function" => Ok(Value::Function(
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
                    Value::String("Vendas".to_string()),
                    Value::List(vec![
                        Value::String("Jan".to_string()),
                        Value::String("Fev".to_string()),
                    ]),
                    Value::List(vec![Value::Int(10), Value::Int(24)]),
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
                    Value::String("Uso".to_string()),
                    Value::List(vec![Value::String("A".to_string())]),
                    Value::List(vec![Value::Int(1), Value::Int(2)]),
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
        chart.insert("kind".to_string(), Value::String("bar".to_string()));
        chart.insert("title".to_string(), Value::String("Receita".to_string()));
        chart.insert(
            "labels".to_string(),
            Value::List(vec![
                Value::String("Jan".to_string()),
                Value::String("Fev".to_string()),
            ]),
        );
        chart.insert(
            "values".to_string(),
            Value::List(vec![Value::Int(10), Value::Int(20)]),
        );

        let result = graph
            .call(
                "dashboard",
                vec![
                    Value::String(path.display().to_string()),
                    Value::String("Painel".to_string()),
                    Value::List(vec![Value::Map(chart)]),
                ],
            )
            .unwrap();

        assert_eq!(result, Value::String(path.display().to_string()));
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
                vec![Value::List(vec![
                    Value::Int(12),
                    Value::Int(20),
                    Value::Int(16),
                    Value::Int(28),
                ])],
            )
            .unwrap();

        let Value::Map(stats) = result else {
            panic!("expected stats map");
        };
        assert_eq!(stats.get("count"), Some(&Value::Int(4)));
        assert_eq!(stats.get("total"), Some(&Value::Int(76)));
        assert_eq!(stats.get("min"), Some(&Value::Int(12)));
        assert_eq!(stats.get("max"), Some(&Value::Int(28)));
        assert_eq!(stats.get("average"), Some(&Value::Int(19)));
    }
}
