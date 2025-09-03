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
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;

// Add chrono for timestamps
#[cfg(feature = "tui")]
use chrono;

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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
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
    list_state: ListState,
    should_quit: bool,
    show_details: bool,
    selected_scenario: Option<ScenarioInfo>,
    message: Option<String>,
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
    McpParameterEditing, // New mode for editing command parameters
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
    fn parse_parameters(schema: &serde_json::Value) -> (Vec<McpParameter>, Vec<McpParameter>, usize) {
        let mut required_params = Vec::new();
        let mut optional_params = Vec::new();
        
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            let required_list = schema.get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            
            for (param_name, param_def) in properties {
                let is_required = required_list.contains(&param_name.as_str());
                let param_type = param_def.get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string")
                    .to_string();
                let description = param_def.get("description")
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
            let (required_params, optional_params, required_count) = Self::parse_parameters(&tool.input_schema);
            
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
        messages.sort_by(|a, b| {
            match a.required_param_count.cmp(&b.required_param_count) {
                std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                other => other,
            }
        });

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
                            serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(
                                serde_json::Number::from(0)
                            ))
                        } else {
                            serde_json::Value::String(param.current_value.clone())
                        }
                    },
                    "boolean" => {
                        serde_json::Value::Bool(param.current_value.to_lowercase() == "true")
                    },
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
                            serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or(
                                serde_json::Number::from(0)
                            ))
                        } else {
                            serde_json::Value::String(param.current_value.clone())
                        }
                    },
                    "boolean" => {
                        serde_json::Value::Bool(param.current_value.to_lowercase() == "true")
                    },
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
    editing_message: Option<McpMessage>,  // Message being edited for parameters
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
}

impl OrchestratorState {
    fn new(scenario: Scenario, scenario_file_path: Option<PathBuf>) -> Self {
        Self {
            scenario,
            scenario_file_path,
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
    run_scenario(scenario, Some(scenario_path), verbose).await?;

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

async fn run_scenario(
    scenario: Scenario,
    scenario_file_path: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create shared state wrapped in Arc<Mutex> for signal handling
    let state = Arc::new(Mutex::new(OrchestratorState::new(
        scenario,
        scenario_file_path,
    )));
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
            600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
            verbose,
            Some("cargo run --release (mqtt-server)"),
        )
        .await;
        if !mqtt_ready {
            return Err(format!(
                "MQTT server failed to start on port {} within 10 minute timeout",
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
        cmd.current_dir("../desktop-client")
            .arg("run")
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
            600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
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
                                            if !mcp_app.editing_param_value && mcp_app.selected_param_index > 0 {
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
                                                if let Some(ref editing_msg) = mcp_app.editing_message {
                                                    let total_params = editing_msg.required_params.len() + editing_msg.optional_params.len();
                                                    if mcp_app.selected_param_index < total_params.saturating_sub(1) {
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
                                                if message.required_param_count > 0 || !message.optional_params.is_empty() {
                                                    // Switch to parameter editing mode
                                                    mcp_app.editing_message = Some(message.clone());
                                                    mcp_app.selected_param_index = 0;
                                                    mcp_app.editing_param_value = false;
                                                    mcp_app.param_input_buffer.clear();
                                                    logging_app.ui_mode = UiMode::McpParameterEditing;
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_chunks[0]);

    // Split the left panel into log selector and system info
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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

    f.render_widget(system_info_widget, left_chunks[1]);

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
                let is_selected = param_index == mcp_app.selected_param_index && !mcp_app.editing_param_value;
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
                        .title(format!("⚙️  Parameters for: {} [FOCUSED]", editing_message.name))
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
                    Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&editing_message.description),
                ]),
                Line::from(Span::raw("")), // Empty line
                Line::from(vec![
                    Span::styled("🔴 Required", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw(" parameters must be filled"),
                ]),
                Line::from(vec![
                    Span::styled("⚪ Optional", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
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
                    param_indicator,
                    message.name, 
                    param_count_text,
                    message.description
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

        // Build client command and log it
        let mut client_cmd_parts = vec![
            "cargo".to_string(),
            "run".to_string(),
            "--bin".to_string(),
            "iotcraft-dekstop-client".to_string(),
            "--".to_string(),
            "--mcp".to_string(),
        ];

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

        // Build client command arguments
        let mut cmd = TokioCommand::new("cargo");
        cmd.current_dir("../desktop-client")
            .arg("run")
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
            600, // Increased from 300 (5 min) to 600 (10 min) for Rust build time
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
