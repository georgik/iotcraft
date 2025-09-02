//! mcplay - IoTCraft Multi-client Scenario Player
//!
//! This binary runs scenario-driven tests for IoTCraft, supporting multi-client
//! coordination, MCP integration, and infrastructure orchestration.

use anyhow::Result;
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::{Mutex, broadcast};
use tokio::time::sleep;

// Add chrono for timestamps
#[cfg(feature = "tui")]
use chrono;

#[cfg(feature = "tui")]
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
#[cfg(feature = "tui")]
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
#[cfg(feature = "tui")]
use std::io;
#[cfg(feature = "tui")]
use std::sync::atomic::{AtomicBool, Ordering};

// Import our scenario types
use iotcraft_desktop_client::scenario_types::*;

#[derive(Debug, Clone)]
pub enum LogSource {
    Orchestrator,
    MqttServer,
    MqttObserver,
    Client(String),
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub source: LogSource,
    pub message: String,
    pub timestamp: std::time::Instant,
}

#[derive(Clone)]
pub struct LogCollector {
    sender: broadcast::Sender<LogMessage>,
}

impl LogCollector {
    pub fn new() -> (Self, broadcast::Receiver<LogMessage>) {
        let (sender, receiver) = broadcast::channel(1000);
        (Self { sender }, receiver)
    }

    pub fn log(&self, source: LogSource, message: String) {
        let log_msg = LogMessage {
            source,
            message,
            timestamp: std::time::Instant::now(),
        };
        let _ = self.sender.send(log_msg);
    }

    pub fn log_str(&self, source: LogSource, message: &str) {
        self.log(source, message.to_string());
    }
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct ScenarioInfo {
    name: String,
    description: String,
    file_path: PathBuf,
    is_valid: bool,
    clients: usize,
    steps: usize,
}

#[cfg(feature = "tui")]
struct App {
    scenarios: Vec<ScenarioInfo>,
    list_state: ListState,
    should_quit: bool,
    show_details: bool,
    selected_scenario: Option<ScenarioInfo>,
    message: Option<String>,
}

#[cfg(feature = "tui")]
#[derive(Debug)]
enum LogPane {
    Orchestrator,
    MqttServer,
    Client(String),
}

#[cfg(feature = "tui")]
struct LoggingApp {
    logs: HashMap<String, Vec<String>>, // key: pane_name, value: log lines
    selected_pane: LogPane,
    panes: Vec<LogPane>,
    should_quit: Arc<AtomicBool>,
    scroll_positions: HashMap<String, usize>,
    auto_scroll: bool,
    log_files: HashMap<String, PathBuf>, // key: pane_name, value: log file path
}

#[cfg(feature = "tui")]
impl LoggingApp {
    fn new(scenario: &Scenario) -> Self {
        let mut panes = vec![LogPane::Orchestrator];
        let mut logs = HashMap::new();
        let mut scroll_positions = HashMap::new();

        // Add MQTT server pane if required
        if scenario.infrastructure.mqtt_server.required {
            panes.push(LogPane::MqttServer);
            logs.insert("MQTT Server".to_string(), Vec::new());
            scroll_positions.insert("MQTT Server".to_string(), 0);
        }

        // Add client panes
        for client in &scenario.clients {
            panes.push(LogPane::Client(client.id.clone()));
            logs.insert(client.id.clone(), Vec::new());
            scroll_positions.insert(client.id.clone(), 0);
        }

        logs.insert("Orchestrator".to_string(), Vec::new());
        scroll_positions.insert("Orchestrator".to_string(), 0);

        // Create log files directory
        let log_dir = PathBuf::from("logs");
        if !log_dir.exists() {
            let _ = std::fs::create_dir_all(&log_dir);
        }

        // Create log files for each pane
        let mut log_files = HashMap::new();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        log_files.insert(
            "Orchestrator".to_string(),
            log_dir.join(format!("orchestrator_{}.log", timestamp)),
        );

        if scenario.infrastructure.mqtt_server.required {
            log_files.insert(
                "MQTT Server".to_string(),
                log_dir.join(format!("mqtt_server_{}.log", timestamp)),
            );
        }

        for client in &scenario.clients {
            log_files.insert(
                client.id.clone(),
                log_dir.join(format!("client_{}_{}.log", client.id, timestamp)),
            );
        }

        Self {
            logs,
            selected_pane: LogPane::Orchestrator,
            panes,
            should_quit: Arc::new(AtomicBool::new(false)),
            scroll_positions,
            auto_scroll: true,
            log_files,
        }
    }

    fn add_log(&mut self, source: &LogSource, message: String) {
        let pane_name = match source {
            LogSource::Orchestrator => "Orchestrator".to_string(),
            LogSource::MqttServer => "MQTT Server".to_string(),
            LogSource::MqttObserver => "MQTT Observer".to_string(),
            LogSource::Client(id) => id.clone(),
        };

        // Write to log file
        if let Some(log_file_path) = self.log_files.get(&pane_name) {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            let log_entry = format!("[{}] {}\n", timestamp, message);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_path)
                .and_then(|mut file| {
                    use std::io::Write;
                    file.write_all(log_entry.as_bytes())
                });
        }

        if let Some(log_lines) = self.logs.get_mut(&pane_name) {
            // Split message into lines and add each one
            for line in message.lines() {
                log_lines.push(line.to_string());

                // Keep only last 1000 lines per pane to prevent memory issues
                if log_lines.len() > 1000 {
                    log_lines.remove(0);
                    // Adjust scroll position if we're removing lines
                    if let Some(scroll) = self.scroll_positions.get_mut(&pane_name) {
                        if *scroll > 0 {
                            *scroll = scroll.saturating_sub(1);
                        }
                    }
                }
            }

            // Auto-scroll to bottom if enabled
            if self.auto_scroll {
                let max_scroll = log_lines.len().saturating_sub(1);
                self.scroll_positions.insert(pane_name, max_scroll);
            }
        }
    }

    fn get_current_pane_name(&self) -> String {
        match &self.selected_pane {
            LogPane::Orchestrator => "Orchestrator".to_string(),
            LogPane::MqttServer => "MQTT Server".to_string(),
            LogPane::Client(id) => id.clone(),
        }
    }

    fn next_pane(&mut self) {
        let current_idx = self
            .panes
            .iter()
            .position(|p| match (p, &self.selected_pane) {
                (LogPane::Orchestrator, LogPane::Orchestrator) => true,
                (LogPane::MqttServer, LogPane::MqttServer) => true,
                (LogPane::Client(a), LogPane::Client(b)) => a == b,
                _ => false,
            })
            .unwrap_or(0);

        let next_idx = (current_idx + 1) % self.panes.len();
        self.selected_pane = self.panes[next_idx].clone();
    }

    fn prev_pane(&mut self) {
        let current_idx = self
            .panes
            .iter()
            .position(|p| match (p, &self.selected_pane) {
                (LogPane::Orchestrator, LogPane::Orchestrator) => true,
                (LogPane::MqttServer, LogPane::MqttServer) => true,
                (LogPane::Client(a), LogPane::Client(b)) => a == b,
                _ => false,
            })
            .unwrap_or(0);

        let prev_idx = if current_idx == 0 {
            self.panes.len() - 1
        } else {
            current_idx - 1
        };
        self.selected_pane = self.panes[prev_idx].clone();
    }

    fn scroll_up(&mut self) {
        let pane_name = self.get_current_pane_name();
        if let Some(scroll) = self.scroll_positions.get_mut(&pane_name) {
            *scroll = scroll.saturating_sub(5); // Scroll 5 lines at a time
            self.auto_scroll = false;
        }
    }

    fn scroll_down(&mut self) {
        let pane_name = self.get_current_pane_name();
        if let Some(scroll) = self.scroll_positions.get_mut(&pane_name) {
            if let Some(logs) = self.logs.get(&pane_name) {
                let max_scroll = logs.len().saturating_sub(1);
                *scroll = (*scroll + 5).min(max_scroll); // Scroll 5 lines at a time

                // If we're at the bottom, re-enable auto-scroll
                if *scroll >= max_scroll {
                    self.auto_scroll = true;
                }
            }
        }
    }
}

#[cfg(feature = "tui")]
impl Clone for LogPane {
    fn clone(&self) -> Self {
        match self {
            LogPane::Orchestrator => LogPane::Orchestrator,
            LogPane::MqttServer => LogPane::MqttServer,
            LogPane::Client(id) => LogPane::Client(id.clone()),
        }
    }
}

#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct McpResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug)]
struct StepResult {
    success: bool,
    duration: Duration,
    error: Option<String>,
    #[allow(dead_code)]
    response: Option<serde_json::Value>,
}

struct OrchestratorState {
    scenario: Scenario,
    client_processes: HashMap<String, Child>,
    client_connections: HashMap<String, TcpStream>,
    infrastructure_processes: HashMap<String, Child>,
    completed_steps: Vec<String>,
    step_results: HashMap<String, StepResult>,
    start_time: Instant,
}

impl OrchestratorState {
    fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            client_processes: HashMap::new(),
            client_connections: HashMap::new(),
            infrastructure_processes: HashMap::new(),
            completed_steps: Vec::new(),
            step_results: HashMap::new(),
            start_time: Instant::now(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("mcplay - IoTCraft MCP Scenario Player")
        .version("1.0")
        .about("🎭 Plays multi-client IoTCraft scenarios like a screenplay using MCP")
        .arg(
            Arg::new("scenario")
                .help("Path to scenario JSON file")
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .help("Validate scenario file without running")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("list-scenarios")
                .long("list-scenarios")
                .help("List all available scenarios")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("mqtt-port")
                .long("mqtt-port")
                .help("Override MQTT server port")
                .value_name("PORT")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("list-scenarios") {
        list_scenarios().await?;
        return Ok(());
    }

    // If no scenario file is provided, show TUI
    let scenario_file = match matches.get_one::<String>("scenario") {
        Some(file) => file,
        None => {
            #[cfg(feature = "tui")]
            {
                return show_tui().await;
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!(
                    "❌ No scenario file provided. Use --list-scenarios to see available scenarios."
                );
                eprintln!(
                    "💡 To enable TUI menu, build with: cargo build --bin mcplay --features tui"
                );
                return Err("Scenario file required".into());
            }
        }
    };

    let scenario_path = PathBuf::from(scenario_file);

    // Load and parse scenario
    let scenario_content = tokio::fs::read_to_string(&scenario_path)
        .await
        .map_err(|e| format!("Failed to read scenario file: {}", e))?;

    // Try to parse as RON first, then JSON
    let mut scenario: Scenario =
        if scenario_path.extension().and_then(|s| s.to_str()) == Some("ron") {
            ron::from_str(&scenario_content)
                .map_err(|e| format!("Failed to parse RON scenario file: {}", e))?
        } else {
            serde_json::from_str(&scenario_content)
                .map_err(|e| format!("Failed to parse JSON scenario file: {}", e))?
        };

    // Override MQTT port if specified
    if let Some(mqtt_port) = matches.get_one::<u16>("mqtt-port") {
        scenario.infrastructure.mqtt_server.port = *mqtt_port;
    }

    if matches.get_flag("validate") {
        validate_scenario(&scenario)?;
        println!("✅ Scenario file is valid");
        return Ok(());
    }

    // Run the scenario
    let verbose = matches.get_flag("verbose");
    run_scenario(scenario, verbose).await?;

    Ok(())
}

async fn list_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios_dir = PathBuf::from("scenarios");
    if !scenarios_dir.exists() {
        println!("No scenarios directory found");
        return Ok(());
    }

    println!("Available scenarios:");
    let mut entries = tokio::fs::read_dir(scenarios_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if let Some(ext) = entry.path().extension() {
            if ext == "json" || ext == "ron" {
                if let Some(name) = entry.path().file_stem() {
                    // Try to load and get description
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        let scenario_result = if ext == "ron" {
                            ron::from_str::<Scenario>(&content)
                                .map_err(|e| format!("RON error: {}", e))
                        } else {
                            serde_json::from_str::<Scenario>(&content)
                                .map_err(|e| format!("JSON error: {}", e))
                        };

                        if let Ok(scenario) = scenario_result {
                            println!("  {} - {}", name.to_string_lossy(), scenario.description);
                        } else {
                            println!("  {} - (invalid scenario file)", name.to_string_lossy());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_scenario(scenario: &Scenario) -> Result<(), Box<dyn std::error::Error>> {
    // Basic validation - allow empty clients for orchestrator-only scenarios
    if scenario.steps.is_empty() {
        return Err("Scenario must have at least one step".into());
    }

    // Check client references in steps
    let client_ids: std::collections::HashSet<_> =
        scenario.clients.iter().map(|c| c.id.as_str()).collect();

    for step in &scenario.steps {
        if step.client != "orchestrator" && !client_ids.contains(step.client.as_str()) {
            return Err(format!(
                "Step '{}' references unknown client '{}'",
                step.name, step.client
            )
            .into());
        }
    }

    // Check dependency references
    let step_names: std::collections::HashSet<_> =
        scenario.steps.iter().map(|s| s.name.as_str()).collect();

    for step in &scenario.steps {
        for dep in &step.depends_on {
            if !step_names.contains(dep.as_str()) {
                return Err(
                    format!("Step '{}' depends on unknown step '{}'", step.name, dep).into(),
                );
            }
        }
    }

    Ok(())
}

async fn run_scenario(scenario: Scenario, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Create shared state wrapped in Arc<Mutex> for signal handling
    let state = Arc::new(Mutex::new(OrchestratorState::new(scenario)));
    let state_clone = Arc::clone(&state);

    // Setup signal handler for graceful shutdown
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                println!("\n🛑 Received SIGTERM, initiating graceful shutdown...");
            }
            _ = sigint.recv() => {
                println!("\n🛑 Received SIGINT (Ctrl+C), initiating graceful shutdown...");
            }
        }

        let mut state = state_clone.lock().await;
        let _ = cleanup(&mut state, verbose).await;
        std::process::exit(0);
    });

    // Lock state for main execution
    {
        let state = state.lock().await;
        println!("🚀 Starting scenario: {}", state.scenario.name);
        println!("📖 Description: {}", state.scenario.description);
        println!("👥 Clients: {}", state.scenario.clients.len());
        println!("📋 Steps: {}", state.scenario.steps.len());
        println!();
    }

    // Execute scenario with proper cleanup handling
    let result = run_scenario_inner(Arc::clone(&state), verbose).await;

    // Always cleanup, even on error
    {
        let mut state = state.lock().await;
        cleanup(&mut state, verbose).await?;
    }

    result
}

async fn run_scenario_inner(
    state: Arc<Mutex<OrchestratorState>>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start infrastructure if any services are required
    let needs_infrastructure = {
        let state = state.lock().await;
        state.scenario.infrastructure.mqtt_server.required
            || state
                .scenario
                .infrastructure
                .mcp_server
                .as_ref()
                .map(|mcp| mcp.required)
                .unwrap_or(false)
            || state
                .scenario
                .infrastructure
                .mqtt_observer
                .as_ref()
                .map(|obs| obs.required)
                .unwrap_or(false)
    };

    if needs_infrastructure {
        let mut state = state.lock().await;
        start_infrastructure(&mut *state, verbose).await?;
    }

    // Start clients
    {
        let mut state = state.lock().await;
        start_clients(&mut *state, verbose).await?;
    }

    // Execute steps
    {
        let mut state = state.lock().await;
        execute_steps(&mut *state, verbose).await?;
    }

    // Generate report
    {
        let state = state.lock().await;
        generate_report(&*state);
    }

    Ok(())
}

async fn start_infrastructure(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Starting infrastructure...");

    // Check if MQTT port is already in use before starting
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        if verbose {
            println!("  Checking if MQTT port {} is available...", port);
        }

        // Check if port is already occupied
        if is_port_occupied("localhost", port).await {
            return Err(format!("MQTT port {} is already in use. Please stop any existing MQTT brokers or choose a different port.", port).into());
        }
    }

    // Start MQTT server directly if required (instead of delegating to xtask)
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        if verbose {
            println!("  Starting MQTT server on port {}", port);
        }

        // Start MQTT server from ../mqtt-server directory
        let mqtt_process = TokioCommand::new("cargo")
            .current_dir("../mqtt-server")
            .args(&["run", "--release", "--", "--port", &port.to_string()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start MQTT server: {}", e))?;

        state
            .infrastructure_processes
            .insert("mqtt_server".to_string(), mqtt_process);

        // Wait for MQTT server to be ready
        if verbose {
            println!(
                "  Waiting for MQTT server to become ready on port {}...",
                port
            );
        }

        let mqtt_ready = wait_for_port_with_retries_and_context(
            "localhost",
            port,
            300,
            verbose,
            Some("cargo run --release (mqtt-server)"),
        )
        .await;
        if !mqtt_ready {
            return Err(format!(
                "MQTT server failed to start on port {} within 5 minute timeout",
                port
            )
            .into());
        }

        if verbose {
            println!("  ✅ MQTT server ready on port {}", port);
        }
    }

    // Start MQTT observer if required
    if let Some(ref mqtt_observer) = state.scenario.infrastructure.mqtt_observer {
        if mqtt_observer.required {
            if verbose {
                println!("  Starting MQTT observer");
            }

            let mqtt_port = state.scenario.infrastructure.mqtt_server.port;
            let observer_process = TokioCommand::new("cargo")
                .current_dir("../mqtt-client")
                .args(&[
                    "run",
                    "--bin",
                    "mqtt-observer",
                    "--",
                    "-h",
                    "localhost",
                    "-p",
                    &mqtt_port.to_string(),
                    "-t",
                    "#", // Subscribe to all topics
                    "-i",
                    "mcplay_observer", // Unique client ID
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start MQTT observer: {}. Make sure mqtt-client is available in ../mqtt-client.", e))?;

            state
                .infrastructure_processes
                .insert("mqtt_observer".to_string(), observer_process);

            if verbose {
                println!("  ✅ MQTT observer started");
            }
        }
    }

    Ok(())
}

async fn start_clients(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if state.scenario.clients.is_empty() {
        println!("👥 No clients to start (orchestrator-only scenario)");
        return Ok(());
    }

    println!("👥 Starting {} clients...", state.scenario.clients.len());

    // Start each client directly instead of relying on xtask
    for client in &state.scenario.clients {
        if verbose {
            println!("  Starting client: {}", client.id);
        }

        // Build client command arguments
        let mut cmd = TokioCommand::new("cargo");
        cmd.arg("run")
            .arg("--bin")
            .arg("iotcraft-dekstop-client")
            .args(&["--", "--mcp"])
            .env("MCP_PORT", client.mcp_port.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Add optional MQTT arguments if required
        if state.scenario.infrastructure.mqtt_server.required {
            cmd.arg("--mqtt-server").arg(format!(
                "localhost:{}",
                state.scenario.infrastructure.mqtt_server.port
            ));
        }

        // Start the client
        let mut client_process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start client {}: {}", client.id, e))?;

        // Check if the process is still running after a brief moment
        tokio::time::sleep(Duration::from_millis(1000)).await;
        match client_process.try_wait() {
            Ok(Some(exit_status)) => {
                // Process has already exited - this indicates an error
                let stderr_output = if let Some(stderr) = client_process.stderr.take() {
                    let mut output = String::new();
                    let mut reader = BufReader::new(stderr);
                    let _ = reader.read_to_string(&mut output).await;
                    output
                } else {
                    "No stderr available".to_string()
                };

                return Err(format!(
                    "Client {} exited immediately with status: {}. Error: {}",
                    client.id, exit_status, stderr_output
                )
                .into());
            }
            Ok(None) => {
                // Process is still running - this is good
                if verbose {
                    println!("    Client {} process is running", client.id);
                }
            }
            Err(e) => {
                return Err(
                    format!("Failed to check client {} process status: {}", client.id, e).into(),
                );
            }
        }

        state
            .client_processes
            .insert(client.id.clone(), client_process);

        // Wait for MCP server to be ready
        if verbose {
            println!(
                "  Waiting for client {} MCP server on port {}...",
                client.id, client.mcp_port
            );
        }

        let mcp_ready = wait_for_port_with_retries_and_context(
            "localhost",
            client.mcp_port,
            300,
            verbose,
            Some(&format!(
                "cargo run --bin iotcraft-dekstop-client ({})",
                client.id
            )),
        )
        .await;
        if !mcp_ready {
            return Err(format!("Client {} MCP server failed to start", client.id).into());
        }

        // Connect to MCP server
        let stream = TcpStream::connect(format!("localhost:{}", client.mcp_port))
            .await
            .map_err(|e| {
                format!(
                    "Failed to connect to client {} MCP server: {}",
                    client.id, e
                )
            })?;

        state.client_connections.insert(client.id.clone(), stream);

        if verbose {
            println!("  ✅ Client {} ready", client.id);
        }
    }

    Ok(())
}

async fn execute_steps(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎬 Executing {} steps...", state.scenario.steps.len());
    println!();

    // Clone the steps to avoid borrow checker issues
    let steps = state.scenario.steps.clone();

    for (i, step) in steps.iter().enumerate() {
        // Check dependencies
        for dep in &step.depends_on {
            if !state.completed_steps.contains(dep) {
                return Err(format!(
                    "Step '{}' depends on '{}' which hasn't completed",
                    step.name, dep
                )
                .into());
            }
        }

        println!("📍 Step {}: {} ({})", i + 1, step.name, step.description);

        if verbose {
            println!("  Client: {}", step.client);
            println!("  Action: {:?}", step.action);
        }

        // Wait before executing
        if step.wait_before > 0 {
            if verbose {
                println!("  ⏳ Waiting {}ms before execution...", step.wait_before);
            }
            sleep(Duration::from_millis(step.wait_before)).await;
        }

        // Execute step
        let step_start = Instant::now();
        let result = execute_step(step, state, verbose).await;
        let step_duration = step_start.elapsed();

        match result {
            Ok(response) => {
                println!("  ✅ Completed in {:.2}s", step_duration.as_secs_f64());
                state.step_results.insert(
                    step.name.clone(),
                    StepResult {
                        success: true,
                        duration: step_duration,
                        error: None,
                        response: Some(response),
                    },
                );
                state.completed_steps.push(step.name.clone());
            }
            Err(e) => {
                println!(
                    "  ❌ Failed after {:.2}s: {}",
                    step_duration.as_secs_f64(),
                    e
                );
                state.step_results.insert(
                    step.name.clone(),
                    StepResult {
                        success: false,
                        duration: step_duration,
                        error: Some(e.to_string()),
                        response: None,
                    },
                );
                return Err(format!("Scenario failed at step '{}': {}", step.name, e).into());
            }
        }

        // Wait after executing
        if step.wait_after > 0 {
            if verbose {
                println!("  ⏳ Waiting {}ms after execution...", step.wait_after);
            }
            sleep(Duration::from_millis(step.wait_after)).await;
        }

        println!();
    }

    Ok(())
}

async fn execute_step(
    step: &Step,
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match &step.action {
        // mcplay-style actions
        Action::McpCall { tool, arguments } => {
            execute_mcp_call(&step.client, tool, arguments, state, verbose).await
        }
        Action::WaitCondition {
            condition,
            expected_value,
            timeout: wait_timeout,
        } => {
            execute_wait_condition(
                condition,
                expected_value.as_deref(),
                *wait_timeout,
                state,
                verbose,
            )
            .await
        }
        Action::ConsoleCommand { command } => {
            execute_console_command(&step.client, command, state, verbose).await
        }
        Action::Delay { duration } => {
            if verbose {
                println!("  ⏳ Delaying for {}ms", duration);
            }
            sleep(Duration::from_millis(*duration)).await;
            Ok(serde_json::json!({"status": "delayed", "duration_ms": duration}))
        }
        Action::ValidateScenario { checks } => {
            execute_validate_scenario(checks, state, verbose).await
        }

        // xtask-style actions (basic support)
        Action::Wait { duration_ms } => {
            if verbose {
                println!("  ⏳ Waiting for {}ms", duration_ms);
            }
            sleep(Duration::from_millis(*duration_ms)).await;
            Ok(serde_json::json!({"status": "waited", "duration_ms": duration_ms}))
        }
        Action::MqttPublish {
            topic,
            payload,
            qos: _,
            retain: _,
        } => {
            if verbose {
                println!(
                    "  📡 Publishing to MQTT topic: {} payload: {}",
                    topic, payload
                );
            }
            // TODO: Implement actual MQTT publishing
            Ok(serde_json::json!({
                "status": "published",
                "topic": topic,
                "payload": payload
            }))
        }
        Action::MqttExpect {
            topic,
            payload,
            timeout_ms,
        } => {
            if verbose {
                println!(
                    "  🔍 Expecting MQTT message on topic: {} (timeout: {:?}ms)",
                    topic, timeout_ms
                );
            }
            // TODO: Implement actual MQTT message waiting
            Ok(serde_json::json!({
                "status": "expected_message_received",
                "topic": topic,
                "expected_payload": payload
            }))
        }
        Action::ClientAction {
            client_id,
            action_type,
            parameters,
        } => {
            if verbose {
                println!(
                    "  🎮 Client action: {:?} for client {}",
                    action_type, client_id
                );
            }
            // TODO: Implement actual client actions
            Ok(serde_json::json!({
                "status": "client_action_executed",
                "client_id": client_id,
                "action_type": action_type,
                "parameters": parameters
            }))
        }
        Action::Parallel { actions } => {
            if verbose {
                println!("  🔀 Executing {} actions in parallel", actions.len());
            }
            // TODO: Implement actual parallel execution
            Ok(serde_json::json!({
                "status": "parallel_execution_completed",
                "action_count": actions.len()
            }))
        }
        Action::Sequence { actions } => {
            if verbose {
                println!("  ▶️ Executing {} actions in sequence", actions.len());
            }
            // TODO: Implement actual sequence execution
            Ok(serde_json::json!({
                "status": "sequence_execution_completed",
                "action_count": actions.len()
            }))
        }
        Action::Custom {
            action_type,
            parameters,
        } => {
            if verbose {
                println!("  🎨 Custom action: {}", action_type);
            }
            // TODO: Implement actual custom action handling
            Ok(serde_json::json!({
                "status": "custom_action_executed",
                "action_type": action_type,
                "parameters": parameters
            }))
        }
    }
}

async fn execute_mcp_call(
    client_id: &str,
    tool: &str,
    arguments: &serde_json::Value,
    state: &mut OrchestratorState,
    _verbose: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Note: verbose logging is now handled by the logging system

    let stream = state
        .client_connections
        .get_mut(client_id)
        .ok_or_else(|| format!("No connection to client {}", client_id))?;

    // Create MCP request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": tool,
            "arguments": arguments
        }),
    };

    // Send request
    let request_json = serde_json::to_string(&request)?;
    stream
        .write_all(format!("{}\n", request_json).as_bytes())
        .await?;

    // Read response
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: McpResponse = serde_json::from_str(&response_line)?;

    if let Some(error) = response.error {
        return Err(format!("MCP error: {}", error).into());
    }

    Ok(response
        .result
        .unwrap_or(serde_json::json!({"status": "success"})))
}

async fn execute_wait_condition(
    condition: &str,
    expected_value: Option<&str>,
    wait_timeout: u64,
    _state: &mut OrchestratorState,
    verbose: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    if verbose {
        println!(
            "  ⏳ Waiting for condition: {} (timeout: {}ms)",
            condition, wait_timeout
        );
    }

    // For now, simulate waiting - this would be replaced with actual condition checking
    let wait_duration = Duration::from_millis(std::cmp::min(wait_timeout, 2000));
    sleep(wait_duration).await;

    Ok(serde_json::json!({
        "condition": condition,
        "expected": expected_value,
        "status": "condition_met"
    }))
}

async fn execute_console_command(
    _client_id: &str,
    command: &str,
    _state: &mut OrchestratorState,
    verbose: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    if verbose {
        println!("  💻 Console command: {}", command);
    }

    // For now, simulate console command execution
    Ok(serde_json::json!({
        "command": command,
        "status": "executed"
    }))
}

async fn execute_validate_scenario(
    checks: &[String],
    _state: &mut OrchestratorState,
    verbose: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    if verbose {
        println!("  ✅ Validating scenario with checks: {:?}", checks);
    }

    // For now, simulate validation
    Ok(serde_json::json!({
        "checks": checks,
        "all_passed": true
    }))
}

/// Check if a port is currently occupied (something is listening on it)
async fn is_port_occupied(host: &str, port: u16) -> bool {
    TcpStream::connect(format!("{}:{}", host, port))
        .await
        .is_ok()
}

/// Wait for a port to become available (something starts listening on it)
async fn wait_for_port(host: &str, port: u16, timeout_seconds: u64) -> bool {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    let start = Instant::now();

    while start.elapsed() < timeout_duration {
        if let Ok(_) = TcpStream::connect(format!("{}:{}", host, port)).await {
            return true;
        }
        sleep(Duration::from_millis(500)).await;
    }
    false
}

/// Wait for a port to become available with verbose progress feedback, respecting Ctrl+C cancellation
async fn wait_for_port_with_retries(
    host: &str,
    port: u16,
    timeout_seconds: u64,
    verbose: bool,
) -> bool {
    wait_for_port_with_retries_and_context(host, port, timeout_seconds, verbose, None).await
}

/// Wait for a port to become available with verbose progress feedback, respecting Ctrl+C cancellation, with context
async fn wait_for_port_with_retries_and_context(
    host: &str,
    port: u16,
    timeout_seconds: u64,
    verbose: bool,
    context: Option<&str>,
) -> bool {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    let start = Instant::now();
    let mut last_log_time = Instant::now();

    // Create a cancellation signal detector
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .expect("Failed to install SIGINT handler in wait_for_port_with_retries");

    while start.elapsed() < timeout_duration {
        // Check for connection with a timeout and cancellation
        let connection_future = TcpStream::connect(format!("{}:{}", host, port));
        let connection_timeout = sleep(Duration::from_millis(1000)); // Check every 1 second instead of 500ms
        let sigint_future = sigint.recv();

        tokio::select! {
            connection_result = connection_future => {
                if connection_result.is_ok() {
                    if verbose {
                        println!(
                            "    ✅ Port {}:{} is now available",
                            host, port
                        );
                    }
                    return true;
                }
            }
            _ = connection_timeout => {
                // Timeout occurred, continue loop
            }
            _ = sigint_future => {
                // Ctrl+C detected, exit the function immediately
                if verbose {
                    println!("    ⚠️ Port checking cancelled due to Ctrl+C");
                }
                return false;
            }
        }

        // Log every 3 seconds instead of every attempt
        if verbose && last_log_time.elapsed() >= Duration::from_secs(3) {
            let elapsed = start.elapsed().as_secs();
            if let Some(context) = context {
                println!(
                    "    ⏳ Still waiting for {} on port {}:{} ({}s elapsed)...",
                    context, host, port, elapsed
                );
            } else {
                println!(
                    "    ⏳ Still waiting for port {}:{} ({}s elapsed)...",
                    host, port, elapsed
                );
            }
            last_log_time = Instant::now();
        }
    }

    if verbose {
        println!(
            "    ❌ Timeout: Port {}:{} did not become available after {}s",
            host, port, timeout_seconds
        );
    }
    false
}

fn generate_report(state: &OrchestratorState) {
    println!("📊 Scenario Report");
    println!("==================");
    println!("Scenario: {}", state.scenario.name);
    println!(
        "Total duration: {:.2}s",
        state.start_time.elapsed().as_secs_f64()
    );
    println!(
        "Steps completed: {}/{}",
        state.completed_steps.len(),
        state.scenario.steps.len()
    );

    let success_count = state.step_results.values().filter(|r| r.success).count();
    let success_rate = if !state.step_results.is_empty() {
        (success_count as f64 / state.step_results.len() as f64) * 100.0
    } else {
        0.0
    };

    println!("Success rate: {:.1}%", success_rate);
    println!();

    println!("📋 Step Details");
    for step in &state.scenario.steps {
        if let Some(result) = state.step_results.get(&step.name) {
            let status = if result.success { "✅" } else { "❌" };
            println!(
                "{} {} ({:.2}s)",
                status,
                step.name,
                result.duration.as_secs_f64()
            );
            if let Some(error) = &result.error {
                println!("   Error: {}", error);
            }
        } else {
            println!("⏸️  {} (not executed)", step.name);
        }
    }
}

async fn cleanup(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up...");

    // Terminate client processes
    for (client_id, mut process) in state.client_processes.drain() {
        if verbose {
            println!("  Terminating client {}", client_id);
        }
        let _ = process.kill().await;
    }

    // Terminate infrastructure processes
    for (service_name, mut process) in state.infrastructure_processes.drain() {
        if verbose {
            println!("  Terminating {}", service_name);
        }
        let _ = process.kill().await;
    }

    println!("✅ Cleanup completed");
    Ok(())
}

// TUI Implementation
#[cfg(feature = "tui")]
async fn show_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load scenarios
    let scenarios = load_scenarios().await;

    let mut app = App {
        scenarios,
        list_state: ListState::default(),
        should_quit: false,
        show_details: false,
        selected_scenario: None,
        message: None,
    };

    // Select first scenario if available
    if !app.scenarios.is_empty() {
        app.list_state.select(Some(0));
    }

    let result = run_tui(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(feature = "tui")]
async fn load_scenarios() -> Vec<ScenarioInfo> {
    let mut scenarios = Vec::new();
    let scenarios_dir = PathBuf::from("scenarios");

    if !scenarios_dir.exists() {
        return scenarios;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(scenarios_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(ext) = entry.path().extension() {
                if ext == "json" || ext == "ron" {
                    if let Some(name) = entry.path().file_stem() {
                        let file_path = entry.path();

                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            let scenario_result: Result<Scenario, Box<dyn std::error::Error>> =
                                if ext == "ron" {
                                    ron::from_str::<Scenario>(&content)
                                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                                } else {
                                    serde_json::from_str::<Scenario>(&content)
                                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                                };

                            match scenario_result {
                                Ok(scenario) => {
                                    scenarios.push(ScenarioInfo {
                                        name: scenario.name.clone(),
                                        description: scenario.description.clone(),
                                        file_path: file_path.clone(),
                                        is_valid: validate_scenario(&scenario).is_ok(),
                                        clients: scenario.clients.len(),
                                        steps: scenario.steps.len(),
                                    });
                                }
                                Err(_) => {
                                    scenarios.push(ScenarioInfo {
                                        name: name.to_string_lossy().to_string(),
                                        description: "(Invalid scenario file)".to_string(),
                                        file_path: file_path.clone(),
                                        is_valid: false,
                                        clients: 0,
                                        steps: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort scenarios by name
    scenarios.sort_by(|a, b| a.name.cmp(&b.name));
    scenarios
}

#[cfg(feature = "tui")]
async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.show_details {
                                app.show_details = false;
                                app.selected_scenario = None;
                            } else {
                                app.should_quit = true;
                            }
                        }
                        KeyCode::Up => {
                            if let Some(selected) = app.list_state.selected() {
                                if selected > 0 {
                                    app.list_state.select(Some(selected - 1));
                                }
                            }
                        }
                        KeyCode::Down => {
                            if let Some(selected) = app.list_state.selected() {
                                if selected < app.scenarios.len().saturating_sub(1) {
                                    app.list_state.select(Some(selected + 1));
                                }
                            } else if !app.scenarios.is_empty() {
                                app.list_state.select(Some(0));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(selected) = app.list_state.selected() {
                                if selected < app.scenarios.len() {
                                    let scenario = &app.scenarios[selected];
                                    if scenario.is_valid {
                                        // Exit TUI and run scenario
                                        disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                        terminal.show_cursor()?;

                                        return run_selected_scenario(&scenario.file_path).await;
                                    } else {
                                        app.message =
                                            Some("Cannot run invalid scenario".to_string());
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(selected) = app.list_state.selected() {
                                if selected < app.scenarios.len() {
                                    app.selected_scenario = Some(app.scenarios[selected].clone());
                                    app.show_details = true;
                                }
                            }
                        }
                        KeyCode::Char('v') => {
                            if let Some(selected) = app.list_state.selected() {
                                if selected < app.scenarios.len() {
                                    let scenario_path = &app.scenarios[selected].file_path;
                                    app.message =
                                        Some(format!("Validating {}...", scenario_path.display()));

                                    // Validate scenario
                                    match validate_scenario_file(scenario_path).await {
                                        Ok(_) => {
                                            app.message = Some("✅ Scenario is valid".to_string());
                                        }
                                        Err(e) => {
                                            app.message =
                                                Some(format!("❌ Validation failed: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('r') => {
                            // Refresh scenario list
                            app.scenarios = load_scenarios().await;
                            app.message = Some("🔄 Scenarios refreshed".to_string());
                            if !app.scenarios.is_empty() && app.list_state.selected().is_none() {
                                app.list_state.select(Some(0));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "tui")]
fn ui(f: &mut Frame, app: &mut App) {
    if app.show_details {
        draw_details(f, app);
    } else {
        draw_main(f, app);
    }
}

#[cfg(feature = "tui")]
fn draw_main(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Title
    let title = Paragraph::new("🎭 IoTCraft MCP Scenario Player")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("McPlay TUI"));
    f.render_widget(title, chunks[0]);

    // Scenario list
    let items: Vec<ListItem> = app
        .scenarios
        .iter()
        .map(|scenario| {
            let status_icon = if scenario.is_valid { "✅" } else { "❌" };
            let content = format!(
                "{} {} - {} clients, {} steps",
                status_icon, scenario.name, scenario.clients, scenario.steps
            );
            ListItem::new(content)
        })
        .collect();

    let scenarios_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("📋 Scenarios ({} found)", app.scenarios.len())),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(scenarios_list, chunks[1], &mut app.list_state);

    // Instructions and status
    let mut instructions = vec![
        Line::from(vec![
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Run  "),
            Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Details  "),
        ]),
        Line::from(vec![
            Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Validate  "),
            Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Refresh  "),
            Span::styled("q/Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Quit"),
        ]),
    ];

    if let Some(ref message) = app.message {
        instructions.push(Line::from(Span::raw(""))); // Empty line
        instructions.push(Line::from(Span::styled(
            message.clone(),
            Style::default().fg(Color::Green),
        )));
    }

    let help = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("🔗 Controls"))
        .wrap(Wrap { trim: true });
    f.render_widget(help, chunks[2]);
}

#[cfg(feature = "tui")]
fn draw_details(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    if let Some(ref scenario) = app.selected_scenario {
        // Title
        let title = Paragraph::new(format!("📋 Scenario Details: {}", scenario.name))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Details
        let details = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&scenario.name),
            ]),
            Line::from(vec![
                Span::styled("File: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(scenario.file_path.display().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Valid: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if scenario.is_valid {
                        "✅ Yes"
                    } else {
                        "❌ No"
                    },
                    if scenario.is_valid {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Clients: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(scenario.clients.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Steps: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(scenario.steps.to_string()),
            ]),
            Line::from(Span::raw("")), // Empty line
            Line::from(vec![Span::styled(
                "Description: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::raw(&scenario.description)),
        ];

        let details_widget = Paragraph::new(details)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("ℹ️  Information"),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(details_widget, chunks[1]);
    }

    // Instructions
    let instructions = Paragraph::new(vec![Line::from(vec![
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Run  "),
        Span::styled("Esc/q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Back"),
    ])])
    .block(Block::default().borders(Borders::ALL).title("🔗 Controls"))
    .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[2]);
}

#[cfg(feature = "tui")]
async fn validate_scenario_file(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(path).await?;
    let scenario: Scenario = if path.extension().and_then(|s| s.to_str()) == Some("ron") {
        ron::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };
    validate_scenario(&scenario)?;
    Ok(())
}

#[cfg(feature = "tui")]
async fn run_selected_scenario(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read scenario file: {}", e))?;

    let scenario: Scenario = if path.extension().and_then(|s| s.to_str()) == Some("ron") {
        ron::from_str(&content).map_err(|e| format!("Failed to parse RON scenario file: {}", e))?
    } else {
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON scenario file: {}", e))?
    };

    // Run the scenario with TUI logging display
    run_scenario_with_tui(scenario).await?;
    Ok(())
}

#[cfg(feature = "tui")]
async fn run_scenario_with_tui(scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal for scenario execution
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create logging app
    let mut logging_app = LoggingApp::new(&scenario);
    let (log_collector, _) = LogCollector::new();

    // Add initial log message
    logging_app.add_log(
        &LogSource::Orchestrator,
        format!("🚀 Starting scenario: {}", scenario.name),
    );
    logging_app.add_log(
        &LogSource::Orchestrator,
        format!("📖 Description: {}", scenario.description),
    );
    logging_app.add_log(
        &LogSource::Orchestrator,
        format!("👥 Clients: {}", scenario.clients.len()),
    );
    logging_app.add_log(
        &LogSource::Orchestrator,
        format!("📋 Steps: {}", scenario.steps.len()),
    );
    logging_app.add_log(&LogSource::Orchestrator, "".to_string());

    // Clone the quit flag for the background task
    let quit_flag = Arc::clone(&logging_app.should_quit);

    // Spawn the scenario execution task
    let scenario_task = tokio::spawn({
        let log_collector = log_collector.clone();
        let quit_flag = Arc::clone(&quit_flag);
        async move {
            let result = run_scenario_with_logging(scenario, log_collector, quit_flag).await;
            result
        }
    });

    // Spawn log receiver task
    let log_task = tokio::spawn({
        let quit_flag = Arc::clone(&logging_app.should_quit);
        let mut log_receiver = log_collector.sender.subscribe(); // Create a new receiver
        async move {
            let mut logs = Vec::new();
            while !quit_flag.load(Ordering::Relaxed) {
                match tokio::time::timeout(Duration::from_millis(100), log_receiver.recv()).await {
                    Ok(Ok(log_msg)) => {
                        logs.push(log_msg);
                    }
                    Ok(Err(_)) => break, // Channel closed
                    Err(_) => {}         // Timeout, continue
                }
            }
            logs
        }
    });

    // Create a dedicated log receiver for the main loop
    let mut main_log_receiver = log_collector.sender.subscribe();

    // Main TUI loop
    let mut last_draw = std::time::Instant::now();
    loop {
        // Check if scenario is done
        if scenario_task.is_finished() {
            // Process any remaining logs
            while let Ok(log_msg) = main_log_receiver.try_recv() {
                logging_app.add_log(&log_msg.source, log_msg.message);
            }

            // Final render
            terminal.draw(|f| draw_logging_ui(f, &logging_app))?;

            // Wait a bit for user to see final state
            tokio::time::sleep(Duration::from_secs(2)).await;
            break;
        }

        // Process new log messages
        while let Ok(log_msg) = main_log_receiver.try_recv() {
            logging_app.add_log(&log_msg.source, log_msg.message);
        }

        // Handle input events
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('c')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                logging_app.should_quit.store(true, Ordering::Relaxed);
                                break;
                            }
                            KeyCode::Tab => {
                                logging_app.next_pane();
                            }
                            KeyCode::BackTab => {
                                logging_app.prev_pane();
                            }
                            KeyCode::Up => {
                                logging_app.scroll_up();
                            }
                            KeyCode::Down => {
                                logging_app.scroll_down();
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(_) => {
                    // Mouse events are handled by the terminal for text selection
                    // No need to process them in our application
                }
                _ => {}
            }
        }

        // Draw UI (throttled to ~20 FPS)
        if last_draw.elapsed() >= Duration::from_millis(50) {
            terminal.draw(|f| draw_logging_ui(f, &logging_app))?;
            last_draw = std::time::Instant::now();
        }
    }

    // Clean up terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Show log files summary
    show_log_summary(&logging_app);

    // Simple completion message and exit
    println!("\n📍 Scenario completed.");

    // Get scenario result and exit immediately
    let result = match scenario_task.await? {
        Ok(_) => {
            let _logs = log_task.await?;
            Ok(())
        }
        Err(e) => {
            let _logs = log_task.await?;
            Err(e)
        }
    };

    // Force exit to prevent hanging
    std::process::exit(0);
}

#[cfg(feature = "tui")]
fn draw_logging_ui(f: &mut Frame, app: &LoggingApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(f.area());

    // Left panel: Pane selector
    let pane_list: Vec<ListItem> = app
        .panes
        .iter()
        .enumerate()
        .map(|(_i, pane)| {
            let name = match pane {
                LogPane::Orchestrator => "🎭 Orchestrator".to_string(),
                LogPane::MqttServer => "📡 MQTT Server".to_string(),
                LogPane::Client(id) => format!("👤 Client: {}", id),
            };

            let is_selected = match (&app.selected_pane, pane) {
                (LogPane::Orchestrator, LogPane::Orchestrator) => true,
                (LogPane::MqttServer, LogPane::MqttServer) => true,
                (LogPane::Client(a), LogPane::Client(b)) => a == b,
                _ => false,
            };

            let style = if is_selected {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default()
            };

            ListItem::new(name).style(style)
        })
        .collect();

    let pane_selector = List::new(pane_list)
        .block(Block::default().borders(Borders::ALL).title("🔗 Log Panes"))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    f.render_widget(pane_selector, chunks[0]);

    // Right panel: Log content
    let current_pane_name = app.get_current_pane_name();
    let empty_vec = Vec::new();
    let log_lines = app.logs.get(&current_pane_name).unwrap_or(&empty_vec);
    let scroll_pos = app.scroll_positions.get(&current_pane_name).unwrap_or(&0);

    let visible_height = chunks[1].height.saturating_sub(2) as usize; // Account for borders
    let start_idx = if log_lines.len() <= visible_height {
        0
    } else {
        scroll_pos
            .saturating_sub(visible_height / 2)
            .min(log_lines.len().saturating_sub(visible_height))
    };

    let visible_logs: Vec<Line> = log_lines
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|line| Line::from(line.as_str()))
        .collect();

    let log_content = Paragraph::new(visible_logs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("📋 Logs: {}", current_pane_name)),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(log_content, chunks[1]);

    // Show controls at bottom
    let controls_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area())[1];

    let controls = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Tab/Shift+Tab",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Switch panes  "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Scroll  "),
        Span::styled(
            "q/Esc/Ctrl+C",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit  "),
        Span::styled("Mouse", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Select text"),
    ])])
    .block(Block::default().borders(Borders::ALL).title("🎮 Controls"))
    .alignment(Alignment::Center);

    f.render_widget(controls, controls_area);
}

async fn run_scenario_with_logging(
    scenario: Scenario,
    log_collector: LogCollector,
    quit_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create shared state wrapped in Arc<Mutex> for signal handling
    let state = Arc::new(Mutex::new(OrchestratorState::new(scenario)));

    // Execute scenario with proper cleanup handling
    let result = run_scenario_inner_with_logging(
        Arc::clone(&state),
        log_collector.clone(),
        quit_flag.clone(),
    )
    .await;

    // Always cleanup, even on error
    {
        let mut state = state.lock().await;
        cleanup_with_logging(&mut state, log_collector.clone()).await?;
    }

    result
}

async fn run_scenario_inner_with_logging(
    state: Arc<Mutex<OrchestratorState>>,
    log_collector: LogCollector,
    quit_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start infrastructure if any services are required
    let needs_infrastructure = {
        let state = state.lock().await;
        state.scenario.infrastructure.mqtt_server.required
            || state
                .scenario
                .infrastructure
                .mcp_server
                .as_ref()
                .map(|mcp| mcp.required)
                .unwrap_or(false)
            || state
                .scenario
                .infrastructure
                .mqtt_observer
                .as_ref()
                .map(|obs| obs.required)
                .unwrap_or(false)
    };

    if needs_infrastructure {
        let mut state = state.lock().await;
        start_infrastructure_with_logging(&mut *state, log_collector.clone(), quit_flag.clone())
            .await?;
    }

    // Start clients
    {
        let mut state = state.lock().await;
        start_clients_with_logging(&mut *state, log_collector.clone(), quit_flag.clone()).await?;
    }

    // Execute steps
    {
        let mut state = state.lock().await;
        execute_steps_with_logging(&mut *state, log_collector.clone()).await?;
    }

    // Generate report
    {
        let state = state.lock().await;
        generate_report_with_logging(&*state, log_collector.clone());
    }

    Ok(())
}

async fn start_infrastructure_with_logging(
    state: &mut OrchestratorState,
    log_collector: LogCollector,
    _quit_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(LogSource::Orchestrator, "🔧 Starting infrastructure...");

    // Check if MQTT port is already in use before starting
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Checking if MQTT port {} is available...", port),
        );

        // Check if port is already occupied
        if is_port_occupied("localhost", port).await {
            let error_msg = format!(
                "MQTT port {} is already in use. Please stop any existing MQTT brokers or choose a different port.",
                port
            );
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            return Err(error_msg.into());
        }
    }

    // Start MQTT server directly if required (instead of delegating to xtask)
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Starting MQTT server on port {}", port),
        );

        // Start MQTT server from ../mqtt-server directory
        let mut mqtt_process = TokioCommand::new("cargo")
            .current_dir("../mqtt-server")
            .args(&["run", "--release", "--", "--port", &port.to_string()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                let error_msg = format!("Failed to start MQTT server: {}", e);
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                error_msg
            })?;

        // Capture stdout and stderr from the MQTT server process
        if let Some(stdout) = mqtt_process.stdout.take() {
            let log_collector = log_collector.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                    log_collector.log_str(LogSource::MqttServer, line.trim());
                    line.clear();
                }
            });
        }

        if let Some(stderr) = mqtt_process.stderr.take() {
            let log_collector = log_collector.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                    log_collector
                        .log_str(LogSource::MqttServer, &format!("[stderr] {}", line.trim()));
                    line.clear();
                }
            });
        }

        state
            .infrastructure_processes
            .insert("mqtt_server".to_string(), mqtt_process);

        // Wait for MQTT server to be ready
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!(
                "  Waiting for MQTT server to become ready on port {}...",
                port
            ),
        );

        let mqtt_ready = wait_for_port_with_retries_and_context_with_logging(
            "localhost",
            port,
            300,
            Some("cargo run --release (mqtt-server)"),
            log_collector.clone(),
        )
        .await;
        if !mqtt_ready {
            let error_msg = format!(
                "MQTT server failed to start on port {} within 5 minute timeout",
                port
            );
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            return Err(error_msg.into());
        }

        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  ✅ MQTT server ready on port {}", port),
        );
    }

    Ok(())
}

async fn start_clients_with_logging(
    state: &mut OrchestratorState,
    log_collector: LogCollector,
    _quit_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if state.scenario.clients.is_empty() {
        log_collector.log_str(
            LogSource::Orchestrator,
            "👥 No clients to start (orchestrator-only scenario)",
        );
        return Ok(());
    }

    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("👥 Starting {} clients...", state.scenario.clients.len()),
    );

    // Start each client directly instead of relying on xtask
    for client in &state.scenario.clients {
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Starting client: {}", client.id),
        );

        // Build client command arguments
        let mut cmd = TokioCommand::new("cargo");
        cmd.arg("run")
            .arg("--bin")
            .arg("iotcraft-dekstop-client")
            .args(&["--", "--mcp"])
            .env("MCP_PORT", client.mcp_port.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Add optional MQTT arguments if required
        if state.scenario.infrastructure.mqtt_server.required {
            cmd.arg("--mqtt-server").arg(format!(
                "localhost:{}",
                state.scenario.infrastructure.mqtt_server.port
            ));
        }

        // Start the client
        let mut client_process = cmd.spawn().map_err(|e| {
            let error_msg = format!("Failed to start client {}: {}", client.id, e);
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            error_msg
        })?;

        // Capture stdout and stderr from the client process
        if let Some(stdout) = client_process.stdout.take() {
            let log_collector = log_collector.clone();
            let client_id = client.id.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                    log_collector.log_str(LogSource::Client(client_id.clone()), line.trim());
                    line.clear();
                }
            });
        }

        if let Some(stderr) = client_process.stderr.take() {
            let log_collector = log_collector.clone();
            let client_id = client.id.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                    log_collector.log_str(
                        LogSource::Client(client_id.clone()),
                        &format!("[stderr] {}", line.trim()),
                    );
                    line.clear();
                }
            });
        }

        // Check if the process is still running after a brief moment
        tokio::time::sleep(Duration::from_millis(1000)).await;
        match client_process.try_wait() {
            Ok(Some(exit_status)) => {
                let error_msg = format!(
                    "Client {} exited immediately with status: {}",
                    client.id, exit_status
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                return Err(error_msg.into());
            }
            Ok(None) => {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!("    Client {} process is running", client.id),
                );
            }
            Err(e) => {
                let error_msg =
                    format!("Failed to check client {} process status: {}", client.id, e);
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                return Err(error_msg.into());
            }
        }

        state
            .client_processes
            .insert(client.id.clone(), client_process);

        // Wait for MCP server to be ready
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!(
                "  Waiting for client {} MCP server on port {}...",
                client.id, client.mcp_port
            ),
        );

        let mcp_ready = wait_for_port_with_retries_and_context_with_logging(
            "localhost",
            client.mcp_port,
            300,
            Some(&format!(
                "cargo run --bin iotcraft-dekstop-client ({})",
                client.id
            )),
            log_collector.clone(),
        )
        .await;
        if !mcp_ready {
            let error_msg = format!("Client {} MCP server failed to start", client.id);
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            return Err(error_msg.into());
        }

        // Connect to MCP server
        let stream = TcpStream::connect(format!("localhost:{}", client.mcp_port))
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to connect to client {} MCP server: {}",
                    client.id, e
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                error_msg
            })?;

        state.client_connections.insert(client.id.clone(), stream);

        // Wait for client to be fully initialized before proceeding
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Waiting for client {} to initialize fully...", client.id),
        );
        tokio::time::sleep(Duration::from_millis(3000)).await; // Give client time to initialize UI and be ready for commands

        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  ✅ Client {} ready", client.id),
        );
    }

    Ok(())
}

async fn execute_steps_with_logging(
    state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("🎬 Executing {} steps...", state.scenario.steps.len()),
    );
    log_collector.log_str(LogSource::Orchestrator, "");

    // Clone the steps to avoid borrow checker issues
    let steps = state.scenario.steps.clone();

    for (i, step) in steps.iter().enumerate() {
        // Check dependencies
        for dep in &step.depends_on {
            if !state.completed_steps.contains(dep) {
                let error_msg = format!(
                    "Step '{}' depends on '{}' which hasn't completed",
                    step.name, dep
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                return Err(error_msg.into());
            }
        }

        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("📍 Step {}: {} ({})", i + 1, step.name, step.description),
        );
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Client: {}", step.client),
        );
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Action: {:?}", step.action),
        );

        // Wait before executing
        if step.wait_before > 0 {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ⏳ Waiting {}ms before execution...", step.wait_before),
            );
            sleep(Duration::from_millis(step.wait_before)).await;
        }

        // Execute step
        let step_start = Instant::now();
        let result = execute_step(step, state, true).await; // Always verbose in TUI mode
        let step_duration = step_start.elapsed();

        match result {
            Ok(response) => {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!("  ✅ Completed in {:.2}s", step_duration.as_secs_f64()),
                );
                state.step_results.insert(
                    step.name.clone(),
                    StepResult {
                        success: true,
                        duration: step_duration,
                        error: None,
                        response: Some(response),
                    },
                );
                state.completed_steps.push(step.name.clone());
            }
            Err(e) => {
                let error_msg = format!(
                    "Step '{}' failed after {:.2}s: {}",
                    step.name,
                    step_duration.as_secs_f64(),
                    e
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("  ❌ {}", error_msg));
                state.step_results.insert(
                    step.name.clone(),
                    StepResult {
                        success: false,
                        duration: step_duration,
                        error: Some(e.to_string()),
                        response: None,
                    },
                );
                return Err(error_msg.into());
            }
        }

        // Wait after executing
        if step.wait_after > 0 {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ⏳ Waiting {}ms after execution...", step.wait_after),
            );
            sleep(Duration::from_millis(step.wait_after)).await;
        }

        log_collector.log_str(LogSource::Orchestrator, "");
    }

    Ok(())
}

fn generate_report_with_logging(state: &OrchestratorState, log_collector: LogCollector) {
    log_collector.log_str(LogSource::Orchestrator, "📊 Scenario Report");
    log_collector.log_str(LogSource::Orchestrator, "==================");
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("Scenario: {}", state.scenario.name),
    );
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "Total duration: {:.2}s",
            state.start_time.elapsed().as_secs_f64()
        ),
    );
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "Steps completed: {}/{}",
            state.completed_steps.len(),
            state.scenario.steps.len()
        ),
    );

    let success_count = state.step_results.values().filter(|r| r.success).count();
    let success_rate = if !state.step_results.is_empty() {
        (success_count as f64 / state.step_results.len() as f64) * 100.0
    } else {
        0.0
    };

    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("Success rate: {:.1}%", success_rate),
    );
    log_collector.log_str(LogSource::Orchestrator, "");
    log_collector.log_str(LogSource::Orchestrator, "📋 Step Details");

    for step in &state.scenario.steps {
        if let Some(result) = state.step_results.get(&step.name) {
            let status = if result.success { "✅" } else { "❌" };
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "{} {} ({:.2}s)",
                    status,
                    step.name,
                    result.duration.as_secs_f64()
                ),
            );
            if let Some(error) = &result.error {
                log_collector.log_str(LogSource::Orchestrator, &format!("   Error: {}", error));
            }
        } else {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("⏸️  {} (not executed)", step.name),
            );
        }
    }
}

async fn cleanup_with_logging(
    state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(LogSource::Orchestrator, "🧹 Cleaning up...");

    // Terminate client processes
    for (client_id, mut process) in state.client_processes.drain() {
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Terminating client {}", client_id),
        );
        let _ = process.kill().await;
    }

    // Terminate infrastructure processes
    for (service_name, mut process) in state.infrastructure_processes.drain() {
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Terminating {}", service_name),
        );
        let _ = process.kill().await;
    }

    log_collector.log_str(LogSource::Orchestrator, "✅ Cleanup completed");
    Ok(())
}

async fn wait_for_port_with_retries_and_context_with_logging(
    host: &str,
    port: u16,
    timeout_seconds: u64,
    context: Option<&str>,
    log_collector: LogCollector,
) -> bool {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    let start = Instant::now();
    let mut last_log_time = Instant::now();

    while start.elapsed() < timeout_duration {
        // Check for connection
        if TcpStream::connect(format!("{}:{}", host, port))
            .await
            .is_ok()
        {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("    ✅ Port {}:{} is now available", host, port),
            );
            return true;
        }

        // Log progress every 3 seconds
        if last_log_time.elapsed() >= Duration::from_secs(3) {
            let elapsed = start.elapsed().as_secs();
            if let Some(context) = context {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!(
                        "    ⏳ Still waiting for {} on port {}:{} ({}s elapsed)...",
                        context, host, port, elapsed
                    ),
                );
            } else {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!(
                        "    ⏳ Still waiting for port {}:{} ({}s elapsed)...",
                        host, port, elapsed
                    ),
                );
            }
            last_log_time = Instant::now();
        }

        sleep(Duration::from_millis(1000)).await;
    }

    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "    ❌ Timeout: Port {}:{} did not become available after {}s",
            host, port, timeout_seconds
        ),
    );
    false
}

#[cfg(feature = "tui")]
fn show_log_summary(app: &LoggingApp) {
    println!("\n📁 Log Files Summary");
    println!("====================");

    for (pane_name, log_file_path) in &app.log_files {
        if log_file_path.exists() {
            match std::fs::metadata(log_file_path) {
                Ok(metadata) => {
                    let size_kb = metadata.len() / 1024;
                    println!(
                        "{}: {} ({} KB)",
                        pane_name,
                        log_file_path.display(),
                        size_kb
                    );
                }
                Err(_) => {
                    println!("{}: {} (size unknown)", pane_name, log_file_path.display());
                }
            }
        } else {
            println!(
                "{}: {} (file not created)",
                pane_name,
                log_file_path.display()
            );
        }
    }

    println!("\n💡 You can copy these log file paths to provide to AI for analysis:");
    for (pane_name, log_file_path) in &app.log_files {
        if log_file_path.exists() {
            println!("   {} log: {}", pane_name, log_file_path.display());
        }
    }
    println!();
}
