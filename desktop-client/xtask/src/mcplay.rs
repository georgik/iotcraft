/// Run mcplay scenario orchestrator functions
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Run mcplay scenario orchestrator via xtask wrapper
/// This is a simple wrapper around the main mcplay binary for CI integration
pub async fn run_mcplay_command(
    scenario: &Option<String>,
    validate: bool,
    list_scenarios: bool,
    mqtt_port: Option<u16>,
    verbose: bool,
) -> Result<()> {
    // Build mcplay binary first
    let mcplay_binary = build_mcplay_binary(verbose).await?;

    // Prepare command line arguments for mcplay
    let mut args = Vec::new();

    // Handle the scenario file argument
    if let Some(scenario_file) = scenario {
        args.push(scenario_file.clone());
    }

    // Add flags
    if validate {
        args.push("--validate".to_string());
    }
    if list_scenarios {
        args.push("--list-scenarios".to_string());
    }
    if let Some(port) = mqtt_port {
        args.push("--mqtt-port".to_string());
        args.push(port.to_string());
    }
    if verbose {
        args.push("--verbose".to_string());
    }

    println!("🎬 Running mcplay via xtask wrapper...");
    println!("   Binary: {}", mcplay_binary.display());
    if !args.is_empty() {
        println!("   Arguments: {}", args.join(" "));
    }
    println!();

    // Execute mcplay binary
    let mut child = Command::new(&mcplay_binary)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to execute mcplay binary: {}",
                mcplay_binary.display()
            )
        })?;

    let status = child
        .wait()
        .with_context(|| "Failed to wait for mcplay process to finish")?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "mcplay process failed with exit code: {:?}",
            status.code()
        ));
    }

    Ok(())
}

/// Build the mcplay binary if it doesn't exist or is outdated
async fn build_mcplay_binary(verbose: bool) -> Result<std::path::PathBuf> {
    let binary_path = std::path::Path::new("target/release/mcplay");

    // Check if binary exists and if source files are newer
    let should_build = if !binary_path.exists() {
        if verbose {
            println!("🔨 mcplay binary not found, building...");
        }
        true
    } else {
        // Check if any source files are newer than the binary
        let binary_modified = binary_path.metadata()?.modified()?;
        let cargo_toml_modified = std::path::Path::new("Cargo.toml").metadata()?.modified()?;
        let src_modified = get_newest_file_time(std::path::Path::new("src")).await?;

        let needs_rebuild = cargo_toml_modified > binary_modified || src_modified > binary_modified;

        if needs_rebuild && verbose {
            println!("🔨 Source files newer than binary, rebuilding mcplay...");
        }

        needs_rebuild
    };

    if should_build {
        println!("🔨 Building mcplay binary...");

        let status = Command::new("cargo")
            .args(&["build", "--release", "--bin", "mcplay"])
            .status()
            .context("Failed to build mcplay binary")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build mcplay binary"));
        }

        println!("✅ mcplay binary built successfully");
    } else if verbose {
        println!("✅ mcplay binary is up to date");
    }

    Ok(binary_path.to_path_buf())
}

/// Get the newest modification time of any file in a directory recursively
async fn get_newest_file_time(dir: &Path) -> Result<std::time::SystemTime> {
    let mut newest_time = std::time::UNIX_EPOCH;
    get_newest_file_time_recursive(dir, &mut newest_time).await?;
    Ok(newest_time)
}

fn get_newest_file_time_recursive<'a>(
    dir: &'a Path,
    newest_time: &'a mut std::time::SystemTime,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_file() {
                let modified = metadata.modified()?;
                if modified > *newest_time {
                    *newest_time = modified;
                }
            } else if metadata.is_dir() {
                get_newest_file_time_recursive(&path, newest_time).await?;
            }
        }

        Ok(())
    })
}
