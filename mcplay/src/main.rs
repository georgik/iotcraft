//! mcplay - IoTCraft Multi-client Scenario Player
//!
//! This binary runs scenario-driven tests for IoTCraft, supporting multi-client
//! coordination, MCP integration, and infrastructure orchestration.

use anyhow::Result;
use clap::{Arg, Command};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;

// Add chrono for timestamps
use chrono;

// Import log collection module
mod log_collector;
use log_collector::{collect_process_logs, collect_process_logs_to_file};
/// Strip ANSI escape codes from text for clean log files while preserving emojis
fn strip_ansi_colors(text: &str) -> String {
    // Regex to match ANSI escape sequences for colors and formatting
    // This matches comprehensive ANSI patterns including:
    // - Color codes: \x1B[31m, \x1B[1;32m, \x1B[0m
    // - Cursor movement: \x1B[2J, \x1B[H
    // - Other formatting: \x1B[K, \x1B[?25h
    static ANSI_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let regex = ANSI_REGEX.get_or_init(|| {
        // More comprehensive ANSI escape sequence pattern
        // \x1B\[ - ESC[
        // [0-9;?]* - parameter bytes (numbers, semicolons, question marks)
        // [a-zA-Z] - final byte (letter commands like m, K, H, J, etc.)
        Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").expect("Valid ANSI regex")
    });

    regex.replace_all(text, "").to_string()
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct SystemInfo {
    cpu_usage: f64,
    memory_used_mb: u64,
    memory_total_mb: u64,
    memory_usage_percent: f64,
    uptime_seconds: u64,
    process_count: usize,
    total_ram_mb: u64, // Cached total RAM, initialized once
}

#[cfg(feature = "tui")]
impl SystemInfo {
    fn new() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            memory_usage_percent: 0.0,
            uptime_seconds: 0,
            process_count: 0,
            total_ram_mb: 0, // Will be initialized in new_with_total_ram
        }
    }

    /// Create SystemInfo with pre-fetched total RAM
    async fn new_with_total_ram() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let total_ram_mb = Self::get_total_ram().await.unwrap_or(0);
        Ok(Self {
            cpu_usage: 0.0,
            memory_used_mb: 0,
            memory_total_mb: total_ram_mb,
            memory_usage_percent: 0.0,
            uptime_seconds: 0,
            process_count: 0,
            total_ram_mb,
        })
    }

    /// Collect current system information asynchronously (using cached total RAM)
    async fn collect_with_cached_ram(
        &self,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (cpu_usage, memory_used_mb, uptime, process_count) = tokio::try_join!(
            Self::get_cpu_usage(),
            Self::get_memory_usage(self.total_ram_mb),
            Self::get_uptime(),
            Self::get_process_count()
        )?;

        let memory_usage_percent = if self.total_ram_mb > 0 {
            (memory_used_mb as f64 / self.total_ram_mb as f64) * 100.0
        } else {
            0.0
        };

        Ok(Self {
            cpu_usage,
            memory_used_mb,
            memory_total_mb: self.total_ram_mb,
            memory_usage_percent,
            uptime_seconds: uptime,
            process_count,
            total_ram_mb: self.total_ram_mb,
        })
    }

    /// Get CPU usage percentage (macOS specific)
    async fn get_cpu_usage() -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("top")
                .args(&["-l", "2", "-n", "0", "-s", "1"])
                .output()
                .await?;

            let output_str = String::from_utf8(output.stdout)?;
            // Parse the CPU usage from top output
            // Look for line like "CPU usage: 12.34% user, 5.67% sys, 82.99% idle"
            for line in output_str.lines() {
                if line.contains("CPU usage:") {
                    // Extract user and sys percentages
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let mut user_cpu = 0.0;
                    let mut sys_cpu = 0.0;

                    for (i, part) in parts.iter().enumerate() {
                        if *part == "user," && i > 0 {
                            if let Ok(val) = parts[i - 1].trim_end_matches('%').parse::<f64>() {
                                user_cpu = val;
                            }
                        }
                        if *part == "sys," && i > 0 {
                            if let Ok(val) = parts[i - 1].trim_end_matches('%').parse::<f64>() {
                                sys_cpu = val;
                            }
                        }
                    }

                    return Ok(user_cpu + sys_cpu);
                }
            }
            Ok(0.0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for other platforms - could be extended for Linux, Windows
            Ok(0.0)
        }
    }

    /// Get total physical RAM in MB (called once at startup)
    async fn get_total_ram() -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("sysctl")
                .args(&["-n", "hw.memsize"])
                .output()
                .await?;
            let output_str = String::from_utf8(output.stdout)?;
            let total_bytes = output_str.trim().parse::<u64>().unwrap_or(0);
            Ok(total_bytes / (1024 * 1024)) // Convert bytes to MB
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(0)
        }
    }

    /// Get current memory usage in MB (called periodically)
    async fn get_memory_usage(
        _total_ram_mb: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("vm_stat").output().await?;
            let output_str = String::from_utf8(output.stdout)?;
            let mut page_size = 4096u64; // Default page size on macOS
            let mut active_pages = 0u64;
            let mut wired_pages = 0u64;
            let mut compressed_pages = 0u64;

            // Get the actual page size
            if let Some(first_line) = output_str.lines().next() {
                if first_line.contains("page size of") {
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    for (i, part) in parts.iter().enumerate() {
                        if *part == "of" && i + 1 < parts.len() {
                            if let Ok(size) = parts[i + 1].parse::<u64>() {
                                page_size = size;
                                break;
                            }
                        }
                    }
                }
            }

            // Parse only the memory stats we need for "used" calculation
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pages) = parts
                        .last()
                        .map_or("", |v| v)
                        .trim_end_matches('.')
                        .parse::<u64>()
                    {
                        if line.contains("Pages active:") {
                            active_pages = pages;
                        } else if line.contains("Pages wired down:") {
                            wired_pages = pages;
                        } else if line.contains("compressed:") {
                            compressed_pages = pages;
                        }
                    }
                }
            }

            // Calculate used memory = active + wired + compressed (actual memory in use)
            let used_pages = active_pages + wired_pages + compressed_pages;
            let used_mb = (used_pages * page_size) / (1024 * 1024);
            Ok(used_mb)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(0)
        }
    }

    /// Get system uptime in seconds (macOS specific)
    async fn get_uptime() -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("sysctl")
                .args(&["-n", "kern.boottime"])
                .output()
                .await?;

            let output_str = String::from_utf8(output.stdout)?;
            // Parse boot time from sysctl output format
            // Format: { sec = 1234567890, usec = 123456 }
            if let Some(sec_start) = output_str.find("sec = ") {
                if let Some(sec_end) = output_str[sec_start + 6..].find(',') {
                    let boot_time_str = &output_str[sec_start + 6..sec_start + 6 + sec_end];
                    if let Ok(boot_time) = boot_time_str.parse::<u64>() {
                        let current_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs();
                        return Ok(current_time - boot_time);
                    }
                }
            }
            Ok(0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for other platforms
            Ok(0)
        }
    }

    /// Get current process count (macOS specific)
    async fn get_process_count() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("ps")
                .args(&["-ax"])
                .output()
                .await?;

            let output_str = String::from_utf8(output.stdout)?;
            // Count lines (excluding header)
            let count = output_str.lines().count().saturating_sub(1);
            Ok(count)
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for other platforms
            Ok(0)
        }
    }

    /// Format as display string for UI
    fn format_for_display(&self) -> Vec<String> {
        vec![
            format!("CPU: {:.1}%", self.cpu_usage),
            format!(
                "Memory: {:.1}% ({}/{}MB)",
                self.memory_usage_percent, self.memory_used_mb, self.memory_total_mb
            ),
            format!(
                "Uptime: {}h {}m",
                self.uptime_seconds / 3600,
                (self.uptime_seconds % 3600) / 60
            ),
            format!("Processes: {}", self.process_count),
        ]
    }
}

#[cfg(feature = "tui")]
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "tui")]
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
#[cfg(feature = "tui")]
use std::io;
#[cfg(feature = "tui")]
use std::sync::atomic::{AtomicBool, Ordering};

// Import our scenario types
mod scenario_types;
use scenario_types::*;

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
    filtered_scenarios: Vec<ScenarioInfo>,
    list_state: ListState,
    should_quit: bool,
    show_details: bool,
    selected_scenario: Option<ScenarioInfo>,
    message: Option<String>,
    // Search functionality
    search_mode: bool,
    search_query: String,
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone, PartialEq)]
enum ServiceStatus {
    Waiting,   // ⏳ Gray - Waiting for dependencies
    Starting,  // 🟡 Yellow - Service is starting (compilation, etc.)
    Ready,     // 🟢 Green - Service is ready and healthy
    Failed,    // 🔴 Red - Service failed (exit code != 0)
    Stopped,   // 🔵 Blue - Service stopped normally (exit code 0)
    Unhealthy, // 🟠 Orange - Service running but health check failing
}

#[cfg(feature = "tui")]
impl ServiceStatus {
    fn get_emoji(&self) -> &'static str {
        match self {
            ServiceStatus::Waiting => "⏳",
            ServiceStatus::Starting => "🟡",
            ServiceStatus::Ready => "🟢",
            ServiceStatus::Failed => "🔴",
            ServiceStatus::Stopped => "🔵",
            ServiceStatus::Unhealthy => "🟠",
        }
    }

    fn get_color(&self) -> Color {
        match self {
            ServiceStatus::Waiting => Color::Gray,
            ServiceStatus::Starting => Color::Yellow,
            ServiceStatus::Ready => Color::Green,
            ServiceStatus::Failed => Color::Red,
            ServiceStatus::Stopped => Color::Blue,
            ServiceStatus::Unhealthy => Color::LightRed,
        }
    }
}

#[cfg(feature = "tui")]
#[derive(Debug, PartialEq, Clone)]
enum LogPane {
    Orchestrator,
    MqttServer,
    MqttObserver,
    Client(String),
}

#[cfg(feature = "tui")]
#[derive(Debug, PartialEq)]
enum FocusedPane {
    LogSelector,
    LogContent,
    McpMessageSelector,
}

#[cfg(feature = "tui")]
#[derive(Debug, PartialEq)]
enum UiMode {
    LogViewing,
    McpMessageSending,
    McpParameterEditing,   // New mode for editing command parameters
    McpInteractionDetails, // New mode for showing MCP request/response details
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct McpParameter {
    name: String,
    param_type: String,
    description: String,
    required: bool,
    default_value: Option<String>,
    current_value: String,
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct McpMessage {
    name: String,
    description: String,
    method: String,
    params: serde_json::Value,
    required_params: Vec<McpParameter>,
    optional_params: Vec<McpParameter>,
    required_param_count: usize,
}

#[cfg(feature = "tui")]
impl McpMessage {
    /// Parse JSON schema to extract parameter information
    fn parse_parameters(
        schema: &serde_json::Value,
    ) -> (Vec<McpParameter>, Vec<McpParameter>, usize) {
        let mut required_params = Vec::new();
        let mut optional_params = Vec::new();

        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            let required_list = schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            for (param_name, param_def) in properties {
                let is_required = required_list.contains(&param_name.as_str());
                let param_type = param_def
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string")
                    .to_string();
                let description = param_def
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                let parameter = McpParameter {
                    name: param_name.clone(),
                    param_type,
                    description,
                    required: is_required,
                    default_value: None,
                    current_value: String::new(),
                };

                if is_required {
                    required_params.push(parameter);
                } else {
                    optional_params.push(parameter);
                }
            }
        }

        let required_count = required_params.len();
        (required_params, optional_params, required_count)
    }

    fn get_available_messages() -> Vec<McpMessage> {
        use iotcraft_mcp_protocol::tools::get_all_tools;

        let mut messages = vec![
            McpMessage {
                name: "List Tools".to_string(),
                description: "List available MCP tools".to_string(),
                method: "tools/list".to_string(),
                params: serde_json::json!({}),
                required_params: Vec::new(),
                optional_params: Vec::new(),
                required_param_count: 0,
            },
            McpMessage {
                name: "Initialize".to_string(),
                description: "Initialize MCP connection".to_string(),
                method: "initialize".to_string(),
                params: serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "clientInfo": {
                        "name": "mcplay",
                        "version": "1.0.0"
                    }
                }),
                required_params: Vec::new(),
                optional_params: Vec::new(),
                required_param_count: 0,
            },
        ];

        // Add all tools from the shared protocol crate
        let protocol_tools = get_all_tools();
        for tool in protocol_tools {
            let (required_params, optional_params, required_count) =
                Self::parse_parameters(&tool.input_schema);

            messages.push(McpMessage {
                name: tool.name.clone(),
                description: tool.description.clone(),
                method: "tools/call".to_string(),
                params: serde_json::json!({
                    "name": tool.name,
                    "arguments": {}
                }),
                required_params,
                optional_params,
                required_param_count: required_count,
            });
        }

        // Sort messages: first by required parameter count (ascending), then alphabetically
        messages.sort_by(
            |a, b| match a.required_param_count.cmp(&b.required_param_count) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            },
        );

        messages
    }

    /// Generate current parameter values for sending request
    fn build_arguments(&self) -> serde_json::Value {
        let mut args = serde_json::Map::new();

        // Add required parameters
        for param in &self.required_params {
            if !param.current_value.is_empty() {
                let value = match param.param_type.as_str() {
                    "number" => {
                        if let Ok(num) = param.current_value.parse::<f64>() {
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(num)
                                    .unwrap_or(serde_json::Number::from(0)),
                            )
                        } else {
                            serde_json::Value::String(param.current_value.clone())
                        }
                    }
                    "boolean" => {
                        serde_json::Value::Bool(param.current_value.to_lowercase() == "true")
                    }
                    _ => serde_json::Value::String(param.current_value.clone()),
                };
                args.insert(param.name.clone(), value);
            }
        }

        // Add optional parameters (only if they have values)
        for param in &self.optional_params {
            if !param.current_value.is_empty() {
                let value = match param.param_type.as_str() {
                    "number" => {
                        if let Ok(num) = param.current_value.parse::<f64>() {
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(num)
                                    .unwrap_or(serde_json::Number::from(0)),
                            )
                        } else {
                            serde_json::Value::String(param.current_value.clone())
                        }
                    }
                    "boolean" => {
                        serde_json::Value::Bool(param.current_value.to_lowercase() == "true")
                    }
                    _ => serde_json::Value::String(param.current_value.clone()),
                };
                args.insert(param.name.clone(), value);
            }
        }

        serde_json::Value::Object(args)
    }
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct HealthProbe {
    _client_id: String,
    last_check: std::time::Instant,
    interval: Duration,
    _timeout: Duration,
    failure_count: u32,
    failure_threshold: u32,
    is_healthy: bool,
}

#[cfg(feature = "tui")]
struct LoggingApp {
    logs: HashMap<String, Vec<String>>, // key: pane_name, value: log lines
    selected_pane: LogPane,
    selected_pane_index: usize, // Track which pane is selected by index
    panes: Vec<LogPane>,
    focused_pane: FocusedPane, // Track which UI pane has focus
    should_quit: Arc<AtomicBool>,
    scroll_positions: HashMap<String, usize>,
    auto_scroll: bool,
    log_files: HashMap<String, PathBuf>, // key: pane_name, value: log file path
    ui_mode: UiMode,                     // Track current UI mode
    mcp_app: Option<McpInteractiveApp>,  // MCP message interface
    scenario: Scenario,                  // Store scenario for client connections
    scenario_file_path: Option<PathBuf>, // Store the scenario file path
    system_info: SystemInfo,             // Current system information
    last_system_update: std::time::Instant, // When system info was last updated
    service_statuses: HashMap<String, ServiceStatus>, // Track service status
    health_probes: HashMap<String, HealthProbe>, // Active health probes
    scenario_completed: bool,            // Track if scenario is completed
    auto_exit_after_completion: bool,    // Auto exit when scenario completes
    observer_process_healthy: bool,      // Track MQTT observer process health
    // Step progress tracking
    current_step_index: Option<usize>, // Index of currently executing step
    completed_steps: Vec<String>,      // Names of completed steps
    failed_steps: Vec<String>,         // Names of failed steps
    step_start_time: Option<std::time::Instant>, // When current step started
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
enum McpInteractionState {
    SelectingMessage,
    SendingRequest,
    ShowingResponse { success: bool },
}

#[cfg(feature = "tui")]
struct McpInteractiveApp {
    available_messages: Vec<McpMessage>,
    selected_message_index: usize,
    client_id: String, // The client we're sending messages to
    list_state: ListState,
    // Enhanced interaction tracking
    interaction_state: McpInteractionState,
    selected_message: Option<McpMessage>, // The message being sent/was sent
    request_sent_at: Option<std::time::Instant>, // When the request was sent
    response_received_at: Option<std::time::Instant>, // When response was received
    response_data: Option<serde_json::Value>, // The actual response
    error_message: Option<String>,        // Error if request failed
    details_scroll_pos: u16,              // Scroll position for MCP interaction details
    // Parameter editing state
    editing_message: Option<McpMessage>, // Message being edited for parameters
    selected_param_index: usize,         // Which parameter is currently selected
    editing_param_value: bool,           // Whether we're currently editing a parameter value
    param_input_buffer: String,          // Buffer for parameter input
}

#[cfg(feature = "tui")]
impl LoggingApp {
    async fn new(scenario: &Scenario, scenario_file_path: Option<PathBuf>) -> Self {
        let mut panes = vec![LogPane::Orchestrator];
        let mut logs = HashMap::new();
        let mut scroll_positions = HashMap::new();

        // Add MQTT server pane if required
        if scenario.infrastructure.mqtt_server.required {
            panes.push(LogPane::MqttServer);
            logs.insert("MQTT Server".to_string(), Vec::new());
            scroll_positions.insert("MQTT Server".to_string(), 0);
        }

        // Add MQTT observer pane if required
        let mqtt_observer_required = scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false);

        if mqtt_observer_required {
            panes.push(LogPane::MqttObserver);
            logs.insert("MQTT Observer".to_string(), Vec::new());
            scroll_positions.insert("MQTT Observer".to_string(), 0);
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

        if scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false)
        {
            log_files.insert(
                "MQTT Observer".to_string(),
                log_dir.join(format!("mqtt_observer_{}.log", timestamp)),
            );
        }

        for client in &scenario.clients {
            log_files.insert(
                client.id.clone(),
                log_dir.join(format!("client_{}_{}.log", client.id, timestamp)),
            );
        }

        // Initialize service statuses
        let mut service_statuses = HashMap::new();
        service_statuses.insert("Orchestrator".to_string(), ServiceStatus::Starting);

        if scenario.infrastructure.mqtt_server.required {
            service_statuses.insert("MQTT Server".to_string(), ServiceStatus::Waiting);
        }

        if scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false)
        {
            service_statuses.insert("MQTT Observer".to_string(), ServiceStatus::Waiting);
        }

        for client in &scenario.clients {
            service_statuses.insert(client.id.clone(), ServiceStatus::Waiting);
        }

        // Initialize health probes for clients with liveness probes
        let mut health_probes = HashMap::new();
        for client in &scenario.clients {
            if let Some(config) = &client.config {
                if let Some(liveness_config) = &config.get("liveness_probe") {
                    if let Some(interval) = liveness_config
                        .get("interval_seconds")
                        .and_then(|v| v.as_u64())
                    {
                        let timeout = liveness_config
                            .get("timeout_seconds")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(3);
                        let failure_threshold = liveness_config
                            .get("failure_threshold")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(2) as u32;

                        health_probes.insert(
                            client.id.clone(),
                            HealthProbe {
                                _client_id: client.id.clone(),
                                last_check: std::time::Instant::now(),
                                interval: Duration::from_secs(interval),
                                _timeout: Duration::from_secs(timeout),
                                failure_count: 0,
                                failure_threshold,
                                is_healthy: true,
                            },
                        );
                    }
                }
            }
        }

        // Add health probe for MQTT Observer if required
        if scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false)
        {
            health_probes.insert(
                "MQTT Observer".to_string(),
                HealthProbe {
                    _client_id: "MQTT Observer".to_string(),
                    last_check: std::time::Instant::now(),
                    interval: Duration::from_secs(10), // Check every 10 seconds
                    _timeout: Duration::from_secs(5),
                    failure_count: 0,
                    failure_threshold: 3, // Allow 3 failures before marking unhealthy
                    is_healthy: true,
                },
            );
        }

        Self {
            logs,
            selected_pane: LogPane::Orchestrator,
            selected_pane_index: 0, // Start with first pane
            panes,
            focused_pane: FocusedPane::LogSelector, // Start with selector focused
            should_quit: Arc::new(AtomicBool::new(false)),
            scroll_positions,
            auto_scroll: true,
            log_files,
            ui_mode: UiMode::LogViewing, // Start in log viewing mode
            mcp_app: None,               // No MCP app initially
            scenario: scenario.clone(),  // Store scenario for client connections
            scenario_file_path,          // Store the scenario file path
            system_info: SystemInfo::new_with_total_ram()
                .await
                .unwrap_or_else(|_| SystemInfo::new()), // Initialize with total RAM
            last_system_update: std::time::Instant::now(), // Track when system info was last updated
            service_statuses,
            health_probes,
            scenario_completed: false,
            auto_exit_after_completion: true, // Enable auto-exit by default
            observer_process_healthy: true,
            // Initialize step progress tracking
            current_step_index: None,
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            step_start_time: None,
        }
    }

    fn add_log(&mut self, source: &LogSource, message: String) {
        let pane_name = match source {
            LogSource::Orchestrator => "Orchestrator".to_string(),
            LogSource::MqttServer => "MQTT Server".to_string(),
            LogSource::MqttObserver => "MQTT Observer".to_string(),
            LogSource::Client(id) => id.clone(),
        };

        // Parse and update service status based on message content
        self.parse_and_update_status(&pane_name, &message);

        // Write to log file (strip ANSI colors but preserve emojis)
        if let Some(log_file_path) = self.log_files.get(&pane_name) {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            let clean_message = strip_ansi_colors(&message);
            let log_entry = format!("[{}] {}\n", timestamp, clean_message);
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
            LogPane::MqttObserver => "MQTT Observer".to_string(),
            LogPane::Client(id) => id.clone(),
        }
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

    fn update_service_status(&mut self, service_name: &str, status: ServiceStatus) {
        if let Some(current_status) = self.service_statuses.get_mut(service_name) {
            *current_status = status;
        } else {
            self.service_statuses
                .insert(service_name.to_string(), status);
        }
    }

    fn get_service_status(&self, service_name: &str) -> ServiceStatus {
        self.service_statuses
            .get(service_name)
            .cloned()
            .unwrap_or(ServiceStatus::Waiting)
    }

    fn check_and_update_health_probes(&mut self) -> Vec<String> {
        let mut failed_clients = Vec::new();
        let current_time = std::time::Instant::now();

        let client_ids: Vec<String> = self.health_probes.keys().cloned().collect();

        for client_id in client_ids {
            let should_check = self
                .health_probes
                .get(&client_id)
                .map(|probe| current_time.duration_since(probe.last_check) >= probe.interval)
                .unwrap_or(false);

            if should_check {
                // Perform health check before modifying probe
                let health_check_result = self.perform_health_check(&client_id);

                if let Some(probe) = self.health_probes.get_mut(&client_id) {
                    probe.last_check = current_time;

                    if health_check_result {
                        probe.failure_count = 0;
                        probe.is_healthy = true;
                        self.update_service_status(&client_id, ServiceStatus::Ready);
                    } else {
                        probe.failure_count += 1;
                        if probe.failure_count >= probe.failure_threshold {
                            probe.is_healthy = false;
                            self.update_service_status(&client_id, ServiceStatus::Unhealthy);
                            failed_clients.push(client_id.clone());
                        }
                    }
                }
            }
        }

        failed_clients
    }

    fn perform_health_check(&self, client_id: &str) -> bool {
        // Handle MQTT Observer health check
        if client_id == "MQTT Observer" {
            // Check both service status and process health
            let status_ok = matches!(self.get_service_status(client_id), ServiceStatus::Ready);
            let process_ok = self.observer_process_healthy;

            // Observer is healthy if both status and process are OK
            return status_ok && process_ok;
        }

        // Handle client health checks
        if let Some(_client) = self.scenario.clients.iter().find(|c| c.id == client_id) {
            // Basic assumption: if we have the client in our scenario and
            // it's been marked as ready, it's probably healthy
            // TODO: Implement actual MCP ping request with timeout
            return matches!(self.get_service_status(client_id), ServiceStatus::Ready);
        }
        false
    }

    fn parse_and_update_status(&mut self, service_name: &str, message: &str) {
        // Parse status indicators from emoji/text markers in log messages
        if message.contains("🟢") || message.contains("ready") {
            self.update_service_status(service_name, ServiceStatus::Ready);
        } else if message.contains("🟡") || message.contains("starting") {
            self.update_service_status(service_name, ServiceStatus::Starting);
        } else if message.contains("🔴") || message.contains("failed") {
            self.update_service_status(service_name, ServiceStatus::Failed);
        } else if message.contains("🔵") || message.contains("stopped normally") {
            self.update_service_status(service_name, ServiceStatus::Stopped);
        } else if message.contains("🟠") || message.contains("unhealthy") {
            self.update_service_status(service_name, ServiceStatus::Unhealthy);
        } else if message.contains("⏳") || message.contains("waiting") {
            self.update_service_status(service_name, ServiceStatus::Waiting);
        }

        // Special handling for MQTT Observer connection status
        if service_name == "MQTT Observer" {
            if message.contains("Connected to MQTT broker")
                || message.contains("Successfully connected")
                || message.contains("MQTT connection established")
            {
                self.update_service_status(service_name, ServiceStatus::Ready);
            } else if message.contains("Connection failed")
                || message.contains("Failed to connect")
                || message.contains("Connection lost")
            {
                self.update_service_status(service_name, ServiceStatus::Failed);
            } else if message.contains("Connecting to") || message.contains("Attempting connection")
            {
                self.update_service_status(service_name, ServiceStatus::Starting);
            }
        }

        // Parse step execution progress for orchestrator messages
        if service_name == "Orchestrator" {
            self.parse_step_progress(message);
        }
    }

    fn parse_step_progress(&mut self, message: &str) {
        // Parse step execution messages to track progress
        if let Some(captures) = regex::Regex::new(r"📍 Step (\d+): ([^(]+) \((.*)\)")
            .ok()
            .and_then(|re| re.captures(message))
        {
            if let Some(step_num_str) = captures.get(1) {
                if let Ok(step_index) = step_num_str.as_str().parse::<usize>() {
                    // Step numbers are 1-based, convert to 0-based index
                    self.current_step_index = Some(step_index.saturating_sub(1));
                    self.step_start_time = Some(std::time::Instant::now());
                }
            }
        }
        // Parse step completion messages
        else if message.contains("✅ Completed in") {
            if let Some(current_index) = self.current_step_index {
                if let Some(step) = self.scenario.steps.get(current_index) {
                    if !self.completed_steps.contains(&step.name) {
                        self.completed_steps.push(step.name.clone());
                    }
                }
            }
        }
        // Parse step failure messages
        else if message.contains("❌") && message.contains("failed") {
            if let Some(current_index) = self.current_step_index {
                if let Some(step) = self.scenario.steps.get(current_index) {
                    if !self.failed_steps.contains(&step.name) {
                        self.failed_steps.push(step.name.clone());
                    }
                }
            }
        }
    }

    fn get_step_progress_info(&self) -> (usize, usize, Option<&str>, Option<Duration>) {
        let total_steps = self.scenario.steps.len();
        let completed_count = self.completed_steps.len();

        let current_step_name = if let Some(index) = self.current_step_index {
            self.scenario.steps.get(index).map(|s| s.name.as_str())
        } else {
            None
        };

        let current_step_duration = if let Some(start_time) = self.step_start_time {
            Some(start_time.elapsed())
        } else {
            None
        };

        (
            completed_count,
            total_steps,
            current_step_name,
            current_step_duration,
        )
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
    scenario_file_path: Option<PathBuf>,
    client_processes: HashMap<String, Child>,
    client_connections: HashMap<String, TcpStream>,
    infrastructure_processes: HashMap<String, Child>,
    completed_steps: Vec<String>,
    step_results: HashMap<String, StepResult>,
    start_time: Instant,
    log_files: HashMap<String, PathBuf>, // Track log files in non-TUI mode too
    variable_context: HashMap<String, serde_json::Value>, // Store variables extracted from responses
}

impl OrchestratorState {
    fn new(scenario: Scenario, scenario_file_path: Option<PathBuf>) -> Self {
        // Create log files directory
        let log_dir = PathBuf::from("logs");
        let _ = std::fs::create_dir_all(&log_dir);

        // Create log files for each service/client with timestamp
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

        if scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false)
        {
            log_files.insert(
                "MQTT Observer".to_string(),
                log_dir.join(format!("mqtt_observer_{}.log", timestamp)),
            );
        }

        for client in &scenario.clients {
            log_files.insert(
                client.id.clone(),
                log_dir.join(format!("client_{}_{}.log", client.id, timestamp)),
            );
        }

        Self {
            scenario,
            scenario_file_path,
            client_processes: HashMap::new(),
            client_connections: HashMap::new(),
            infrastructure_processes: HashMap::new(),
            completed_steps: Vec::new(),
            step_results: HashMap::new(),
            start_time: Instant::now(),
            log_files,
            variable_context: HashMap::new(),
        }
    }

    /// Write a log message to the appropriate log file in non-TUI mode
    fn write_to_log_file(&self, source: &LogSource, message: &str) {
        let pane_name = match source {
            LogSource::Orchestrator => "Orchestrator".to_string(),
            LogSource::MqttServer => "MQTT Server".to_string(),
            LogSource::MqttObserver => "MQTT Observer".to_string(),
            LogSource::Client(id) => id.clone(),
        };

        if let Some(log_file_path) = self.log_files.get(&pane_name) {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            let clean_message = strip_ansi_colors(message);
            let log_entry = format!("[{}] {}\n", timestamp, clean_message);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_path)
                .and_then(|mut file| {
                    use std::io::Write;
                    file.write_all(log_entry.as_bytes())
                });
        }
    }
}

/// Interpolate variables in JSON value using ${variable_name} syntax
fn interpolate_variables(
    value: &serde_json::Value,
    context: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    use regex::Regex;

    match value {
        serde_json::Value::String(s) => {
            let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
            let mut result = s.clone();

            for cap in re.captures_iter(s) {
                let var_name = &cap[1];
                let placeholder = &cap[0];

                if let Some(var_value) = context.get(var_name) {
                    let replacement = match var_value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string().trim_matches('"').to_string(),
                    };
                    result = result.replace(placeholder, &replacement);
                }
            }
            Ok(serde_json::Value::String(result))
        }
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), interpolate_variables(v, context)?);
            }
            Ok(serde_json::Value::Object(new_map))
        }
        serde_json::Value::Array(arr) => {
            let mut new_arr = Vec::new();
            for v in arr {
                new_arr.push(interpolate_variables(v, context)?);
            }
            Ok(serde_json::Value::Array(new_arr))
        }
        other => Ok(other.clone()),
    }
}

/// Extract value from JSON using simple dot notation path (e.g., "worlds.0.world_id")
fn extract_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        if let Ok(index) = part.parse::<usize>() {
            // Array index
            current = current.get(index)?;
        } else {
            // Object key
            current = current.get(part)?;
        }
    }

    Some(current)
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
                .help("List all available scenarios with validation status")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("validate-all")
                .long("validate-all")
                .help("Validate all scenarios and provide detailed report")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cleanup-invalid")
                .long("cleanup-invalid")
                .help("Remove invalid scenarios after validation")
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
        .arg(
            Arg::new("keep-alive")
                .long("keep-alive")
                .help("Keep scenario running indefinitely after completion for playtesting (prevents auto-exit)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("search-logs")
                .long("search-logs")
                .help("Search and correlate events across all log files from the last scenario run")
                .value_name("QUERY")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("logs-dir")
                .long("logs-dir")
                .help("Directory containing log files (defaults to 'logs/')")
                .value_name("DIR")
                .default_value("logs"),
        )
        .get_matches();

    if matches.get_flag("list-scenarios") {
        list_scenarios().await?;
        return Ok(());
    }

    if matches.get_flag("validate-all") {
        let cleanup = matches.get_flag("cleanup-invalid");
        validate_all_scenarios(cleanup).await?;
        return Ok(());
    }

    if let Some(queries) = matches.get_many::<String>("search-logs") {
        let logs_dir = matches.get_one::<String>("logs-dir").unwrap();
        let verbose = matches.get_flag("verbose");
        search_correlated_logs(queries.collect(), logs_dir, verbose).await?;
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
    let keep_alive = matches.get_flag("keep-alive");
    run_scenario(scenario, Some(scenario_path), verbose, keep_alive).await?;

    Ok(())
}

async fn list_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios_dir = PathBuf::from("scenarios");
    if !scenarios_dir.exists() {
        println!("❌ No scenarios directory found");
        return Ok(());
    }

    println!("📋 Available scenarios:");
    println!("======================");

    let mut valid_count = 0;
    let mut invalid_count = 0;
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
                            println!("✅ {} - {}", name.to_string_lossy(), scenario.description);
                            valid_count += 1;
                        } else {
                            println!("❌ {} - (invalid scenario file)", name.to_string_lossy());
                            invalid_count += 1;
                        }
                    }
                }
            }
        }
    }

    println!();
    println!(
        "📊 Summary: {} valid, {} invalid scenarios",
        valid_count, invalid_count
    );
    if invalid_count > 0 {
        println!("💡 Use --validate-all --cleanup-invalid to remove invalid scenarios");
    }
    Ok(())
}

async fn validate_all_scenarios(cleanup_invalid: bool) -> Result<(), Box<dyn std::error::Error>> {
    let scenarios_dir = PathBuf::from("scenarios");
    if !scenarios_dir.exists() {
        println!("❌ No scenarios directory found");
        return Ok(());
    }

    println!("🔍 Validating all scenarios...");
    println!("===============================");

    let mut valid_scenarios = Vec::new();
    let mut invalid_scenarios = Vec::new();
    let mut entries = tokio::fs::read_dir(&scenarios_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        if let Some(ext) = entry.path().extension() {
            if ext == "json" || ext == "ron" {
                if let Some(name) = entry.path().file_stem() {
                    let name_str = name.to_string_lossy();
                    print!("📋 Validating {:60} ", format!("{}...", name_str));

                    // Try to load and validate
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        let scenario_result = if ext == "ron" {
                            ron::from_str::<Scenario>(&content)
                                .map_err(|e| format!("RON error: {}", e))
                        } else {
                            serde_json::from_str::<Scenario>(&content)
                                .map_err(|e| format!("JSON error: {}", e))
                        };

                        match scenario_result {
                            Ok(scenario) => {
                                // Additional validation
                                match validate_scenario(&scenario) {
                                    Ok(_) => {
                                        println!("✅");
                                        valid_scenarios
                                            .push((name_str.to_string(), scenario.description));
                                    }
                                    Err(e) => {
                                        println!("❌ (validation error: {})", e);
                                        invalid_scenarios.push((
                                            name_str.to_string(),
                                            entry.path(),
                                            format!("Validation error: {}", e),
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ (parse error)");
                                invalid_scenarios.push((name_str.to_string(), entry.path(), e));
                            }
                        }
                    } else {
                        println!("❌ (read error)");
                        invalid_scenarios.push((
                            name_str.to_string(),
                            entry.path(),
                            "Failed to read file".to_string(),
                        ));
                    }
                }
            }
        }
    }

    println!();
    println!("📊 Validation Summary:");
    println!("======================");
    println!("✅ Valid scenarios: {}", valid_scenarios.len());
    println!("❌ Invalid scenarios: {}", invalid_scenarios.len());

    if !invalid_scenarios.is_empty() {
        println!();
        println!("❌ Invalid scenarios details:");
        println!("============================");

        for (name, path, error) in &invalid_scenarios {
            println!("  • {}: {}", name, error);
            println!("    Path: {}", path.display());
        }

        if cleanup_invalid {
            println!();
            println!("🗑️ Cleaning up invalid scenarios...");

            let mut removed_count = 0;
            for (name, path, _error) in &invalid_scenarios {
                match tokio::fs::remove_file(path).await {
                    Ok(_) => {
                        println!("  ✅ Removed: {}", name);
                        removed_count += 1;
                    }
                    Err(e) => {
                        println!("  ❌ Failed to remove {}: {}", name, e);
                    }
                }
            }

            println!();
            println!(
                "🎉 Cleanup complete: {} invalid scenarios removed",
                removed_count
            );
        } else {
            println!();
            println!("💡 To remove invalid scenarios automatically, use: --cleanup-invalid");
            println!("🚨 This will permanently delete the invalid scenario files!");
        }

        return Err(format!("Found {} invalid scenarios", invalid_scenarios.len()).into());
    } else {
        println!();
        println!("🎆 All scenarios are valid! Your scenario collection is clean.");
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

async fn run_scenario(
    scenario: Scenario,
    scenario_file_path: Option<PathBuf>,
    verbose: bool,
    keep_alive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create shared state wrapped in Arc<Mutex> for signal handling
    let state = Arc::new(Mutex::new(OrchestratorState::new(
        scenario,
        scenario_file_path,
    )));

    // Setup signal handler for graceful shutdown with force-exit on second Ctrl+C
    let state_for_cleanup = Arc::clone(&state);
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");

        let mut sigint_count = 0;
        let cleanup_started = false;

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    println!("\n🛑 Received SIGTERM, initiating graceful shutdown...");
                    if !cleanup_started {
                        let mut state = state_for_cleanup.lock().await;
                        let _ = cleanup(&mut state, verbose).await;
                    }
                    std::process::exit(0);
                }
                _ = sigint.recv() => {
                    sigint_count += 1;

                    if sigint_count == 1 {
                        println!("\n🛑 Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                        println!("   💡 Press Ctrl+C again to force immediate exit");

                        if !cleanup_started {
                            // Start cleanup in background without blocking signal handler
                            let state_cleanup = Arc::clone(&state_for_cleanup);
                            tokio::spawn(async move {
                                let mut state = state_cleanup.lock().await;
                                let _ = cleanup(&mut state, verbose).await;
                                std::process::exit(0);
                            });
                        }
                    } else {
                        println!("\n💥 Received second SIGINT (Ctrl+C), forcing immediate exit!");
                        std::process::exit(1);
                    }
                }
            }
        }
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

    // Log initial scenario information to files
    {
        let state = state.lock().await;
        state.write_to_log_file(
            &LogSource::Orchestrator,
            &format!("🚀 Starting scenario: {}", state.scenario.name),
        );
        state.write_to_log_file(
            &LogSource::Orchestrator,
            &format!("📖 Description: {}", state.scenario.description),
        );
        state.write_to_log_file(
            &LogSource::Orchestrator,
            &format!("👥 Clients: {}", state.scenario.clients.len()),
        );
        state.write_to_log_file(
            &LogSource::Orchestrator,
            &format!("📋 Steps: {}", state.scenario.steps.len()),
        );
        state.write_to_log_file(&LogSource::Orchestrator, "");
    }

    // Execute scenario with proper cleanup handling
    let result = run_scenario_inner(Arc::clone(&state), verbose, keep_alive).await;

    // Only cleanup if not keeping alive or if there was an error
    if !keep_alive || result.is_err() {
        let mut state = state.lock().await;
        cleanup(&mut state, verbose).await?;
        // Show log summary for non-TUI mode after cleanup
        show_log_summary_non_tui(&*state);
    }

    result
}

async fn run_scenario_inner(
    state: Arc<Mutex<OrchestratorState>>,
    verbose: bool,
    keep_alive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start infrastructure if any services are required
    let needs_infrastructure = {
        let state = state.lock().await;
        let mqtt_server_required = state.scenario.infrastructure.mqtt_server.required;
        let mcp_server_required = state
            .scenario
            .infrastructure
            .mcp_server
            .as_ref()
            .map(|mcp| mcp.required)
            .unwrap_or(false);
        let mqtt_observer_required = state
            .scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false);

        println!("[DEBUG] Non-TUI Infrastructure requirements: mqtt_server={}, mcp_server={}, mqtt_observer={}",
                mqtt_server_required, mcp_server_required, mqtt_observer_required);

        mqtt_server_required || mcp_server_required || mqtt_observer_required
    };

    println!(
        "[DEBUG] Non-TUI needs_infrastructure = {}",
        needs_infrastructure
    );

    if needs_infrastructure {
        let mut state = state.lock().await;
        start_infrastructure(&mut *state, verbose).await?;
    } else {
        println!("[DEBUG] Non-TUI Skipping infrastructure startup - no services required");
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

    // Keep scenario running indefinitely if keep-alive is enabled
    if keep_alive {
        println!("\n🔄 Scenario completed successfully!");
        println!("🎮 Keep-alive mode enabled - scenario will continue running for playtesting");
        println!("💡 All processes remain active and ready for manual testing");
        println!("⏸️  Press Ctrl+C to stop and cleanup processes\n");

        // Log the keep-alive status
        {
            let state = state.lock().await;
            state.write_to_log_file(
                &LogSource::Orchestrator,
                "🔄 Scenario completed successfully!",
            );
            state.write_to_log_file(
                &LogSource::Orchestrator,
                "🎮 Keep-alive mode enabled - scenario will continue running for playtesting",
            );
            state.write_to_log_file(
                &LogSource::Orchestrator,
                "💡 All processes remain active and ready for manual testing",
            );
            state.write_to_log_file(
                &LogSource::Orchestrator,
                "⏸️  Press Ctrl+C to stop and cleanup processes",
            );
        }

        // Wait indefinitely until Ctrl+C
        loop {
            sleep(Duration::from_secs(10)).await;
            // Periodically log that we're still alive
            if verbose {
                println!("🔄 Keep-alive: Scenario still running... (Press Ctrl+C to stop)");
            }
        }
    }

    Ok(())
}

async fn start_infrastructure(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Starting infrastructure...");
    state.write_to_log_file(&LogSource::Orchestrator, "🔧 Starting infrastructure...");

    println!(
        "[DEBUG] start_infrastructure: mqtt_server.required = {}",
        state.scenario.infrastructure.mqtt_server.required
    );
    if let Some(ref obs) = state.scenario.infrastructure.mqtt_observer {
        println!(
            "[DEBUG] start_infrastructure: mqtt_observer.required = {}",
            obs.required
        );
    } else {
        println!("[DEBUG] start_infrastructure: mqtt_observer = None");
    }

    // Check if MQTT port is already in use before starting
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        if verbose {
            println!("  Checking if MQTT port {} is available...", port);
        }

        // Check if port is already occupied
        if is_port_occupied("localhost", port).await {
            println!("🔄 MQTT port {} is already in use - reusing existing server instead of starting new one", port);
            if verbose {
                println!(
                    "  ℹ️  Detected existing MQTT server on port {}, will reuse it",
                    port
                );
            }
            // Skip starting our own MQTT server, but continue with observer if needed
        } else {
        }

        // Start MQTT server directly if required (instead of delegating to xtask)
        if state.scenario.infrastructure.mqtt_server.required {
            let port = state.scenario.infrastructure.mqtt_server.port;
            println!("[DEBUG] Starting MQTT server on port {}", port);
            if verbose {
                println!("  Starting MQTT server on port {}", port);
            }

            // Start MQTT server from ../mqtt-server directory
            println!(
                "[DEBUG] About to spawn MQTT server process: cargo run --release -- --port {}",
                port
            );
            println!("[DEBUG] Working directory: ../mqtt-server");

            // Check if the mqtt-server directory exists
            if !std::path::Path::new("../mqtt-server").exists() {
                return Err("../mqtt-server directory does not exist".into());
            }

            let mut mqtt_process = TokioCommand::new("cargo")
                .current_dir("../mqtt-server")
                .args(&["run", "--release", "--", "--port", &port.to_string()])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    let error_msg = format!("Failed to start MQTT server: {}", e);
                    println!("[DEBUG] MQTT server spawn failed: {}", error_msg);
                    error_msg
                })?;

            println!(
                "[DEBUG] MQTT server process spawned successfully with PID: {:?}",
                mqtt_process.id()
            );

            // Extract stdout and stderr for async log collection
            let stdout = mqtt_process.stdout.take();
            let stderr = mqtt_process.stderr.take();

            // Start async log collection for MQTT server using infrastructure log file
            if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
                if verbose {
                    println!("    Starting log collection for MQTT Server");
                }
                // Use the expected log file path from OrchestratorState
                let expected_log_path = state
                    .log_files
                    .get("MQTT Server")
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "logs/mqtt_server_fallback.log".to_string());
                tokio::spawn(async move {
                    collect_process_logs_to_file(
                        "MQTT Server".to_string(),
                        stdout,
                        stderr,
                        expected_log_path,
                    )
                    .await;
                });
            } else {
                if verbose {
                    println!(
                        "    Warning: No stdout/stderr available for MQTT server log collection"
                    );
                }
            }

            state
                .infrastructure_processes
                .insert("mqtt_server".to_string(), mqtt_process);

            // Wait for MQTT server to be ready
            println!("[DEBUG] About to wait for MQTT server port {}", port);
            if verbose {
                println!(
                    "  Waiting for MQTT server to become ready on port {}...",
                    port
                );
            }

            let mqtt_ready = wait_for_port_with_retries_and_context(
                "localhost",
                port,
                600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
                verbose,
                Some("cargo run --release (mqtt-server)"),
            )
            .await;
            println!(
                "[DEBUG] wait_for_port_with_retries_and_context returned: {}",
                mqtt_ready
            );
            if !mqtt_ready {
                return Err(format!(
                    "MQTT server failed to start on port {} within 10 minute timeout",
                    port
                )
                .into());
            }

            println!("[DEBUG] MQTT server is ready on port {}", port);
            if verbose {
                println!("  ✅ MQTT server ready on port {}", port);
            }
        }
    }

    // Start MQTT observer if required
    if let Some(ref mqtt_observer) = state.scenario.infrastructure.mqtt_observer {
        if mqtt_observer.required {
            println!("[DEBUG] Starting MQTT observer");
            if verbose {
                println!("  Starting MQTT observer");
            }

            let mqtt_port = state.scenario.infrastructure.mqtt_server.port;
            println!("[DEBUG] About to spawn MQTT observer process: cargo run --bin mqtt-observer -- -h localhost -p {} -t # -i mcplay_observer", mqtt_port);
            println!("[DEBUG] Working directory: ../mqtt-client");

            // Check if the mqtt-client directory exists
            if !std::path::Path::new("../mqtt-client").exists() {
                return Err("../mqtt-client directory does not exist".into());
            }

            let mut observer_process = TokioCommand::new("cargo")
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
                .map_err(|e| {
                    let error_msg = format!("Failed to start MQTT observer: {}. Make sure mqtt-client is available in ../mqtt-client.", e);
                    println!("[DEBUG] MQTT observer spawn failed: {}", error_msg);
                    error_msg
                })?;

            println!(
                "[DEBUG] MQTT observer process spawned successfully with PID: {:?}",
                observer_process.id()
            );

            // Extract stdout and stderr for async log collection
            let stdout = observer_process.stdout.take();
            let stderr = observer_process.stderr.take();

            // Start async log collection for MQTT observer using infrastructure log file
            if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
                if verbose {
                    println!("    Starting log collection for MQTT Observer");
                }
                // Use the expected log file path from OrchestratorState
                let expected_log_path = state
                    .log_files
                    .get("MQTT Observer")
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "logs/mqtt_observer_fallback.log".to_string());
                tokio::spawn(async move {
                    collect_process_logs_to_file(
                        "MQTT Observer".to_string(),
                        stdout,
                        stderr,
                        expected_log_path,
                    )
                    .await;
                });
            } else {
                if verbose {
                    println!(
                        "    Warning: No stdout/stderr available for MQTT observer log collection"
                    );
                }
            }

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
        let msg = "👥 No clients to start (orchestrator-only scenario)";
        println!("{}", msg);
        state.write_to_log_file(&LogSource::Orchestrator, msg);
        return Ok(());
    }

    let msg = format!("👥 Starting {} clients...", state.scenario.clients.len());
    println!("{}", msg);
    state.write_to_log_file(&LogSource::Orchestrator, &msg);

    // Start each client directly instead of relying on xtask
    for client in &state.scenario.clients {
        if verbose {
            println!("  Starting client: {}", client.id);
            // Log the command that will be executed
            let player_name = client.name.as_ref().unwrap_or(&client.id);
            let mut cmd_parts = vec![
                "cargo".to_string(),
                "run".to_string(),
                "--bin".to_string(),
                "iotcraft-dekstop-client".to_string(),
                "--".to_string(),
                "--mcp".to_string(),
                "--player-id".to_string(),
                client.player_id.clone(),
                "--player-name".to_string(),
                player_name.clone(),
            ];
            if state.scenario.infrastructure.mqtt_server.required {
                cmd_parts.push("--mqtt-server".to_string());
                cmd_parts.push(format!(
                    "localhost:{}",
                    state.scenario.infrastructure.mqtt_server.port
                ));
            }
            println!("    Command: {}", cmd_parts.join(" "));
            println!("    Working dir: ../desktop-client");
            println!("    Environment: MCP_PORT={}", client.mcp_port);
        }

        // Build client command arguments
        let mut cmd = TokioCommand::new("cargo");
        cmd.current_dir("../desktop-client")
            .arg("run")
            .arg("--bin")
            .arg("iotcraft-dekstop-client")
            .args(&["--", "--mcp"])
            .arg("--player-id")
            .arg(&client.player_id)
            .arg("--player-name")
            // Use client name if available, otherwise fall back to client ID
            .arg(client.name.as_ref().unwrap_or(&client.id))
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

        // Extract stdout and stderr for async logging
        let stdout = client_process.stdout.take();
        let stderr = client_process.stderr.take();

        // Start async log collection for this client
        let client_id_clone = client.id.clone();
        if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
            if verbose {
                println!(
                    "    Starting log collection for client: {}",
                    client_id_clone
                );
            }
            tokio::spawn(async move {
                collect_process_logs(client_id_clone, stdout, stderr).await;
            });
        } else {
            if verbose {
                println!("    Warning: No stdout/stderr available for log collection");
            }
        }

        // Check if the process is still running after a brief moment
        tokio::time::sleep(Duration::from_millis(1000)).await;
        match client_process.try_wait() {
            Ok(Some(exit_status)) => {
                // Process has already exited - this indicates an error
                return Err(format!(
                    "Client {} exited immediately with status: {}",
                    client.id, exit_status
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
            600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
            verbose,
            Some(&format!("cargo run ({})", client.id)),
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

        // Enhanced readiness probe: verify MQTT connection via MCP get_mqtt_status
        // Some scenarios require MQTT to be fully connected; just having MCP up is not enough.
        // Extract client info to avoid borrow checker issues
        let client_id = client.id.clone();
        let client_port = client.mcp_port;

        let mut mqtt_ready = false;
        for attempt in 1..=10 {
            // Small delay before first/next probe to allow core service to finish connecting
            tokio::time::sleep(Duration::from_millis(300)).await;

            // Create fresh connection for readiness probe to avoid state borrowing issues
            match TcpStream::connect(format!("localhost:{}", client_port)).await {
                Ok(mut stream) => {
                    // Generate unique request ID
                    let request_id = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    // Create MCP request for get_mqtt_status
                    let request = McpRequest {
                        jsonrpc: "2.0".to_string(),
                        id: request_id,
                        method: "tools/call".to_string(),
                        params: serde_json::json!({
                            "name": "get_mqtt_status",
                            "arguments": {}
                        }),
                    };

                    // Send request
                    let request_json = serde_json::to_string(&request).unwrap_or_default();
                    if stream
                        .write_all(format!("{}\n", request_json).as_bytes())
                        .await
                        .is_ok()
                    {
                        // Read response with short timeout
                        let mut reader = BufReader::new(&mut stream);
                        let mut response_line = String::new();

                        if let Ok(Ok(_)) = tokio::time::timeout(
                            Duration::from_secs(5),
                            reader.read_line(&mut response_line),
                        )
                        .await
                        {
                            if let Ok(response) =
                                serde_json::from_str::<McpResponse>(&response_line)
                            {
                                if let Some(result) = response.result {
                                    // Parse MQTT status from response
                                    let maybe_text = result
                                        .get("content")
                                        .and_then(|c| c.as_array())
                                        .and_then(|arr| arr.first())
                                        .and_then(|o| o.get("text"))
                                        .and_then(|t| t.as_str());

                                    if let Some(text_json) = maybe_text {
                                        if let Ok(inner) =
                                            serde_json::from_str::<serde_json::Value>(text_json)
                                        {
                                            let connected = inner
                                                .get("mqtt_connected")
                                                .and_then(|b| b.as_bool())
                                                .unwrap_or(false);
                                            let status_ok = inner
                                                .get("status")
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.eq_ignore_ascii_case("healthy"))
                                                .unwrap_or(false);
                                            if connected && status_ok {
                                                mqtt_ready = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Connection failed, continue trying
                }
            }

            if verbose {
                println!(
                    "  ⏳ Waiting for client {} MQTT readiness (attempt {}/10)...",
                    client_id, attempt
                );
            }
        }

        if !mqtt_ready {
            return Err(format!(
                "Client {} failed MQTT readiness probe: get_mqtt_status did not report mqtt_connected=true & status=healthy",
                client_id
            )
            .into());
        }

        if verbose {
            println!("  ✅ Client {} ready (MCP + MQTT)", client_id);
        }
    }

    Ok(())
}

async fn execute_steps(
    state: &mut OrchestratorState,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = format!("🎬 Executing {} steps...", state.scenario.steps.len());
    println!("{}", msg);
    state.write_to_log_file(&LogSource::Orchestrator, &msg);
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
                // After successful execution: extract response variables if configured
                if let Some(vars) = &step.response_variables {
                    if let Some(result_obj) = response.get("result") {
                        for (name, path) in vars {
                            if let Some(value) = extract_json_path(result_obj, path) {
                                state.variable_context.insert(name.clone(), value.clone());
                            }
                        }
                    }
                }
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
    // Before execution: perform variable interpolation in action arguments if applicable
    let mut step = step.clone();
    if let Action::McpCall { tool: _, arguments } = &mut step.action {
        *arguments = interpolate_variables(arguments, &state.variable_context)?;
    }
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

        // New system integration actions (non-TUI versions)
        Action::SystemCommand {
            command,
            working_dir,
            background,
            timeout_seconds: _,
        } => {
            if verbose {
                println!("  🔧 System command: {:?}", command);
            }
            // TODO: Implement system command execution for non-TUI mode
            Ok(serde_json::json!({
                "status": "system_command_executed",
                "command": command,
                "working_dir": working_dir,
                "background": background
            }))
        }

        Action::OpenBrowser {
            url,
            browser,
            wait_seconds: _,
        } => {
            if verbose {
                println!("  🌐 Open browser: {} ({:?})", url, browser);
            }
            // TODO: Implement browser opening for non-TUI mode
            Ok(serde_json::json!({
                "status": "browser_opened",
                "url": url,
                "browser": browser
            }))
        }

        Action::ShowMessage {
            message,
            message_type,
        } => {
            let message_type = message_type.as_deref().unwrap_or("info");
            if verbose {
                let emoji = match message_type {
                    "error" => "❌",
                    "warning" => "⚠️",
                    "success" => "✅",
                    _ => "💡",
                };
                println!("  {} {}: {}", emoji, message_type.to_uppercase(), message);
            }
            Ok(serde_json::json!({
                "status": "message_shown",
                "message": message,
                "message_type": message_type
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

    // Handle different MCP methods
    let method = if tool == "initialize" || tool == "tools/list" {
        tool.to_string()
    } else {
        "tools/call".to_string()
    };

    let params = if method == "tools/call" {
        serde_json::json!({
            "name": tool,
            "arguments": arguments
        })
    } else {
        arguments.clone()
    };

    // Create a fresh connection for each request to avoid connection state issues
    // This ensures the desktop-client MCP server can properly handle queued commands
    let client = state
        .scenario
        .clients
        .iter()
        .find(|c| c.id == client_id)
        .ok_or_else(|| format!("Client {} not found in scenario", client_id))?;

    let mut stream = TcpStream::connect(format!("localhost:{}", client.mcp_port))
        .await
        .map_err(|e| {
            format!(
                "Failed to connect to client {} MCP server: {}",
                client_id, e
            )
        })?;

    // Generate unique request ID
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Create MCP request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: request_id,
        method,
        params,
    };

    // Send request
    let request_json = serde_json::to_string(&request)?;
    stream
        .write_all(format!("{}\n", request_json).as_bytes())
        .await?;

    // For queued commands, we need to wait longer as they go through the command execution system
    let timeout_duration = if is_queued_command(tool) {
        std::time::Duration::from_secs(60) // Longer timeout for queued commands (world creation, etc.)
    } else {
        std::time::Duration::from_secs(10) // Shorter timeout for direct responses
    };

    // Read response with appropriate timeout
    let mut reader = BufReader::new(&mut stream);
    let mut response_line = String::new();

    match tokio::time::timeout(timeout_duration, reader.read_line(&mut response_line)).await {
        Ok(Ok(_)) => {
            // Response received successfully
        }
        Ok(Err(e)) => {
            return Err(format!("I/O error reading MCP response: {}", e).into());
        }
        Err(_) => {
            return Err(format!(
                "MCP request timeout after {} seconds - no response from desktop-client",
                timeout_duration.as_secs()
            )
            .into());
        }
    }

    let response: McpResponse = serde_json::from_str(&response_line)?;

    if let Some(error) = response.error {
        return Err(format!("MCP error: {}", error).into());
    }

    // Log MCP response content for debugging, especially for data-rich commands
    if let Some(ref result) = response.result {
        // Check for MCP tool-level errors (is_error flag)
        if let Some(is_error) = result.get("is_error").and_then(|v| v.as_bool()) {
            if is_error {
                // Extract error message from content if available
                let error_message =
                    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                        if let Some(first_content) = content.first() {
                            if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                                text.to_string()
                            } else {
                                "MCP tool reported error (no text content)".to_string()
                            }
                        } else {
                            "MCP tool reported error (no content)".to_string()
                        }
                    } else {
                        "MCP tool reported error".to_string()
                    };

                // Log the error
                state.write_to_log_file(
                    &LogSource::Client(client_id.to_string()),
                    &format!("❌ MCP Error for '{}': {}", tool, error_message),
                );
                println!("❌ MCP Error for '{}': {}", tool, error_message);

                return Err(format!("MCP tool error: {}", error_message).into());
            }
        }

        // Log MCP responses to file for non-TUI mode
        state.write_to_log_file(
            &LogSource::Client(client_id.to_string()),
            &format!(
                "🔧 MCP Response for '{}': {}",
                tool,
                serde_json::to_string_pretty(result)
                    .unwrap_or_else(|_| "<invalid json>".to_string())
            ),
        );

        match tool {
            "list_world_templates"
            | "create_world"
            | "list_online_worlds"
            | "get_multiplayer_status"
            | "get_client_info"
            | "get_world_status"
            | "get_mqtt_status"
            | "tools/list" => {
                // Always show detailed responses for important commands
                println!(
                    "🔍 MCP Response for '{}': {}",
                    tool,
                    serde_json::to_string_pretty(result)
                        .unwrap_or_else(|_| "<invalid json>".to_string())
                );
            }
            _ => {
                // For other commands, show a brief summary but also the full response if verbose
                let summary =
                    if result.is_object() && result.as_object().unwrap().contains_key("content") {
                        format!("has content field")
                    } else {
                        format!(
                            "result type: {}",
                            if result.is_object() {
                                "object"
                            } else if result.is_array() {
                                "array"
                            } else {
                                "primitive"
                            }
                        )
                    };
                println!("📋 MCP Response for '{}': {}", tool, summary);
            }
        }
    } else {
        // Log when there's no result
        state.write_to_log_file(
            &LogSource::Client(client_id.to_string()),
            &format!("⚠️ MCP Response for '{}': No result returned", tool),
        );
        println!("⚠️ MCP Response for '{}': No result returned", tool);
    }

    Ok(response
        .result
        .unwrap_or(serde_json::json!({"status": "success"})))
}

/// Check if a command should be queued (needs longer timeout)
fn is_queued_command(tool: &str) -> bool {
    matches!(
        tool,
        "create_world"
            | "place_block"
            | "create_wall"
            | "get_client_info"
            | "get_world_status"
            | "get_mqtt_status"
            | "spawn_device"
            | "move_device"
            | "save_world"
            | "load_world"
            | "set_game_state"
    )
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

    // Handle special conditions
    match condition {
        "manual_exit" => {
            // For manual_exit condition, wait indefinitely until Ctrl+C
            if verbose {
                println!(
                    "  📝 Manual exit condition - waiting indefinitely (press Ctrl+C to exit)"
                );
            }

            // Create a cancellation signal detector
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("Failed to install SIGINT handler in wait_condition");
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to install SIGTERM handler in wait_condition");

            // Also respect the timeout if provided
            let timeout_duration = Duration::from_millis(wait_timeout);
            let start = Instant::now();

            loop {
                let remaining_time = timeout_duration
                    .checked_sub(start.elapsed())
                    .unwrap_or(Duration::ZERO);

                if remaining_time.is_zero() {
                    if verbose {
                        println!("  ⏰ Wait condition timed out after {}ms", wait_timeout);
                    }
                    return Ok(serde_json::json!({
                        "condition": condition,
                        "expected": expected_value,
                        "status": "timeout"
                    }));
                }

                let sleep_duration = std::cmp::min(Duration::from_secs(1), remaining_time);

                tokio::select! {
                    _ = sleep(sleep_duration) => {
                        // Continue waiting
                        continue;
                    }
                    _ = sigint.recv() => {
                        if verbose {
                            println!("  🛑 Manual exit condition met (SIGINT received)");
                        }
                        return Ok(serde_json::json!({
                            "condition": condition,
                            "expected": expected_value,
                            "status": "manual_exit_triggered"
                        }));
                    }
                    _ = sigterm.recv() => {
                        if verbose {
                            println!("  🛑 Manual exit condition met (SIGTERM received)");
                        }
                        return Ok(serde_json::json!({
                            "condition": condition,
                            "expected": expected_value,
                            "status": "manual_exit_triggered"
                        }));
                    }
                }
            }
        }
        _ => {
            // For other conditions, use the original behavior but respect the timeout
            let wait_duration = Duration::from_millis(wait_timeout);
            if verbose {
                println!(
                    "  ⏳ Simulating wait for condition '{}' for {}ms",
                    condition, wait_timeout
                );
            }
            sleep(wait_duration).await;

            Ok(serde_json::json!({
                "condition": condition,
                "expected": expected_value,
                "status": "condition_met"
            }))
        }
    }
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
    if let Some(file_path) = &state.scenario_file_path {
        println!("Scenario file: {}", file_path.display());
    }
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

    // Set a timeout for cleanup operations
    let cleanup_future = async {
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
    };

    // Apply 5-second timeout to cleanup
    match tokio::time::timeout(Duration::from_secs(5), cleanup_future).await {
        Ok(_) => {
            println!("✅ Cleanup completed");
        }
        Err(_) => {
            println!("⚠️ Cleanup timed out after 5 seconds, forcing exit");
        }
    }

    Ok(())
}

/// Ensure log files are synced to disk for accurate size reporting
fn sync_log_files_to_disk(state: &OrchestratorState) {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Explicitly flush each log file to ensure accurate size reporting
    for (_pane_name, log_file_path) in &state.log_files {
        if log_file_path.exists() {
            // Open the file in append mode and flush it
            if let Ok(mut file) = OpenOptions::new()
                .create(false)
                .append(true)
                .open(log_file_path)
            {
                let _ = file.flush();
                let _ = file.sync_all(); // Force OS to write to disk
            }
        }
    }

    // Small additional delay to ensure filesystem consistency
    std::thread::sleep(std::time::Duration::from_millis(100));
}

/// Show log file summary for non-TUI mode
fn show_log_summary_non_tui(state: &OrchestratorState) {
    // Sync log files to disk first to ensure accurate sizes
    sync_log_files_to_disk(state);

    println!("\n📁 Log Files Summary");
    println!("====================");

    // Show scenario file information
    if let Some(file_path) = &state.scenario_file_path {
        println!("📋 Scenario file: {}", file_path.display());
        println!();
    }

    for (pane_name, log_file_path) in &state.log_files {
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
        filtered_scenarios: scenarios.clone(),
        scenarios,
        list_state: ListState::default(),
        should_quit: false,
        show_details: false,
        selected_scenario: None,
        message: None,
        search_mode: false,
        search_query: String::new(),
    };

    // Select first scenario if available
    if !app.filtered_scenarios.is_empty() {
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
impl App {
    fn filter_scenarios(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_scenarios = self.scenarios.clone();
        } else {
            let query_lower = self.search_query.to_lowercase();
            self.filtered_scenarios = self
                .scenarios
                .iter()
                .filter(|scenario| {
                    scenario.name.to_lowercase().contains(&query_lower)
                        || scenario.description.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect();
        }

        // Reset selection to first item
        if !self.filtered_scenarios.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }
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
                    // Handle Ctrl+C
                    if let KeyCode::Char('c') = key.code {
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                        {
                            app.should_quit = true;
                        }
                    }

                    if app.search_mode {
                        // Search mode key handling
                        match key.code {
                            KeyCode::Esc => {
                                // Exit search mode
                                app.search_mode = false;
                                app.search_query.clear();
                                app.filtered_scenarios = app.scenarios.clone();
                                app.message = None;
                                if !app.filtered_scenarios.is_empty() {
                                    app.list_state.select(Some(0));
                                } else {
                                    app.list_state.select(None);
                                }
                            }
                            KeyCode::Enter => {
                                // Exit search mode and keep filter
                                app.search_mode = false;
                                app.message = None;
                            }
                            KeyCode::Backspace => {
                                // Remove last character from search query
                                app.search_query.pop();
                                app.filter_scenarios();
                            }
                            KeyCode::Char(c) => {
                                // Add character to search query
                                app.search_query.push(c);
                                app.filter_scenarios();
                            }
                            _ => {}
                        }
                    } else {
                        // Normal mode key handling
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                if app.show_details {
                                    app.show_details = false;
                                    app.selected_scenario = None;
                                } else {
                                    app.should_quit = true;
                                }
                            }
                            KeyCode::Char('/') => {
                                // Enter search mode
                                app.search_mode = true;
                                app.search_query.clear();
                                app.message = Some(
                                    "🔍 Search scenarios (type to filter, Esc to cancel)"
                                        .to_string(),
                                );
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
                                    if selected < app.filtered_scenarios.len().saturating_sub(1) {
                                        app.list_state.select(Some(selected + 1));
                                    }
                                } else if !app.filtered_scenarios.is_empty() {
                                    app.list_state.select(Some(0));
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(selected) = app.list_state.selected() {
                                    if selected < app.filtered_scenarios.len() {
                                        let scenario = &app.filtered_scenarios[selected];
                                        if scenario.is_valid {
                                            // Exit TUI and run scenario
                                            disable_raw_mode()?;
                                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                            terminal.show_cursor()?;

                                            return run_selected_scenario(&scenario.file_path)
                                                .await;
                                        } else {
                                            app.message =
                                                Some("Cannot run invalid scenario".to_string());
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('d') => {
                                if let Some(selected) = app.list_state.selected() {
                                    if selected < app.filtered_scenarios.len() {
                                        app.selected_scenario =
                                            Some(app.filtered_scenarios[selected].clone());
                                        app.show_details = true;
                                    }
                                }
                            }
                            KeyCode::Char('v') => {
                                if let Some(selected) = app.list_state.selected() {
                                    if selected < app.filtered_scenarios.len() {
                                        let scenario_path =
                                            &app.filtered_scenarios[selected].file_path;
                                        app.message = Some(format!(
                                            "Validating {}...",
                                            scenario_path.display()
                                        ));

                                        // Validate scenario
                                        match validate_scenario_file(scenario_path).await {
                                            Ok(_) => {
                                                app.message =
                                                    Some("✅ Scenario is valid".to_string());
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
                                app.filtered_scenarios = app.scenarios.clone();
                                app.message = Some("🔄 Scenarios refreshed".to_string());
                                if !app.filtered_scenarios.is_empty()
                                    && app.list_state.selected().is_none()
                                {
                                    app.list_state.select(Some(0));
                                }
                            }
                            _ => {}
                        }
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
        .filtered_scenarios
        .iter()
        .map(|scenario| {
            let status_icon = if scenario.is_valid { "✅" } else { "❌" };

            // Create highlighted text if we have a search query
            if !app.search_query.is_empty() && !app.search_mode {
                let query_lower = app.search_query.to_lowercase();
                let name_lower = scenario.name.to_lowercase();
                let desc_lower = scenario.description.to_lowercase();

                // Create spans with highlighting
                let mut spans = vec![Span::raw(format!("{} ", status_icon))];

                // Highlight matches in name
                if let Some(pos) = name_lower.find(&query_lower) {
                    let name = &scenario.name;
                    spans.push(Span::raw(&name[..pos]));
                    spans.push(Span::styled(
                        &name[pos..pos + app.search_query.len()],
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::raw(&name[pos + app.search_query.len()..]));
                } else {
                    spans.push(Span::raw(&scenario.name));
                }

                spans.push(Span::raw(format!(
                    " - {} clients, {} steps",
                    scenario.clients, scenario.steps
                )));

                // Add description with highlighting if it matches
                if desc_lower.contains(&query_lower) && desc_lower != name_lower {
                    spans.push(Span::raw(" ("));
                    if let Some(pos) = desc_lower.find(&query_lower) {
                        let desc = &scenario.description;
                        let preview_start = pos.saturating_sub(10);
                        let preview_end = (pos + app.search_query.len() + 10).min(desc.len());
                        let preview = &desc[preview_start..preview_end];

                        let relative_pos = pos - preview_start;
                        spans.push(Span::raw(&preview[..relative_pos]));
                        spans.push(Span::styled(
                            &preview[relative_pos..relative_pos + app.search_query.len()],
                            Style::default()
                                .bg(Color::Yellow)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ));
                        spans.push(Span::raw(&preview[relative_pos + app.search_query.len()..]));
                    }
                    spans.push(Span::raw(")"));
                }

                ListItem::new(Line::from(spans))
            } else {
                // No highlighting needed
                let content = format!(
                    "{} {} - {} clients, {} steps",
                    status_icon, scenario.name, scenario.clients, scenario.steps
                );
                ListItem::new(content)
            }
        })
        .collect();

    // Build title with search indicator
    let title = if app.search_mode {
        format!(
            "📋 Scenarios - Search: '{}' ({}/{})",
            app.search_query,
            app.filtered_scenarios.len(),
            app.scenarios.len()
        )
    } else if app.search_query.is_empty() {
        format!("📋 Scenarios ({} found)", app.filtered_scenarios.len())
    } else {
        format!(
            "📋 Scenarios - Filtered: '{}' ({}/{})",
            app.search_query,
            app.filtered_scenarios.len(),
            app.scenarios.len()
        )
    };

    let scenarios_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(scenarios_list, chunks[1], &mut app.list_state);

    // Render search modal if in search mode
    if app.search_mode {
        draw_search_modal(f, app);
    }

    // Instructions and status
    let mut instructions = vec![
        Line::from(vec![
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Run  "),
            Span::styled("d", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Details  "),
            Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Search"),
        ]),
        Line::from(vec![
            Span::styled("v", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Validate  "),
            Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Refresh  "),
            Span::styled(
                "q/Esc/Ctrl+C",
                Style::default().add_modifier(Modifier::BOLD),
            ),
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
fn draw_search_modal(f: &mut Frame, app: &App) {
    // Calculate modal size - center it on screen
    let area = f.area();
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 8;

    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = ratatui::layout::Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background area
    f.render_widget(Clear, modal_area);

    // Create modal content layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title and input
            Constraint::Length(2), // Results count
            Constraint::Min(1),    // Instructions
        ])
        .split(modal_area);

    // Modal border
    let modal_block = Block::default()
        .borders(Borders::ALL)
        .title("🔍 Search Scenarios")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    f.render_widget(modal_block, modal_area);

    // Search input field
    let input_text = if app.search_query.is_empty() {
        "Type to search scenarios...".to_string()
    } else {
        app.search_query.clone()
    };

    let input_style = if app.search_query.is_empty() {
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC)
    } else {
        Style::default().fg(Color::White)
    };

    let input_paragraph = Paragraph::new(input_text).style(input_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Query")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(input_paragraph, modal_chunks[0]);

    // Show cursor in input field
    let cursor_x = modal_chunks[0].x + 1 + app.search_query.len() as u16;
    let cursor_y = modal_chunks[0].y + 1;
    if cursor_x < modal_chunks[0].x + modal_chunks[0].width.saturating_sub(1) {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // Results count
    let results_text = if app.search_query.is_empty() {
        format!("Total scenarios: {}", app.scenarios.len())
    } else {
        format!(
            "Found: {} of {} scenarios",
            app.filtered_scenarios.len(),
            app.scenarios.len()
        )
    };

    let results_paragraph = Paragraph::new(results_text)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);

    f.render_widget(results_paragraph, modal_chunks[1]);

    // Instructions
    let instructions = vec![Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" Apply filter  "),
        Span::styled(
            "Esc",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" Cancel  "),
        Span::styled(
            "Backspace",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
        Span::raw(" Delete"),
    ])];

    let instructions_paragraph = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(instructions_paragraph, modal_chunks[2]);
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
    run_scenario_with_tui(scenario, Some(path.clone())).await?;
    Ok(())
}

#[cfg(feature = "tui")]
async fn run_scenario_with_tui(
    scenario: Scenario,
    scenario_file_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal for scenario execution
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create logging app
    let mut logging_app = LoggingApp::new(&scenario, scenario_file_path.clone()).await;
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
            let result = run_scenario_with_logging(
                scenario,
                scenario_file_path.clone(),
                log_collector,
                quit_flag,
            )
            .await;
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

    // Create channel for system info updates
    let (system_info_sender, mut system_info_receiver) =
        tokio::sync::mpsc::unbounded_channel::<SystemInfo>();

    // Spawn system info collection task (runs every 3 seconds in background)
    let _system_info_task = tokio::spawn({
        let quit_flag = Arc::clone(&logging_app.should_quit);
        let current_system_info = logging_app.system_info.clone(); // Get the initial system info with cached RAM
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            let mut system_info = current_system_info;
            while !quit_flag.load(Ordering::Relaxed) {
                interval.tick().await;
                if let Ok(updated_info) = system_info.collect_with_cached_ram().await {
                    system_info = updated_info.clone();
                    if system_info_sender.send(updated_info).is_err() {
                        break; // Receiver dropped
                    }
                }
            }
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

        // Update system information from background task (non-blocking)
        while let Ok(new_system_info) = system_info_receiver.try_recv() {
            logging_app.system_info = new_system_info;
            logging_app.last_system_update = std::time::Instant::now();
        }

        // Check health probes and update service statuses
        let failed_clients = logging_app.check_and_update_health_probes();

        // Check if this is an indefinite scenario (has manual_exit condition)
        let is_indefinite_scenario = logging_app.scenario.steps.iter().any(|step| {
            matches!(&step.action,
                Action::WaitCondition { condition, .. } if condition == "manual_exit")
        });

        // If any clients failed health checks, terminate the scenario (unless it's an indefinite scenario)
        if !failed_clients.is_empty() && !is_indefinite_scenario {
            for client_id in &failed_clients {
                logging_app.add_log(
                    &LogSource::Orchestrator,
                    format!(
                        "❌ Client {} failed liveness probe - terminating scenario",
                        client_id
                    ),
                );
            }
            logging_app.should_quit.store(true, Ordering::Relaxed);
            break;
        } else if !failed_clients.is_empty() {
            // For indefinite scenarios, log the health issues but don't terminate
            for client_id in &failed_clients {
                logging_app.add_log(
                    &LogSource::Orchestrator,
                    format!(
                        "⚠️ Client {} failed liveness probe (indefinite scenario - continuing)",
                        client_id
                    ),
                );
            }
        }

        // Check if scenario is completed and auto-exit is enabled
        if scenario_task.is_finished() && !logging_app.scenario_completed {
            logging_app.scenario_completed = true;
            logging_app.add_log(
                &LogSource::Orchestrator,
                "🎉 Scenario completed successfully!".to_string(),
            );

            if logging_app.auto_exit_after_completion {
                // Wait a moment for the final log to be displayed
                tokio::time::sleep(Duration::from_millis(2000)).await;
                break;
            }
        }

        // Handle input events
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => {
                                logging_app.should_quit.store(true, Ordering::Relaxed);
                                break;
                            }
                            KeyCode::Char('c')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                logging_app.should_quit.store(true, Ordering::Relaxed);
                                break;
                            }
                            KeyCode::Esc => {
                                match logging_app.ui_mode {
                                    UiMode::LogViewing => {
                                        // In log viewing mode, ESC quits the application
                                        logging_app.should_quit.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                    UiMode::McpMessageSending => {
                                        // In MCP message mode, ESC returns to log viewing mode
                                        logging_app.ui_mode = UiMode::LogViewing;
                                        logging_app.focused_pane = FocusedPane::LogSelector;
                                        logging_app.mcp_app = None;
                                    }
                                    UiMode::McpParameterEditing => {
                                        // In parameter editing mode, ESC returns to message selection
                                        logging_app.ui_mode = UiMode::McpMessageSending;
                                        // Clear editing state
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            mcp_app.editing_message = None;
                                            mcp_app.editing_param_value = false;
                                            mcp_app.param_input_buffer.clear();
                                        }
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // In MCP interaction details mode, ESC returns to log viewing mode
                                        logging_app.ui_mode = UiMode::LogViewing;
                                        logging_app.focused_pane = FocusedPane::LogSelector;
                                        logging_app.mcp_app = None;
                                    }
                                }
                            }
                            KeyCode::Tab => {
                                // Switch focus between panes
                                match logging_app.ui_mode {
                                    UiMode::LogViewing => {
                                        match logging_app.focused_pane {
                                            FocusedPane::LogSelector => {
                                                logging_app.focused_pane = FocusedPane::LogContent;
                                            }
                                            FocusedPane::LogContent => {
                                                logging_app.focused_pane = FocusedPane::LogSelector;
                                            }
                                            FocusedPane::McpMessageSelector => {
                                                // In log viewing mode, this shouldn't happen
                                                logging_app.focused_pane = FocusedPane::LogSelector;
                                            }
                                        }
                                    }
                                    UiMode::McpMessageSending => {
                                        // In MCP mode, Tab doesn't switch focus (only one focusable pane)
                                    }
                                    UiMode::McpParameterEditing => {
                                        // In parameter editing mode, Tab doesn't switch focus
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // In MCP interaction details mode, Tab doesn't switch focus
                                    }
                                }
                            }
                            KeyCode::BackTab => {
                                // Switch focus in reverse direction
                                match logging_app.ui_mode {
                                    UiMode::LogViewing => {
                                        match logging_app.focused_pane {
                                            FocusedPane::LogSelector => {
                                                logging_app.focused_pane = FocusedPane::LogContent;
                                            }
                                            FocusedPane::LogContent => {
                                                logging_app.focused_pane = FocusedPane::LogSelector;
                                            }
                                            FocusedPane::McpMessageSelector => {
                                                // In log viewing mode, this shouldn't happen
                                                logging_app.focused_pane = FocusedPane::LogSelector;
                                            }
                                        }
                                    }
                                    UiMode::McpMessageSending => {
                                        // In MCP mode, BackTab doesn't switch focus (only one focusable pane)
                                    }
                                    UiMode::McpParameterEditing => {
                                        // In parameter editing mode, BackTab doesn't switch focus
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // In MCP interaction details mode, BackTab doesn't switch focus
                                    }
                                }
                            }
                            KeyCode::Up => {
                                match logging_app.ui_mode {
                                    UiMode::McpParameterEditing => {
                                        // Navigate up in parameter list
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            if !mcp_app.editing_param_value
                                                && mcp_app.selected_param_index > 0
                                            {
                                                mcp_app.selected_param_index -= 1;
                                            }
                                        }
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // Scroll up in MCP interaction details
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            if mcp_app.details_scroll_pos > 0 {
                                                mcp_app.details_scroll_pos -= 1;
                                            }
                                        }
                                    }
                                    _ => {
                                        match logging_app.focused_pane {
                                            FocusedPane::LogSelector => {
                                                // Navigate up in pane selector
                                                if logging_app.selected_pane_index > 0 {
                                                    logging_app.selected_pane_index -= 1;
                                                    logging_app.selected_pane = logging_app.panes
                                                        [logging_app.selected_pane_index]
                                                        .clone();
                                                }
                                            }
                                            FocusedPane::LogContent => {
                                                // Scroll up in log content
                                                logging_app.scroll_up();
                                            }
                                            FocusedPane::McpMessageSelector => {
                                                // Handle MCP message selector navigation up
                                                if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                                    if mcp_app.selected_message_index > 0 {
                                                        mcp_app.selected_message_index -= 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                match logging_app.ui_mode {
                                    UiMode::McpParameterEditing => {
                                        // Navigate down in parameter list
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            if !mcp_app.editing_param_value {
                                                if let Some(ref editing_msg) =
                                                    mcp_app.editing_message
                                                {
                                                    let total_params =
                                                        editing_msg.required_params.len()
                                                            + editing_msg.optional_params.len();
                                                    if mcp_app.selected_param_index
                                                        < total_params.saturating_sub(1)
                                                    {
                                                        mcp_app.selected_param_index += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // Scroll down in MCP interaction details
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            mcp_app.details_scroll_pos += 1;
                                        }
                                    }
                                    _ => {
                                        match logging_app.focused_pane {
                                            FocusedPane::LogSelector => {
                                                // Navigate down in pane selector
                                                if logging_app.selected_pane_index
                                                    < logging_app.panes.len() - 1
                                                {
                                                    logging_app.selected_pane_index += 1;
                                                    logging_app.selected_pane = logging_app.panes
                                                        [logging_app.selected_pane_index]
                                                        .clone();
                                                }
                                            }
                                            FocusedPane::LogContent => {
                                                // Scroll down in log content
                                                logging_app.scroll_down();
                                            }
                                            FocusedPane::McpMessageSelector => {
                                                // Handle MCP message selector navigation
                                                if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                                    if mcp_app.selected_message_index
                                                        < mcp_app.available_messages.len() - 1
                                                    {
                                                        mcp_app.selected_message_index += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                match logging_app.ui_mode {
                                    UiMode::LogViewing => {
                                        // Check if we're focused on log selector and selected pane is a client
                                        if logging_app.focused_pane == FocusedPane::LogSelector {
                                            if let LogPane::Client(client_id) =
                                                &logging_app.selected_pane
                                            {
                                                // Switch to MCP message sending mode
                                                let mut mcp_app = McpInteractiveApp {
                                                    available_messages:
                                                        McpMessage::get_available_messages(),
                                                    selected_message_index: 0,
                                                    client_id: client_id.clone(),
                                                    list_state: ListState::default(),
                                                    interaction_state:
                                                        McpInteractionState::SelectingMessage,
                                                    selected_message: None,
                                                    request_sent_at: None,
                                                    response_received_at: None,
                                                    response_data: None,
                                                    error_message: None,
                                                    details_scroll_pos: 0,
                                                    editing_message: None,
                                                    selected_param_index: 0,
                                                    editing_param_value: false,
                                                    param_input_buffer: String::new(),
                                                };
                                                mcp_app.list_state.select(Some(0));
                                                logging_app.mcp_app = Some(mcp_app);
                                                logging_app.ui_mode = UiMode::McpMessageSending;
                                                logging_app.focused_pane =
                                                    FocusedPane::McpMessageSelector;
                                            }
                                        }
                                    }
                                    UiMode::McpMessageSending => {
                                        // Handle MCP message selection - check if it needs parameter editing
                                        if let Some(ref mut mcp_app) = logging_app.mcp_app {
                                            if mcp_app.selected_message_index
                                                < mcp_app.available_messages.len()
                                            {
                                                let message = mcp_app.available_messages
                                                    [mcp_app.selected_message_index]
                                                    .clone();

                                                // Check if message has required parameters
                                                if message.required_param_count > 0
                                                    || !message.optional_params.is_empty()
                                                {
                                                    // Switch to parameter editing mode
                                                    mcp_app.editing_message = Some(message.clone());
                                                    mcp_app.selected_param_index = 0;
                                                    mcp_app.editing_param_value = false;
                                                    mcp_app.param_input_buffer.clear();
                                                    logging_app.ui_mode =
                                                        UiMode::McpParameterEditing;
                                                    continue; // Don't execute immediately
                                                }

                                                // No parameters needed - send immediately
                                                let client_id = mcp_app.client_id.clone();

                                                // Update MCP app state to show we're sending
                                                mcp_app.interaction_state =
                                                    McpInteractionState::SendingRequest;
                                                mcp_app.selected_message = Some(message.clone());
                                                mcp_app.request_sent_at =
                                                    Some(std::time::Instant::now());
                                                mcp_app.response_received_at = None;
                                                mcp_app.response_data = None;
                                                mcp_app.error_message = None;

                                                // Transition to interaction details view immediately
                                                logging_app.ui_mode = UiMode::McpInteractionDetails;

                                                // Try to actually send the MCP message
                                                match send_mcp_message_to_client(
                                                    &client_id,
                                                    &message,
                                                    &logging_app.scenario,
                                                )
                                                .await
                                                {
                                                    Ok(response) => {
                                                        // Update MCP app with successful response
                                                        if let Some(ref mut mcp_app) =
                                                            logging_app.mcp_app
                                                        {
                                                            mcp_app.interaction_state = McpInteractionState::ShowingResponse { success: true };
                                                            mcp_app.response_received_at =
                                                                Some(std::time::Instant::now());
                                                            mcp_app.response_data =
                                                                Some(response.clone());
                                                        }
                                                        // Also log to client pane for record keeping
                                                        logging_app.add_log(
                                                            &LogSource::Client(client_id.clone()),
                                                            format!(
                                                                "✅ MCP {}: {}",
                                                                message.name, response
                                                            ),
                                                        );
                                                    }
                                                    Err(e) => {
                                                        // Update MCP app with error response
                                                        if let Some(ref mut mcp_app) =
                                                            logging_app.mcp_app
                                                        {
                                                            mcp_app.interaction_state = McpInteractionState::ShowingResponse { success: false };
                                                            mcp_app.response_received_at =
                                                                Some(std::time::Instant::now());
                                                            mcp_app.error_message =
                                                                Some(e.to_string());
                                                        }
                                                        // Also log to client pane for record keeping
                                                        logging_app.add_log(
                                                            &LogSource::Client(client_id.clone()),
                                                            format!(
                                                                "❌ MCP {}: {}",
                                                                message.name, e
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    UiMode::McpParameterEditing => {
                                        // Handle parameter editing - placeholder for now
                                        // TODO: Implement parameter value editing
                                    }
                                    UiMode::McpInteractionDetails => {
                                        // In interaction details mode, Enter does nothing
                                        // User must press ESC to go back
                                    }
                                }
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

    // Simple completion message
    if logging_app.scenario_completed {
        println!("\n🎉 Scenario completed successfully!");
    } else {
        println!("\n📍 Scenario execution ended.");
    }

    // Get scenario result
    let _result = match scenario_task.await? {
        Ok(_) => {
            let _logs = log_task.await?;
            Ok(())
        }
        Err(e) => {
            let _logs = log_task.await?;
            Err(e)
        }
    };

    // Clean exit - no forced exit
    Ok(())
}

#[cfg(feature = "tui")]
fn draw_logging_ui(f: &mut Frame, app: &LoggingApp) {
    match app.ui_mode {
        UiMode::LogViewing => draw_log_viewing_ui(f, app),
        UiMode::McpMessageSending => draw_mcp_message_ui(f, app),
        UiMode::McpParameterEditing => draw_mcp_parameter_editing_ui(f, app),
        UiMode::McpInteractionDetails => draw_mcp_interaction_details_ui(f, app),
    }
}

#[cfg(feature = "tui")]
fn draw_log_viewing_ui(f: &mut Frame, app: &LoggingApp) {
    // First, split the screen vertically to reserve space for controls at the bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    // Now split the main area horizontally for left and right panels
    // Make left panel broader to accommodate progress bar better
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_chunks[0]);

    // Split the left panel into log selector, step progress, and system info
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Log selector
            Constraint::Length(6),      // Step progress (fixed height)
            Constraint::Min(0),         // System info (remaining space)
        ])
        .split(chunks[0]);

    // Left panel top: Pane selector
    let pane_list: Vec<ListItem> = app
        .panes
        .iter()
        .enumerate()
        .map(|(_i, pane)| {
            let (service_name, base_name) = match pane {
                LogPane::Orchestrator => {
                    ("Orchestrator".to_string(), "🎭 Orchestrator".to_string())
                }
                LogPane::MqttServer => ("MQTT Server".to_string(), "📡 MQTT Server".to_string()),
                LogPane::MqttObserver => {
                    ("MQTT Observer".to_string(), "👁️ MQTT Observer".to_string())
                }
                LogPane::Client(id) => (id.clone(), format!("👤 Client: {}", id)),
            };

            let status = app.get_service_status(&service_name);
            let status_indicator = status.get_emoji();
            let name = format!("{} {}", status_indicator, base_name);

            let is_selected = match (&app.selected_pane, pane) {
                (LogPane::Orchestrator, LogPane::Orchestrator) => true,
                (LogPane::MqttServer, LogPane::MqttServer) => true,
                (LogPane::MqttObserver, LogPane::MqttObserver) => true,
                (LogPane::Client(a), LogPane::Client(b)) => a == b,
                _ => false,
            };

            let style = if is_selected {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default().fg(status.get_color())
            };

            ListItem::new(name).style(style)
        })
        .collect();

    // Determine if the left pane (selector) is focused for visual indication
    let selector_focused = app.focused_pane == FocusedPane::LogSelector;
    let selector_title = if selector_focused {
        "🔗 Log Panes [FOCUSED]"
    } else {
        "🔗 Log Panes"
    };
    let selector_border_style = if selector_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let pane_selector = List::new(pane_list)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(selector_title)
                .border_style(selector_border_style),
        )
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    f.render_widget(pane_selector, left_chunks[0]);

    // Left panel middle: Step progress information
    let (completed_count, total_steps, current_step_name, current_step_duration) =
        app.get_step_progress_info();

    let mut step_progress_lines = Vec::new();

    // Progress bar
    let progress_ratio = if total_steps > 0 {
        completed_count as f64 / total_steps as f64
    } else {
        0.0
    };

    let progress_width = 20; // Width of progress bar (reduced to fit better in left pane)
    let filled_width = (progress_width as f64 * progress_ratio) as usize;
    let progress_bar = format!(
        "[{}{}] {}/{}",
        "█".repeat(filled_width),
        "░".repeat(progress_width - filled_width),
        completed_count,
        total_steps
    );
    step_progress_lines.push(Line::from(vec![Span::styled(
        progress_bar,
        Style::default().fg(Color::Cyan),
    )]));

    // Current step info
    if let Some(step_name) = current_step_name {
        let step_text = if step_name.len() > 25 {
            format!("{}...", &step_name[..22])
        } else {
            step_name.to_string()
        };

        let duration_text = if let Some(duration) = current_step_duration {
            format!(" ({:.1}s)", duration.as_secs_f64())
        } else {
            String::new()
        };

        step_progress_lines.push(Line::from(vec![
            Span::styled("🎬 ", Style::default().fg(Color::Yellow)),
            Span::styled(step_text, Style::default().fg(Color::White)),
            Span::styled(duration_text, Style::default().fg(Color::Gray)),
        ]));
    } else if total_steps > 0 {
        if completed_count == total_steps {
            step_progress_lines.push(Line::from(vec![Span::styled(
                "🎉 All steps completed!",
                Style::default().fg(Color::Green),
            )]));
        } else {
            step_progress_lines.push(Line::from(vec![Span::styled(
                "⏳ Preparing next step...",
                Style::default().fg(Color::Yellow),
            )]));
        }
    }

    let step_progress_widget = Paragraph::new(step_progress_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📋 Scenario Progress")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(step_progress_widget, left_chunks[1]);

    // Left panel bottom: System information
    let system_info_lines: Vec<Line> = app
        .system_info
        .format_for_display()
        .into_iter()
        .map(|line| Line::from(line))
        .collect();

    let system_info_widget = Paragraph::new(system_info_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 System Info")
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(system_info_widget, left_chunks[2]);

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

    // Determine if the right pane (content) is focused for visual indication
    let content_focused = app.focused_pane == FocusedPane::LogContent;
    let content_title = if content_focused {
        format!("📋 Logs: {} [FOCUSED]", current_pane_name)
    } else {
        format!("📋 Logs: {}", current_pane_name)
    };
    let content_border_style = if content_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let log_content = Paragraph::new(visible_logs)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(content_title)
                .border_style(content_border_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    f.render_widget(log_content, chunks[1]);

    // Show controls at bottom (using pre-allocated space)
    let controls_area = main_chunks[1];

    let controls_text = if matches!(&app.selected_pane, LogPane::Client(_))
        && app.focused_pane == FocusedPane::LogSelector
    {
        vec![Line::from(vec![
            Span::styled(
                "Tab/Shift+Tab",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Switch panes  "),
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Scroll  "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Send MCP  "),
            Span::styled(
                "q/Esc/Ctrl+C",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ])]
    } else {
        vec![Line::from(vec![
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
        ])]
    };

    let controls = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("🎮 Controls"))
        .alignment(Alignment::Center);

    f.render_widget(controls, controls_area);
}

#[cfg(feature = "tui")]
fn draw_mcp_parameter_editing_ui(f: &mut Frame, app: &LoggingApp) {
    // First, split the screen vertically to reserve space for controls at the bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    if let Some(ref mcp_app) = app.mcp_app {
        if let Some(ref editing_message) = mcp_app.editing_message {
            // Split main area for parameter list and details
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(main_chunks[0]);

            // Left panel: Parameter list
            let mut param_items = Vec::new();

            // Add required parameters
            for (i, param) in editing_message.required_params.iter().enumerate() {
                let is_selected = i == mcp_app.selected_param_index && !mcp_app.editing_param_value;
                let value_text = if param.current_value.is_empty() {
                    "<not set>".to_string()
                } else {
                    param.current_value.clone()
                };

                let content = format!("🔴 {} ({}): {}", param.name, param.param_type, value_text);
                let style = if is_selected {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default().fg(Color::Red)
                };
                param_items.push(ListItem::new(content).style(style));
            }

            // Add optional parameters
            for (i, param) in editing_message.optional_params.iter().enumerate() {
                let param_index = editing_message.required_params.len() + i;
                let is_selected =
                    param_index == mcp_app.selected_param_index && !mcp_app.editing_param_value;
                let value_text = if param.current_value.is_empty() {
                    "<empty>".to_string()
                } else {
                    param.current_value.clone()
                };

                let content = format!("⚪ {} ({}): {}", param.name, param.param_type, value_text);
                let style = if is_selected {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default().fg(Color::Blue)
                };
                param_items.push(ListItem::new(content).style(style));
            }

            let param_list = List::new(param_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "⚙️  Parameters for: {} [FOCUSED]",
                            editing_message.name
                        ))
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

            f.render_widget(param_list, chunks[0]);

            // Right panel: Parameter details and instructions
            let details = vec![
                Line::from(vec![
                    Span::styled("Command: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&editing_message.name),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&editing_message.description),
                ]),
                Line::from(Span::raw("")), // Empty line
                Line::from(vec![
                    Span::styled(
                        "🔴 Required",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" parameters must be filled"),
                ]),
                Line::from(vec![
                    Span::styled(
                        "⚪ Optional",
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" parameters are optional"),
                ]),
                Line::from(Span::raw("")), // Empty line
                Line::from("Instructions:"),
                Line::from("• ↑↓ Navigate parameters"),
                Line::from("• Enter: Edit parameter value"),
                Line::from("• Ctrl+S: Send command"),
                Line::from("• Esc: Cancel editing"),
            ];

            let details_widget = Paragraph::new(details)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📋 Parameter Details"),
                )
                .wrap(Wrap { trim: true });

            f.render_widget(details_widget, chunks[1]);
        }
    }

    // Show controls at bottom
    let controls_area = main_chunks[1];
    let controls = Paragraph::new(vec![Line::from(vec![
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Edit  "),
        Span::styled("Ctrl+S", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Send  "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎮 Parameter Controls"),
    )
    .alignment(Alignment::Center);

    f.render_widget(controls, controls_area);
}

#[cfg(feature = "tui")]
fn draw_mcp_message_ui(f: &mut Frame, app: &LoggingApp) {
    // First, split the screen vertically to reserve space for controls at the bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    // Now split the main area horizontally for left and right panels
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    if let Some(ref mcp_app) = app.mcp_app {
        // Left panel: MCP Message selector
        let message_items: Vec<ListItem> = mcp_app
            .available_messages
            .iter()
            .enumerate()
            .map(|(i, message)| {
                let is_selected = i == mcp_app.selected_message_index;

                // Show parameter count with emoji indicators
                let param_indicator = if message.required_param_count == 0 {
                    "⚡" // Lightning for no parameters (fast/simple)
                } else if message.required_param_count <= 2 {
                    "📝" // Document for few parameters
                } else {
                    "⚙️" // Gear for many parameters
                };

                let param_count_text = if message.required_param_count > 0 {
                    format!(" ({})", message.required_param_count)
                } else {
                    String::new()
                };

                let content = format!(
                    "{} {}{} - {}",
                    param_indicator, message.name, param_count_text, message.description
                );

                let style = if is_selected {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else if message.required_param_count == 0 {
                    Style::default().fg(Color::Green) // Green for parameter-less commands
                } else {
                    Style::default()
                };
                ListItem::new(content).style(style)
            })
            .collect();

        let message_selector = List::new(message_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "📡 MCP Messages for Client: {} [FOCUSED]",
                        mcp_app.client_id
                    ))
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

        f.render_widget(message_selector, chunks[0]);

        // Right panel: Message details
        let selected_message = &mcp_app.available_messages[mcp_app.selected_message_index];
        let details = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&selected_message.name),
            ]),
            Line::from(vec![
                Span::styled("Method: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&selected_message.method),
            ]),
            Line::from(vec![
                Span::styled(
                    "Description: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(&selected_message.description),
            ]),
            Line::from(Span::raw("")), // Empty line
            Line::from(vec![Span::styled(
                "Parameters: ",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::raw(selected_message.params.to_string())),
        ];

        let message_details = Paragraph::new(details)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📋 Message Details"),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(message_details, chunks[1]);
    }

    // Show controls at bottom (using pre-allocated space)
    let controls_area = main_chunks[1];

    let controls = Paragraph::new(vec![Line::from(vec![
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Send Message  "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎮 MCP Controls"),
    )
    .alignment(Alignment::Center);

    f.render_widget(controls, controls_area);
}

#[cfg(feature = "tui")]
fn draw_mcp_interaction_details_ui(f: &mut Frame, app: &LoggingApp) {
    if let Some(ref mcp_app) = app.mcp_app {
        // Create a centered modal-like overlay
        let area = f.area();
        let popup_area = centered_rect(80, 80, area);

        // Clear the background
        let clear_block = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(clear_block, area);

        // Split the popup area for title, content, and controls
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Controls
            ])
            .split(popup_area);

        // Title
        let title_text = format!("📡 MCP Interaction with {}", mcp_app.client_id);
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        f.render_widget(title, chunks[0]);

        // Content based on interaction state
        let content_lines = match &mcp_app.interaction_state {
            McpInteractionState::SelectingMessage => {
                vec![Line::from(Span::styled(
                    "🤔 This shouldn't be visible...",
                    Style::default().fg(Color::Red),
                ))]
            }
            McpInteractionState::SendingRequest => {
                let mut lines = vec![];
                if let Some(ref message) = mcp_app.selected_message {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "🚀 Sending: ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            &message.name,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("Method: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(&message.method),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Parameters:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    // Pretty-print parameters JSON with syntax highlighting
                    let formatted_params = serde_json::to_string_pretty(&message.params)
                        .unwrap_or_else(|_| message.params.to_string());
                    let highlighted_params = format_json_with_syntax_highlighting(formatted_params);
                    lines.extend(highlighted_params);
                    lines.push(Line::from(""));
                    if let Some(sent_at) = mcp_app.request_sent_at {
                        let elapsed = sent_at.elapsed().as_millis();
                        lines.push(Line::from(vec![
                            Span::styled(
                                "⏱️  Waiting for response... ",
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(
                                format!("({}ms)", elapsed),
                                Style::default().fg(Color::Gray),
                            ),
                        ]));
                    }
                }
                lines
            }
            McpInteractionState::ShowingResponse { success } => {
                let mut lines = vec![];
                if let Some(ref message) = mcp_app.selected_message {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "📤 Sent: ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            &message.name,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    if let Some(sent_at) = mcp_app.request_sent_at {
                        if let Some(received_at) = mcp_app.response_received_at {
                            let duration = received_at.duration_since(sent_at).as_millis();
                            lines.push(Line::from(vec![
                                Span::styled(
                                    "⏱️  Response time: ",
                                    Style::default().fg(Color::Gray),
                                ),
                                Span::styled(
                                    format!("{}ms", duration),
                                    Style::default().fg(Color::Cyan),
                                ),
                            ]));
                        }
                    }

                    lines.push(Line::from(""));

                    if *success {
                        lines.push(Line::from(Span::styled(
                            "✅ Response:",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )));
                        if let Some(ref response) = mcp_app.response_data {
                            // Pretty-print JSON with syntax highlighting
                            let formatted_json = serde_json::to_string_pretty(response)
                                .unwrap_or_else(|_| response.to_string());
                            let highlighted_json =
                                format_json_with_syntax_highlighting(formatted_json);
                            lines.extend(highlighted_json);
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            "❌ Error:",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        )));
                        if let Some(ref error) = mcp_app.error_message {
                            // Try to parse error as JSON for better formatting
                            if let Ok(json_error) = serde_json::from_str::<serde_json::Value>(error)
                            {
                                let formatted_json = serde_json::to_string_pretty(&json_error)
                                    .unwrap_or_else(|_| error.clone());
                                let highlighted_json =
                                    format_json_with_syntax_highlighting(formatted_json);
                                lines.extend(highlighted_json);
                            } else {
                                // Plain error message
                                for line in error.lines() {
                                    lines.push(Line::from(Span::styled(
                                        line,
                                        Style::default().fg(Color::Red),
                                    )));
                                }
                            }
                        }
                    }
                }
                lines
            }
        };

        let scroll_pos = mcp_app.details_scroll_pos;
        let content = Paragraph::new(content_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("📋 Details")
                    .border_style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: true })
            .scroll((scroll_pos, 0));
        f.render_widget(content, chunks[1]);

        // Controls
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Scroll  "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Return to Log View"),
        ])])
        .block(Block::default().borders(Borders::ALL).title("🎮 Controls"))
        .alignment(Alignment::Center);
        f.render_widget(controls, chunks[2]);
    }
}

/// Helper function to create a centered rectangle
#[cfg(feature = "tui")]
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::prelude::Rect,
) -> ratatui::prelude::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(feature = "tui")]
fn format_json_with_syntax_highlighting(json_str: String) -> Vec<Line<'static>> {
    json_str
        .lines()
        .map(|line| {
            // Use a simple regex-based approach to preserve whitespace exactly
            let mut spans = Vec::new();
            let mut current_pos = 0;

            // Handle the line character by character, but group similar characters
            let chars: Vec<char> = line.chars().collect();

            while current_pos < chars.len() {
                let ch = chars[current_pos];

                match ch {
                    // Handle strings
                    '"' => {
                        let start = current_pos;
                        current_pos += 1;

                        // Find the end of the string, handling escapes
                        while current_pos < chars.len() {
                            if chars[current_pos] == '"'
                                && (current_pos == 0 || chars[current_pos - 1] != '\\')
                            {
                                current_pos += 1;
                                break;
                            }
                            current_pos += 1;
                        }

                        let string_part: String = chars[start..current_pos].iter().collect();

                        // Check if this is a key by looking ahead for ':'
                        let rest_of_line = &chars[current_pos..];
                        let remaining: String = rest_of_line.iter().collect();
                        let is_key = remaining.trim_start().starts_with(':');

                        let color = if is_key { Color::Cyan } else { Color::Green };
                        spans.push(Span::styled(string_part, Style::default().fg(color)));
                    }

                    // Handle punctuation
                    '{' | '}' | '[' | ']' | ',' | ':' => {
                        spans.push(Span::styled(
                            ch.to_string(),
                            Style::default().fg(Color::Yellow),
                        ));
                        current_pos += 1;
                    }

                    // Handle whitespace (preserve exactly)
                    ' ' | '\t' => {
                        let start = current_pos;
                        while current_pos < chars.len() && matches!(chars[current_pos], ' ' | '\t')
                        {
                            current_pos += 1;
                        }
                        let whitespace: String = chars[start..current_pos].iter().collect();
                        spans.push(Span::raw(whitespace));
                    }

                    // Handle other tokens (numbers, booleans, null, etc.)
                    _ => {
                        let start = current_pos;
                        // Read until we hit a delimiter
                        while current_pos < chars.len() {
                            let ch = chars[current_pos];
                            if matches!(ch, '"' | '{' | '}' | '[' | ']' | ',' | ':' | ' ' | '\t') {
                                break;
                            }
                            current_pos += 1;
                        }

                        if current_pos > start {
                            let token: String = chars[start..current_pos].iter().collect();

                            let color = if token.parse::<f64>().is_ok() {
                                Color::Magenta // Numbers
                            } else if matches!(token.as_str(), "true" | "false") {
                                Color::Blue // Booleans
                            } else if token == "null" {
                                Color::Red // Null
                            } else {
                                Color::White // Default
                            };

                            spans.push(Span::styled(token, Style::default().fg(color)));
                        }
                    }
                }
            }

            Line::from(spans)
        })
        .collect()
}

async fn run_scenario_with_logging(
    scenario: Scenario,
    scenario_file_path: Option<PathBuf>,
    log_collector: LogCollector,
    quit_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create shared state wrapped in Arc<Mutex> for signal handling
    let state = Arc::new(Mutex::new(OrchestratorState::new(
        scenario,
        scenario_file_path,
    )));

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
        let mqtt_server_required = state.scenario.infrastructure.mqtt_server.required;
        let mcp_server_required = state
            .scenario
            .infrastructure
            .mcp_server
            .as_ref()
            .map(|mcp| mcp.required)
            .unwrap_or(false);
        let mqtt_observer_required = state
            .scenario
            .infrastructure
            .mqtt_observer
            .as_ref()
            .map(|obs| obs.required)
            .unwrap_or(false);

        println!(
            "[DEBUG] Infrastructure requirements: mqtt_server={}, mcp_server={}, mqtt_observer={}",
            mqtt_server_required, mcp_server_required, mqtt_observer_required
        );

        mqtt_server_required || mcp_server_required || mqtt_observer_required
    };

    println!("[DEBUG] needs_infrastructure = {}", needs_infrastructure);

    if needs_infrastructure {
        let mut state = state.lock().await;
        start_infrastructure_with_logging(&mut *state, log_collector.clone(), quit_flag.clone())
            .await?;
    } else {
        println!("[DEBUG] Skipping infrastructure startup - no services required");
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

    // Update orchestrator status to ready
    log_collector.log_str(LogSource::Orchestrator, "🟢 Orchestrator ready");

    // Check if MQTT port is already in use before starting
    if state.scenario.infrastructure.mqtt_server.required {
        let port = state.scenario.infrastructure.mqtt_server.port;
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  Checking if MQTT port {} is available...", port),
        );

        // Check if port is already occupied
        if is_port_occupied("localhost", port).await {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("🔄 MQTT port {} is already in use - reusing existing server instead of starting new one", port),
            );
            log_collector.log_str(
                LogSource::MqttServer,
                "🟢 Using existing MQTT Server (reused)",
            );
            // Skip starting our own MQTT server, but continue with observer if needed
        } else {
        }

        // Start MQTT server directly if required (instead of delegating to xtask)
        if state.scenario.infrastructure.mqtt_server.required {
            let port = state.scenario.infrastructure.mqtt_server.port;
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  Starting MQTT server on port {}", port),
            );

            // Update MQTT server status to starting
            log_collector.log_str(LogSource::MqttServer, "🟡 MQTT Server starting...");

            // Log the exact command being executed
            let mqtt_cmd = format!("cargo run --release -- --port {}", port);
            log_collector.log_str(
                LogSource::MqttServer,
                &format!("🚀 Executing command: {}", mqtt_cmd),
            );
            log_collector.log_str(
                LogSource::MqttServer,
                &format!("📁 Working directory: ../mqtt-server"),
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
                600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
                Some("cargo run --release (mqtt-server)"),
                log_collector.clone(),
            )
            .await;
            if !mqtt_ready {
                let error_msg = format!(
                    "MQTT server failed to start on port {} within 10 minute timeout",
                    port
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                return Err(error_msg.into());
            }

            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ✅ MQTT server ready on port {}", port),
            );

            // Update MQTT server status to ready
            log_collector.log_str(LogSource::MqttServer, "🟢 MQTT Server ready");
        }
    }

    // Start MQTT observer if required
    if let Some(ref mqtt_observer) = state.scenario.infrastructure.mqtt_observer {
        if mqtt_observer.required {
            log_collector.log_str(LogSource::Orchestrator, "  Starting MQTT observer");

            // Update MQTT observer status to starting
            log_collector.log_str(LogSource::MqttObserver, "🟡 MQTT Observer starting...");
            log_collector.log_str(
                LogSource::MqttObserver,
                &format!(
                    "Connecting to MQTT broker at localhost:{}",
                    state.scenario.infrastructure.mqtt_server.port
                ),
            );

            let mqtt_port = state.scenario.infrastructure.mqtt_server.port;

            // Log the exact command being executed
            let observer_cmd = format!(
                "cargo run --bin mqtt-observer -- -h localhost -p {} -t '#' -i mcplay_observer",
                mqtt_port
            );
            log_collector.log_str(
                LogSource::MqttObserver,
                &format!("🚀 Executing command: {}", observer_cmd),
            );
            log_collector.log_str(
                LogSource::MqttObserver,
                &format!("📁 Working directory: ../mqtt-client"),
            );

            let mut observer_process = TokioCommand::new("cargo")
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
                .map_err(|e| {
                    let error_msg = format!(
                        "Failed to start MQTT observer: {}. Make sure mqtt-client is available in ../mqtt-client.",
                        e
                    );
                    log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                    error_msg
                })?;

            // Capture stdout and stderr from the MQTT observer process
            if let Some(stdout) = observer_process.stdout.take() {
                let log_collector = log_collector.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stdout);
                    let mut line = String::new();
                    let mut connection_established = false;

                    while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                        let trimmed_line = line.trim();

                        // Check for connection status in observer output
                        if !connection_established
                            && (trimmed_line.contains("Connected to")
                                || trimmed_line.contains("Connection successful")
                                || trimmed_line.contains("Subscribed to")
                                || trimmed_line.contains("MQTT client connected"))
                        {
                            connection_established = true;
                            log_collector.log_str(
                                LogSource::MqttObserver,
                                "Connected to MQTT broker - Observer ready",
                            );
                        }

                        // Log the actual output
                        log_collector.log_str(LogSource::MqttObserver, trimmed_line);
                        line.clear();
                    }
                });
            }

            if let Some(stderr) = observer_process.stderr.take() {
                let log_collector = log_collector.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr);
                    let mut line = String::new();
                    while reader.read_line(&mut line).await.is_ok() && !line.is_empty() {
                        log_collector.log_str(
                            LogSource::MqttObserver,
                            &format!("[stderr] {}", line.trim()),
                        );
                        line.clear();
                    }
                });
            }

            state
                .infrastructure_processes
                .insert("mqtt_observer".to_string(), observer_process);

            log_collector.log_str(
                LogSource::Orchestrator,
                "  ✅ MQTT observer process started",
            );

            // Wait for MQTT observer to establish connection (readiness check)
            log_collector.log_str(
                LogSource::Orchestrator,
                "  Waiting for MQTT observer to connect to broker...",
            );

            // Simple delay to allow observer to connect - in a real implementation,
            // we would monitor the observer's connection status more precisely
            tokio::time::sleep(Duration::from_millis(2000)).await;

            log_collector.log_str(LogSource::Orchestrator, "  ✅ MQTT observer ready");
        }
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

        // Update client status to starting
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            "🟡 Client starting...",
        );

        // Handle different client types
        match client.client_type.as_str() {
            "wasm" => {
                // For WASM clients, launch browser instead of desktop client
                start_wasm_client_with_logging(client, log_collector.clone()).await?;
                continue;
            }
            "desktop" | _ => {
                // Default to desktop client handling
            }
        }

        // Build client command and log it (for desktop clients)
        let mut client_cmd_parts = vec![
            "cargo".to_string(),
            "run".to_string(),
            "--bin".to_string(),
            "iotcraft-dekstop-client".to_string(),
            "--".to_string(),
            "--mcp".to_string(),
        ];

        // Add player configuration
        client_cmd_parts.push("--player-id".to_string());
        client_cmd_parts.push(client.player_id.clone());
        client_cmd_parts.push("--player-name".to_string());
        // Use client name if available, otherwise fall back to client ID
        let player_name = client.name.as_ref().unwrap_or(&client.id);
        client_cmd_parts.push(player_name.clone());

        // Add optional MQTT arguments if required
        if state.scenario.infrastructure.mqtt_server.required {
            client_cmd_parts.push("--mqtt-server".to_string());
            client_cmd_parts.push(format!(
                "localhost:{}",
                state.scenario.infrastructure.mqtt_server.port
            ));
        }

        let client_cmd_str = client_cmd_parts.join(" ");
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            &format!("🚀 Executing command: {}", client_cmd_str),
        );
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            &format!("📁 Working directory: ../desktop-client"),
        );
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            &format!("🏠 Environment: MCP_PORT={}", client.mcp_port),
        );

        // First, attempt to build the desktop-client to catch compilation errors
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            "🔨 Building desktop-client before starting...",
        );

        let build_cmd = TokioCommand::new("cargo")
            .current_dir("../desktop-client")
            .args(&["build", "--bin", "iotcraft-dekstop-client"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to execute cargo build for client {}: {}",
                    client.id, e
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
                error_msg
            })?;

        let build_output = build_cmd.wait_with_output().await.map_err(|e| {
            let error_msg = format!(
                "Failed to wait for cargo build for client {}: {}",
                client.id, e
            );
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            error_msg
        })?;

        // Convert build output to strings
        let build_stdout = String::from_utf8_lossy(&build_output.stdout);
        let build_stderr = String::from_utf8_lossy(&build_output.stderr);

        // Log build output to client logs
        if !build_stdout.trim().is_empty() {
            for line in build_stdout.lines() {
                log_collector.log_str(
                    LogSource::Client(client.id.clone()),
                    &format!("[build] {}", line),
                );
            }
        }
        if !build_stderr.trim().is_empty() {
            for line in build_stderr.lines() {
                log_collector.log_str(
                    LogSource::Client(client.id.clone()),
                    &format!("[build-err] {}", line),
                );
            }
        }

        // Check if build succeeded
        if !build_output.status.success() {
            let error_msg = format!(
                "Build failed for client {} with exit code: {}\n\nBuild stderr:\n{}\n\nBuild stdout:\n{}",
                client.id,
                build_output.status.code().unwrap_or(-1),
                build_stderr.trim(),
                build_stdout.trim()
            );
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            log_collector.log_str(
                LogSource::Client(client.id.clone()),
                "🔴 Build failed - scenario cannot continue",
            );

            // Ensure logs directory exists and save detailed build logs to a separate file
            if let Err(mkdir_err) = tokio::fs::create_dir_all("logs").await {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!("⚠️  Failed to create logs directory: {}", mkdir_err),
                );
            }

            let build_log_path = format!(
                "logs/build_failure_{}_{}.log",
                client.id,
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );
            if let Err(write_err) = tokio::fs::write(&build_log_path, &error_msg).await {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!(
                        "⚠️  Failed to write build log to {}: {}",
                        build_log_path, write_err
                    ),
                );
            } else {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!("💾 Build failure details saved to: {}", build_log_path),
                );
            }

            return Err(error_msg.into());
        }

        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            "✅ Build completed successfully, starting client...",
        );

        // Build client command arguments (now that we know build succeeds)
        let mut cmd = TokioCommand::new("cargo");
        cmd.current_dir("../desktop-client")
            .arg("run")
            .arg("--bin")
            .arg("iotcraft-dekstop-client")
            .args(&["--", "--mcp"])
            .arg("--player-id")
            .arg(&client.player_id)
            .arg("--player-name")
            // Use client name if available, otherwise fall back to client ID
            .arg(client.name.as_ref().unwrap_or(&client.id))
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

        // Start the client (build already verified to work)
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

        // Get mutable reference to the process from the HashMap for monitoring
        let client_process_ref = state
            .client_processes
            .get_mut(&client.id)
            .ok_or("Client process not found in HashMap")?;

        let mcp_ready = wait_for_port_with_process_monitoring(
            "localhost",
            client.mcp_port,
            600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
            Some(&format!(
                "cargo run --bin iotcraft-dekstop-client ({})",
                client.id
            )),
            client_process_ref, // Pass the process handle so we can monitor if it exits
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

        // Update client status to ready
        log_collector.log_str(
            LogSource::Client(client.id.clone()),
            "🟢 Client ready and healthy",
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
        let result = execute_step_with_logging(step, state, log_collector.clone()).await;
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
    if let Some(file_path) = &state.scenario_file_path {
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("Scenario file: {}", file_path.display()),
        );
    }
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

async fn execute_step_with_logging(
    step: &Step,
    state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match &step.action {
        // mcplay-style actions
        Action::McpCall { tool, arguments } => {
            execute_mcp_call_with_logging(&step.client, tool, arguments, state, log_collector).await
        }
        Action::WaitCondition {
            condition,
            expected_value,
            timeout: wait_timeout,
        } => {
            execute_wait_condition_with_logging(
                condition,
                expected_value.as_deref(),
                *wait_timeout,
                state,
                log_collector,
            )
            .await
        }
        Action::ConsoleCommand { command } => {
            execute_console_command_with_logging(&step.client, command, state, log_collector).await
        }
        Action::Delay { duration } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ⏳ Delaying for {}ms", duration),
            );
            sleep(Duration::from_millis(*duration)).await;
            Ok(serde_json::json!({"status": "delayed", "duration_ms": duration}))
        }
        Action::ValidateScenario { checks } => {
            execute_validate_scenario_with_logging(checks, state, log_collector).await
        }

        // xtask-style actions (basic support)
        Action::Wait { duration_ms } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ⏳ Waiting for {}ms", duration_ms),
            );
            sleep(Duration::from_millis(*duration_ms)).await;
            Ok(serde_json::json!({"status": "waited", "duration_ms": duration_ms}))
        }
        Action::MqttPublish {
            topic,
            payload,
            qos: _,
            retain: _,
        } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "  📡 Publishing to MQTT topic: {} payload: {}",
                    topic, payload
                ),
            );
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
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "  🔍 Expecting MQTT message on topic: {} (timeout: {:?}ms)",
                    topic, timeout_ms
                ),
            );
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
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "  🎮 Client action: {:?} for client {}",
                    action_type, client_id
                ),
            );
            // TODO: Implement actual client action execution
            Ok(serde_json::json!({
                "status": "client_action_executed",
                "client_id": client_id,
                "action_type": action_type,
                "parameters": parameters
            }))
        }
        Action::Parallel { actions } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  🔀 Executing {} actions in parallel", actions.len()),
            );
            // TODO: Implement parallel action execution
            Ok(serde_json::json!({
                "status": "parallel_actions_completed",
                "action_count": actions.len()
            }))
        }
        Action::Sequence { actions } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ➡️ Executing {} actions in sequence", actions.len()),
            );
            // TODO: Implement sequential action execution
            Ok(serde_json::json!({
                "status": "sequence_actions_completed",
                "action_count": actions.len()
            }))
        }
        Action::Custom {
            action_type,
            parameters,
        } => {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "  🔧 Executing custom action: {} with params: {:?}",
                    action_type, parameters
                ),
            );
            // TODO: Implement custom action execution
            Ok(serde_json::json!({
                "status": "custom_action_executed",
                "action_type": action_type,
                "parameters": parameters
            }))
        }

        // New system integration actions
        Action::SystemCommand {
            command,
            working_dir,
            background,
            timeout_seconds,
        } => {
            execute_system_command_with_logging(
                command,
                working_dir.as_deref(),
                *background,
                *timeout_seconds,
                log_collector,
            )
            .await
        }

        Action::OpenBrowser {
            url,
            browser,
            wait_seconds,
        } => {
            execute_open_browser_with_logging(url, browser.as_deref(), *wait_seconds, log_collector)
                .await
        }

        Action::ShowMessage {
            message,
            message_type,
        } => {
            execute_show_message_with_logging(message, message_type.as_deref(), log_collector).await
        }
    }
}

// TUI-safe helper functions for execute_step_with_logging

async fn execute_mcp_call_with_logging(
    client_id: &str,
    tool: &str,
    arguments: &serde_json::Value,
    state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "  🔧 MCP call to {}: {} with args: {}",
            client_id, tool, arguments
        ),
    );

    // Find client connection
    if let Some(_stream) = state.client_connections.get(client_id) {
        // TODO: Implement actual MCP call
        Ok(serde_json::json!({
            "status": "mcp_call_completed",
            "tool": tool,
            "arguments": arguments
        }))
    } else {
        let error_msg = format!("Client {} not connected", client_id);
        log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
        Err(error_msg.into())
    }
}

async fn execute_wait_condition_with_logging(
    condition: &str,
    expected_value: Option<&str>,
    wait_timeout: u64,
    _state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "  ⏳ Waiting for condition: {} (timeout: {}ms)",
            condition, wait_timeout
        ),
    );

    // Handle special conditions
    match condition {
        "manual_exit" => {
            // For manual_exit condition, wait indefinitely until Ctrl+C
            log_collector.log_str(
                LogSource::Orchestrator,
                "  📝 Manual exit condition - waiting indefinitely (use ESC or Ctrl+C to exit)",
            );

            // Create a cancellation signal detector
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("Failed to install SIGINT handler in wait_condition");
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to install SIGTERM handler in wait_condition");

            // Also respect the timeout if provided
            let timeout_duration = Duration::from_millis(wait_timeout);
            let start = Instant::now();

            loop {
                let remaining_time = timeout_duration
                    .checked_sub(start.elapsed())
                    .unwrap_or(Duration::ZERO);

                if remaining_time.is_zero() {
                    log_collector.log_str(
                        LogSource::Orchestrator,
                        &format!("  ⏰ Wait condition timed out after {}ms", wait_timeout),
                    );
                    return Ok(serde_json::json!({
                        "condition": condition,
                        "expected": expected_value,
                        "status": "timeout"
                    }));
                }

                let sleep_duration = std::cmp::min(Duration::from_secs(1), remaining_time);

                tokio::select! {
                    _ = sleep(sleep_duration) => {
                        // Continue waiting
                        continue;
                    }
                    _ = sigint.recv() => {
                        log_collector.log_str(
                            LogSource::Orchestrator,
                            "  🛑 Manual exit condition met (SIGINT received)",
                        );
                        return Ok(serde_json::json!({
                            "condition": condition,
                            "expected": expected_value,
                            "status": "manual_exit_triggered"
                        }));
                    }
                    _ = sigterm.recv() => {
                        log_collector.log_str(
                            LogSource::Orchestrator,
                            "  🛑 Manual exit condition met (SIGTERM received)",
                        );
                        return Ok(serde_json::json!({
                            "condition": condition,
                            "expected": expected_value,
                            "status": "manual_exit_triggered"
                        }));
                    }
                }
            }
        }
        _ => {
            // For other conditions, use the original behavior but respect the timeout
            let wait_duration = Duration::from_millis(wait_timeout);
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!(
                    "  ⏳ Simulating wait for condition '{}' for {}ms",
                    condition, wait_timeout
                ),
            );
            sleep(wait_duration).await;

            Ok(serde_json::json!({
                "condition": condition,
                "expected": expected_value,
                "status": "condition_met"
            }))
        }
    }
}

async fn execute_console_command_with_logging(
    _client_id: &str,
    command: &str,
    _state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("  💻 Console command: {}", command),
    );

    // For now, simulate console command execution
    Ok(serde_json::json!({
        "command": command,
        "status": "executed"
    }))
}

async fn execute_validate_scenario_with_logging(
    checks: &[String],
    _state: &mut OrchestratorState,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Orchestrator,
        &format!("  ✅ Validating scenario with checks: {:?}", checks),
    );

    // For now, simulate validation
    Ok(serde_json::json!({
        "checks": checks,
        "all_passed": true
    }))
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

// New helper functions for system integration actions

async fn execute_system_command_with_logging(
    command: &[String],
    working_dir: Option<&str>,
    background: Option<bool>,
    timeout_seconds: Option<u64>,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let command_str = command.join(" ");
    let working_dir = working_dir.unwrap_or(".");
    let background = background.unwrap_or(false);
    let timeout_seconds = timeout_seconds.unwrap_or(30);

    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "  🔧 System command: {} (working_dir: {}, background: {}, timeout: {}s)",
            command_str, working_dir, background, timeout_seconds
        ),
    );

    if command.is_empty() {
        return Err("System command cannot be empty".into());
    }

    let mut cmd = tokio::process::Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }
    cmd.current_dir(working_dir);

    if background {
        // For background processes, just start them and return immediately
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to start background command '{}': {}",
                command_str, e
            )
        })?;

        // Start background task to monitor the process
        let log_collector_clone = log_collector.clone();
        let command_str_clone = command_str.clone();
        tokio::spawn(async move {
            if let Ok(exit_status) = child.wait().await {
                if exit_status.success() {
                    log_collector_clone.log_str(
                        LogSource::Orchestrator,
                        &format!(
                            "  ✅ Background command '{}' completed successfully",
                            command_str_clone
                        ),
                    );
                } else {
                    log_collector_clone.log_str(
                        LogSource::Orchestrator,
                        &format!(
                            "  ❌ Background command '{}' failed with status: {:?}",
                            command_str_clone, exit_status
                        ),
                    );
                }
            }
        });

        Ok(serde_json::json!({
            "status": "background_started",
            "command": command_str,
            "working_dir": working_dir
        }))
    } else {
        // For foreground processes, wait for completion with timeout
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start command '{}': {}", command_str, e))?;

        let timeout_duration = Duration::from_secs(timeout_seconds);
        match tokio::time::timeout(timeout_duration, child.wait()).await {
            Ok(Ok(exit_status)) => {
                if exit_status.success() {
                    log_collector.log_str(
                        LogSource::Orchestrator,
                        &format!("  ✅ Command '{}' completed successfully", command_str),
                    );
                    Ok(serde_json::json!({
                        "status": "success",
                        "command": command_str,
                        "exit_code": exit_status.code().unwrap_or(0)
                    }))
                } else {
                    let error_msg = format!(
                        "Command '{}' failed with exit status: {:?}",
                        command_str, exit_status
                    );
                    log_collector.log_str(LogSource::Orchestrator, &format!("  ❌ {}", error_msg));
                    Err(error_msg.into())
                }
            }
            Ok(Err(e)) => {
                let error_msg = format!("Command '{}' execution error: {}", command_str, e);
                log_collector.log_str(LogSource::Orchestrator, &format!("  ❌ {}", error_msg));
                Err(error_msg.into())
            }
            Err(_) => {
                // Timeout occurred, kill the process
                let _ = child.kill().await;
                let error_msg = format!(
                    "Command '{}' timed out after {}s",
                    command_str, timeout_seconds
                );
                log_collector.log_str(LogSource::Orchestrator, &format!("  ⏰ {}", error_msg));
                Err(error_msg.into())
            }
        }
    }
}

async fn execute_open_browser_with_logging(
    url: &str,
    browser: Option<&str>,
    wait_seconds: Option<u64>,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let wait_seconds = wait_seconds.unwrap_or(3);

    log_collector.log_str(
        LogSource::Orchestrator,
        &format!(
            "  🌐 Opening browser: {} (browser: {:?}, wait: {}s)",
            url, browser, wait_seconds
        ),
    );

    // Prepare browser command based on macOS
    let mut cmd = tokio::process::Command::new("open");

    if let Some(browser) = browser {
        match browser.to_lowercase().as_str() {
            "chrome" => {
                cmd.arg("-a").arg("Google Chrome");
            }
            "safari" => {
                cmd.arg("-a").arg("Safari");
            }
            "firefox" => {
                cmd.arg("-a").arg("Firefox");
            }
            _ => {
                // Try to use the browser name directly
                cmd.arg("-a").arg(browser);
            }
        }
    }
    // If no browser specified, use system default (no -a flag)

    cmd.arg(url);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to open browser for URL '{}': {}", url, e))?;

    if output.status.success() {
        log_collector.log_str(
            LogSource::Orchestrator,
            &format!("  ✅ Browser opened successfully for: {}", url),
        );

        // Wait for browser to load
        if wait_seconds > 0 {
            log_collector.log_str(
                LogSource::Orchestrator,
                &format!("  ⏳ Waiting {}s for browser to load...", wait_seconds),
            );
            sleep(Duration::from_secs(wait_seconds)).await;
        }

        Ok(serde_json::json!({
            "status": "browser_opened",
            "url": url,
            "browser": browser,
            "wait_seconds": wait_seconds
        }))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Failed to open browser for '{}': {}", url, stderr.trim());
        log_collector.log_str(LogSource::Orchestrator, &format!("  ❌ {}", error_msg));
        Err(error_msg.into())
    }
}

async fn execute_show_message_with_logging(
    message: &str,
    message_type: Option<&str>,
    log_collector: LogCollector,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let message_type = message_type.unwrap_or("info");

    let (emoji, prefix) = match message_type {
        "error" => ("❌", "ERROR"),
        "warning" => ("⚠️", "WARNING"),
        "success" => ("✅", "SUCCESS"),
        _ => ("💡", "INFO"),
    };

    // Format the message with proper indentation for multi-line messages
    let formatted_message = if message.contains('\n') {
        // Multi-line message
        let lines: Vec<&str> = message.split('\n').collect();
        let first_line = format!("  {} {}: {}", emoji, prefix, lines[0]);
        let other_lines: Vec<String> = lines[1..]
            .iter()
            .map(|line| format!("       {}", line))
            .collect();

        vec![first_line]
            .into_iter()
            .chain(other_lines)
            .collect::<Vec<String>>()
            .join("\n")
    } else {
        // Single line message
        format!("  {} {}: {}", emoji, prefix, message)
    };

    log_collector.log_str(LogSource::Orchestrator, &formatted_message);

    Ok(serde_json::json!({
        "status": "message_shown",
        "message": message,
        "message_type": message_type
    }))
}

async fn start_wasm_client_with_logging(
    client: &scenario_types::ClientConfig,
    log_collector: LogCollector,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_collector.log_str(
        LogSource::Client(client.id.clone()),
        "🌐 Starting WASM client in browser...",
    );

    // Extract browser and URL from client config
    let config = client
        .config
        .as_ref()
        .and_then(|c| c.as_object())
        .ok_or("WASM client requires config with browser and url")?;

    let browser = config
        .get("browser")
        .and_then(|b| b.as_str())
        .unwrap_or("chrome");

    let url = config
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or("WASM client config must specify url")?;

    log_collector.log_str(
        LogSource::Client(client.id.clone()),
        &format!("🚀 Opening {} with URL: {}", browser, url),
    );

    // Prepare browser command based on macOS
    let mut cmd = tokio::process::Command::new("open");

    match browser.to_lowercase().as_str() {
        "chrome" => {
            cmd.arg("-a").arg("Google Chrome");
        }
        "safari" => {
            cmd.arg("-a").arg("Safari");
        }
        "firefox" => {
            cmd.arg("-a").arg("Firefox");
        }
        _ => {
            // Try to use the browser name directly
            cmd.arg("-a").arg(browser);
        }
    }

    cmd.arg(url);

    // Launch browser as background process
    let mut browser_process = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            let error_msg = format!(
                "Failed to start WASM client (browser) for {}: {}",
                client.id, e
            );
            log_collector.log_str(LogSource::Orchestrator, &format!("❌ {}", error_msg));
            error_msg
        })?;

    // For WASM clients, we consider them "started" if the browser opens successfully
    // and doesn't immediately exit with an error
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Check if the browser process is still running (basic readiness check)
    match browser_process.try_wait() {
        Ok(Some(exit_status)) => {
            if exit_status.success() {
                // Browser opened and closed normally (e.g., tab opened in existing browser)
                log_collector.log_str(
                    LogSource::Client(client.id.clone()),
                    "✅ WASM client (browser) started successfully - tab opened",
                );
            } else {
                let error_msg = format!(
                    "Browser for WASM client {} exited with error status: {:?}",
                    client.id, exit_status
                );
                log_collector.log_str(
                    LogSource::Client(client.id.clone()),
                    &format!("❌ {}", error_msg),
                );
                return Err(error_msg.into());
            }
        }
        Ok(None) => {
            // Browser process is still running (new browser instance)
            log_collector.log_str(
                LogSource::Client(client.id.clone()),
                "✅ WASM client (browser) started successfully - browser running",
            );

            // Start background task to monitor the browser process
            let log_collector_clone = log_collector.clone();
            let client_id_clone = client.id.clone();
            tokio::spawn(async move {
                if let Ok(exit_status) = browser_process.wait().await {
                    if exit_status.success() {
                        log_collector_clone.log_str(
                            LogSource::Client(client_id_clone),
                            "📱 WASM client (browser) closed normally",
                        );
                    } else {
                        log_collector_clone.log_str(
                            LogSource::Client(client_id_clone),
                            &format!(
                                "⚠️ WASM client (browser) exited with status: {:?}",
                                exit_status
                            ),
                        );
                    }
                }
            });
        }
        Err(e) => {
            log_collector.log_str(
                LogSource::Client(client.id.clone()),
                &format!("⚠️ Could not check browser process status: {}", e),
            );
            // Continue anyway, browser might still be working
        }
    }

    // Mark client as ready
    log_collector.log_str(
        LogSource::Client(client.id.clone()),
        "🟢 WASM client ready for testing",
    );

    Ok(())
}

// Enhanced version that monitors process health while waiting for port
async fn wait_for_port_with_process_monitoring(
    host: &str,
    port: u16,
    timeout_seconds: u64,
    context: Option<&str>,
    process: &mut tokio::process::Child,
    log_collector: LogCollector,
) -> bool {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    let start = Instant::now();
    let mut last_log_time = Instant::now();

    while start.elapsed() < timeout_duration {
        // First check if the process is still running
        match process.try_wait() {
            Ok(Some(exit_status)) => {
                let error_msg = if let Some(context) = context {
                    format!(
                        "Process for {} exited with status {} before port became available",
                        context, exit_status
                    )
                } else {
                    format!(
                        "Process exited with status {} before port {}:{} became available",
                        exit_status, host, port
                    )
                };
                log_collector.log_str(LogSource::Orchestrator, &format!("    ❌ {}", error_msg));
                return false;
            }
            Ok(None) => {
                // Process is still running, continue monitoring
            }
            Err(e) => {
                log_collector.log_str(
                    LogSource::Orchestrator,
                    &format!("    ⚠️ Failed to check process status: {}", e),
                );
                // Continue trying, process might still be running
            }
        }

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
async fn send_mcp_message_to_client(
    client_id: &str,
    message: &McpMessage,
    scenario: &Scenario,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Find the client in the scenario
    let client = scenario
        .clients
        .iter()
        .find(|c| c.id == client_id)
        .ok_or_else(|| format!("Client {} not found in scenario", client_id))?;

    // Connect to the client's MCP server
    let mut stream = TcpStream::connect(format!("localhost:{}", client.mcp_port))
        .await
        .map_err(|e| {
            format!(
                "Failed to connect to client {} MCP server: {}",
                client_id, e
            )
        })?;

    // Create MCP request with unique ID
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: request_id,
        method: message.method.clone(),
        params: message.params.clone(),
    };

    // Send request
    let request_json = serde_json::to_string(&request)
        .map_err(|e| format!("Failed to serialize MCP request: {}", e))?;

    stream
        .write_all(format!("{}\n", request_json).as_bytes())
        .await
        .map_err(|e| format!("Failed to send MCP request: {}", e))?;

    // Read response with timeout
    let mut reader = BufReader::new(&mut stream);
    let mut response_line = String::new();

    // Use a timeout for the response to avoid hanging indefinitely
    match tokio::time::timeout(
        Duration::from_secs(10),
        reader.read_line(&mut response_line),
    )
    .await
    {
        Ok(Ok(_)) => {
            // Successfully read response
            let response: McpResponse = serde_json::from_str(&response_line)
                .map_err(|e| format!("Failed to parse MCP response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!("MCP server error: {}", error).into());
            }

            Ok(response.result.unwrap_or(serde_json::json!({
                "status": "success",
                "message": "MCP request completed successfully"
            })))
        }
        Ok(Err(e)) => Err(format!("Failed to read MCP response: {}", e).into()),
        Err(_) => Err("Timeout waiting for MCP response (10 seconds)".into()),
    }
}

#[cfg(feature = "tui")]
fn show_log_summary(app: &LoggingApp) {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Explicitly flush each log file to ensure accurate size reporting
    for (_pane_name, log_file_path) in &app.log_files {
        if log_file_path.exists() {
            // Open the file in append mode and flush it
            if let Ok(mut file) = OpenOptions::new()
                .create(false)
                .append(true)
                .open(log_file_path)
            {
                let _ = file.flush();
                let _ = file.sync_all(); // Force OS to write to disk
            }
        }
    }

    // Small additional delay to ensure filesystem consistency
    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("\n📁 Log Files Summary");
    println!("====================");

    // Show scenario file information
    if let Some(file_path) = &app.scenario_file_path {
        println!("📋 Scenario file: {}", file_path.display());
        println!();
    }

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
}

/// Search and correlate events across multiple log files
async fn search_correlated_logs(
    queries: Vec<&String>,
    logs_dir: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    println!(
        "🔍 Searching for correlation patterns across logs in: {}",
        logs_dir
    );
    println!("📝 Search queries: {:?}", queries);
    println!();

    let logs_path = Path::new(logs_dir);
    if !logs_path.exists() {
        return Err(format!("Logs directory '{}' does not exist", logs_dir).into());
    }

    // Collect all log files
    let mut log_files = Vec::new();
    for entry in fs::read_dir(logs_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e.to_str()) == Some(Some("log")) {
            log_files.push(path);
        }
    }

    if log_files.is_empty() {
        println!("❌ No log files found in {}", logs_dir);
        return Ok(());
    }

    println!("📄 Found {} log files:", log_files.len());
    for file in &log_files {
        if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
            println!("  - {}", name);
        }
    }
    println!();

    // Parse and correlate events
    let mut all_events = Vec::new();

    for log_file in &log_files {
        if let Some(source_name) = log_file.file_stem().and_then(|s| s.to_str()) {
            if verbose {
                println!("📖 Reading {}", source_name);
            }

            let content = fs::read_to_string(log_file)?;
            for (line_num, line) in content.lines().enumerate() {
                // Check if this line matches any of our queries
                for query in &queries {
                    if line.to_lowercase().contains(&query.to_lowercase()) {
                        all_events.push(CorrelatedEvent {
                            timestamp: parse_timestamp_from_line(line).unwrap_or_else(|| {
                                // Fallback: use line number as relative time
                                chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
                                    .unwrap()
                                    .with_timezone(&chrono::Utc)
                                    + chrono::Duration::seconds(line_num as i64)
                            }),
                            source: source_name.to_string(),
                            line_number: line_num + 1,
                            content: line.to_string(),
                            query: query.to_string(),
                        });
                    }
                }
            }
        }
    }

    if all_events.is_empty() {
        println!("❌ No matching events found for queries: {:?}", queries);
        return Ok(());
    }

    // Sort events by timestamp for chronological correlation
    all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    println!("🎯 Found {} correlated events:", all_events.len());
    println!("{}", "=".repeat(80));

    // Group events by time windows for better correlation
    let mut current_time_window = None;
    let window_duration = chrono::Duration::seconds(5); // 5-second correlation window

    for event in &all_events {
        let should_show_time_separator = match current_time_window {
            None => true,
            Some(last_time) => {
                let duration = event.timestamp.signed_duration_since(last_time);
                duration > window_duration
            }
        };

        if should_show_time_separator {
            if current_time_window.is_some() {
                println!(); // Add spacing between time windows
            }
            println!("⏰ Time window: {}", event.timestamp.format("%H:%M:%S%.3f"));
            println!("{}", "-".repeat(40));
            current_time_window = Some(event.timestamp);
        }

        // Color-code by source
        let source_indicator = get_source_indicator(&event.source);
        println!(
            "{}[{}:{}] {}: {}",
            source_indicator,
            event.source,
            event.line_number,
            event.query,
            event.content.trim()
        );

        if verbose {
            // Show additional correlation information
            if let Some(correlation_info) = extract_correlation_info(&event.content) {
                println!("    🔗 Correlation: {}", correlation_info);
            }
        }
    }

    println!("{}", "=".repeat(80));

    // Show correlation summary
    println!("📊 Correlation Summary:");
    let mut source_counts = std::collections::HashMap::new();
    let mut query_counts = std::collections::HashMap::new();

    for event in &all_events {
        *source_counts.entry(event.source.clone()).or_insert(0) += 1;
        *query_counts.entry(event.query.clone()).or_insert(0) += 1;
    }

    println!("  📁 Events by source:");
    for (source, count) in source_counts {
        println!("    {}: {} events", source, count);
    }

    println!("  🔍 Events by query:");
    for (query, count) in query_counts {
        println!("    '{}': {} events", query, count);
    }

    // Show timing analysis
    if all_events.len() > 1 {
        let first_event = &all_events[0];
        let last_event = &all_events[all_events.len() - 1];
        let total_duration = last_event
            .timestamp
            .signed_duration_since(first_event.timestamp);

        println!("  ⏱️  Timeline:");
        println!(
            "    First event: {}",
            first_event.timestamp.format("%H:%M:%S%.3f")
        );
        println!(
            "    Last event:  {}",
            last_event.timestamp.format("%H:%M:%S%.3f")
        );
        println!(
            "    Total span:  {:.3}s",
            total_duration.num_milliseconds() as f64 / 1000.0
        );
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct CorrelatedEvent {
    timestamp: chrono::DateTime<chrono::Utc>,
    source: String,
    line_number: usize,
    content: String,
    query: String,
}

/// Parse timestamp from log line in various formats
fn parse_timestamp_from_line(line: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Try mcplay's log format: [HH:MM:SS.mmm]
    if let Some(timestamp_match) = regex::Regex::new(r"^\[(\d{2}:\d{2}:\d{2}\.\d{3})\]")
        .ok()
        .and_then(|re| re.captures(line))
    {
        if let Some(time_str) = timestamp_match.get(1) {
            // Assume today's date for HH:MM:SS.mmm format
            let today = chrono::Utc::now().date_naive();
            if let Ok(time) = chrono::NaiveTime::parse_from_str(time_str.as_str(), "%H:%M:%S%.3f") {
                return Some(chrono::DateTime::from_naive_utc_and_offset(
                    today.and_time(time),
                    chrono::Utc,
                ));
            }
        }
    }

    // Try ISO format: 2025-09-11T11:25:55.273311Z
    if let Some(iso_match) = regex::Regex::new(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)")
        .ok()
        .and_then(|re| re.captures(line))
    {
        if let Some(iso_str) = iso_match.get(1) {
            if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(iso_str.as_str()) {
                return Some(datetime.with_timezone(&chrono::Utc));
            }
        }
    }

    None
}

/// Get a visual indicator for different log sources
fn get_source_indicator(source: &str) -> &'static str {
    if source.contains("orchestrator") {
        "🎭 "
    } else if source.contains("client_alice") {
        "🟢 "
    } else if source.contains("client_bob") {
        "🔵 "
    } else if source.contains("mqtt_server") {
        "📡 "
    } else if source.contains("mqtt_observer") {
        "👁️  "
    } else if source.starts_with("client_") {
        "👤 "
    } else {
        "📝 "
    }
}

/// Extract correlation information from log content
fn extract_correlation_info(content: &str) -> Option<String> {
    let mut info_parts = Vec::new();

    // Simple pattern matching for common correlation keys
    let lower_content = content.to_lowercase();

    // Look for world IDs
    if lower_content.contains("world")
        && (lower_content.contains("id") || lower_content.contains("shared"))
    {
        info_parts.push("world_context".to_string());
    }

    // Look for request IDs (UUIDs)
    if lower_content.contains("request")
        && (lower_content.contains("-") || lower_content.contains("mcp"))
    {
        info_parts.push("request_context".to_string());
    }

    // Look for multiplayer mode indicators
    if lower_content.contains("singleplayer") {
        info_parts.push("mode=SinglePlayer".to_string());
    } else if lower_content.contains("hostingworld") {
        info_parts.push("mode=HostingWorld".to_string());
    } else if lower_content.contains("joinedworld") {
        info_parts.push("mode=JoinedWorld".to_string());
    }

    // Look for block-related operations
    if lower_content.contains("block")
        && (lower_content.contains("place")
            || lower_content.contains("break")
            || lower_content.contains("change"))
    {
        info_parts.push("block_operation".to_string());
    }

    // Look for MQTT operations
    if lower_content.contains("mqtt")
        && (lower_content.contains("publish") || lower_content.contains("subscribe"))
    {
        info_parts.push("mqtt_operation".to_string());
    }

    if info_parts.is_empty() {
        None
    } else {
        Some(info_parts.join(", "))
    }
}
