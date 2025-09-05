//! Core MCP types and data structures

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// MCP tool definition
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct McpTool {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON schema for input validation
    #[cfg(feature = "serde")]
    pub input_schema: serde_json::Value,
    #[cfg(not(feature = "serde"))]
    pub input_schema: String,
}

/// MCP tool execution result
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct McpToolResult {
    /// Result content
    pub content: Vec<McpContent>,
    /// Whether this is an error result
    #[cfg_attr(feature = "serde", serde(rename = "isError"))]
    pub is_error: Option<bool>,
}

/// Content types for MCP responses
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum McpContent {
    /// Plain text content
    #[cfg_attr(feature = "serde", serde(rename = "text"))]
    Text { text: String },
    /// JSON content
    #[cfg_attr(feature = "serde", serde(rename = "json"))]
    Json {
        #[cfg(feature = "serde")]
        json: serde_json::Value,
        #[cfg(not(feature = "serde"))]
        json: String,
    },
    /// Image content (base64 encoded)
    #[cfg_attr(feature = "serde", serde(rename = "image"))]
    Image { data: String, mime_type: String },
}

/// MCP error information
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct McpError {
    /// Error code (JSON-RPC style)
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[cfg(feature = "serde")]
    pub data: Option<serde_json::Value>,
    #[cfg(not(feature = "serde"))]
    pub data: Option<String>,
}

/// Common error types
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid tool name: {0}")]
    InvalidToolName(String),
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Network error: {0}")]
    Network(String),
}

/// Coordinates for 3D positioning
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

/// Block types supported in IoTCraft
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
    QuartzBlock,
    GlassPane,
    CyanTerracotta,
    Water,
}

impl BlockType {
    /// Get all supported block types
    pub fn all() -> Vec<BlockType> {
        vec![
            BlockType::Grass,
            BlockType::Dirt,
            BlockType::Stone,
            BlockType::QuartzBlock,
            BlockType::GlassPane,
            BlockType::CyanTerracotta,
            BlockType::Water,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Grass => "grass",
            BlockType::Dirt => "dirt",
            BlockType::Stone => "stone",
            BlockType::QuartzBlock => "quartz_block",
            BlockType::GlassPane => "glass_pane",
            BlockType::CyanTerracotta => "cyan_terracotta",
            BlockType::Water => "water",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> Result<BlockType, ProtocolError> {
        match s {
            "grass" => Ok(BlockType::Grass),
            "dirt" => Ok(BlockType::Dirt),
            "stone" => Ok(BlockType::Stone),
            "quartz_block" => Ok(BlockType::QuartzBlock),
            "glass_pane" => Ok(BlockType::GlassPane),
            "cyan_terracotta" => Ok(BlockType::CyanTerracotta),
            "water" => Ok(BlockType::Water),
            _ => Err(ProtocolError::InvalidParameters(format!(
                "Unknown block type: {}",
                s
            ))),
        }
    }
}

/// Device types supported in IoTCraft
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DeviceType {
    Lamp,
    Door,
}

impl DeviceType {
    /// Get all supported device types
    pub fn all() -> Vec<DeviceType> {
        vec![DeviceType::Lamp, DeviceType::Door]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Lamp => "lamp",
            DeviceType::Door => "door",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> Result<DeviceType, ProtocolError> {
        match s {
            "lamp" => Ok(DeviceType::Lamp),
            "door" => Ok(DeviceType::Door),
            _ => Err(ProtocolError::InvalidParameters(format!(
                "Unknown device type: {}",
                s
            ))),
        }
    }
}

/// Game states for UI transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GameState {
    MainMenu,
    WorldSelection,
    InGame,
    Settings,
    GameplayMenu,
}

impl GameState {
    /// Get all supported game states
    pub fn all() -> Vec<GameState> {
        vec![
            GameState::MainMenu,
            GameState::WorldSelection,
            GameState::InGame,
            GameState::Settings,
            GameState::GameplayMenu,
        ]
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            GameState::MainMenu => "MainMenu",
            GameState::WorldSelection => "WorldSelection",
            GameState::InGame => "InGame",
            GameState::Settings => "Settings",
            GameState::GameplayMenu => "GameplayMenu",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> Result<GameState, ProtocolError> {
        match s {
            "MainMenu" => Ok(GameState::MainMenu),
            "WorldSelection" => Ok(GameState::WorldSelection),
            "InGame" => Ok(GameState::InGame),
            "Settings" => Ok(GameState::Settings),
            "GameplayMenu" => Ok(GameState::GameplayMenu),
            _ => Err(ProtocolError::InvalidParameters(format!(
                "Unknown game state: {}",
                s
            ))),
        }
    }
}
