pub fn execute_script(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_script() {
        let script = "# Comment\n    COMMAND1\n\nCOMMAND2\n# Another comment\n COMMAND3\n\n";
        let expected_output = vec!["COMMAND1", "COMMAND2", "COMMAND3"];
        assert_eq!(execute_script(script), expected_output);
    }
}
