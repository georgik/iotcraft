//! mcplay - IoTCraft Multi-client Scenario Player
//!
//! This library provides functionality for parsing, validating, and executing
//! multi-client scenarios for IoTCraft testing.

pub mod scenario_types;

pub use scenario_types::*;

/// Validate a scenario structure
pub fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
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
            ));
        }
    }

    // Check dependency references
    let step_names: std::collections::HashSet<_> =
        scenario.steps.iter().map(|s| s.name.as_str()).collect();

    for step in &scenario.steps {
        for dep in &step.depends_on {
            if !step_names.contains(dep.as_str()) {
                return Err(format!(
                    "Step '{}' depends on unknown step '{}'",
                    step.name, dep
                ));
            }
        }
    }

    Ok(())
}

/// Parse a RON scenario string
pub fn parse_ron_scenario(content: &str) -> Result<Scenario, String> {
    ron::from_str(content).map_err(|e| format!("RON parsing error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_scenario() -> Scenario {
        Scenario {
            name: "test_scenario".to_string(),
            description: "A test scenario for unit testing".to_string(),
            version: "1.0".to_string(),
            infrastructure: InfrastructureConfig {
                mqtt_server: MqttServerConfig {
                    required: true,
                    port: 1883,
                    config_file: None,
                    options: None,
                },
                mqtt_observer: Some(MqttObserverConfig {
                    required: true,
                    topics: Some(vec!["test/topic".to_string()]),
                    client_id: Some("test_observer".to_string()),
                }),
                mcp_server: None,
                services: None,
            },
            clients: vec![ClientConfig {
                id: "alice".to_string(),
                player_id: "alice_player".to_string(),
                mcp_port: 8080,
                client_type: "desktop".to_string(),
                name: Some("Alice".to_string()),
                config: None,
            }],
            steps: vec![Step {
                name: "step1".to_string(),
                description: "First test step".to_string(),
                client: "alice".to_string(),
                action: Action::Wait { duration_ms: 1000 },
                wait_before: 0,
                wait_after: 0,
                timeout: 30,
                success_condition: Some(SuccessCondition::AllChecksPassed),
                depends_on: vec![],
                timing: None,
                conditions: None,
                expectations: None,
                response_variables: None,
            }],
            config: None,
        }
    }

    #[test]
    fn test_validate_scenario_success() {
        let scenario = create_test_scenario();
        assert!(validate_scenario(&scenario).is_ok());
    }

    #[test]
    fn test_validate_scenario_empty_steps() {
        let mut scenario = create_test_scenario();
        scenario.steps.clear();

        let result = validate_scenario(&scenario);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have at least one step"));
    }

    #[test]
    fn test_validate_scenario_unknown_client() {
        let mut scenario = create_test_scenario();
        scenario.steps[0].client = "unknown_client".to_string();

        let result = validate_scenario(&scenario);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("references unknown client"));
    }

    #[test]
    fn test_validate_scenario_unknown_dependency() {
        let mut scenario = create_test_scenario();
        scenario.steps[0].depends_on = vec!["unknown_step".to_string()];

        let result = validate_scenario(&scenario);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("depends on unknown step"));
    }

    #[test]
    fn test_validate_scenario_orchestrator_client() {
        let mut scenario = create_test_scenario();
        scenario.steps[0].client = "orchestrator".to_string();

        // Orchestrator steps should be valid even without a client definition
        assert!(validate_scenario(&scenario).is_ok());
    }

    #[test]
    fn test_parse_ron_scenario_valid() {
        let ron_content = r#"(
            name: "test",
            description: "Test scenario",
            version: "1.0",
            infrastructure: (
                mqtt_server: (
                    required: true,
                    port: 1883,
                    config_file: None,
                    options: None,
                ),
                mqtt_observer: Some((
                    required: true,
                    topics: Some(["test/topic"]),
                    client_id: Some("test_observer"),
                )),
                mcp_server: None,
                services: None,
            ),
            clients: [],
            steps: [
                (
                    name: "test_step",
                    description: "Test step",
                    client: "orchestrator",
                    action: (
                        type: "wait",
                        duration_ms: 1000,
                    ),
                    wait_before: 0,
                    wait_after: 0,
                    timeout: 30,
                    success_condition: Some((
                        type: "all_checks_passed",
                    )),
                    depends_on: [],
                    timing: None,
                    conditions: None,
                    expectations: None,
                    response_variables: None,
                )
            ],
            config: None,
        )"#;

        let result = parse_ron_scenario(ron_content);
        if let Err(e) = &result {
            println!("RON parsing error: {}", e);
        }
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.steps.len(), 1);
    }

    #[test]
    fn test_parse_ron_scenario_invalid() {
        let invalid_ron = "invalid ron content {";
        let result = parse_ron_scenario(invalid_ron);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RON parsing error"));
    }
}
