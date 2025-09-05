use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Workspace-level build automation for IoTCraft")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format all workspace members
    Format {
        /// Check formatting without modifying files
        #[arg(short, long)]
        check: bool,
    },
}

#[derive(Deserialize)]
struct WorkspaceCargo {
    workspace: Workspace,
}

#[derive(Deserialize)]
struct Workspace {
    members: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Format { check } => {
            format_workspace_members(*check)?;
        }
    }

    Ok(())
}

/// Read workspace members from Cargo.toml
fn read_workspace_members() -> Result<Vec<String>> {
    let cargo_toml_path = Path::new("Cargo.toml");

    if !cargo_toml_path.exists() {
        return Err(anyhow::anyhow!(
            "Cargo.toml not found. Please run this command from the workspace root."
        ));
    }

    let cargo_toml_content =
        std::fs::read_to_string(cargo_toml_path).context("Failed to read Cargo.toml")?;

    let workspace_cargo: WorkspaceCargo =
        toml::from_str(&cargo_toml_content).context("Failed to parse Cargo.toml as TOML")?;

    Ok(workspace_cargo.workspace.members)
}

/// Format all workspace members
fn format_workspace_members(check_only: bool) -> Result<()> {
    let members = read_workspace_members().context("Failed to read workspace members")?;

    if check_only {
        println!("🔍 Checking code formatting for all workspace members...");
    } else {
        println!("🎨 Formatting code for all workspace members...");
    }
    println!("   Found {} workspace members", members.len());
    println!();

    let mut failed_members = Vec::new();
    let mut processed_members = 0;

    for member in &members {
        let member_path = Path::new(member);

        // Skip members that don't exist or don't have a Cargo.toml
        if !member_path.exists() || !member_path.join("Cargo.toml").exists() {
            println!("⏭️  Skipping {}: directory or Cargo.toml not found", member);
            continue;
        }

        print!("📦 Processing {}... ", member);

        let mut cmd = Command::new("cargo");
        cmd.current_dir(member_path).args(&["fmt", "--all", "--"]);

        // Add formatting options
        cmd.args(&["--color", "always"]);

        if check_only {
            cmd.arg("--check");
        }

        let status = cmd
            .status()
            .with_context(|| format!("Failed to execute cargo fmt for {}", member))?;

        if status.success() {
            if check_only {
                println!("✅ properly formatted");
            } else {
                println!("✅ formatted successfully");
            }
            processed_members += 1;
        } else {
            if check_only {
                println!("❌ formatting issues found");
            } else {
                println!("❌ formatting failed");
            }
            failed_members.push(member.clone());
        }
    }

    println!();
    println!("📊 Summary:");
    println!("   Members processed: {}", processed_members);

    if failed_members.is_empty() {
        if check_only {
            println!("   ✅ All members have proper formatting");
        } else {
            println!("   ✅ All members formatted successfully");
        }
        println!();
        println!("🎉 Formatting complete! Your code is ready for commit.");
    } else {
        println!("   ❌ Members with issues: {}", failed_members.len());
        for member in &failed_members {
            if check_only {
                println!("      • {} (needs formatting)", member);
            } else {
                println!("      • {} (failed to format)", member);
            }
        }
        println!();

        if check_only {
            println!("💡 Run 'cargo xtask format' (without --check) to fix formatting issues.");
            return Err(anyhow::anyhow!(
                "Formatting issues found in {} members",
                failed_members.len()
            ));
        } else {
            return Err(anyhow::anyhow!(
                "Failed to format {} members",
                failed_members.len()
            ));
        }
    }

    Ok(())
}
