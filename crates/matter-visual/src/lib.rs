#![allow(deprecated, dead_code, clippy::get_first, clippy::useless_format)]
//! Visual Backend for Matter Core
//! Integração com PVM/PXL como backend visual desacoplado
//!
//! Arquitetura:
//! - Matter Core permanece linguagem geral
//! - PVM/PXL é um backend/plugin visual
//! - Visual é um target, não uma dependência core

use eframe::egui;
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

#[derive(Debug, Clone)]
pub struct VisualLayoutSpec {
    pub scene: String,
    pub kind: String,
    pub gap: i64,
}

#[derive(Debug, Clone)]
pub struct VisualComponentSpec {
    pub name: String,
    pub defaults: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct VisualLoopState {
    pub frame: i64,
    pub time_ms: i64,
    pub running: bool,
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
pub struct AntigravityMessage {
    text: String,
    is_user: bool,
    time: String,
}

struct AIRequest {
    prompt: String,
    ctx: egui::Context,
}

/// Bridges the visual GUI to the user's existing matter-cli agent system.
/// Instead of reimplementing API calls, we spawn `matter-cli agent-chat`
/// as a subprocess and pipe messages through stdin/stdout.
/// This means the GUI inherits ALL agent capabilities:
/// providers, profiles, models, memory, sessions, tools, fallback, etc.
fn spawn_agent_bridge() -> (
    std::sync::mpsc::Sender<AIRequest>,
    std::sync::mpsc::Receiver<String>,
) {
    let (ui_tx, worker_rx) = std::sync::mpsc::channel::<AIRequest>();
    let (worker_tx, ui_rx) = std::sync::mpsc::channel::<String>();

    std::thread::spawn(move || {
        // Try to find the matter-cli binary
        let cli_path = find_matter_cli_binary();

        while let Ok(req) = worker_rx.recv() {
            let response = if let Some(ref cli) = cli_path {
                agent_query_via_cli(cli, &req.prompt)
            } else {
                // Fallback: use the direct curl-style approach if CLI binary not found
                agent_query_direct(&req.prompt)
            };
            let _ = worker_tx.send(response);
            req.ctx.request_repaint();
        }
    });

    (ui_tx, ui_rx)
}

fn find_matter_cli_binary() -> Option<String> {
    // 1. Check custom target dir (user's config)
    let custom = "F:/Users/almir/Desktop/matter_target/release/matter-cli.exe";
    if std::path::Path::new(custom).exists() {
        return Some(custom.to_string());
    }
    // 2. Check local target dir
    let local = "./target/release/matter-cli.exe";
    if std::path::Path::new(local).exists() {
        return Some(local.to_string());
    }
    // 3. Check PATH
    if let Ok(output) = std::process::Command::new("where")
        .arg("matter-cli")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path.lines().next().unwrap_or(&path).to_string());
            }
        }
    }
    None
}

/// Query the AI via matter-cli's JSON API bridge
fn agent_query_via_cli(cli_path: &str, prompt: &str) -> String {
    // Use the agent's existing provider/model resolution
    let provider = std::env::var("MATTER_AGENT_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let profile = std::env::var("MATTER_AGENT_PROFILE").unwrap_or_else(|_| "coding".to_string());
    let model = std::env::var("MATTER_AGENT_MODEL").unwrap_or_default();

    // Build the JSON payload using the same format as chat_completion_via_curl
    let system_prompt = "You are Matter Agent, a pragmatic coding assistant inside the Matter Core visual interface. \
        Keep answers concise, actionable and in the user's language (Portuguese/BR). \
        You have access to the entire Matter Core workspace.";

    let payload = serde_json::json!({
        "provider": provider,
        "profile": profile,
        "model": if model.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(model) },
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": prompt}
        ]
    });

    // Write payload to temp file
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let payload_path = std::env::temp_dir().join(format!("matter_visual_agent_{}.json", nanos));
    if let Err(e) = std::fs::write(&payload_path, payload.to_string()) {
        return format!("Erro ao preparar consulta: {}", e);
    }

    // Call matter-cli api-chat-json (single-shot mode)
    let output = std::process::Command::new(cli_path)
        .arg("api-chat-json")
        .arg(payload_path.to_string_lossy().as_ref())
        .output();
    let _ = std::fs::remove_file(&payload_path);

    match output {
        Ok(out) => {
            if out.status.success() {
                let body = String::from_utf8_lossy(&out.stdout);
                // Try to parse JSON response
                if let Ok(doc) = serde_json::from_str::<serde_json::Value>(body.trim()) {
                    if let Some(text) = doc.get("content").and_then(|v| v.as_str()) {
                        return text.to_string();
                    }
                    if let Some(text) = doc
                        .get("choices")
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get("message"))
                        .and_then(|v| v.get("content"))
                        .and_then(|v| v.as_str())
                    {
                        return text.to_string();
                    }
                }
                // If not JSON, return raw output
                body.trim().to_string()
            } else {
                // api-chat-json may not exist yet, fall back to direct
                agent_query_direct(prompt)
            }
        }
        Err(_) => agent_query_direct(prompt),
    }
}

/// Direct HTTP fallback using the user's existing env vars
fn agent_query_direct(prompt: &str) -> String {
    let provider = std::env::var("MATTER_AGENT_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("MATTER_AGENT_API_KEY"))
        .unwrap_or_default();

    if api_key.trim().is_empty() {
        return "⚠️ Agente IA conectado ao sistema Matter Core.\n\n\
            Para ativar respostas de IA em tempo real, configure uma das variáveis:\n\n\
            • `OPENAI_API_KEY` — para usar GPT-4/GPT-4o\n\
            • `MATTER_AGENT_API_KEY` — para usar o provedor configurado\n\
            • `MATTER_AGENT_PROVIDER` — para trocar de provedor (openai, nvidia)\n\n\
            Ou use a CLI diretamente: `matter-cli agent-chat`"
            .to_string();
    }

    let base_url = match provider.to_ascii_lowercase().as_str() {
        "nvidia" => "https://integrate.api.nvidia.com/v1",
        _ => "https://api.openai.com/v1",
    };

    let model = std::env::var("MATTER_AGENT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are Matter Agent, a pragmatic coding assistant inside the Matter Core visual interface. Keep answers concise, actionable and in the user's language (Portuguese/BR). You have access to the entire Matter Core workspace."},
            {"role": "user", "content": prompt}
        ]
    });

    let client = reqwest::blocking::Client::new();
    match client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
    {
        Ok(res) => {
            if !res.status().is_success() {
                return format!(
                    "Erro da API ({}): tente verificar sua chave com `matter-cli agent-doctor`",
                    res.status()
                );
            }
            match res.json::<serde_json::Value>() {
                Ok(json) => {
                    if let Some(text) = json["choices"][0]["message"]["content"].as_str() {
                        text.to_string()
                    } else {
                        "Resposta da API em formato inesperado".to_string()
                    }
                }
                Err(e) => format!("Erro ao decodificar resposta: {}", e),
            }
        }
        Err(e) => format!("Erro de conexão: {}", e),
    }
}

struct AntigravityApp {
    input_text: String,
    messages: Vec<AntigravityMessage>,
    projects: Vec<(String, String, String)>,
    walkthrough_content: String,
    active_tab: String,
    tx: std::sync::mpsc::Sender<AIRequest>,
    rx: std::sync::mpsc::Receiver<String>,
    is_loading: bool,
    agent_provider: String,
    agent_model: String,
}

impl Default for AntigravityApp {
    fn default() -> Self {
        let (ui_tx, ui_rx) = spawn_agent_bridge();

        let provider =
            std::env::var("MATTER_AGENT_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let model =
            std::env::var("MATTER_AGENT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        Self {
            input_text: String::new(),
            active_tab: "Walkthrough".to_string(),
            messages: vec![
                AntigravityMessage {
                    text: "vc ver furuto?".to_string(),
                    is_user: true,
                    time: "16:40".to_string(),
                },
                AntigravityMessage {
                    text: "Sim, eu vejo um futuro muito promissor para o Matter Core! Três grandes caminhos:\n\n1. 🔌 **Camada de Abstração pós-Silício**: Compilar direto para aceleradores fotônicos e neuromórficos.\n2. 🤖 **Virtual Machine Nativa para Agentes de IA**: Fila de eventos e persistência nativas para rodar LLMs autônomos.\n3. 🎓 **Pesquisa Científica Open-Source**: Publicação do manifesto em conferências científicas globais (PLDI/ASPLOS).".to_string(),
                    is_user: false,
                    time: "16:41".to_string(),
                },
                AntigravityMessage {
                    text: "testar meu sistema quero vc recrie sua interface aqui dentro do meu sistema usando o matter ok ?".to_string(),
                    is_user: true,
                    time: "16:47".to_string(),
                },
                AntigravityMessage {
                    text: "Com certeza! Iniciando a recriação da interface nativa do Antigravity rodando diretamente dentro da engine gráfica (Egui/Rust) do Matter Core... 🚀".to_string(),
                    is_user: false,
                    time: "16:48".to_string(),
                }
            ],
            projects: vec![
                ("app-10-aether-prime-ai-build...".to_string(), "Optimizing Aether P...".to_string(), "1mo".to_string()),
                ("cassino-vip-mobile%20%28...".to_string(), "Modernizing Casino ...".to_string(), "23d".to_string()),
                ("MANIFESTO DA LINGUAGEM MATTER CORE".to_string(), "Reviewing Matter Co...".to_string(), "now".to_string()),
            ],
            walkthrough_content: "### Relatório de QA - Matter Core\n\n- **math.pow**: Ajustada asserção para float de `256.0` em `test_api_bridge.ps1`.\n- **Value::Null**: Corrigida compilação não exaustiva in `crates/matter-bridge-go-native/src/real.rs`.\n- **FFI Polyglot**: Bridges Node.js e Go compilados e verificados com sucesso!\n- **test_all.ps1**: Suíte de testes geral executada com 100% de sucesso!".to_string(),
            tx: ui_tx,
            rx: ui_rx,
            is_loading: false,
            agent_provider: provider,
            agent_model: model,
        }
    }
}

impl eframe::App for AntigravityApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Ok(reply) = self.rx.try_recv() {
            self.messages.push(AntigravityMessage {
                text: reply,
                is_user: false,
                time: "Agora".to_string(),
            });
            self.is_loading = false;
        }

        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(226, 232, 240));
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(14, 14, 18);
        ui.ctx().set_visuals(visuals);

        let height = ui.available_height();

        // 1. LEFT SIDEBAR PANEL
        egui::SidePanel::left("antigravity_left_sidebar")
            .resizable(false)
            .default_width(260.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(15, 15, 18))
                    .inner_margin(12.0),
            )
            .show_inside(ui, |ui| {
                // Top Menu Simulation
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Antigravity")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(59, 130, 246)),
                    );
                    ui.label(
                        egui::RichText::new("File")
                            .size(11.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(
                        egui::RichText::new("View")
                            .size(11.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(
                        egui::RichText::new("Window")
                            .size(11.0)
                            .color(egui::Color32::GRAY),
                    );
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⬅")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(
                        egui::RichText::new("➡")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label(
                        egui::RichText::new("🔄")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                });
                ui.add_space(8.0);

                // + New Conversation Button
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(25, 25, 32))
                    .corner_radius(6)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("➕").size(12.0));
                            ui.label(egui::RichText::new("New Conversation").strong().size(13.0));
                        });
                    });

                ui.add_space(12.0);

                // Navigation Items
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("💬")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label("Conversation History");
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⏱")
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label("Scheduled Tasks");
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                // Projects Section
                ui.label(
                    egui::RichText::new("Projects")
                        .strong()
                        .color(egui::Color32::from_rgb(156, 163, 175))
                        .size(11.0),
                );
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("projects_scroll")
                    .show(ui, |ui| {
                        for (title, detail, time) in &self.projects {
                            let is_active = title.contains("MANIFESTO");
                            let bg_color = if is_active {
                                egui::Color32::from_rgb(30, 30, 42)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(6)
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("📁").size(13.0).color(
                                            if is_active {
                                                egui::Color32::from_rgb(59, 130, 246)
                                            } else {
                                                egui::Color32::GRAY
                                            },
                                        ));
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(title)
                                                    .strong()
                                                    .size(12.0)
                                                    .color(if is_active {
                                                        egui::Color32::WHITE
                                                    } else {
                                                        egui::Color32::from_rgb(180, 180, 190)
                                                    }),
                                            );
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(detail)
                                                        .size(10.0)
                                                        .color(egui::Color32::GRAY),
                                                );
                                                ui.label(
                                                    egui::RichText::new(time).size(10.0).color(
                                                        egui::Color32::from_rgb(100, 100, 120),
                                                    ),
                                                );
                                            });
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }
                    });

                // Settings at the bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("⚙");
                        ui.label("Settings");
                    });
                });
            });

        // 2. RIGHT PANEL - WALKTHROUGH
        egui::SidePanel::right("antigravity_right_sidebar")
            .resizable(false)
            .default_width(320.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(15, 15, 18))
                    .inner_margin(12.0),
            )
            .show_inside(ui, |ui| {
                // Tab select
                ui.horizontal(|ui| {
                    let btn_overview = self.active_tab == "Overview";
                    let btn_walkthrough = self.active_tab == "Walkthrough";

                    if ui.selectable_label(btn_overview, "Overview").clicked() {
                        self.active_tab = "Overview".to_string();
                    }
                    if ui.selectable_label(btn_walkthrough, "Walkthrough").clicked() {
                        self.active_tab = "Walkthrough".to_string();
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(egui::RichText::new(&self.active_tab).strong().size(16.0).color(egui::Color32::WHITE));
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .id_salt("right_scroll")
                    .show(ui, |ui| {
                        if self.active_tab == "Walkthrough" {
                            ui.label(egui::RichText::new("✅ Todas as asserções passaram no compilador!").color(egui::Color32::from_rgb(0, 230, 118)));
                            ui.add_space(10.0);

                            egui::Frame::NONE
                                .fill(egui::Color32::from_rgb(25, 25, 32))
                                .corner_radius(6)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new("Ajuste de math.pow").strong().size(12.0));
                                        ui.label(egui::RichText::new("Asserção corrigida para float 256.0 no arquivo test_api_bridge.ps1.").size(11.0).color(egui::Color32::GRAY));
                                    });
                                });
                            ui.add_space(6.0);

                            egui::Frame::NONE
                                .fill(egui::Color32::from_rgb(25, 25, 32))
                                .corner_radius(6)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new("Value::Null em Go").strong().size(12.0));
                                        ui.label(egui::RichText::new("Adicionada exaustividade no real.rs do bridge de Go para não quebrar no build native.").size(11.0).color(egui::Color32::GRAY));
                                    });
                                });

                            ui.add_space(12.0);
                            ui.label(egui::RichText::new("Relatórios Gerados").strong().size(12.0));
                            ui.label("- ffi-validation-matrix.json");
                            ui.label("- release-readiness.json");
                            ui.label("- ffi-validation-report.md");
                        } else {
                            ui.label("Visão Geral do projeto Matter Core Language.");
                            ui.label("Todo o ecossistema está validado como experimental_release_candidate.");
                        }
                    });

                // Status Bar info at the bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🌤 29°C Pred. Ensolarado").size(10.0).color(egui::Color32::GRAY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("POR PTB2 16:47").size(10.0).color(egui::Color32::GRAY));
                        });
                    });
                });
            });

        // 3. CENTRAL PANEL - CHAT THREAD
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(10, 10, 13))
                    .inner_margin(12.0),
            )
            .show_inside(ui, |ui| {
                // Header of Chat
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("MANIFESTO DA LINGUAGEM MATTER CORE")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.label("/");
                    ui.label(
                        egui::RichText::new("Reviewing Matter Core Language")
                            .size(12.0)
                            .color(egui::Color32::WHITE),
                    );
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(12.0);

                // Scroll message area
                let chat_height = height - 130.0;
                egui::ScrollArea::vertical()
                    .id_salt("antigravity_chat_scroll")
                    .max_height(chat_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());

                        for msg in &self.messages {
                            ui.horizontal(|ui| {
                                if msg.is_user {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::TOP),
                                        |ui| {
                                            egui::Frame::NONE
                                                .fill(egui::Color32::from_rgb(30, 30, 40))
                                                .corner_radius(egui::CornerRadius {
                                                    nw: 10,
                                                    ne: 0,
                                                    sw: 10,
                                                    se: 10,
                                                })
                                                .inner_margin(10.0)
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(&msg.text)
                                                                .size(13.0)
                                                                .color(egui::Color32::from_rgb(
                                                                    230, 230, 240,
                                                                )),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            egui::RichText::new(&msg.time)
                                                                .size(9.0)
                                                                .color(egui::Color32::GRAY),
                                                        );
                                                    });
                                                });
                                        },
                                    );
                                } else {
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::TOP),
                                        |ui| {
                                            egui::Frame::NONE
                                                .fill(egui::Color32::from_rgb(20, 20, 26))
                                                .corner_radius(egui::CornerRadius {
                                                    nw: 0,
                                                    ne: 10,
                                                    sw: 10,
                                                    se: 10,
                                                })
                                                .inner_margin(10.0)
                                                .show(ui, |ui| {
                                                    ui.set_max_width(550.0);
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(&msg.text)
                                                                .size(13.0)
                                                                .color(egui::Color32::from_rgb(
                                                                    220, 220, 230,
                                                                )),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(&msg.time)
                                                                    .size(9.0)
                                                                    .color(egui::Color32::GRAY),
                                                            );
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            "👍 👎 📋",
                                                                        )
                                                                        .size(11.0)
                                                                        .color(egui::Color32::GRAY),
                                                                    );
                                                                },
                                                            );
                                                        });
                                                    });
                                                });
                                        },
                                    );
                                }
                            });
                            ui.add_space(8.0);
                        }
                        if self.is_loading {
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(20, 20, 26))
                                            .corner_radius(egui::CornerRadius {
                                                nw: 0,
                                                ne: 10,
                                                sw: 10,
                                                se: 10,
                                            })
                                            .inner_margin(10.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("Pensando... ⏳")
                                                        .size(13.0)
                                                        .color(egui::Color32::GRAY),
                                                );
                                            });
                                    },
                                );
                            });
                            ui.add_space(8.0);
                        }
                    });

                // Input Bar at the bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(20, 20, 26))
                        .corner_radius(10)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("📎")
                                        .size(14.0)
                                        .color(egui::Color32::GRAY),
                                );

                                // Model & context tags
                                egui::Frame::NONE
                                    .fill(egui::Color32::from_rgb(30, 30, 42))
                                    .corner_radius(4)
                                    .inner_margin(egui::vec2(6.0, 3.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Worktree")
                                                .size(10.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(59, 130, 246)),
                                        );
                                    });

                                egui::Frame::NONE
                                    .fill(egui::Color32::from_rgb(30, 30, 42))
                                    .corner_radius(4)
                                    .inner_margin(egui::vec2(6.0, 3.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(&self.agent_model)
                                                .size(10.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(14, 165, 233)),
                                        );
                                    });

                                let hint = if self.is_loading {
                                    "Processando resposta..."
                                } else {
                                    "Como posso ajudar com o Matter Core?"
                                };
                                let response = ui.add_sized(
                                    egui::vec2(ui.available_width() - 80.0, 24.0),
                                    egui::TextEdit::singleline(&mut self.input_text)
                                        .hint_text(hint)
                                        .margin(egui::vec2(6.0, 4.0)),
                                );

                                let button_color = if self.is_loading {
                                    egui::Color32::GRAY
                                } else {
                                    egui::Color32::from_rgb(59, 130, 246)
                                };
                                let trigger_send = ui
                                    .button(
                                        egui::RichText::new("➡")
                                            .size(14.0)
                                            .strong()
                                            .color(button_color),
                                    )
                                    .clicked()
                                    || (response.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                                if trigger_send && !self.input_text.is_empty() && !self.is_loading {
                                    let user_msg = self.input_text.clone();
                                    self.messages.push(AntigravityMessage {
                                        text: user_msg.clone(),
                                        is_user: true,
                                        time: "Agora".to_string(),
                                    });
                                    self.input_text.clear();
                                    self.is_loading = true;
                                    let _ = self.tx.send(AIRequest {
                                        prompt: user_msg,
                                        ctx: ui.ctx().clone(),
                                    });
                                }
                            });
                        });
                });
            });
    }
}

struct ContactInfo {
    name: String,
    avatar_color: egui::Color32,
    status: &'static str,
    last_seen: &'static str,
    motto: &'static str,
}

struct MatterApp {
    input_text: String,
    active_contact: String,
    chats: std::collections::HashMap<String, Vec<(String, bool, String)>>,
    contacts: Vec<ContactInfo>,
}

impl Default for MatterApp {
    fn default() -> Self {
        let mut chats = std::collections::HashMap::new();

        chats.insert(
            "Matter Bot".to_string(),
            vec![
                ("Olá! Seja muito bem-vindo ao MatterZap Desktop Nativo! 🚀".to_string(), false, "10:00".to_string()),
                ("Eu fui criado 100% em código de máquina pela linguagem Matter com aceleração por GPU!".to_string(), false, "10:01".to_string()),
                ("Digite 'ping' para testar minha inteligência!".to_string(), false, "10:01".to_string()),
            ],
        );

        chats.insert(
            "Almir Pro".to_string(),
            vec![
                (
                    "E aí Almir, curtindo os testes gráficos nativos?".to_string(),
                    false,
                    "09:30".to_string(),
                ),
                (
                    "Ficou absurdamente foda e rápido!".to_string(),
                    true,
                    "09:35".to_string(),
                ),
            ],
        );

        chats.insert(
            "Matter Compiler".to_string(),
            vec![
                (
                    "[COMPILER INFO] JIT compiler optimized with x86_64 target.".to_string(),
                    false,
                    "08:15".to_string(),
                ),
                (
                    "[COMPILER INFO] FPU protection enabled, F64 casting secure.".to_string(),
                    false,
                    "08:16".to_string(),
                ),
            ],
        );

        chats.insert(
            "Grupo de IA".to_string(),
            vec![
                (
                    "Qual é a arquitetura da rede neural em Matter?".to_string(),
                    false,
                    "Ontem".to_string(),
                ),
                (
                    "Nós temos a rede neural implementada com 1 neurônio e pesos ajustáveis!"
                        .to_string(),
                    true,
                    "Ontem".to_string(),
                ),
            ],
        );

        let contacts = vec![
            ContactInfo {
                name: "Matter Bot".to_string(),
                avatar_color: egui::Color32::from_rgb(0, 168, 132),
                status: "Online",
                last_seen: "online",
                motto: "A inteligência artificial do Matter Core v25",
            },
            ContactInfo {
                name: "Almir Pro".to_string(),
                avatar_color: egui::Color32::from_rgb(59, 130, 246),
                status: "Online",
                last_seen: "visto por último às 09:35",
                motto: "Focado em construir o futuro da tecnologia",
            },
            ContactInfo {
                name: "Matter Compiler".to_string(),
                avatar_color: egui::Color32::from_rgb(249, 115, 22),
                status: "Ausente",
                last_seen: "visto por último há 2h",
                motto: "Otimizando bytecode em tempo real",
            },
            ContactInfo {
                name: "Grupo de IA".to_string(),
                avatar_color: egui::Color32::from_rgb(168, 85, 247),
                status: "Offline",
                last_seen: "online ontem",
                motto: "Discutindo redes neurais em Matter",
            },
        ];

        Self {
            input_text: String::new(),
            active_contact: "Matter Bot".to_string(),
            chats,
            contacts,
        }
    }
}

impl eframe::App for MatterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(233, 237, 239));
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(17, 27, 33);
        ui.ctx().set_visuals(visuals);

        let height = ui.available_height();

        egui::SidePanel::left("whatsapp_sidebar")
            .resizable(false)
            .default_width(260.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(17, 27, 33))
                    .inner_margin(8.0),
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_at_least(egui::vec2(36.0, 36.0), egui::Sense::hover());
                    ui.painter().circle_filled(
                        rect.center(),
                        18.0,
                        egui::Color32::from_rgb(33, 150, 243),
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "A",
                        egui::FontId::proportional(18.0),
                        egui::Color32::WHITE,
                    );

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Almir (Você)").strong().size(14.0));
                        ui.label(
                            egui::RichText::new("Status: Matter Dev")
                                .color(egui::Color32::from_rgb(134, 150, 160))
                                .size(11.0),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new("CONVERSAS RECENTES")
                        .strong()
                        .color(egui::Color32::from_rgb(0, 168, 132))
                        .size(11.0),
                );
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .show(ui, |ui| {
                        for contact in &self.contacts {
                            let is_active = self.active_contact == contact.name;

                            let bg_color = if is_active {
                                egui::Color32::from_rgb(42, 57, 66)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(6)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (avatar_rect, _) = ui.allocate_at_least(
                                            egui::vec2(32.0, 32.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(
                                            avatar_rect.center(),
                                            16.0,
                                            contact.avatar_color,
                                        );

                                        let initials =
                                            contact.name.chars().next().unwrap_or(' ').to_string();
                                        ui.painter().text(
                                            avatar_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &initials,
                                            egui::FontId::proportional(15.0),
                                            egui::Color32::WHITE,
                                        );

                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                if ui
                                                    .selectable_label(
                                                        is_active,
                                                        egui::RichText::new(&contact.name)
                                                            .strong()
                                                            .size(13.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.active_contact = contact.name.clone();
                                                }

                                                let dot_color = match contact.status {
                                                    "Online" => {
                                                        egui::Color32::from_rgb(0, 230, 118)
                                                    }
                                                    "Ausente" => {
                                                        egui::Color32::from_rgb(255, 152, 0)
                                                    }
                                                    _ => egui::Color32::from_rgb(158, 158, 158),
                                                };
                                                let (dot_rect, _) = ui.allocate_at_least(
                                                    egui::vec2(8.0, 8.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().circle_filled(
                                                    dot_rect.center(),
                                                    4.0,
                                                    dot_color,
                                                );
                                            });

                                            ui.label(
                                                egui::RichText::new(contact.motto)
                                                    .color(egui::Color32::from_rgb(134, 150, 160))
                                                    .size(10.0),
                                            );
                                        });
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(11, 20, 26)).inner_margin(8.0))
            .show_inside(ui, |ui| {
                let active_c = self.active_contact.clone();
                let current_contact = self.contacts.iter().find(|c| c.name == active_c).unwrap();

                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(32, 44, 51))
                    .corner_radius(6)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (header_avatar, _) = ui.allocate_at_least(egui::vec2(32.0, 32.0), egui::Sense::hover());
                            ui.painter().circle_filled(header_avatar.center(), 16.0, current_contact.avatar_color);
                            let initials = current_contact.name.chars().next().unwrap_or(' ').to_string();
                            ui.painter().text(
                                header_avatar.center(),
                                egui::Align2::CENTER_CENTER,
                                &initials,
                                egui::FontId::proportional(15.0),
                                egui::Color32::WHITE,
                            );

                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&current_contact.name).strong().size(14.0).color(egui::Color32::WHITE));
                                ui.label(egui::RichText::new(current_contact.last_seen).color(egui::Color32::from_rgb(134, 150, 160)).size(11.0));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("GPU Ativa ✔").color(egui::Color32::from_rgb(0, 168, 132)).size(11.0).strong());
                            });
                        });
                    });

                ui.add_space(8.0);

                let chat_height = height - 120.0;
                egui::ScrollArea::vertical().id_salt("chat_scroll").max_height(chat_height).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    if let Some(msg_list) = self.chats.get(&active_c) {
                        for (msg, is_user, time) in msg_list {
                            ui.horizontal(|ui| {
                                if *is_user {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(0, 92, 75))
                                            .corner_radius(egui::CornerRadius { nw: 8, ne: 0, sw: 8, se: 8 })
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(233, 237, 239)).size(13.0));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                                                        ui.label(egui::RichText::new(format!("{} ✔✔", time)).color(egui::Color32::from_rgb(134, 150, 160)).size(9.0));
                                                    });
                                                });
                                            });
                                    });
                                } else {
                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(32, 44, 51))
                                            .corner_radius(egui::CornerRadius { nw: 0, ne: 8, sw: 8, se: 8 })
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(233, 237, 239)).size(13.0));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                                                        ui.label(egui::RichText::new(time).color(egui::Color32::from_rgb(134, 150, 160)).size(9.0));
                                                    });
                                                });
                                            });
                                    });
                                }
                            });
                            ui.add_space(6.0);
                        }
                    }
                });

                ui.add_space(8.0);

                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(32, 44, 51))
                    .corner_radius(8)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📎").size(16.0).color(egui::Color32::from_rgb(134, 150, 160)));

                            let hint_text = format!("Mensagem para {}...", active_c);
                            let response = ui.add_sized(
                                egui::vec2(ui.available_width() - 80.0, 24.0),
                                egui::TextEdit::singleline(&mut self.input_text)
                                    .hint_text(hint_text)
                                    .margin(egui::vec2(6.0, 4.0))
                            );

                            let trigger_send = ui.button(egui::RichText::new("Enviar").strong().color(egui::Color32::from_rgb(0, 168, 132))).clicked()
                                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                            if trigger_send && !self.input_text.is_empty() {
                                let user_msg = self.input_text.clone();
                                let current_time = "16:47".to_string();

                                self.chats.entry(active_c.clone()).or_default().push(
                                    (user_msg.clone(), true, current_time.clone())
                                );

                                let lower_msg = user_msg.to_lowercase();
                                let bot_reply = if lower_msg.contains("ping") {
                                    "Pong! 🏓 (100% Nativo via Matter JIT Core)".to_string()
                                } else if lower_msg.contains("rico") || lower_msg.contains("foda") {
                                    "Com certeza! Você está criando a linguagem do zero, com JIT e janelas nativas por GPU. Você é um gigante!".to_string()
                                } else if lower_msg.contains("ajuda") {
                                    "Comandos disponíveis: 'ping', 'rico', 'neural' ou 'ajuda'.".to_string()
                                } else if lower_msg.contains("neural") {
                                    "Nossa rede neural é 100% nativa em Matter. Podemos testá-la a qualquer momento!".to_string()
                                } else {
                                    format!("Recebi sua mensagem! O Matter Core v25 processou isso em 0.01ms.")
                                };

                                self.chats.entry(active_c.clone()).or_default().push(
                                    (bot_reply, false, current_time)
                                );

                                self.input_text.clear();
                            }
                        });
                    });
            });
    }
}

struct Particle {
    pos: egui::Pos2,
    vel: egui::Vec2,
    color: egui::Color32,
    size: f32,
}

struct PhysicsStressApp {
    particles: Vec<Particle>,
    particle_count: usize,
    last_time: std::time::Instant,
    fps: f32,
    frame_count: usize,
    fps_timer: std::time::Instant,
}

impl PhysicsStressApp {
    fn new(count: usize) -> Self {
        let mut rng = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut next_random = || {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            (rng as f64 / u64::MAX as f64) as f32
        };

        let mut particles = Vec::with_capacity(20000);
        for _ in 0..count {
            particles.push(Particle {
                pos: egui::pos2(400.0, 300.0),
                vel: egui::vec2((next_random() - 0.5) * 500.0, (next_random() - 0.5) * 500.0),
                color: egui::Color32::from_rgb(
                    (next_random() * 155.0 + 100.0) as u8,
                    (next_random() * 155.0 + 100.0) as u8,
                    (next_random() * 155.0 + 100.0) as u8,
                ),
                size: next_random() * 4.0 + 2.0,
            });
        }

        Self {
            particles,
            particle_count: count,
            last_time: std::time::Instant::now(),
            fps: 60.0,
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
        }
    }
}

impl eframe::App for PhysicsStressApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        let now = std::time::Instant::now();
        let mut dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        if dt > 0.05 {
            dt = 0.05;
        }

        self.frame_count += 1;
        let fps_elapsed = now.duration_since(self.fps_timer).as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / fps_elapsed;
            self.frame_count = 0;
            self.fps_timer = now;
        }

        if self.particles.len() < self.particle_count {
            let mut rng = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            let mut next_random = || {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                (rng as f64 / u64::MAX as f64) as f32
            };
            while self.particles.len() < self.particle_count {
                self.particles.push(Particle {
                    pos: egui::pos2(400.0, 300.0),
                    vel: egui::vec2((next_random() - 0.5) * 500.0, (next_random() - 0.5) * 500.0),
                    color: egui::Color32::from_rgb(
                        (next_random() * 155.0 + 100.0) as u8,
                        (next_random() * 155.0 + 100.0) as u8,
                        (next_random() * 155.0 + 100.0) as u8,
                    ),
                    size: next_random() * 4.0 + 2.0,
                });
            }
        } else if self.particles.len() > self.particle_count {
            self.particles.truncate(self.particle_count);
        }

        let rect = ui.max_rect();
        let width = rect.width();
        let height = rect.height();

        let painter = ui.painter();

        for p in &mut self.particles {
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;

            p.vel.y += 200.0 * dt;

            if p.pos.x < p.size {
                p.pos.x = p.size;
                p.vel.x = -p.vel.x * 0.9;
            } else if p.pos.x > width - p.size {
                p.pos.x = width - p.size;
                p.vel.x = -p.vel.x * 0.9;
            }

            if p.pos.y < p.size {
                p.pos.y = p.size;
                p.vel.y = -p.vel.y * 0.9;
            } else if p.pos.y > height - p.size {
                p.pos.y = height - p.size;
                p.vel.y = -p.vel.y * 0.85;
                p.vel.x *= 0.98;
            }

            painter.circle_filled(p.pos, p.size, p.color);
        }

        egui::Area::new("overlay_panel".into())
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(15.0, 15.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(200))
                    .corner_radius(8)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("⚡ MATTER JIT PHYSICS STRESS TEST")
                                    .strong()
                                    .size(15.0)
                                    .color(egui::Color32::from_rgb(255, 152, 0)),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Performance da GPU:");
                                ui.label(
                                    egui::RichText::new(format!("{:.1} FPS", self.fps))
                                        .strong()
                                        .color(if self.fps > 55.0 {
                                            egui::Color32::from_rgb(0, 230, 118)
                                        } else {
                                            egui::Color32::from_rgb(255, 152, 0)
                                        }),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Quantidade de Partículas:");
                                ui.add(
                                    egui::Slider::new(&mut self.particle_count, 100..=20000)
                                        .text(""),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Aceleração: wgpu (Hardware DirectX12/Vulkan)")
                                    .size(10.0)
                                    .color(egui::Color32::GRAY),
                            );
                        });
                    });
            });

        ctx.request_repaint();
    }
}

#[allow(dead_code)]
struct InteractiveGameEngineApp {
    cam_pos: [f32; 3],
    cam_yaw: f32,
    cam_pitch: f32,
    fov: f32,
    speed: f32,
    ticks: usize,
    ping: u32,
    network_timer: f32,
    network_logs: Vec<String>,
    remote_player_pos: [f32; 3],
    placed_blocks: Vec<[f32; 3]>,
    fps: f32,
    frame_count: usize,
    fps_timer: std::time::Instant,
    last_time: std::time::Instant,
    sky_color: Option<egui::Color32>,
    gravity: f32,
}

#[allow(dead_code)]
impl InteractiveGameEngineApp {
    fn new() -> Self {
        Self::new_with_config(
            vec![[1.5, 0.0, 2.0], [-1.5, 0.0, 2.0], [0.0, 1.0, 3.0]],
            None,
            9.8,
        )
    }

    fn new_with_config(
        custom_blocks: Vec<[f32; 3]>,
        sky_color: Option<egui::Color32>,
        gravity: f32,
    ) -> Self {
        Self {
            cam_pos: [0.0, 1.5, -6.0],
            cam_yaw: 0.0,
            cam_pitch: 0.0,
            fov: 350.0,
            speed: 5.0,
            ticks: 0,
            ping: 15,
            network_timer: 0.0,
            network_logs: vec![
                "[SERVER] Connected to multiplayer room: matter_3d_arena".to_string(),
                "[SERVER] Protocol: UDP-JIT v25 Active".to_string(),
            ],
            remote_player_pos: [2.0, 0.0, 4.0],
            placed_blocks: custom_blocks,
            fps: 60.0,
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
            last_time: std::time::Instant::now(),
            sky_color,
            gravity,
        }
    }
}

fn project_3d(
    p: [f32; 3],
    cam_pos: [f32; 3],
    cam_yaw: f32,
    cam_pitch: f32,
    screen_width: f32,
    screen_height: f32,
    fov: f32,
) -> Option<egui::Pos2> {
    let tx = p[0] - cam_pos[0];
    let ty = p[1] - cam_pos[1];
    let tz = p[2] - cam_pos[2];

    let cos_yaw = cam_yaw.cos();
    let sin_yaw = cam_yaw.sin();
    let rx1 = tx * cos_yaw - tz * sin_yaw;
    let rz1 = tx * sin_yaw + tz * cos_yaw;
    let ry1 = ty;

    let cos_pitch = cam_pitch.cos();
    let sin_pitch = cam_pitch.sin();
    let rx = rx1;
    let ry = ry1 * cos_pitch - rz1 * sin_pitch;
    let rz = ry1 * sin_pitch + rz1 * cos_pitch;

    if rz <= 0.1 {
        return None;
    }

    let scale = fov / rz;
    let screen_x = screen_width / 2.0 + rx * scale;
    let screen_y = screen_height / 2.0 - ry * scale;

    Some(egui::pos2(screen_x, screen_y))
}

impl eframe::App for InteractiveGameEngineApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32().min(0.05);
        self.last_time = now;

        self.frame_count += 1;
        let fps_elapsed = now.duration_since(self.fps_timer).as_secs_f32();
        if fps_elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / fps_elapsed;
            self.frame_count = 0;
            self.fps_timer = now;
        }

        self.ticks += 1;

        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(10, 10, 12);
        ctx.set_visuals(visuals);

        egui::SidePanel::left("engine_hud")
            .resizable(false)
            .default_width(300.0)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(18, 18, 24))
                    .inner_margin(12.0),
            )
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("🎮 MATTER 3D ENGINE JIT")
                            .strong()
                            .size(18.0)
                            .color(egui::Color32::from_rgb(255, 120, 0)),
                    );
                    ui.label(
                        egui::RichText::new("Runtime Multiplayer & Input Hub")
                            .color(egui::Color32::GRAY)
                            .size(10.0),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("🎥 TELEMETRIA DA CÂMERA")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(59, 130, 246)),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Posição X:");
                        ui.label(egui::RichText::new(format!("{:.2}", self.cam_pos[0])).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Posição Y:");
                        ui.label(egui::RichText::new(format!("{:.2}", self.cam_pos[1])).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Posição Z:");
                        ui.label(egui::RichText::new(format!("{:.2}", self.cam_pos[2])).strong());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Yaw / Pitch:");
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.2} rad / {:.2} rad",
                                self.cam_yaw, self.cam_pitch
                            ))
                            .strong(),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("FOV:");
                        ui.add(egui::Slider::new(&mut self.fov, 100.0..=800.0).text(""));
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("⌨ ENTRADA DE TECLADO")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(0, 230, 118)),
                    );
                    ui.add_space(4.0);

                    let w_pressed = ui.input(|i| i.key_down(egui::Key::W));
                    let a_pressed = ui.input(|i| i.key_down(egui::Key::A));
                    let s_pressed = ui.input(|i| i.key_down(egui::Key::S));
                    let d_pressed = ui.input(|i| i.key_down(egui::Key::D));

                    ui.horizontal(|ui| {
                        let btn_w = if w_pressed {
                            egui::Color32::from_rgb(0, 230, 118)
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        let btn_a = if a_pressed {
                            egui::Color32::from_rgb(0, 230, 118)
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        let btn_s = if s_pressed {
                            egui::Color32::from_rgb(0, 230, 118)
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        let btn_d = if d_pressed {
                            egui::Color32::from_rgb(0, 230, 118)
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };

                        ui.colored_label(btn_w, " [ W ] ");
                        ui.colored_label(btn_a, "[ A ]");
                        ui.colored_label(btn_s, "[ S ]");
                        ui.colored_label(btn_d, "[ D ]");
                    });
                    ui.add_space(4.0);
                    ui.label("Teclas: W, A, S, D + Space/Shift");
                    ui.label("Mouse: Arraste para olhar ao redor!");
                    ui.label("Clique no mundo para colocar blocos!");

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🌐 REDE MULTIPLAYER")
                                .strong()
                                .size(12.0)
                                .color(egui::Color32::from_rgb(168, 85, 247)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} ms", self.ping))
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 230, 118)),
                            );
                        });
                    });
                    ui.add_space(4.0);

                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(12, 12, 16))
                        .corner_radius(6)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("net_logs")
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    for log in &self.network_logs {
                                        ui.label(
                                            egui::RichText::new(log)
                                                .color(egui::Color32::from_rgb(150, 150, 180))
                                                .size(10.0),
                                        );
                                    }
                                });
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("FPS:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} FPS", self.fps))
                                .strong()
                                .color(egui::Color32::from_rgb(255, 193, 7)),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Blocos 3D:");
                        ui.label(
                            egui::RichText::new(format!("{}", self.placed_blocks.len())).strong(),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Gravidade:");
                        ui.label(
                            egui::RichText::new(format!("{:.1} m/s²", self.gravity))
                                .strong()
                                .color(egui::Color32::from_rgb(0, 191, 255)),
                        );
                    });
                });
            });

        let bg = self.sky_color.unwrap_or(egui::Color32::from_rgb(8, 8, 10));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(bg))
            .show_inside(ui, |ui| {
                let rect = ui.max_rect();
                let width = rect.width();
                let height = rect.height();

                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                if response.clicked() {
                    let forward_x = self.cam_yaw.sin() * 2.5;
                    let forward_z = self.cam_yaw.cos() * 2.5;
                    let spawn_pos = [
                        self.cam_pos[0] + forward_x,
                        self.cam_pos[1] - 0.2,
                        self.cam_pos[2] + forward_z,
                    ];
                    self.placed_blocks.push(spawn_pos);
                    self.network_logs.push(format!(
                        "[MULTIPLAYER SENT] Placed block at {:?}",
                        spawn_pos
                    ));

                    std::thread::spawn(|| {
                        #[cfg(target_os = "windows")]
                        unsafe {
                            extern "system" {
                                fn Beep(dwFreq: u32, dwDuration: u32) -> i32;
                            }
                            for freq in (800..=1200).step_by(100) {
                                Beep(freq, 10);
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            print!("\x07");
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    });
                }

                if response.dragged() {
                    let delta = response.drag_delta();
                    self.cam_yaw += delta.x * 0.005;
                    self.cam_pitch = (self.cam_pitch - delta.y * 0.005).clamp(-1.4, 1.4);
                }

                let move_speed = self.speed * dt;
                let rot_speed = 1.8 * dt;

                ui.input(|i| {
                    if i.key_down(egui::Key::W) {
                        self.cam_pos[0] += self.cam_yaw.sin() * move_speed;
                        self.cam_pos[2] += self.cam_yaw.cos() * move_speed;
                    }
                    if i.key_down(egui::Key::S) {
                        self.cam_pos[0] -= self.cam_yaw.sin() * move_speed;
                        self.cam_pos[2] -= self.cam_yaw.cos() * move_speed;
                    }
                    if i.key_down(egui::Key::A) {
                        self.cam_pos[0] -= self.cam_yaw.cos() * move_speed;
                        self.cam_pos[2] += self.cam_yaw.sin() * move_speed;
                    }
                    if i.key_down(egui::Key::D) {
                        self.cam_pos[0] += self.cam_yaw.cos() * move_speed;
                        self.cam_pos[2] -= self.cam_yaw.sin() * move_speed;
                    }
                    if i.key_down(egui::Key::Space) {
                        self.cam_pos[1] += move_speed;
                    }
                    if i.modifiers.shift {
                        self.cam_pos[1] -= move_speed;
                    }
                    if i.key_down(egui::Key::ArrowLeft) {
                        self.cam_yaw -= rot_speed;
                    }
                    if i.key_down(egui::Key::ArrowRight) {
                        self.cam_yaw += rot_speed;
                    }
                    if i.key_down(egui::Key::ArrowUp) {
                        self.cam_pitch = (self.cam_pitch + rot_speed).clamp(-1.4, 1.4);
                    }
                    if i.key_down(egui::Key::ArrowDown) {
                        self.cam_pitch = (self.cam_pitch - rot_speed).clamp(-1.4, 1.4);
                    }

                    if i.key_pressed(egui::Key::C) {
                        self.placed_blocks.clear();
                        self.network_logs
                            .push("[WORLD] Cleared all custom blocks!".to_string());
                        std::thread::spawn(|| {
                            #[cfg(target_os = "windows")]
                            unsafe {
                                extern "system" {
                                    fn Beep(dwFreq: u32, dwDuration: u32) -> i32;
                                }
                                for i in 0..5 {
                                    let freq = 800 - i * 100;
                                    Beep(freq, 15);
                                }
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                print!("\x07");
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                            }
                        });
                    }
                });

                self.network_timer += dt;
                if self.network_timer > 0.8 {
                    self.network_timer = 0.0;
                    self.ping = 12
                        + (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                            % 9) as u32;

                    let packet_type = (self.ping % 4) as usize;
                    let log_msg = match packet_type {
                        0 => format!("[SIMULATED UDP] Sync pos for player_921 ({}ms)", self.ping),
                        1 => "[SIMULATED UDP] Broadcast local position success".to_string(),
                        2 => {
                            self.remote_player_pos[0] = 2.0 * (self.ticks as f32 * 0.03).cos();
                            self.remote_player_pos[2] =
                                4.0 + 2.0 * (self.ticks as f32 * 0.03).sin();
                            format!(
                                "[MULTIPLAYER] Player 'matter_pro' moved to {:?}",
                                self.remote_player_pos
                            )
                        }
                        _ => "[SERVER] Handshake keep-alive OK".to_string(),
                    };
                    self.network_logs.push(log_msg);
                    if self.network_logs.len() > 12 {
                        self.network_logs.remove(0);
                    }
                }

                let painter = ui.painter_at(rect);

                let grid_size = 10;
                let grid_y = -1.0;
                for i in -grid_size..=grid_size {
                    let f = i as f32;
                    let p0 = [f, grid_y, -grid_size as f32];
                    let p1 = [f, grid_y, grid_size as f32];

                    let p2 = [-grid_size as f32, grid_y, f];
                    let p3 = [grid_size as f32, grid_y, f];

                    if let (Some(s0), Some(s1)) = (
                        project_3d(
                            p0,
                            self.cam_pos,
                            self.cam_yaw,
                            self.cam_pitch,
                            width,
                            height,
                            self.fov,
                        ),
                        project_3d(
                            p1,
                            self.cam_pos,
                            self.cam_yaw,
                            self.cam_pitch,
                            width,
                            height,
                            self.fov,
                        ),
                    ) {
                        painter.line_segment(
                            [s0, s1],
                            egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_unmultiplied(80, 80, 100, 40),
                            ),
                        );
                    }
                    if let (Some(s2), Some(s3)) = (
                        project_3d(
                            p2,
                            self.cam_pos,
                            self.cam_yaw,
                            self.cam_pitch,
                            width,
                            height,
                            self.fov,
                        ),
                        project_3d(
                            p3,
                            self.cam_pos,
                            self.cam_yaw,
                            self.cam_pitch,
                            width,
                            height,
                            self.fov,
                        ),
                    ) {
                        painter.line_segment(
                            [s2, s3],
                            egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_unmultiplied(80, 80, 100, 40),
                            ),
                        );
                    }
                }

                let draw_cube =
                    |center: [f32; 3], size: f32, color: egui::Color32, painter: &egui::Painter| {
                        let half = size / 2.0;
                        let v = [
                            [center[0] - half, center[1] - half, center[2] - half],
                            [center[0] + half, center[1] - half, center[2] - half],
                            [center[0] + half, center[1] + half, center[2] - half],
                            [center[0] - half, center[1] + half, center[2] - half],
                            [center[0] - half, center[1] - half, center[2] + half],
                            [center[0] + half, center[1] - half, center[2] + half],
                            [center[0] + half, center[1] + half, center[2] + half],
                            [center[0] - half, center[1] + half, center[2] + half],
                        ];

                        let mut s = Vec::new();
                        for pt in &v {
                            s.push(project_3d(
                                *pt,
                                self.cam_pos,
                                self.cam_yaw,
                                self.cam_pitch,
                                width,
                                height,
                                self.fov,
                            ));
                        }

                        let edges = [
                            (0, 1),
                            (1, 2),
                            (2, 3),
                            (3, 0),
                            (4, 5),
                            (5, 6),
                            (6, 7),
                            (7, 4),
                            (0, 4),
                            (1, 5),
                            (2, 6),
                            (3, 7),
                        ];

                        for (start, end) in &edges {
                            if let (Some(p_start), Some(p_end)) = (s[*start], s[*end]) {
                                painter
                                    .line_segment([p_start, p_end], egui::Stroke::new(1.5, color));
                            }
                        }
                    };

                let py_angle = self.ticks as f32 * 0.02;
                let py_cos = py_angle.cos();
                let py_sin = py_angle.sin();
                let py_size = 1.2;

                let py_verts = [
                    [0.0, py_size / 2.0, 3.0],
                    [
                        -py_size / 2.0 * py_cos,
                        -py_size / 2.0,
                        3.0 - py_size / 2.0 * py_sin,
                    ],
                    [
                        py_size / 2.0 * py_cos,
                        -py_size / 2.0,
                        3.0 + py_size / 2.0 * py_sin,
                    ],
                    [
                        -py_size / 2.0 * py_sin,
                        -py_size / 2.0,
                        3.0 + py_size / 2.0 * py_cos,
                    ],
                    [
                        py_size / 2.0 * py_sin,
                        -py_size / 2.0,
                        3.0 - py_size / 2.0 * py_cos,
                    ],
                ];

                let mut py_projected = Vec::new();
                for pt in &py_verts {
                    py_projected.push(project_3d(
                        *pt,
                        self.cam_pos,
                        self.cam_yaw,
                        self.cam_pitch,
                        width,
                        height,
                        self.fov,
                    ));
                }

                let py_edges = [
                    (0, 1),
                    (0, 2),
                    (0, 3),
                    (0, 4),
                    (1, 2),
                    (2, 3),
                    (3, 4),
                    (4, 1),
                ];

                for (start, end) in &py_edges {
                    if let (Some(p_start), Some(p_end)) = (py_projected[*start], py_projected[*end])
                    {
                        painter.line_segment(
                            [p_start, p_end],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 80, 0)),
                        );
                    }
                }

                for block in &self.placed_blocks {
                    draw_cube(*block, 0.8, egui::Color32::from_rgb(0, 150, 255), &painter);
                }

                draw_cube(
                    self.remote_player_pos,
                    0.9,
                    egui::Color32::from_rgb(168, 85, 247),
                    &painter,
                );

                painter.text(
                    egui::pos2(width / 2.0, 30.0),
                    egui::Align2::CENTER_CENTER,
                    "🎮 MATTER 3D JIT MULTIPLAYER WORLD",
                    egui::FontId::proportional(16.0),
                    egui::Color32::from_rgb(255, 255, 255),
                );
            });

        ctx.request_repaint();
    }
}

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
    layouts: Vec<VisualLayoutSpec>,
    components: HashMap<String, VisualComponentSpec>,
    loop_state: VisualLoopState,
    editor_enabled: bool,
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
            layouts: Vec::new(),
            components: HashMap::new(),
            loop_state: VisualLoopState {
                frame: 0,
                time_ms: 0,
                running: false,
            },
            editor_enabled: false,
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

    pub fn export_web_runtime(&self, dir: &str, app_name: &str) -> Result<(), VisualError> {
        let root = Path::new(dir);
        fs::create_dir_all(root).map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        let index = render_pxl_canvas(self);
        let pxl = self.pxl_snapshot();
        let manifest = render_web_manifest(app_name);
        let package = render_web_package_manifest(app_name);
        let lock = render_web_package_lock(
            app_name,
            &[
                ("index.html", &index),
                ("pxl.json", &pxl),
                ("manifest.json", &manifest),
                ("matter-package.json", &package),
            ],
        );

        fs::write(root.join("index.html"), index)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        fs::write(root.join("pxl.json"), pxl)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        fs::write(root.join("manifest.json"), manifest)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        fs::write(root.join("matter-package.json"), package)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        fs::write(root.join("matter-lock.json"), lock)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        Ok(())
    }

    pub fn verify_web_runtime(&self, dir: &str) -> Result<Value, VisualError> {
        verify_web_package_lock(Path::new(dir))
    }

    pub fn save_state(&self, path: &str) -> Result<(), VisualError> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
            }
        }

        fs::write(path, render_visual_state_document(self))
            .map_err(|error| VisualError::RuntimeError(error.to_string()))
    }

    pub fn load_state(&mut self, path: &str) -> Result<(), VisualError> {
        let content = fs::read_to_string(path)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        apply_visual_state_document(self, &content)
    }

    pub fn load_events(&self, path: &str) -> Result<Value, VisualError> {
        let content = fs::read_to_string(path)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        parse_visual_event_document(&content)
    }

    pub fn dispatch_events(&mut self, path: &str) -> Result<Value, VisualError> {
        let content = fs::read_to_string(path)
            .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        apply_visual_event_document(self, &content)
    }

    pub fn app_step(&mut self, event_path: &str, delta_ms: i64) -> Result<Value, VisualError> {
        if delta_ms <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.app_step delta must be greater than 0".to_string(),
            ));
        }
        let dispatch = self.dispatch_events(event_path)?;
        let loop_state = self.tick(delta_ms)?;

        Ok(Value::new_map(HashMap::from([
            ("dispatch".to_string(), dispatch),
            ("loop".to_string(), loop_state),
            (
                "snapshot".to_string(),
                Value::new_string(self.pxl_snapshot()),
            ),
        ])))
    }

    pub fn app_run(
        &mut self,
        event_path: &str,
        frames: i64,
        delta_ms: i64,
    ) -> Result<Value, VisualError> {
        if frames <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.app_run frames must be greater than 0".to_string(),
            ));
        }
        if delta_ms <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.app_run delta must be greater than 0".to_string(),
            ));
        }
        let dispatch = self.dispatch_events(event_path)?;
        let loop_state = self.run_loop(frames, delta_ms)?;

        Ok(Value::new_map(HashMap::from([
            ("dispatch".to_string(), dispatch),
            ("loop".to_string(), loop_state),
            ("frames".to_string(), Value::Int(frames)),
            (
                "snapshot".to_string(),
                Value::new_string(self.pxl_snapshot()),
            ),
        ])))
    }

    pub fn tick(&mut self, delta_ms: i64) -> Result<Value, VisualError> {
        if delta_ms <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.tick delta must be greater than 0".to_string(),
            ));
        }
        self.loop_state.frame += 1;
        self.loop_state.time_ms += delta_ms;
        self.loop_state.running = true;
        Ok(loop_state_value(&self.loop_state))
    }

    pub fn run_loop(&mut self, frames: i64, delta_ms: i64) -> Result<Value, VisualError> {
        if frames <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.loop frames must be greater than 0".to_string(),
            ));
        }
        if delta_ms <= 0 {
            return Err(VisualError::InvalidArgument(
                "visual.loop delta must be greater than 0".to_string(),
            ));
        }
        for _ in 0..frames {
            self.tick(delta_ms)?;
        }
        Ok(loop_state_value(&self.loop_state))
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
                .or_insert_with(|| Value::new_string(scene.clone()));
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
            println!(
                "[VISUAL] set {} {} = {}",
                target,
                key,
                value.to_display_string()
            );
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
            "run_game_engine" => {
                let title = args
                    .get(0)
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "Matter Core 3D JIT Game Engine".to_string());

                // Parse optional configuration map
                let mut custom_blocks = vec![[1.5, 0.0, 2.0], [-1.5, 0.0, 2.0], [0.0, 1.0, 3.0]];
                let mut sky_color = None;
                let mut custom_gravity = 9.8f32;

                if let Some(Value::Map(cfg)) = args.get(1) {
                    if let Some(Value::List(blocks_val)) = cfg.get("blocks") {
                        let mut loaded_blocks = Vec::new();
                        for val in blocks_val.iter() {
                            if let Value::List(coord) = val {
                                if coord.len() >= 3 {
                                    let x = coord[0]
                                        .as_float()
                                        .unwrap_or_else(|_| coord[0].as_int().unwrap_or(0) as f64)
                                        as f32;
                                    let y = coord[1]
                                        .as_float()
                                        .unwrap_or_else(|_| coord[1].as_int().unwrap_or(0) as f64)
                                        as f32;
                                    let z = coord[2]
                                        .as_float()
                                        .unwrap_or_else(|_| coord[2].as_int().unwrap_or(0) as f64)
                                        as f32;
                                    loaded_blocks.push([x, y, z]);
                                }
                            }
                        }
                        if !loaded_blocks.is_empty() {
                            custom_blocks = loaded_blocks;
                        }
                    }
                    if let Some(Value::List(color_val)) = cfg.get("sky_color") {
                        if color_val.len() >= 3 {
                            let r = color_val[0].as_int().unwrap_or(10) as u8;
                            let g = color_val[1].as_int().unwrap_or(10) as u8;
                            let b = color_val[2].as_int().unwrap_or(12) as u8;
                            sky_color = Some(egui::Color32::from_rgb(r, g, b));
                        }
                    }
                    if let Some(gravity_val) = cfg.get("gravity") {
                        custom_gravity = gravity_val
                            .as_float()
                            .unwrap_or_else(|_| gravity_val.as_int().unwrap_or(9) as f64)
                            as f32;
                    }
                }

                let options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
                    ..Default::default()
                };

                let result = eframe::run_native(
                    &title,
                    options,
                    Box::new(move |_cc| {
                        Ok(Box::new(InteractiveGameEngineApp::new_with_config(
                            custom_blocks,
                            sky_color,
                            custom_gravity,
                        )))
                    }),
                );

                if let Err(e) = result {
                    return Err(format!("Failed to start native window: {}", e));
                }

                Ok(Value::Bool(true))
            }
            "run_physics_test" => {
                let count = args.get(0).and_then(|v| v.as_int().ok()).unwrap_or(5000) as usize;
                let title = "Matter JIT GPU Physical Stress Test - 5000+ Particles";

                let options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
                    ..Default::default()
                };

                let result = eframe::run_native(
                    title,
                    options,
                    Box::new(move |_cc| Ok(Box::new(PhysicsStressApp::new(count)))),
                );

                if let Err(e) = result {
                    return Err(format!("Failed to start native window: {}", e));
                }

                Ok(Value::Bool(true))
            }
            "create_window" => {
                let title = args
                    .get(0)
                    .map(|v| v.to_display_string())
                    .unwrap_or_else(|| "Matter Native App".to_string());

                let options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
                    ..Default::default()
                };

                let result = if title.to_lowercase().contains("antigravity") {
                    eframe::run_native(
                        &title,
                        options,
                        Box::new(|_cc| Ok(Box::new(AntigravityApp::default()))),
                    )
                } else {
                    eframe::run_native(
                        &title,
                        options,
                        Box::new(|_cc| Ok(Box::new(MatterApp::default()))),
                    )
                };

                if let Err(e) = result {
                    return Err(format!("Failed to start native window: {}", e));
                }

                Ok(Value::Bool(true))
            }
            "run" => {
                if args.len() != 1 {
                    return Err(format!("visual.run expects 1 argument, got {}", args.len()));
                }
                let name = args[0]
                    .as_string()
                    .map_err(|_| "visual.run expects string argument".to_string())?;
                self.run_app(&name).map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "load" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.load expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.load expects string argument".to_string())?;
                self.load_pvmbc(&path).map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "surface" => {
                if args.len() != 3 {
                    return Err(format!(
                        "visual.surface expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let name = args[0]
                    .as_string()
                    .map_err(|_| "visual.surface expects string name".to_string())?;
                let width = args[1]
                    .as_int()
                    .map_err(|_| "visual.surface expects int width".to_string())?;
                let height = args[2]
                    .as_int()
                    .map_err(|_| "visual.surface expects int height".to_string())?;
                self.create_surface(&name, width, height)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "scene" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.scene expects 1 argument, got {}",
                        args.len()
                    ));
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
            "layout" => {
                if args.len() != 3 {
                    return Err(format!(
                        "visual.layout expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let scene = args[0]
                    .as_string()
                    .map_err(|_| "visual.layout expects string scene".to_string())?;
                let kind = args[1]
                    .as_string()
                    .map_err(|_| "visual.layout expects string kind".to_string())?;
                let gap = args[2]
                    .as_int()
                    .map_err(|_| "visual.layout expects int gap".to_string())?;
                if gap < 0 {
                    return Err("visual.layout gap must be greater than or equal to 0".to_string());
                }
                match kind.as_str() {
                    "absolute" | "vertical" | "horizontal" | "grid" => {}
                    _ => {
                        return Err(
                            "visual.layout kind must be absolute, vertical, horizontal, or grid"
                                .to_string(),
                        );
                    }
                }
                if !self.scenes.iter().any(|known_scene| known_scene == &scene) {
                    self.scenes.push(scene.clone());
                }
                if let Some(layout) = self.layouts.iter_mut().find(|layout| layout.scene == scene) {
                    layout.kind = kind;
                    layout.gap = gap;
                } else {
                    self.layouts.push(VisualLayoutSpec { scene, kind, gap });
                }
                Ok(Value::Unit)
            }
            "component" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.component expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let name = args[0]
                    .as_string()
                    .map_err(|_| "visual.component expects string name".to_string())?;
                let defaults = value_map("visual.component defaults", &args[1])?.clone();
                if !defaults.contains_key("w") || !defaults.contains_key("h") {
                    return Err("visual.component defaults require int w and h".to_string());
                }
                defaults
                    .get("w")
                    .ok_or_else(|| "visual.component defaults require int w".to_string())?
                    .as_int()
                    .map_err(|_| "visual.component defaults w must be int".to_string())?;
                defaults
                    .get("h")
                    .ok_or_else(|| "visual.component defaults require int h".to_string())?
                    .as_int()
                    .map_err(|_| "visual.component defaults h must be int".to_string())?;
                self.components
                    .insert(name.clone(), VisualComponentSpec { name, defaults });
                Ok(Value::Unit)
            }
            "mount" | "use" => {
                if args.len() != 4 {
                    return Err(format!(
                        "visual.{} expects 4 arguments, got {}",
                        method,
                        args.len()
                    ));
                }
                let component_name = args[0]
                    .as_string()
                    .map_err(|_| format!("visual.{} expects string component", method))?;
                let name = args[1]
                    .as_string()
                    .map_err(|_| format!("visual.{} expects string name", method))?;
                let x = args[2]
                    .as_int()
                    .map_err(|_| format!("visual.{} expects int x", method))?;
                let y = args[3]
                    .as_int()
                    .map_err(|_| format!("visual.{} expects int y", method))?;
                let component = self
                    .components
                    .get(&component_name)
                    .cloned()
                    .ok_or_else(|| {
                        format!("visual.{} unknown component '{}'", method, component_name)
                    })?;
                let w = component_int(&component, "w")?;
                let h = component_int(&component, "h")?;
                let region = VisualRegionSpec {
                    name: name.clone(),
                    x,
                    y,
                    w,
                    h,
                    semantic: component_string(&component, "semantic")?,
                    behavior: component_string(&component, "behavior")?,
                    material: component_string(&component, "material")?,
                    energy: component_int_opt(&component, "energy")?.map(|value| value as f64),
                };
                self.create_region(region).map_err(|e| e.to_string())?;
                self.set_property(&name, "component", Value::new_string(component_name))
                    .map_err(|e| e.to_string())?;
                for (key, value) in component.defaults {
                    if !component_region_key(&key) {
                        self.set_property(&name, &key, value)
                            .map_err(|e| e.to_string())?;
                    }
                }
                Ok(Value::Unit)
            }
            "region" => {
                // Forma simples: visual.region(name, x, y, w, h)
                if args.len() == 5 {
                    let name = args[0]
                        .as_string()
                        .map_err(|_| "visual.region expects string name".to_string())?;
                    let x = args[1]
                        .as_int()
                        .map_err(|_| "visual.region expects int x".to_string())?;
                    let y = args[2]
                        .as_int()
                        .map_err(|_| "visual.region expects int y".to_string())?;
                    let w = args[3]
                        .as_int()
                        .map_err(|_| "visual.region expects int w".to_string())?;
                    let h = args[4]
                        .as_int()
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

                    self.create_region(region).map_err(|e| e.to_string())?;
                    Ok(Value::Unit)
                }
                // Forma com map (futuro): visual.region(name, {x: 100, y: 200, ...})
                else if args.len() == 2 {
                    let name = args[0]
                        .as_string()
                        .map_err(|_| "visual.region expects string name".to_string())?;

                    // Extrair propriedades do map
                    if let Value::Map(ref props) = args[1] {
                        let x = props
                            .get("x")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'x' property".to_string())?;
                        let y = props
                            .get("y")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'y' property".to_string())?;
                        let w = props
                            .get("w")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'w' property".to_string())?;
                        let h = props
                            .get("h")
                            .and_then(|v| v.as_int().ok())
                            .ok_or_else(|| "visual.region map requires 'h' property".to_string())?;

                        let semantic = props.get("semantic").and_then(|v| v.as_string().ok());
                        let behavior = props.get("behavior").and_then(|v| v.as_string().ok());
                        let material = props.get("material").and_then(|v| v.as_string().ok());
                        let energy = props
                            .get("energy")
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

                        self.create_region(region).map_err(|e| e.to_string())?;
                        Ok(Value::Unit)
                    } else {
                        Err("visual.region expects map as second argument".to_string())
                    }
                } else {
                    Err(format!(
                        "visual.region expects 2 or 5 arguments, got {}",
                        args.len()
                    ))
                }
            }
            "pulse" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.pulse expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.pulse expects string argument".to_string())?;
                self.pulse(&target).map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "set" => {
                if args.len() != 3 {
                    return Err(format!(
                        "visual.set expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.set expects string target".to_string())?;
                let key = args[1]
                    .as_string()
                    .map_err(|_| "visual.set expects string key".to_string())?;
                let value = args[2].clone();
                self.set_property(&target, &key, value)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "state" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.state expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.state expects string target".to_string())?;
                let state = args[1]
                    .as_string()
                    .map_err(|_| "visual.state expects string state".to_string())?;
                self.set_property(&target, "state", Value::new_string(state))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "layer" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.layer expects 2 arguments, got {}",
                        args.len()
                    ));
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
                    return Err(format!(
                        "visual.camera expects 3 arguments, got {}",
                        args.len()
                    ));
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
                    return Err(format!(
                        "visual.input expects 3 arguments, got {}",
                        args.len()
                    ));
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
                    return Err(format!(
                        "visual.motion expects 3 arguments, got {}",
                        args.len()
                    ));
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
                self.set_property(&target, "motion", Value::new_string(kind))
                    .map_err(|e| e.to_string())?;
                self.set_property(&target, "motionSpeed", Value::Int(speed))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "sprite" => {
                if args.len() != 3 {
                    return Err(format!(
                        "visual.sprite expects 3 arguments, got {}",
                        args.len()
                    ));
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
                self.set_property(&target, "sprite", Value::new_string(source))
                    .map_err(|e| e.to_string())?;
                self.set_property(&target, "spriteFit", Value::new_string(fit))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "text" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.text expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let target = args[0]
                    .as_string()
                    .map_err(|_| "visual.text expects string target".to_string())?;
                let text = args[1]
                    .as_string()
                    .map_err(|_| "visual.text expects string text".to_string())?;
                self.set_property(&target, "text", Value::new_string(text))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "visible" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.visible expects 2 arguments, got {}",
                        args.len()
                    ));
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
                    return Err(format!(
                        "visual.theme expects 2 arguments, got {}",
                        args.len()
                    ));
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
                    return Err(format!(
                        "visual.snapshot expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(Value::new_string(self.pxl_snapshot()))
            }
            "save_state" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.save_state expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.save_state expects string path".to_string())?;
                self.save_state(&path).map_err(|e| e.to_string())?;
                Ok(Value::new_string(path))
            }
            "load_state" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.load_state expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.load_state expects string path".to_string())?;
                self.load_state(&path).map_err(|e| e.to_string())?;
                Ok(Value::new_string(path))
            }
            "load_events" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.load_events expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.load_events expects string path".to_string())?;
                self.load_events(&path).map_err(|e| e.to_string())
            }
            "dispatch_events" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.dispatch_events expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.dispatch_events expects string path".to_string())?;
                self.dispatch_events(&path).map_err(|e| e.to_string())
            }
            "app_step" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.app_step expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let event_path = args[0]
                    .as_string()
                    .map_err(|_| "visual.app_step expects string event path".to_string())?;
                let delta_ms = args[1]
                    .as_int()
                    .map_err(|_| "visual.app_step expects int delta".to_string())?;
                self.app_step(&event_path, delta_ms)
                    .map_err(|e| e.to_string())
            }
            "app_run" => {
                if args.len() != 3 {
                    return Err(format!(
                        "visual.app_run expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let event_path = args[0]
                    .as_string()
                    .map_err(|_| "visual.app_run expects string event path".to_string())?;
                let frames = args[1]
                    .as_int()
                    .map_err(|_| "visual.app_run expects int frames".to_string())?;
                let delta_ms = args[2]
                    .as_int()
                    .map_err(|_| "visual.app_run expects int delta".to_string())?;
                self.app_run(&event_path, frames, delta_ms)
                    .map_err(|e| e.to_string())
            }
            "tick" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.tick expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let delta_ms = args[0]
                    .as_int()
                    .map_err(|_| "visual.tick expects int delta".to_string())?;
                self.tick(delta_ms).map_err(|e| e.to_string())
            }
            "loop" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.loop expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let frames = args[0]
                    .as_int()
                    .map_err(|_| "visual.loop expects int frames".to_string())?;
                let delta_ms = args[1]
                    .as_int()
                    .map_err(|_| "visual.loop expects int delta".to_string())?;
                self.run_loop(frames, delta_ms).map_err(|e| e.to_string())
            }
            "editor" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.editor expects 1 argument, got {}",
                        args.len()
                    ));
                }
                self.editor_enabled = args[0]
                    .as_bool()
                    .map_err(|_| "visual.editor expects bool enabled".to_string())?;
                Ok(Value::Bool(self.editor_enabled))
            }
            "export" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.export expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.export expects string path".to_string())?;
                self.export_pxl(&path).map_err(|e| e.to_string())?;
                Ok(Value::new_string(path))
            }
            "preview" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.preview expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.preview expects string path".to_string())?;
                self.export_preview(&path).map_err(|e| e.to_string())?;
                Ok(Value::new_string(path))
            }
            "canvas" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.canvas expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|_| "visual.canvas expects string path".to_string())?;
                self.export_canvas(&path).map_err(|e| e.to_string())?;
                Ok(Value::new_string(path))
            }
            "web" => {
                if args.len() != 2 {
                    return Err(format!(
                        "visual.web expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let dir = args[0]
                    .as_string()
                    .map_err(|_| "visual.web expects string dir".to_string())?;
                let app_name = args[1]
                    .as_string()
                    .map_err(|_| "visual.web expects string app name".to_string())?;
                self.export_web_runtime(&dir, &app_name)
                    .map_err(|e| e.to_string())?;
                Ok(Value::new_string(dir))
            }
            "verify_web" => {
                if args.len() != 1 {
                    return Err(format!(
                        "visual.verify_web expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let dir = args[0]
                    .as_string()
                    .map_err(|_| "visual.verify_web expects string dir".to_string())?;
                self.verify_web_runtime(&dir).map_err(|e| e.to_string())
            }
            _ => Err(format!("Unknown visual method: {}", method)),
        }
    }
}

fn value_map<'a>(context: &str, value: &'a Value) -> Result<&'a HashMap<String, Value>, String> {
    match value {
        Value::Map(values) => Ok(values),
        _ => Err(format!("{} must be a map", context)),
    }
}

fn component_int(component: &VisualComponentSpec, key: &str) -> Result<i64, String> {
    component
        .defaults
        .get(key)
        .ok_or_else(|| format!("visual.component '{}' requires int {}", component.name, key))?
        .as_int()
        .map_err(|_| format!("visual.component '{}' {} must be int", component.name, key))
}

fn component_int_opt(component: &VisualComponentSpec, key: &str) -> Result<Option<i64>, String> {
    component
        .defaults
        .get(key)
        .map(|value| {
            value
                .as_int()
                .map_err(|_| format!("visual.component '{}' {} must be int", component.name, key))
        })
        .transpose()
}

fn component_string(component: &VisualComponentSpec, key: &str) -> Result<Option<String>, String> {
    component
        .defaults
        .get(key)
        .map(|value| {
            value.as_string().map_err(|_| {
                format!(
                    "visual.component '{}' {} must be string",
                    component.name, key
                )
            })
        })
        .transpose()
}

fn component_region_key(key: &str) -> bool {
    matches!(
        key,
        "x" | "y" | "w" | "h" | "semantic" | "behavior" | "material" | "energy" | "scene"
    )
}

fn loop_state_value(loop_state: &VisualLoopState) -> Value {
    Value::new_map(HashMap::from([
        ("frame".to_string(), Value::Int(loop_state.frame)),
        ("timeMs".to_string(), Value::Int(loop_state.time_ms)),
        ("running".to_string(), Value::Bool(loop_state.running)),
    ]))
}

fn render_loop_state(loop_state: &VisualLoopState) -> String {
    format!(
        "{{\"frame\":{},\"timeMs\":{},\"running\":{}}}",
        loop_state.frame, loop_state.time_ms, loop_state.running
    )
}

fn render_editor_state(enabled: bool) -> String {
    format!("{{\"enabled\":{}}}", enabled)
}

fn render_web_manifest(app_name: &str) -> String {
    format!(
        "{{\"format\":\"MATTER_WEB_RUNTIME\",\"version\":1,\"app\":\"{}\",\"entry\":\"index.html\",\"pxl\":\"pxl.json\",\"runtime\":\"pxl-canvas\",\"package\":\"matter-package.json\",\"lock\":\"matter-lock.json\",\"files\":[\"index.html\",\"pxl.json\",\"manifest.json\",\"matter-package.json\",\"matter-lock.json\"]}}",
        json_escape(app_name)
    )
}

fn render_web_package_manifest(app_name: &str) -> String {
    let package_name = package_slug(app_name);
    format!(
        "{{\"format\":\"MATTER_PACKAGE\",\"version\":1,\"name\":\"{}\",\"displayName\":\"{}\",\"kind\":\"web-runtime\",\"entry\":\"index.html\",\"lock\":\"matter-lock.json\",\"artifacts\":[{{\"kind\":\"pxl\",\"path\":\"pxl.json\"}},{{\"kind\":\"web\",\"path\":\"index.html\"}}],\"dependencies\":{{\"matter.pxl\":\"^0.1.0\"}}}}",
        json_escape(&package_name),
        json_escape(app_name)
    )
}

fn render_web_package_lock(app_name: &str, files: &[(&str, &str)]) -> String {
    let package_name = package_slug(app_name);
    let entries = files
        .iter()
        .map(|(path, content)| {
            format!(
                "{{\"path\":\"{}\",\"bytes\":{},\"fingerprint\":\"{}\"}}",
                json_escape(path),
                content.len(),
                stable_fingerprint(content.as_bytes())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"format\":\"MATTER_LOCK\",\"version\":1,\"package\":\"{}\",\"files\":[{}]}}",
        json_escape(&package_name),
        entries
    )
}

fn stable_fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn verify_web_package_lock(root: &Path) -> Result<Value, VisualError> {
    let lock_path = root.join("matter-lock.json");
    let content = fs::read_to_string(&lock_path)
        .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
    let document: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
    let format = document
        .get("format")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            VisualError::InvalidArgument("matter-lock.json missing format".to_string())
        })?;
    if format != "MATTER_LOCK" {
        return Err(VisualError::InvalidArgument(format!(
            "matter-lock.json format must be MATTER_LOCK, got {}",
            format
        )));
    }
    let package = document
        .get("package")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let files = document
        .get("files")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            VisualError::InvalidArgument("matter-lock.json files must be an array".to_string())
        })?;

    let mut ok = true;
    let mut verified_files = Vec::new();
    for file in files {
        let path = file
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                VisualError::InvalidArgument("matter-lock.json file missing path".to_string())
            })?;
        let expected_bytes = file
            .get("bytes")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as i64;
        let expected_fingerprint = file
            .get("fingerprint")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let file_path = root.join(path);
        let bytes =
            fs::read(&file_path).map_err(|error| VisualError::RuntimeError(error.to_string()))?;
        let actual_bytes = bytes.len() as i64;
        let actual_fingerprint = stable_fingerprint(&bytes);
        let file_ok = actual_bytes == expected_bytes && actual_fingerprint == expected_fingerprint;
        ok = ok && file_ok;
        verified_files.push(Value::new_map(HashMap::from([
            ("path".to_string(), Value::new_string(path.to_string())),
            ("ok".to_string(), Value::Bool(file_ok)),
            ("bytes".to_string(), Value::Int(actual_bytes)),
            (
                "fingerprint".to_string(),
                Value::new_string(actual_fingerprint),
            ),
        ])));
    }

    Ok(Value::new_map(HashMap::from([
        ("ok".to_string(), Value::Bool(ok)),
        ("package".to_string(), Value::new_string(package)),
        ("files".to_string(), Value::new_list(verified_files)),
    ])))
}

fn package_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "matter-app".to_string()
    } else {
        slug
    }
}

fn render_visual_state_document(backend: &TraceVisualBackend) -> String {
    let mut regions: Vec<&VisualRegionSpec> = backend.regions.values().collect();
    regions.sort_by(|left, right| left.name.cmp(&right.name));
    let region_json = regions
        .into_iter()
        .map(|region| {
            let properties = backend
                .properties
                .get(&region.name)
                .map(value_map_json)
                .unwrap_or_else(|| "{}".to_string());
            format!(
                "{{\"name\":\"{}\",\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"properties\":{}}}",
                json_escape(&region.name),
                region.x,
                region.y,
                region.w,
                region.h,
                properties
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let active_scene = backend
        .current_scene
        .clone()
        .unwrap_or_else(|| "main".to_string());

    format!(
        "{{\"format\":\"PXL_STATE\",\"version\":1,\"activeScene\":\"{}\",\"regions\":[{}]}}",
        json_escape(&active_scene),
        region_json
    )
}

fn apply_visual_state_document(
    backend: &mut TraceVisualBackend,
    content: &str,
) -> Result<(), VisualError> {
    let document: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
    let format = document
        .get("format")
        .and_then(|value| value.as_str())
        .ok_or_else(|| VisualError::InvalidArgument("visual state missing format".to_string()))?;
    if format != "PXL_STATE" {
        return Err(VisualError::InvalidArgument(format!(
            "visual state format must be PXL_STATE, got {}",
            format
        )));
    }

    if let Some(active_scene) = document.get("activeScene").and_then(|value| value.as_str()) {
        if !backend.scenes.iter().any(|scene| scene == active_scene) {
            backend.scenes.push(active_scene.to_string());
        }
        backend.current_scene = Some(active_scene.to_string());
    }

    let regions = document
        .get("regions")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            VisualError::InvalidArgument("visual state regions must be an array".to_string())
        })?;
    for region_state in regions {
        let name = region_state
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                VisualError::InvalidArgument("visual state region missing name".to_string())
            })?;
        if let Some(region) = backend.regions.get_mut(name) {
            if let Some(x) = region_state.get("x").and_then(|value| value.as_i64()) {
                region.x = x;
            }
            if let Some(y) = region_state.get("y").and_then(|value| value.as_i64()) {
                region.y = y;
            }
            if let Some(w) = region_state.get("w").and_then(|value| value.as_i64()) {
                region.w = w;
            }
            if let Some(h) = region_state.get("h").and_then(|value| value.as_i64()) {
                region.h = h;
            }
        }
        if let Some(properties) = region_state
            .get("properties")
            .and_then(|value| value.as_object())
        {
            let entry = backend.properties.entry(name.to_string()).or_default();
            for (key, value) in properties {
                entry.insert(key.clone(), json_value_to_backend_value(value));
            }
        }
    }

    Ok(())
}

fn json_value_to_backend_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Bool(value) => Value::Bool(*value),
        serde_json::Value::Number(value) => Value::Int(value.as_i64().unwrap_or_default()),
        serde_json::Value::String(value) => Value::new_string(value.clone()),
        serde_json::Value::Array(values) => {
            Value::new_list(values.iter().map(json_value_to_backend_value).collect())
        }
        serde_json::Value::Object(values) => Value::new_map(
            values
                .iter()
                .map(|(key, value)| (key.clone(), json_value_to_backend_value(value)))
                .collect(),
        ),
    }
}

fn parse_visual_event_document(content: &str) -> Result<Value, VisualError> {
    let document: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
    let format = document
        .get("format")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            VisualError::InvalidArgument("visual event document missing format".to_string())
        })?;
    if format != "PXL_TRACE" && format != "PXL_EVENT_QUEUE" {
        return Err(VisualError::InvalidArgument(format!(
            "visual event format must be PXL_TRACE or PXL_EVENT_QUEUE, got {}",
            format
        )));
    }
    let events = document
        .get("events")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            VisualError::InvalidArgument(
                "visual event document events must be an array".to_string(),
            )
        })?;

    Ok(Value::new_list(
        events.iter().map(json_value_to_backend_value).collect(),
    ))
}

fn apply_visual_event_document(
    backend: &mut TraceVisualBackend,
    content: &str,
) -> Result<Value, VisualError> {
    let document: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| VisualError::RuntimeError(error.to_string()))?;
    let format = document
        .get("format")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            VisualError::InvalidArgument("visual event document missing format".to_string())
        })?;
    if format != "PXL_TRACE" && format != "PXL_EVENT_QUEUE" {
        return Err(VisualError::InvalidArgument(format!(
            "visual event format must be PXL_TRACE or PXL_EVENT_QUEUE, got {}",
            format
        )));
    }
    let events = document
        .get("events")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            VisualError::InvalidArgument(
                "visual event document events must be an array".to_string(),
            )
        })?;

    let mut selected = String::new();
    let mut active_scene = backend.current_scene.clone().unwrap_or_default();
    let mut moved = 0;

    for event in events {
        let Some(event_object) = event.as_object() else {
            continue;
        };
        let event_name = event_object
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        if let Some(scene) = event_object.get("scene").and_then(|value| value.as_str()) {
            backend.current_scene = Some(scene.to_string());
            active_scene = scene.to_string();
            if !backend.scenes.iter().any(|existing| existing == scene) {
                backend.scenes.push(scene.to_string());
            }
        }
        if let Some(target) = event_object.get("target").and_then(|value| value.as_str()) {
            selected = target.to_string();
            let entry = backend.properties.entry(target.to_string()).or_default();
            entry.insert("state".to_string(), Value::new_string("active".to_string()));
            entry.insert("selected".to_string(), Value::Bool(true));
            entry.insert(
                "lastEvent".to_string(),
                Value::new_string(event_name.clone()),
            );

            if event_name == "editor_move"
                || event_object
                    .get("type")
                    .and_then(|value| value.as_str())
                    .is_some_and(|kind| kind == "editor_move")
            {
                if let Some(region) = backend.regions.get_mut(target) {
                    if let Some(x) = event_object.get("x").and_then(|value| value.as_i64()) {
                        region.x = x;
                    }
                    if let Some(y) = event_object.get("y").and_then(|value| value.as_i64()) {
                        region.y = y;
                    }
                    moved += 1;
                }
            }
        }
    }

    Ok(Value::new_map(HashMap::from([
        ("processed".to_string(), Value::Int(events.len() as i64)),
        ("moved".to_string(), Value::Int(moved)),
        ("selected".to_string(), Value::new_string(selected)),
        ("activeScene".to_string(), Value::new_string(active_scene)),
        (
            "events".to_string(),
            Value::new_list(events.iter().map(json_value_to_backend_value).collect()),
        ),
    ])))
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
    let mut layouts: Vec<&VisualLayoutSpec> = backend.layouts.iter().collect();
    layouts.sort_by(|left, right| left.scene.cmp(&right.scene));
    let layout_json = layouts
        .into_iter()
        .map(render_layout)
        .collect::<Vec<_>>()
        .join(",");
    let mut components: Vec<&VisualComponentSpec> = backend.components.values().collect();
    components.sort_by(|left, right| left.name.cmp(&right.name));
    let component_json = components
        .into_iter()
        .map(render_component)
        .collect::<Vec<_>>()
        .join(",");
    let loop_json = render_loop_state(&backend.loop_state);
    let editor_json = render_editor_state(backend.editor_enabled);
    let scenes = if backend.scenes.is_empty() {
        vec!["main".to_string()]
    } else {
        backend.scenes.clone()
    };
    let scene_json = string_array_json(&scenes);
    let active_scene = backend.current_scene.clone().unwrap_or_else(|| {
        scenes
            .first()
            .cloned()
            .unwrap_or_else(|| "main".to_string())
    });

    let camera_json = backend
        .camera
        .as_ref()
        .map(render_camera)
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"format\":\"PXL\",\"version\":1,\"surfaces\":[{}],\"regions\":[{}],\"pulses\":[{}],\"camera\":{},\"inputs\":[{}],\"theme\":{},\"scenes\":{},\"activeScene\":\"{}\",\"layouts\":[{}],\"components\":[{}],\"loop\":{},\"editor\":{},\"loaded\":{},\"apps\":{}}}",
        surface_json,
        region_json,
        pulse_json,
        camera_json,
        input_json,
        theme_json,
        scene_json,
        json_escape(&active_scene),
        layout_json,
        component_json,
        loop_json,
        editor_json,
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

fn render_layout(layout: &VisualLayoutSpec) -> String {
    format!(
        "{{\"scene\":\"{}\",\"kind\":\"{}\",\"gap\":{}}}",
        json_escape(&layout.scene),
        json_escape(&layout.kind),
        layout.gap
    )
}

fn render_component(component: &VisualComponentSpec) -> String {
    format!(
        "{{\"name\":\"{}\",\"defaults\":{}}}",
        json_escape(&component.name),
        value_map_json(&component.defaults)
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
        Value::Null => "null".to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
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
            let mut values = (**fields).clone();
            values.insert(
                "__type".to_string(),
                Value::new_string((**type_name).clone()),
            );
            value_map_json(&values)
        }
    }
}

fn render_pxl_canvas(backend: &TraceVisualBackend) -> String {
    let pxl = render_pxl_document(backend);
    // Use string concatenation to avoid format! escaping issues
    let html_template = r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>PXL Canvas Engine</title><style>body{margin:0;font-family:Arial,sans-serif;background:#f3f6fb;color:#111827}main{min-height:100vh;display:grid;grid-template-columns:minmax(0,1fr) 280px;gap:18px;padding:20px;box-sizing:border-box}.stage{display:grid;place-items:center;background:white;border:1px solid #d8e1ee;border-radius:8px;min-height:calc(100vh - 40px);overflow:auto}canvas{background:#f8fafc;border:1px solid #cbd5e1;box-shadow:0 16px 38px rgba(15,23,42,.12);max-width:100%;height:auto}aside{background:#111827;color:#e2e8f0;border-radius:8px;padding:14px;display:flex;flex-direction:column;gap:12px;min-height:0}h1{font-size:16px;margin:0;color:white}.meta,.bindings{font-size:12px;color:#cbd5e1;line-height:1.45}.bindings div{display:flex;justify-content:space-between;gap:8px;border-top:1px solid rgba(226,232,240,.16);padding-top:6px;margin-top:6px}.bindings button{border:1px solid #93c5fd;background:#e0f2fe;color:#0f172a;border-radius:5px;padding:3px 7px;cursor:pointer}kbd{background:#e2e8f0;color:#111827;border-radius:4px;padding:1px 5px;font-size:11px}button.trace{border:1px solid #93c5fd;background:#dbeafe;color:#0f172a;border-radius:6px;padding:8px 10px;font-weight:700;cursor:pointer}.event-log{display:flex;flex-direction:column;gap:6px;overflow:auto;font-size:12px}.event-log div{border-top:1px solid rgba(226,232,240,.16);padding-top:6px}@media (max-width: 760px){main{grid-template-columns:1fr}.stage{min-height:60vh}}</style></head><body><main><section class="stage"><canvas id="pxl-canvas" width="720" height="520"></canvas></section><aside><h1>PXL Canvas Engine</h1><div class="meta" id="pxl-meta">Loading PXL scene</div><section class="bindings" id="scene-list"></section><section class="bindings" id="input-bindings"></section><button class="trace" id="editor-toggle" type="button">Editor</button><button class="trace" id="export-trace" type="button">Export trace</button><section class="event-log" id="event-log"><div>Click a PXL region</div></section></aside></main><script>const pxl="#;
    let html_end = r#";const theme=pxl.theme||{};const events=[];const canvas=document.getElementById('pxl-canvas');const ctx=canvas.getContext('2d');const meta=document.getElementById('pxl-meta');const log=document.getElementById('event-log');const bindings=document.getElementById('input-bindings');const traceButton=document.getElementById('export-trace');function themeValue(key,fallback){return theme[key]||fallback;}document.body.style.background=themeValue('page','#f3f6fb');document.querySelector('aside').style.background=themeValue('panel','#111827');traceButton.style.background=themeValue('button','#dbeafe');const surface=pxl.surfaces[0]||{name:'empty',width:720,height:520};const camera=pxl.camera||{x:0,y:0,zoom:100};const pulseTargets=new Set((pxl.pulses||[]).map((pulse)=>pulse.target));let selected=null;const zoomScale=Math.max(1,camera.zoom||100)/100;let scale=Math.min(960/Math.max(1,surface.width),640/Math.max(1,surface.height),1);canvas.width=Math.max(1,Math.round(surface.width*scale));canvas.height=Math.max(1,Math.round(surface.height*scale));meta.textContent=surface.name+' '+surface.width+'x'+surface.height+' regions='+(pxl.regions||[]).length+' inputs='+(pxl.inputs||[]).length+' camera='+camera.x+','+camera.y+' zoom='+camera.zoom;bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';function prop(region,key){return region.properties&&region.properties[key]!==undefined?region.properties[key]:region[key];}const spriteImages=new Map();(pxl.regions||[]).forEach((region)=>{const source=prop(region,'sprite');if(source){const image=new Image();image.src=source;spriteImages.set(region.name,image);}});function regionVisible(region){return prop(region,'visible')!==false;}function layerValue(region){const layer=Number(prop(region,'layer')||0);return Number.isFinite(layer)?layer:0;}function sortedRegions(){return [...(pxl.regions||[])].filter(regionVisible).sort((left,right)=>layerValue(left)-layerValue(right)||left.name.localeCompare(right.name));}function eventName(region){return prop(region,'event')||prop(region,'behavior')||'tap';}function regionState(region){return prop(region,'state')||'idle';}function regionText(region){return prop(region,'text')||region.name;}function textSize(region){const size=Number(prop(region,'textSize')||13);return Number.isFinite(size)?size:13;}function fillFor(region,time){const state=regionState(region);const pulsing=pulseTargets.has(region.name);if(state==='active')return themeValue('activeFill','rgba(5,150,105,.28)');if(state==='disabled')return themeValue('disabledFill','rgba(100,116,139,.24)');if(state==='error')return themeValue('errorFill','rgba(185,28,28,.30)');if(pulsing){const alpha=.18+Math.sin(time/180)*.08;return 'rgba(220,38,38,'+alpha.toFixed(3)+')';}return themeValue('regionFill','rgba(37,99,235,.18)');}function strokeFor(region){const state=regionState(region);if(region.name===selected)return themeValue('selected','#f59e0b');if(state==='active')return themeValue('active','#059669');if(state==='disabled')return themeValue('disabled','#64748b');if(state==='error')return themeValue('error','#b91c1c');if(pulseTargets.has(region.name))return themeValue('pulse','#dc2626');return themeValue('accent','#2563eb');}function motionOffset(region,time){const kind=prop(region,'motion')||'none';const speed=Math.max(1,Number(prop(region,'motionSpeed')||1000));const phase=time/speed*Math.PI*2;if(kind==='float')return {x:0,y:Math.sin(phase)*6,s:1};if(kind==='shake')return {x:Math.sin(phase*8)*3,y:0,s:1};if(kind==='breathe')return {x:0,y:0,s:1+Math.sin(phase)*.035};return {x:0,y:0,s:1};}function drawSprite(region,x,y,w,h){const image=spriteImages.get(region.name);if(!image||!image.complete||image.naturalWidth===0)return false;const fit=prop(region,'spriteFit')||'stretch';let dx=x,dy=y,dw=w,dh=h;if(fit==='contain'){const ratio=Math.min(w/image.naturalWidth,h/image.naturalHeight);dw=image.naturalWidth*ratio;dh=image.naturalHeight*ratio;dx=x+(w-dw)/2;dy=y+(h-dh)/2;}ctx.save();ctx.clip();ctx.drawImage(image,dx,dy,dw,dh);ctx.restore();return true;}function recordEvent(entry){const event={time:new Date().toISOString(),...entry};events.push(event);const line=document.createElement('div');line.textContent=Object.entries(event).filter(([key])=>key!=='time').map(([key,value])=>key+'='+value).join(' ');log.appendChild(line);log.scrollTop=log.scrollHeight;}function downloadTrace(){const blob=new Blob([JSON.stringify({format:'PXL_TRACE',version:1,events},null,2)],{type:'application/json'});const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download='pxl-trace.json';link.click();URL.revokeObjectURL(url);}function drawRegion(region,time){if(!regionVisible(region))return;const motion=motionOffset(region,time);const x=(region.x-camera.x)*scale*zoomScale+motion.x;const y=(region.y-camera.y)*scale*zoomScale+motion.y;const w=Math.max(1,region.w*scale*zoomScale)*motion.s;const h=Math.max(1,region.h*scale*zoomScale)*motion.s;ctx.fillStyle=fillFor(region,time);ctx.strokeStyle=strokeFor(region);ctx.lineWidth=region.name===selected?4:2;ctx.beginPath();ctx.roundRect(x,y,w,h,8);ctx.fill();drawSprite(region,x,y,w,h);ctx.stroke();ctx.fillStyle=themeValue('text','#0f172a');ctx.font='700 '+textSize(region)+'px Arial';ctx.textBaseline='top';ctx.fillText(regionText(region),x+8,y+8,Math.max(10,w-16));ctx.font='11px Arial';ctx.fillStyle=themeValue('mutedText','#334155');ctx.fillText('id='+region.name+' z='+layerValue(region)+' '+regionState(region)+' / '+eventName(region),x+8,y+textSize(region)+14,Math.max(10,w-16));}function draw(time){ctx.clearRect(0,0,canvas.width,canvas.height);ctx.fillStyle=themeValue('surface','#f8fafc');ctx.fillRect(0,0,canvas.width,canvas.height);ctx.strokeStyle=themeValue('surfaceBorder','#d8e1ee');ctx.lineWidth=1;ctx.strokeRect(.5,.5,canvas.width-1,canvas.height-1);sortedRegions().forEach((region)=>drawRegion(region,time));requestAnimationFrame(draw);}function hit(clientX,clientY){const rect=canvas.getBoundingClientRect();const x=((clientX-rect.left)*(canvas.width/rect.width))/(scale*zoomScale)+camera.x;const y=((clientY-rect.top)*(canvas.height/rect.height))/(scale*zoomScale)+camera.y;return sortedRegions().reverse().find((region)=>x>=region.x&&x<=region.x+region.w&&y>=region.y&&y<=region.y+region.h);}canvas.addEventListener('click',(event)=>{const region=hit(event.clientX,event.clientY);if(!region)return;selected=region.name;recordEvent({type:'pointer',target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)});});window.addEventListener('keydown',(event)=>{const binding=(pxl.inputs||[]).find((input)=>input.key.toLowerCase()===event.key.toLowerCase());if(!binding)return;const target=(pxl.regions||[]).find((region)=>region.name===binding.target);if(target&&!regionVisible(target))return;selected=binding.target;recordEvent({type:'keyboard',key:binding.key,target:binding.target,event:binding.event});});traceButton.addEventListener('click',downloadTrace);requestAnimationFrame(draw);</script></body></html>"#;

    let mut html = format!("{}{}{}", html_template, pxl, html_end);
    html = html.replace(
        "const bindings=document.getElementById('input-bindings');const traceButton=document.getElementById('export-trace');",
        "const bindings=document.getElementById('input-bindings');const sceneList=document.getElementById('scene-list');const editorButton=document.getElementById('editor-toggle');const traceButton=document.getElementById('export-trace');",
    );
    html = html.replace(
        "traceButton.style.background=themeValue('button','#dbeafe');",
        "traceButton.style.background=themeValue('button','#dbeafe');editorButton.style.background=themeValue('button','#dbeafe');",
    );
    html = html.replace(
        "let selected=null;const zoomScale",
        "let selected=null;let activeScene=pxl.activeScene||(pxl.scenes&&pxl.scenes[0])||'main';let editorEnabled=!!(pxl.editor&&pxl.editor.enabled);let appFrame=(pxl.loop&&pxl.loop.frame)||0;let appTimeMs=(pxl.loop&&pxl.loop.timeMs)||0;const persistenceKey='PXL_STATE:'+surface.name;const eventQueueKey='PXL_EVENT_QUEUE:'+surface.name;const editorPositionsKey='PXL_EDITOR_POSITIONS:'+surface.name;let regionOverrides={};function loadEditorPositions(){try{regionOverrides=JSON.parse(localStorage.getItem(editorPositionsKey)||'{}');}catch(_error){regionOverrides={};}}function saveEditorPositions(){try{localStorage.setItem(editorPositionsKey,JSON.stringify(regionOverrides));}catch(_error){}}function updateEditorButton(){editorButton.textContent=editorEnabled?'Editor on':'Editor off';}function loadPersistedState(){try{const state=JSON.parse(localStorage.getItem(persistenceKey)||'{}');if(state.activeScene)activeScene=state.activeScene;if(state.selected)selected=state.selected;}catch(_error){}}function savePersistedState(){try{localStorage.setItem(persistenceKey,JSON.stringify({format:'PXL_BROWSER_STATE',version:1,activeScene,selected,updatedAt:new Date().toISOString()}));}catch(_error){}}function saveEventQueue(){try{localStorage.setItem(eventQueueKey,JSON.stringify({format:'PXL_EVENT_QUEUE',version:1,surface:surface.name,activeScene,appFrame,appTimeMs,events}));}catch(_error){}}loadPersistedState();loadEditorPositions();updateEditorButton();const layoutsByScene=new Map((pxl.layouts||[]).map((layout)=>[layout.scene,layout]));const zoomScale",
    );
    html = html.replace(
        "function draw(time){ctx.clearRect(0,0,canvas.width,canvas.height);",
        "function draw(time){appFrame+=1;appTimeMs=Math.round(time);ctx.clearRect(0,0,canvas.width,canvas.height);",
    );
    html = html.replace(
        "bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';",
        "function renderSceneList(){sceneList.innerHTML=(pxl.scenes||['main']).map((scene)=>'<div><button type=\"button\" data-scene=\"'+scene+'\">'+scene+'</button><span>'+((scene===activeScene)?'active':'')+'</span></div>').join('');sceneList.querySelectorAll('[data-scene]').forEach((node)=>node.addEventListener('click',()=>{activeScene=node.dataset.scene;selected=null;renderSceneList();savePersistedState();recordEvent({type:'scene',scene:activeScene,event:'scene_change'});}));}renderSceneList();bindings.innerHTML=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('')||'<div>No input bindings</div>';",
    );
    html = html.replace(
        "function regionVisible(region){return prop(region,'visible')!==false;}",
        "function regionScene(region){return prop(region,'scene')||'main';}function regionVisible(region){return prop(region,'visible')!==false&&regionScene(region)===activeScene;}function sceneLayout(scene){return layoutsByScene.get(scene)||{kind:'absolute',gap:0};}function layoutedRegion(region,index){const layout=sceneLayout(regionScene(region));const kind=layout.kind||'absolute';const copy={...region};if(kind!=='absolute'){const gap=Math.max(0,Number(layout.gap||0));if(kind==='vertical'){copy.x=40;copy.y=40+index*(region.h+gap);}else if(kind==='horizontal'){copy.x=40+index*(region.w+gap);copy.y=40;}else if(kind==='grid'){const columns=Math.max(1,Math.floor((surface.width-80+gap)/(Math.max(1,region.w)+gap)));copy.x=40+(index%columns)*(region.w+gap);copy.y=40+Math.floor(index/columns)*(region.h+gap);}}const override=regionOverrides[region.name];if(override){copy.x=override.x;copy.y=override.y;}return copy;}",
    );
    html = html.replace(
        "sortedRegions().forEach((region)=>drawRegion(region,time));",
        "const regions=sortedRegions();regions.forEach((region,index)=>drawRegion(layoutedRegion(region,index),time));",
    );
    html = html.replace(
        "return sortedRegions().reverse().find((region)=>x>=region.x&&x<=region.x+region.w&&y>=region.y&&y<=region.y+region.h);",
        "const regions=sortedRegions();return regions.map((region,index)=>layoutedRegion(region,index)).reverse().find((region)=>x>=region.x&&x<=region.x+region.w&&y>=region.y&&y<=region.y+region.h);",
    );
    html = html.replace(
        "canvas.addEventListener('click',(event)=>{const region=hit(event.clientX,event.clientY);",
        "function pointerPosition(event){const rect=canvas.getBoundingClientRect();return {x:((event.clientX-rect.left)*(canvas.width/rect.width))/(scale*zoomScale)+camera.x,y:((event.clientY-rect.top)*(canvas.height/rect.height))/(scale*zoomScale)+camera.y};}let dragging=null;editorButton.addEventListener('click',()=>{editorEnabled=!editorEnabled;updateEditorButton();recordEvent({type:'editor',enabled:editorEnabled,event:'editor_toggle'});});canvas.addEventListener('pointerdown',(event)=>{if(!editorEnabled)return;const region=hit(event.clientX,event.clientY);if(!region)return;const point=pointerPosition(event);dragging={name:region.name,dx:point.x-region.x,dy:point.y-region.y};selected=region.name;canvas.setPointerCapture(event.pointerId);event.preventDefault();});canvas.addEventListener('pointermove',(event)=>{if(!dragging)return;const point=pointerPosition(event);regionOverrides[dragging.name]={x:Math.round(point.x-dragging.dx),y:Math.round(point.y-dragging.dy)};saveEditorPositions();event.preventDefault();});canvas.addEventListener('pointerup',(event)=>{if(!dragging)return;const moved=regionOverrides[dragging.name];recordEvent({type:'editor_move',target:dragging.name,x:moved.x,y:moved.y,event:'editor_move'});dragging=null;event.preventDefault();});canvas.addEventListener('click',(event)=>{if(dragging)return;const region=hit(event.clientX,event.clientY);",
    );
    html = html.replace(
        "selected=region.name;recordEvent({type:'pointer',target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)});",
        "selected=region.name;savePersistedState();recordEvent({type:'click',payload:{target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)}});",
    );
    html = html.replace(
        "if(!region)return;selected=region.name;savePersistedState();recordEvent({type:'click',payload:{target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)}});",
        "if(!region){recordEvent({type:'click',payload:{target:'canvas',layer:0,state:'empty',event:'canvas'}});return;}selected=region.name;savePersistedState();recordEvent({type:'click',payload:{target:region.name,layer:layerValue(region),state:regionState(region),event:eventName(region)}});",
    );
    html = html.replace(
        "selected=binding.target;recordEvent({type:'keyboard',key:binding.key,target:binding.target,event:binding.event});",
        "selected=binding.target;savePersistedState();recordEvent({type:'key',payload:{key:binding.key,target:binding.target,event:binding.event}});",
    );
    html = html.replace(
        "events.push(event);const line=document.createElement('div');",
        "events.push(event);saveEventQueue();const line=document.createElement('div');",
    );
    html = html.replace(
        "function recordEvent(entry){const event={time:new Date().toISOString(),...entry};events.push(event);saveEventQueue();const line=document.createElement('div');line.textContent=Object.entries(event).filter(([key])=>key!=='time').map(([key,value])=>key+'='+value).join(' ');log.appendChild(line);log.scrollTop=log.scrollHeight;}",
        "function syncEventToMatter(event){try{const encoded=encodeURIComponent(JSON.stringify(event));fetch('/events?e='+encoded,{method:'GET',keepalive:true}).catch(()=>{});}catch(_error){}}function recordEvent(entry){const event={type:entry.type||'input',timestamp:Date.now(),source:'canvas',payload:entry.payload||entry};events.push(event);saveEventQueue();syncEventToMatter(event);const line=document.createElement('div');line.textContent='type='+event.type+' source='+event.source+' payload='+(event.payload&&event.payload.target?event.payload.target:'-');log.appendChild(line);log.scrollTop=log.scrollHeight;}",
    );
    html = html.replace(
        "traceButton.addEventListener('click',downloadTrace);requestAnimationFrame(draw);",
        "traceButton.addEventListener('click',downloadTrace);function triggerVmAction(name){fetch('/actions?name='+encodeURIComponent(name),{method:'GET',keepalive:true}).catch(()=>{});}function renderVmActions(components){const actionComponents=components.filter((c)=>Array.isArray(c.actions)&&c.actions.length>0);const rows=actionComponents.map((c)=>{const label=(c&&c.props&&c.props.label)?c.props.label:c.name;const actions=c.actions.map((a)=>'<button type=\"button\" data-action=\"'+a+'\">'+a+'</button>').join('');return '<div><span>'+label+'</span><span>'+actions+'</span></div>';}).join('');const staticInputs=(pxl.inputs||[]).map((input)=>'<div><kbd>'+input.key+'</kbd><span>'+input.target+' / '+input.event+'</span></div>').join('');bindings.innerHTML=(staticInputs||'<div>No input bindings</div>')+(rows?rows:'');bindings.querySelectorAll('[data-action]').forEach((node)=>{node.addEventListener('click',()=>triggerVmAction(node.getAttribute('data-action')||''));});}function applyVmStateToPxl(counter,color){theme.accent=color;theme.selected=color;(pxl.regions||[]).forEach((region)=>{region.properties=region.properties||{};const semantic=region.properties.semantic||region.semantic;if(semantic==='counter'){const label=(region.properties.label||(region.properties.text||region.name).split(':')[0]||'counter');region.properties.label=label;region.properties.text=label+': '+counter;region.properties.state=counter>0?'active':'idle';}if(region.name==='canvas'||semantic==='interactive_canvas'){region.properties.text='Clique no canvas ('+counter+')';region.properties.state='active';}});}function pollVmState(){fetch('/state/vm',{cache:'no-store'}).then((res)=>res.json()).then((state)=>{if(!state||!state.ok)return;const visual=state.visual||{};const pub=state.public||{};const counter=Number(visual.counter||0);const color=visual.color||'#2563eb';const components=Array.isArray(pub.components)?pub.components:[];const summary=components.map((c)=>{const value=(c&&c.props&&c.props.value!==undefined)?c.props.value:c.value;return c.name+':'+value;}).join(' ');const schema=pub.schemaVersion||1;applyVmStateToPxl(counter,color);meta.textContent=surface.name+' '+surface.width+'x'+surface.height+' clicks='+counter+' color='+color+' schema=v'+schema+(summary?(' '+summary):'');canvas.style.borderColor=color;canvas.style.boxShadow='0 16px 38px '+(color==='#dc2626'?'rgba(220,38,38,.30)':'rgba(37,99,235,.24)');renderVmActions(components);if(log&&log.firstElementChild){log.firstElementChild.textContent='vm '+(summary||('clickCounter='+counter));}}).catch(()=>{});}setInterval(pollVmState,300);pollVmState();requestAnimationFrame(draw);",
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
            return format!("state: {}", **state);
        }
        if let Some(Value::String(semantic)) = properties.get("semantic") {
            return (**semantic).clone();
        }
        if let Some(Value::String(material)) = properties.get("material") {
            return (**material).clone();
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
        Some(Value::String(state)) => Some((**state).clone()),
        _ => None,
    })
}

fn region_event(region: &VisualRegionSpec, properties: Option<&HashMap<String, Value>>) -> String {
    if let Some(properties) = properties {
        if let Some(Value::String(event)) = properties.get("event") {
            return (**event).clone();
        }
        if let Some(Value::String(behavior)) = properties.get("behavior") {
            return (**behavior).clone();
        }
    }
    region.behavior.clone().unwrap_or_else(|| "tap".to_string())
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

impl Default for PvmVisualBackend {
    fn default() -> Self {
        Self::new()
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
        let result = backend.call("run", vec![Value::new_string("pizzaria".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_surface() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call(
            "surface",
            vec![
                Value::new_string("main".to_string()),
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
                Value::new_string("checkout".to_string()),
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
        let result = backend.call("pulse", vec![Value::new_string("checkout".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visual_set() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call(
            "set",
            vec![
                Value::new_string("checkout".to_string()),
                Value::new_string("energy".to_string()),
                Value::Int(80),
            ],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_visual_loop_advances_frames_inside_backend() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call("tick", vec![Value::Int(16)]).unwrap();
        match result {
            Value::Map(state) => {
                assert_eq!(state.get("frame"), Some(&Value::Int(1)));
                assert_eq!(state.get("timeMs"), Some(&Value::Int(16)));
                assert_eq!(state.get("running"), Some(&Value::Bool(true)));
            }
            _ => panic!("visual.tick must return loop state map"),
        }

        let result = backend
            .call("loop", vec![Value::Int(3), Value::Int(16)])
            .unwrap();
        match result {
            Value::Map(state) => {
                assert_eq!(state.get("frame"), Some(&Value::Int(4)));
                assert_eq!(state.get("timeMs"), Some(&Value::Int(64)));
                assert_eq!(state.get("running"), Some(&Value::Bool(true)));
            }
            _ => panic!("visual.loop must return loop state map"),
        }

        let snapshot = backend
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
        assert!(snapshot.contains("\"loop\":{\"frame\":4,\"timeMs\":64,\"running\":true}"));
    }

    #[test]
    fn test_visual_editor_mode_is_exported() {
        let mut backend = TraceVisualBackend::new();
        let result = backend.call("editor", vec![Value::Bool(true)]).unwrap();
        assert_eq!(result, Value::Bool(true));
        let snapshot = backend
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
        assert!(snapshot.contains("\"editor\":{\"enabled\":true}"));
    }

    #[test]
    fn test_visual_region_with_map() {
        let mut backend = TraceVisualBackend::new();
        let mut props = HashMap::new();
        props.insert("x".to_string(), Value::Int(100));
        props.insert("y".to_string(), Value::Int(200));
        props.insert("w".to_string(), Value::Int(300));
        props.insert("h".to_string(), Value::Int(80));
        props.insert(
            "semantic".to_string(),
            Value::new_string("action_button".to_string()),
        );
        props.insert(
            "behavior".to_string(),
            Value::new_string("pulse".to_string()),
        );
        props.insert("energy".to_string(), Value::Int(1));

        let result = backend.call(
            "region",
            vec![
                Value::new_string("checkout".to_string()),
                Value::new_map(props),
            ],
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
                    Value::new_string("main".to_string()),
                    Value::Int(800),
                    Value::Int(600),
                ],
            )
            .unwrap();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("button".to_string()),
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
                    Value::new_string("button".to_string()),
                    Value::new_string("material".to_string()),
                    Value::new_string("glass".to_string()),
                ],
            )
            .unwrap();
        backend
            .call("pulse", vec![Value::new_string("button".to_string())])
            .unwrap();

        let snapshot = backend
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
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
                    Value::new_string("main".to_string()),
                    Value::Int(320),
                    Value::Int(240),
                ],
            )
            .unwrap();

        let result = backend
            .call(
                "export",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();

        assert_eq!(result, Value::new_string(path.display().to_string()));
        let exported = fs::read_to_string(&path).unwrap();
        assert!(exported.contains("\"format\":\"PXL\""));
        assert!(exported.contains("\"main\""));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_web_writes_runtime_bundle() {
        let mut backend = TraceVisualBackend::new();
        let dir = std::env::temp_dir().join("matter_visual_web_runtime_test");
        let _ = fs::remove_dir_all(&dir);
        backend
            .call(
                "surface",
                vec![
                    Value::new_string("main".to_string()),
                    Value::Int(640),
                    Value::Int(360),
                ],
            )
            .unwrap();
        backend.call("editor", vec![Value::Bool(true)]).unwrap();

        let result = backend
            .call(
                "web",
                vec![
                    Value::new_string(dir.display().to_string()),
                    Value::new_string("Matter PXL Demo".to_string()),
                ],
            )
            .unwrap();

        assert_eq!(result, Value::new_string(dir.display().to_string()));
        let index = fs::read_to_string(dir.join("index.html")).unwrap();
        let pxl = fs::read_to_string(dir.join("pxl.json")).unwrap();
        let manifest = fs::read_to_string(dir.join("manifest.json")).unwrap();
        let package = fs::read_to_string(dir.join("matter-package.json")).unwrap();
        let lock = fs::read_to_string(dir.join("matter-lock.json")).unwrap();
        assert!(index.contains("PXL Canvas Engine"));
        assert!(pxl.contains("\"format\":\"PXL\""));
        assert!(manifest.contains("\"format\":\"MATTER_WEB_RUNTIME\""));
        assert!(manifest.contains("\"entry\":\"index.html\""));
        assert!(manifest.contains("\"pxl\":\"pxl.json\""));
        assert!(manifest.contains("\"package\":\"matter-package.json\""));
        assert!(manifest.contains("\"lock\":\"matter-lock.json\""));
        assert!(manifest.contains("\"app\":\"Matter PXL Demo\""));
        assert!(package.contains("\"format\":\"MATTER_PACKAGE\""));
        assert!(package.contains("\"name\":\"matter-pxl-demo\""));
        assert!(package.contains("\"kind\":\"web-runtime\""));
        assert!(package.contains("\"matter.pxl\":\"^0.1.0\""));
        assert!(package.contains("\"lock\":\"matter-lock.json\""));
        assert!(lock.contains("\"format\":\"MATTER_LOCK\""));
        assert!(lock.contains("\"package\":\"matter-pxl-demo\""));
        assert!(lock.contains("\"path\":\"index.html\""));
        assert!(lock.contains("\"path\":\"pxl.json\""));
        assert!(lock.contains("\"fingerprint\""));

        let verification = backend
            .call(
                "verify_web",
                vec![Value::new_string(dir.display().to_string())],
            )
            .unwrap();
        match verification {
            Value::Map(result) => {
                assert_eq!(result.get("ok"), Some(&Value::Bool(true)));
                assert_eq!(
                    result.get("package"),
                    Some(&Value::new_string("matter-pxl-demo".to_string()))
                );
                match result.get("files") {
                    Some(Value::List(files)) => assert_eq!(files.len(), 4),
                    _ => panic!("visual.verify_web must return verified files"),
                }
            }
            _ => panic!("visual.verify_web must return a map"),
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_visual_preview_writes_html() {
        let mut backend = TraceVisualBackend::new();
        let path = std::env::temp_dir().join("matter_visual_preview_test.html");
        backend
            .call(
                "surface",
                vec![
                    Value::new_string("main".to_string()),
                    Value::Int(800),
                    Value::Int(600),
                ],
            )
            .unwrap();
        backend
            .call(
                "component",
                vec![
                    Value::new_string("action_button".to_string()),
                    Value::new_map({
                        let mut defaults = std::collections::HashMap::new();
                        defaults.insert("w".to_string(), Value::Int(260));
                        defaults.insert("h".to_string(), Value::Int(80));
                        defaults.insert(
                            "semantic".to_string(),
                            Value::new_string("primary_action".to_string()),
                        );
                        defaults.insert(
                            "text".to_string(),
                            Value::new_string("Component button".to_string()),
                        );
                        defaults.insert(
                            "event".to_string(),
                            Value::new_string("component_tap".to_string()),
                        );
                        defaults
                    }),
                ],
            )
            .unwrap();
        backend
            .call(
                "mount",
                vec![
                    Value::new_string("action_button".to_string()),
                    Value::new_string("checkout".to_string()),
                    Value::Int(120),
                    Value::Int(220),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("semantic".to_string()),
                    Value::new_string("primary_action".to_string()),
                ],
            )
            .unwrap();

        let result = backend
            .call(
                "preview",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();

        assert_eq!(result, Value::new_string(path.display().to_string()));
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
                    Value::new_string("main".to_string()),
                    Value::Int(800),
                    Value::Int(600),
                ],
            )
            .unwrap();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("checkout".to_string()),
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
                    Value::new_string("checkout".to_string()),
                    Value::new_string("active".to_string()),
                ],
            )
            .unwrap();

        let snapshot = backend
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
        assert!(snapshot.contains("\"state\":\"active\""));

        let path = std::env::temp_dir().join("matter_visual_state_preview_test.html");
        backend
            .call(
                "preview",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("state-active"));
        assert!(html.contains("state: active"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_state_can_be_saved_and_loaded() {
        let path = std::env::temp_dir().join("matter_visual_state_test.json");
        let mut backend = TraceVisualBackend::new();
        backend
            .call("scene", vec![Value::new_string("settings".to_string())])
            .unwrap();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("panel".to_string()),
                    Value::Int(20),
                    Value::Int(30),
                    Value::Int(300),
                    Value::Int(90),
                ],
            )
            .unwrap();
        backend
            .call(
                "state",
                vec![
                    Value::new_string("panel".to_string()),
                    Value::new_string("active".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "visible",
                vec![Value::new_string("panel".to_string()), Value::Bool(false)],
            )
            .unwrap();

        let result = backend
            .call(
                "save_state",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();
        assert_eq!(result, Value::new_string(path.display().to_string()));
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("\"format\":\"PXL_STATE\""));
        assert!(saved.contains("\"activeScene\":\"settings\""));
        assert!(saved.contains("\"visible\":false"));

        let mut restored = TraceVisualBackend::new();
        restored
            .call(
                "region",
                vec![
                    Value::new_string("panel".to_string()),
                    Value::Int(1),
                    Value::Int(1),
                    Value::Int(10),
                    Value::Int(10),
                ],
            )
            .unwrap();
        restored
            .call(
                "load_state",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();
        let snapshot = restored
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
        assert!(snapshot.contains("\"activeScene\":\"settings\""));
        assert!(snapshot.contains("\"x\":20"));
        assert!(snapshot.contains("\"w\":300"));
        assert!(snapshot.contains("\"state\":\"active\""));
        assert!(snapshot.contains("\"visible\":false"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_events_can_be_loaded_back_into_matter() {
        let path = std::env::temp_dir().join("matter_visual_events_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"pointer","target":"checkout","event":"checkout_tap","layer":4},{"type":"keyboard","key":"Enter","target":"checkout","event":"checkout_submit"}]}"#,
        )
        .unwrap();

        let mut backend = TraceVisualBackend::new();
        let events = backend
            .call(
                "load_events",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();
        let Value::List(events) = events else {
            unreachable!("visual.load_events must return a list");
        };
        assert_eq!(events.len(), 2);
        match &events[0] {
            Value::Map(event) => {
                assert_eq!(
                    event.get("type"),
                    Some(&Value::new_string("pointer".to_string()))
                );
                assert_eq!(
                    event.get("target"),
                    Some(&Value::new_string("checkout".to_string()))
                );
                assert_eq!(event.get("layer"), Some(&Value::Int(4)));
            }
            _ => panic!("visual.load_events event must be a map"),
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_events_can_be_dispatched_into_visual_state() {
        let path = std::env::temp_dir().join("matter_visual_dispatch_events_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"scene","scene":"checkout","event":"scene_change"},{"type":"pointer","target":"button","event":"button_tap"},{"type":"editor_move","target":"button","x":42,"y":64,"event":"editor_move"}]}"#,
        )
        .unwrap();

        let mut backend = TraceVisualBackend::new();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("button".to_string()),
                    Value::Int(10),
                    Value::Int(20),
                    Value::Int(120),
                    Value::Int(40),
                ],
            )
            .unwrap();
        let result = backend
            .call(
                "dispatch_events",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();
        match result {
            Value::Map(result) => {
                assert_eq!(result.get("processed"), Some(&Value::Int(3)));
                assert_eq!(result.get("moved"), Some(&Value::Int(1)));
                assert_eq!(
                    result.get("selected"),
                    Some(&Value::new_string("button".to_string()))
                );
                assert_eq!(
                    result.get("activeScene"),
                    Some(&Value::new_string("checkout".to_string()))
                );
            }
            _ => panic!("visual.dispatch_events must return a map"),
        }

        let snapshot = backend
            .call("snapshot", vec![])
            .unwrap()
            .as_string()
            .unwrap();
        assert!(snapshot.contains("\"activeScene\":\"checkout\""));
        assert!(snapshot.contains("\"name\":\"button\",\"x\":42,\"y\":64"));
        assert!(snapshot.contains("\"state\":\"active\""));
        assert!(snapshot.contains("\"selected\":true"));
        assert!(snapshot.contains("\"lastEvent\":\"editor_move\""));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_app_step_dispatches_events_and_advances_loop() {
        let path = std::env::temp_dir().join("matter_visual_app_step_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"pointer","target":"play","event":"play_tap"}]}"#,
        )
        .unwrap();

        let mut backend = TraceVisualBackend::new();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("play".to_string()),
                    Value::Int(12),
                    Value::Int(18),
                    Value::Int(140),
                    Value::Int(48),
                ],
            )
            .unwrap();
        let result = backend
            .call(
                "app_step",
                vec![
                    Value::new_string(path.display().to_string()),
                    Value::Int(33),
                ],
            )
            .unwrap();

        match result {
            Value::Map(result) => {
                match result.get("dispatch") {
                    Some(Value::Map(dispatch)) => {
                        assert_eq!(dispatch.get("processed"), Some(&Value::Int(1)));
                        assert_eq!(
                            dispatch.get("selected"),
                            Some(&Value::new_string("play".to_string()))
                        );
                    }
                    _ => panic!("visual.app_step must return dispatch map"),
                }
                match result.get("loop") {
                    Some(Value::Map(loop_state)) => {
                        assert_eq!(loop_state.get("frame"), Some(&Value::Int(1)));
                        assert_eq!(loop_state.get("timeMs"), Some(&Value::Int(33)));
                        assert_eq!(loop_state.get("running"), Some(&Value::Bool(true)));
                    }
                    _ => panic!("visual.app_step must return loop map"),
                }
                match result.get("snapshot") {
                    Some(Value::String(snapshot)) => {
                        assert!(snapshot.contains("\"lastEvent\":\"play_tap\""));
                        assert!(snapshot
                            .contains("\"loop\":{\"frame\":1,\"timeMs\":33,\"running\":true}"));
                    }
                    _ => panic!("visual.app_step must return snapshot"),
                }
            }
            _ => panic!("visual.app_step must return a map"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_visual_app_run_dispatches_once_and_runs_frames() {
        let path = std::env::temp_dir().join("matter_visual_app_run_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"scene","scene":"game","event":"scene_change"},{"type":"keyboard","key":"Space","target":"ship","event":"boost"}]}"#,
        )
        .unwrap();

        let mut backend = TraceVisualBackend::new();
        backend
            .call(
                "region",
                vec![
                    Value::new_string("ship".to_string()),
                    Value::Int(32),
                    Value::Int(48),
                    Value::Int(88),
                    Value::Int(40),
                ],
            )
            .unwrap();
        let result = backend
            .call(
                "app_run",
                vec![
                    Value::new_string(path.display().to_string()),
                    Value::Int(4),
                    Value::Int(16),
                ],
            )
            .unwrap();

        match result {
            Value::Map(result) => {
                assert_eq!(result.get("frames"), Some(&Value::Int(4)));
                match result.get("dispatch") {
                    Some(Value::Map(dispatch)) => {
                        assert_eq!(dispatch.get("processed"), Some(&Value::Int(2)));
                        assert_eq!(
                            dispatch.get("activeScene"),
                            Some(&Value::new_string("game".to_string()))
                        );
                    }
                    _ => panic!("visual.app_run must return dispatch map"),
                }
                match result.get("loop") {
                    Some(Value::Map(loop_state)) => {
                        assert_eq!(loop_state.get("frame"), Some(&Value::Int(4)));
                        assert_eq!(loop_state.get("timeMs"), Some(&Value::Int(64)));
                        assert_eq!(loop_state.get("running"), Some(&Value::Bool(true)));
                    }
                    _ => panic!("visual.app_run must return loop map"),
                }
                match result.get("snapshot") {
                    Some(Value::String(snapshot)) => {
                        assert!(snapshot.contains("\"activeScene\":\"game\""));
                        assert!(snapshot.contains("\"lastEvent\":\"boost\""));
                        assert!(snapshot
                            .contains("\"loop\":{\"frame\":4,\"timeMs\":64,\"running\":true}"));
                    }
                    _ => panic!("visual.app_run must return snapshot"),
                }
            }
            _ => panic!("visual.app_run must return a map"),
        }

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
                    Value::new_string("main".to_string()),
                    Value::Int(960),
                    Value::Int(540),
                ],
            )
            .unwrap();
        backend
            .call("scene", vec![Value::new_string("home".to_string())])
            .unwrap();
        backend
            .call(
                "layout",
                vec![
                    Value::new_string("home".to_string()),
                    Value::new_string("grid".to_string()),
                    Value::Int(12),
                ],
            )
            .unwrap();
        backend
            .call(
                "component",
                vec![
                    Value::new_string("action_button".to_string()),
                    Value::new_map({
                        let mut defaults = std::collections::HashMap::new();
                        defaults.insert("w".to_string(), Value::Int(260));
                        defaults.insert("h".to_string(), Value::Int(80));
                        defaults.insert(
                            "semantic".to_string(),
                            Value::new_string("primary_action".to_string()),
                        );
                        defaults.insert(
                            "text".to_string(),
                            Value::new_string("Component button".to_string()),
                        );
                        defaults.insert(
                            "event".to_string(),
                            Value::new_string("component_tap".to_string()),
                        );
                        defaults
                    }),
                ],
            )
            .unwrap();
        backend
            .call(
                "mount",
                vec![
                    Value::new_string("action_button".to_string()),
                    Value::new_string("checkout".to_string()),
                    Value::Int(120),
                    Value::Int(220),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("event".to_string()),
                    Value::new_string("checkout_tap".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "state",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("active".to_string()),
                ],
            )
            .unwrap();
        backend
            .call("pulse", vec![Value::new_string("checkout".to_string())])
            .unwrap();
        backend
            .call(
                "layer",
                vec![Value::new_string("checkout".to_string()), Value::Int(4)],
            )
            .unwrap();
        backend
            .call(
                "camera",
                vec![Value::Int(20), Value::Int(40), Value::Int(125)],
            )
            .unwrap();
        backend
            .call(
                "input",
                vec![
                    Value::new_string("Enter".to_string()),
                    Value::new_string("checkout".to_string()),
                    Value::new_string("checkout_submit".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "theme",
                vec![
                    Value::new_string("accent".to_string()),
                    Value::new_string("#0f766e".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "motion",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("breathe".to_string()),
                    Value::Int(1200),
                ],
            )
            .unwrap();
        backend
            .call(
                "sprite",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("assets/checkout.png".to_string()),
                    Value::new_string("contain".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "text",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("Buy now".to_string()),
                ],
            )
            .unwrap();
        backend
            .call(
                "set",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::new_string("textSize".to_string()),
                    Value::Int(16),
                ],
            )
            .unwrap();
        backend
            .call(
                "visible",
                vec![
                    Value::new_string("checkout".to_string()),
                    Value::Bool(false),
                ],
            )
            .unwrap();
        backend
            .call("loop", vec![Value::Int(2), Value::Int(16)])
            .unwrap();
        backend.call("editor", vec![Value::Bool(true)]).unwrap();

        let result = backend
            .call(
                "canvas",
                vec![Value::new_string(path.display().to_string())],
            )
            .unwrap();

        assert_eq!(result, Value::new_string(path.display().to_string()));
        let html = fs::read_to_string(&path).unwrap();
        assert!(html.contains("PXL Canvas Engine"));
        assert!(html.contains("<canvas id=\"pxl-canvas\""));
        assert!(html.contains("requestAnimationFrame"));
        assert!(html.contains("checkout_tap"));
        assert!(html.contains("\"camera\":{\"x\":20,\"y\":40,\"zoom\":125}"));
        assert!(html.contains("\"layer\":4"));
        assert!(html.contains("\"scenes\":[\"home\"]"));
        assert!(html.contains("\"activeScene\":\"home\""));
        assert!(html.contains("\"layouts\":[{\"scene\":\"home\",\"kind\":\"grid\",\"gap\":12}]"));
        assert!(html.contains("\"components\":[{\"name\":\"action_button\""));
        assert!(html.contains("\"component\":\"action_button\""));
        assert!(html.contains("\"semantic\":\"primary_action\""));
        assert!(html.contains("\"loop\":{\"frame\":2,\"timeMs\":32,\"running\":true}"));
        assert!(html.contains("\"editor\":{\"enabled\":true}"));
        assert!(html.contains("editor-toggle"));
        assert!(html.contains("editorEnabled"));
        assert!(html.contains("PXL_EDITOR_POSITIONS"));
        assert!(html.contains("pointerdown"));
        assert!(html.contains("pointermove"));
        assert!(html.contains("editor_move"));
        assert!(html.contains("appFrame"));
        assert!(html.contains("appTimeMs"));
        assert!(html.contains("\"scene\":\"home\""));
        assert!(html.contains("scene-list"));
        assert!(html.contains("activeScene"));
        assert!(html.contains("regionScene"));
        assert!(html.contains("layoutsByScene"));
        assert!(html.contains("sceneLayout"));
        assert!(html.contains("layoutedRegion"));
        assert!(html.contains("localStorage"));
        assert!(html.contains("PXL_EVENT_QUEUE"));
        assert!(html.contains("eventQueueKey"));
        assert!(html.contains("saveEventQueue"));
        assert!(html.contains("/events?e="));
        assert!(html.contains("syncEventToMatter"));
        assert!(html.contains("PXL_BROWSER_STATE"));
        assert!(html.contains("loadPersistedState"));
        assert!(html.contains("savePersistedState"));
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
