//! WASM Performance Limits and Safety Constants
//!
//! This module defines safe performance limits for WASM builds to prevent
//! crashes and ensure acceptable performance across different browsers.
//!
//! See WASM_PERFORMANCE.md for detailed guidelines and implementation requirements.

/// Platform detection constant
#[cfg(target_arch = "wasm32")]
pub const IS_WASM: bool = true;
#[cfg(not(target_arch = "wasm32"))]
pub const IS_WASM: bool = false;

/// Maximum number of visible chunks that can be safely rendered
pub const MAX_VISIBLE_CHUNKS_DESKTOP: usize = 400; // 20x20 chunk grid
pub const MAX_VISIBLE_CHUNKS_WASM: usize = 100; // 10x10 chunk grid

/// Maximum number of blocks per chunk
pub const MAX_BLOCKS_PER_CHUNK_DESKTOP: usize = 4096; // 16x16x16 blocks
pub const MAX_BLOCKS_PER_CHUNK_WASM: usize = 2048; // 16x16x8 or 12x12x12 blocks

/// Render distance in blocks
pub const RENDER_DISTANCE_DESKTOP: f32 = 320.0;
pub const RENDER_DISTANCE_WASM: f32 = 160.0;

/// Maximum number of entities that can be safely managed
pub const MAX_ENTITIES_DESKTOP: usize = 10000;
pub const MAX_ENTITIES_WASM: usize = 3000;

/// Maximum number of concurrent IoT devices
pub const MAX_IOT_DEVICES_DESKTOP: usize = 100;
pub const MAX_IOT_DEVICES_WASM: usize = 25;

/// Maximum number of player avatars in multiplayer
pub const MAX_PLAYER_AVATARS_DESKTOP: usize = 20;
pub const MAX_PLAYER_AVATARS_WASM: usize = 8;

/// Maximum number of dynamic lights
pub const MAX_DYNAMIC_LIGHTS_DESKTOP: usize = 100;
pub const MAX_DYNAMIC_LIGHTS_WASM: usize = 25;

/// Maximum number of particles
pub const MAX_PARTICLES_DESKTOP: usize = 10000;
pub const MAX_PARTICLES_WASM: usize = 2500;

/// Memory usage warning threshold (1.5GB = 75% of 2GB WASM limit)
pub const MEMORY_WARNING_THRESHOLD_BYTES: f64 = 1_500_000_000.0;

/// Critical memory usage threshold (1.8GB = 90% of 2GB WASM limit)
pub const MEMORY_CRITICAL_THRESHOLD_BYTES: f64 = 1_800_000_000.0;

/// Performance warning threshold (30fps = 33.33ms per frame)
pub const FRAME_TIME_WARNING_THRESHOLD: f32 = 0.0333;

/// Critical performance threshold (20fps = 50ms per frame)
pub const FRAME_TIME_CRITICAL_THRESHOLD: f32 = 0.0500;

/// Emergency performance threshold (10fps = 100ms per frame)
pub const FRAME_TIME_EMERGENCY_THRESHOLD: f32 = 0.1000;

/// Performance quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    High,     // Full desktop quality
    Medium,   // Reduced quality for WASM
    Low,      // Aggressive optimization
    Critical, // Emergency mode
}

impl Default for QualityLevel {
    fn default() -> Self {
        if IS_WASM {
            QualityLevel::Medium
        } else {
            QualityLevel::High
        }
    }
}

/// Get maximum visible chunks based on platform
pub fn get_max_visible_chunks() -> usize {
    if IS_WASM {
        MAX_VISIBLE_CHUNKS_WASM
    } else {
        MAX_VISIBLE_CHUNKS_DESKTOP
    }
}

/// Get maximum blocks per chunk based on platform
pub fn get_max_blocks_per_chunk() -> usize {
    if IS_WASM {
        MAX_BLOCKS_PER_CHUNK_WASM
    } else {
        MAX_BLOCKS_PER_CHUNK_DESKTOP
    }
}

/// Get render distance based on platform
pub fn get_render_distance() -> f32 {
    if IS_WASM {
        RENDER_DISTANCE_WASM
    } else {
        RENDER_DISTANCE_DESKTOP
    }
}

/// Get maximum entities based on platform
pub fn get_max_entities() -> usize {
    if IS_WASM {
        MAX_ENTITIES_WASM
    } else {
        MAX_ENTITIES_DESKTOP
    }
}

/// Get maximum IoT devices based on platform
pub fn get_max_iot_devices() -> usize {
    if IS_WASM {
        MAX_IOT_DEVICES_WASM
    } else {
        MAX_IOT_DEVICES_DESKTOP
    }
}

/// Get maximum player avatars based on platform
pub fn get_max_player_avatars() -> usize {
    if IS_WASM {
        MAX_PLAYER_AVATARS_WASM
    } else {
        MAX_PLAYER_AVATARS_DESKTOP
    }
}

/// Get maximum dynamic lights based on platform
pub fn get_max_dynamic_lights() -> usize {
    if IS_WASM {
        MAX_DYNAMIC_LIGHTS_WASM
    } else {
        MAX_DYNAMIC_LIGHTS_DESKTOP
    }
}

/// Get maximum particles based on platform
pub fn get_max_particles() -> usize {
    if IS_WASM {
        MAX_PARTICLES_WASM
    } else {
        MAX_PARTICLES_DESKTOP
    }
}

/// Check if memory usage is within safe limits (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn check_memory_usage() -> Result<(), String> {
    // Note: Memory usage checking in WASM is limited
    // For now, we'll use a conservative approach and assume memory is OK
    // In the future, we can implement more sophisticated memory tracking

    // TODO: Implement proper memory usage checking when web_sys supports it
    // The Performance Memory API is not widely available in web_sys yet

    Ok(()) // Conservative approach - assume memory is OK for now
}

/// Check if memory usage is within safe limits (Desktop - always OK)
#[cfg(not(target_arch = "wasm32"))]
pub fn check_memory_usage() -> Result<(), String> {
    Ok(()) // Desktop has no hard memory limits
}

/// Get quality level based on performance metrics
pub fn get_quality_level_for_performance(avg_frame_time: f32, memory_ok: bool) -> QualityLevel {
    if !memory_ok {
        return QualityLevel::Critical;
    }

    match avg_frame_time {
        t if t > FRAME_TIME_EMERGENCY_THRESHOLD => QualityLevel::Critical,
        t if t > FRAME_TIME_CRITICAL_THRESHOLD => QualityLevel::Low,
        t if t > FRAME_TIME_WARNING_THRESHOLD => QualityLevel::Medium,
        _ => {
            if IS_WASM {
                QualityLevel::Medium
            } else {
                QualityLevel::High
            }
        }
    }
}

/// Browser detection for WASM-specific optimizations
#[cfg(target_arch = "wasm32")]
pub fn get_browser_info() -> (String, f32) {
    // Simplified browser detection that doesn't rely on web_sys
    // For now, return conservative values

    let user_agent = "unknown".to_string();
    let performance_multiplier = 0.8; // Conservative default

    // TODO: Implement proper browser detection when web_sys APIs are stable
    // Can be enhanced later with proper navigator.userAgent access

    (user_agent, performance_multiplier)
}

/// Browser detection stub for desktop
#[cfg(not(target_arch = "wasm32"))]
pub fn get_browser_info() -> (String, f32) {
    ("desktop".to_string(), 1.0)
}

/// Log performance warning
pub fn log_performance_warning(message: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&format!("🚨 WASM Performance: {}", message).into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::warn!("Performance: {}", message);
    }
}

/// Log critical performance issue
pub fn log_performance_critical(message: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::error_1(&format!("💥 WASM Critical: {}", message).into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::error!("Critical Performance: {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_limits() {
        // Ensure WASM limits are always lower than desktop
        assert!(MAX_VISIBLE_CHUNKS_WASM < MAX_VISIBLE_CHUNKS_DESKTOP);
        assert!(MAX_BLOCKS_PER_CHUNK_WASM < MAX_BLOCKS_PER_CHUNK_DESKTOP);
        assert!(RENDER_DISTANCE_WASM < RENDER_DISTANCE_DESKTOP);
        assert!(MAX_ENTITIES_WASM < MAX_ENTITIES_DESKTOP);
        assert!(MAX_IOT_DEVICES_WASM < MAX_IOT_DEVICES_DESKTOP);
        assert!(MAX_PLAYER_AVATARS_WASM < MAX_PLAYER_AVATARS_DESKTOP);
    }

    #[test]
    fn test_memory_thresholds() {
        assert!(MEMORY_WARNING_THRESHOLD_BYTES < MEMORY_CRITICAL_THRESHOLD_BYTES);
        assert!(MEMORY_CRITICAL_THRESHOLD_BYTES < 2_000_000_000.0); // Less than 2GB
    }

    #[test]
    fn test_frame_time_thresholds() {
        assert!(FRAME_TIME_WARNING_THRESHOLD < FRAME_TIME_CRITICAL_THRESHOLD);
        assert!(FRAME_TIME_CRITICAL_THRESHOLD < FRAME_TIME_EMERGENCY_THRESHOLD);
    }

    #[test]
    fn test_quality_level_logic() {
        // High performance should give best quality available
        let quality = get_quality_level_for_performance(0.016, true); // 60fps
        assert!(quality == QualityLevel::High || quality == QualityLevel::Medium);

        // Poor performance should degrade quality
        let quality = get_quality_level_for_performance(0.2, true); // 5fps
        assert_eq!(quality, QualityLevel::Critical);

        // Memory issues should force critical mode
        let quality = get_quality_level_for_performance(0.016, false);
        assert_eq!(quality, QualityLevel::Critical);
    }
}
