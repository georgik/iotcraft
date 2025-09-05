//! Input validation utilities for MCP tools

use crate::types::{BlockType, DeviceType, GameState, Position3D, ProtocolError};

#[cfg(feature = "serde")]
use serde_json::Value;

/// Validate tool parameters
pub struct ToolValidator;

impl ToolValidator {
    /// Validate parameters for any tool by name
    #[cfg(feature = "serde")]
    pub fn validate_tool_params(tool_name: &str, params: &Value) -> Result<(), ProtocolError> {
        match tool_name {
            "ping" => Ok(()), // No parameters to validate
            "place_block" => Self::validate_place_block_params(params),
            "remove_block" => Self::validate_remove_block_params(params),
            "create_wall" => Self::validate_create_wall_params(params),
            "spawn_device" => Self::validate_spawn_device_params(params),
            "control_device" => Self::validate_control_device_params(params),
            "move_device" => Self::validate_move_device_params(params),
            "set_game_state" => Self::validate_set_game_state_params(params),
            "create_world" => Self::validate_create_world_params(params),
            "load_world" => Self::validate_load_world_params(params),
            "join_world" => Self::validate_join_world_params(params),
            "player_move" => Self::validate_player_move_params(params),
            _ => Ok(()), // No validation for other tools yet
        }
    }

    /// Validate place_block parameters
    #[cfg(feature = "serde")]
    fn validate_place_block_params(params: &Value) -> Result<(), ProtocolError> {
        let block_type_str = params
            .get("block_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("block_type is required".to_string())
            })?;

        BlockType::from_str(block_type_str)?;

        Self::validate_coordinates(params)?;
        Ok(())
    }

    /// Validate remove_block parameters
    #[cfg(feature = "serde")]
    fn validate_remove_block_params(params: &Value) -> Result<(), ProtocolError> {
        Self::validate_coordinates(params)
    }

    /// Validate create_wall parameters
    #[cfg(feature = "serde")]
    fn validate_create_wall_params(params: &Value) -> Result<(), ProtocolError> {
        let block_type_str = params
            .get("block_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("block_type is required".to_string())
            })?;

        BlockType::from_str(block_type_str)?;

        // Validate start coordinates
        let _x1 = Self::get_number_param(params, "x1")?;
        let _y1 = Self::get_number_param(params, "y1")?;
        let _z1 = Self::get_number_param(params, "z1")?;

        // Validate end coordinates
        let _x2 = Self::get_number_param(params, "x2")?;
        let _y2 = Self::get_number_param(params, "y2")?;
        let _z2 = Self::get_number_param(params, "z2")?;

        Ok(())
    }

    /// Validate spawn_device parameters
    #[cfg(feature = "serde")]
    fn validate_spawn_device_params(params: &Value) -> Result<(), ProtocolError> {
        let _device_id = params
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("device_id is required".to_string()))?;

        let device_type_str = params
            .get("device_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("device_type is required".to_string())
            })?;

        DeviceType::from_str(device_type_str)?;

        // Coordinates are optional for spawn_device, but validate if present
        if params.get("x").is_some() || params.get("y").is_some() || params.get("z").is_some() {
            Self::validate_optional_coordinates(params)?;
        }

        Ok(())
    }

    /// Validate control_device parameters
    #[cfg(feature = "serde")]
    fn validate_control_device_params(params: &Value) -> Result<(), ProtocolError> {
        let _device_id = params
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("device_id is required".to_string()))?;

        let _command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("command is required".to_string()))?;

        Ok(())
    }

    /// Validate move_device parameters
    #[cfg(feature = "serde")]
    fn validate_move_device_params(params: &Value) -> Result<(), ProtocolError> {
        let _device_id = params
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("device_id is required".to_string()))?;

        Self::validate_coordinates(params)
    }

    /// Validate set_game_state parameters
    #[cfg(feature = "serde")]
    fn validate_set_game_state_params(params: &Value) -> Result<(), ProtocolError> {
        let state_str = params
            .get("state")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("state is required".to_string()))?;

        GameState::from_str(state_str)?;
        Ok(())
    }

    /// Validate create_world parameters
    #[cfg(feature = "serde")]
    fn validate_create_world_params(params: &Value) -> Result<(), ProtocolError> {
        let _world_name = params
            .get("world_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("world_name is required".to_string())
            })?;

        // Description is optional
        Ok(())
    }

    /// Validate load_world parameters
    #[cfg(feature = "serde")]
    fn validate_load_world_params(params: &Value) -> Result<(), ProtocolError> {
        let _world_name = params
            .get("world_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProtocolError::InvalidParameters("world_name is required".to_string())
            })?;

        Ok(())
    }

    /// Validate join_world parameters
    #[cfg(feature = "serde")]
    fn validate_join_world_params(params: &Value) -> Result<(), ProtocolError> {
        let _world_id = params
            .get("world_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidParameters("world_id is required".to_string()))?;

        Ok(())
    }

    /// Validate player_move parameters
    #[cfg(feature = "serde")]
    fn validate_player_move_params(params: &Value) -> Result<(), ProtocolError> {
        Self::validate_coordinates(params)
    }

    /// Validate required coordinates (x, y, z)
    #[cfg(feature = "serde")]
    fn validate_coordinates(params: &Value) -> Result<(), ProtocolError> {
        let _x = Self::get_number_param(params, "x")?;
        let _y = Self::get_number_param(params, "y")?;
        let _z = Self::get_number_param(params, "z")?;
        Ok(())
    }

    /// Validate optional coordinates (x, y, z may not be present)
    #[cfg(feature = "serde")]
    fn validate_optional_coordinates(params: &Value) -> Result<(), ProtocolError> {
        if let Some(x_val) = params.get("x") {
            let _x = x_val.as_f64().ok_or_else(|| {
                ProtocolError::InvalidParameters("x must be a number".to_string())
            })?;
        }

        if let Some(y_val) = params.get("y") {
            let _y = y_val.as_f64().ok_or_else(|| {
                ProtocolError::InvalidParameters("y must be a number".to_string())
            })?;
        }

        if let Some(z_val) = params.get("z") {
            let _z = z_val.as_f64().ok_or_else(|| {
                ProtocolError::InvalidParameters("z must be a number".to_string())
            })?;
        }

        Ok(())
    }

    /// Helper to get a required number parameter
    #[cfg(feature = "serde")]
    fn get_number_param(params: &Value, name: &str) -> Result<f64, ProtocolError> {
        params
            .get(name)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::InvalidParameters(format!("{} must be a number", name)))
    }
}

/// Validate position bounds (optional utility)
pub fn validate_position_bounds(pos: &Position3D, max_coord: f64) -> Result<(), ProtocolError> {
    if pos.x.abs() > max_coord || pos.y.abs() > max_coord || pos.z.abs() > max_coord {
        return Err(ProtocolError::InvalidParameters(format!(
            "Coordinates must be within ±{} bounds",
            max_coord
        )));
    }
    Ok(())
}

/// Validate device ID format
pub fn validate_device_id(device_id: &str) -> Result<(), ProtocolError> {
    if device_id.is_empty() {
        return Err(ProtocolError::InvalidParameters(
            "Device ID cannot be empty".to_string(),
        ));
    }

    if device_id.len() > 64 {
        return Err(ProtocolError::InvalidParameters(
            "Device ID cannot be longer than 64 characters".to_string(),
        ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !device_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ProtocolError::InvalidParameters(
            "Device ID can only contain alphanumeric characters, underscores, and hyphens"
                .to_string(),
        ));
    }

    Ok(())
}

/// Validate world name format
pub fn validate_world_name(world_name: &str) -> Result<(), ProtocolError> {
    if world_name.is_empty() {
        return Err(ProtocolError::InvalidParameters(
            "World name cannot be empty".to_string(),
        ));
    }

    if world_name.len() > 100 {
        return Err(ProtocolError::InvalidParameters(
            "World name cannot be longer than 100 characters".to_string(),
        ));
    }

    // Check for forbidden characters that might cause filesystem issues
    let forbidden_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if world_name.chars().any(|c| forbidden_chars.contains(&c)) {
        return Err(ProtocolError::InvalidParameters(
            "World name contains forbidden characters".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_device_id() {
        // Valid device IDs
        assert!(validate_device_id("lamp_01").is_ok());
        assert!(validate_device_id("door-main").is_ok());
        assert!(validate_device_id("device123").is_ok());

        // Invalid device IDs
        assert!(validate_device_id("").is_err());
        assert!(validate_device_id("device with spaces").is_err());
        assert!(validate_device_id("device@invalid").is_err());
    }

    #[test]
    fn test_validate_world_name() {
        // Valid world names
        assert!(validate_world_name("MyWorld").is_ok());
        assert!(validate_world_name("Test World 123").is_ok());

        // Invalid world names
        assert!(validate_world_name("").is_err());
        assert!(validate_world_name("world/with/slashes").is_err());
        assert!(validate_world_name("world:with:colons").is_err());
    }

    #[test]
    fn test_validate_position_bounds() {
        let valid_pos = Position3D::new(10.0, 20.0, 30.0);
        assert!(validate_position_bounds(&valid_pos, 100.0).is_ok());

        let invalid_pos = Position3D::new(200.0, 20.0, 30.0);
        assert!(validate_position_bounds(&invalid_pos, 100.0).is_err());
    }
}
