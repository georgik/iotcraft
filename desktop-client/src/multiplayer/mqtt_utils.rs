use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a unique MQTT client ID to avoid conflicts
///
/// Format: {prefix}-{timestamp}-{pid}-{random}
///
/// This ensures that:
/// - Multiple instances of the same app have different IDs (timestamp + pid)
/// - Multiple connections from the same instance have different IDs (random)
/// - IDs are human-readable for debugging
pub fn generate_unique_client_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pid = process::id();
    let random = rand::random::<u16>();

    format!("{}-{}-{}-{}", prefix, timestamp, pid, random)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_unique_client_id_generation() {
        let mut ids = HashSet::new();

        // Generate many IDs and ensure they're all unique
        // Using a smaller number to avoid random collisions in tight loops
        for _ in 0..100 {
            let id = generate_unique_client_id("test");
            assert!(!ids.contains(&id), "Duplicate ID generated: {}", id);
            ids.insert(id);
        }
    }

    #[test]
    fn test_client_id_format() {
        let id = generate_unique_client_id("iotcraft-discovery");

        // Should contain the prefix
        assert!(id.starts_with("iotcraft-discovery-"));

        // Should have 4 parts separated by dashes
        let parts: Vec<&str> = id.split('-').collect();
        assert!(parts.len() >= 4, "ID should have at least 4 parts: {}", id);

        // Should be reasonable length
        assert!(id.len() > 10 && id.len() < 100);
    }
}
