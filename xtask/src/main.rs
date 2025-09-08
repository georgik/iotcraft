use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use if_addrs;
use qr2term;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::Command;
use which;

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
    /// Build the web version using wasm-pack
    WebBuild {
        /// Build with release optimizations
        #[arg(short, long)]
        release: bool,
    },
    /// Serve the web version locally for testing
    WebServe {
        /// Port to serve on (default: 8000)
        #[arg(short, long, default_value = "8000")]
        port: u16,
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
        Commands::WebBuild { release } => {
            build_web(*release)?;
        }
        Commands::WebServe { port } => {
            serve_web(*port)?;
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

/// Build the web version using wasm-pack
fn build_web(release: bool) -> Result<()> {
    println!("🌐 Building IoTCraft for web (WASM)...");

    // Detect if we're in workspace root or desktop-client directory
    let (workspace_root, desktop_client_dir) = if Path::new("desktop-client").exists() {
        // We're in workspace root
        (
            Path::new(".").to_path_buf(),
            Path::new("desktop-client").to_path_buf(),
        )
    } else if Path::new("Cargo.toml").exists() && Path::new("src").exists() {
        // We're in desktop-client directory
        (Path::new("..").to_path_buf(), Path::new(".").to_path_buf())
    } else {
        return Err(anyhow::anyhow!(
            "Could not detect project structure. Please run this from either the workspace root or desktop-client directory."
        ));
    };

    println!("   Working directory: {}", desktop_client_dir.display());

    // Check if wasm-pack is installed
    if which::which("wasm-pack").is_err() {
        return Err(anyhow::anyhow!(
            "wasm-pack is not installed. Please install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
        ));
    }

    // Build with wasm-pack
    let mut cmd = Command::new("wasm-pack");
    cmd.current_dir(&desktop_client_dir).args(&[
        "build",
        "--target",
        "web",
        "--out-dir",
        "web",
        "--out-name",
        "iotcraft_web",
    ]);

    if release {
        cmd.arg("--release");
        println!("   Using release optimizations");
    } else {
        cmd.arg("--dev");
        println!("   Using development build");
    }

    println!("   Running: wasm-pack build...");
    let status = cmd.status().context("Failed to execute wasm-pack")?;

    if !status.success() {
        return Err(anyhow::anyhow!("wasm-pack build failed"));
    }

    // Copy assets if they exist
    let assets_dir = desktop_client_dir.join("assets");
    if assets_dir.exists() {
        println!("🎨 Copying assets...");
        copy_assets_directory(&desktop_client_dir)?;
    } else {
        println!("   ⚠️  No assets directory found, skipping asset copy");
    }

    // Copy scripts directory for template support
    println!("📂 Copying scripts directory for template support...");
    copy_scripts_directory(&desktop_client_dir)?;

    println!("✅ Web build completed successfully!");
    println!("   Output directory: desktop-client/web/");
    if assets_dir.exists() {
        println!("   Assets available at: desktop-client/web/assets/");
    }
    println!("   Scripts available at: desktop-client/web/scripts/");
    println!("   You can now serve the web version with: cargo xtask web-serve");

    Ok(())
}

/// Copy scripts directory to web output for template support
fn copy_scripts_directory(desktop_client_dir: &Path) -> Result<()> {
    let scripts_src = desktop_client_dir.join("scripts");
    let scripts_dst = desktop_client_dir.join("web").join("scripts");

    if !scripts_src.exists() {
        println!("   ⚠️  Scripts directory not found, skipping...");
        return Ok(());
    }

    // Remove existing scripts directory
    if scripts_dst.exists() {
        fs::remove_dir_all(&scripts_dst).context("Failed to remove existing scripts directory")?;
    }

    // Copy the entire scripts directory
    copy_dir_all(&scripts_src, &scripts_dst).context("Failed to copy scripts directory")?;

    println!("   ✅ Scripts directory copied successfully");
    Ok(())
}

/// Copy assets directory to web output for graphical assets
fn copy_assets_directory(desktop_client_dir: &Path) -> Result<()> {
    let assets_src = desktop_client_dir.join("assets");
    let assets_dst = desktop_client_dir.join("web").join("assets");

    if !assets_src.exists() {
        println!("   ⚠️  Assets directory not found, skipping...");
        return Ok(());
    }

    // Remove existing assets directory
    if assets_dst.exists() {
        fs::remove_dir_all(&assets_dst).context("Failed to remove existing assets directory")?;
    }

    // Copy the entire assets directory
    copy_dir_all(&assets_src, &assets_dst).context("Failed to copy assets directory")?;

    println!("   ✅ Assets directory copied successfully");
    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).context("Failed to create destination directory")?;

    for entry in fs::read_dir(src).context("Failed to read source directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            copy_dir_all(&path, &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&path, &dst.join(entry.file_name())).context("Failed to copy file")?;
        }
    }

    Ok(())
}

/// Serve the web version locally with enhanced mobile support
fn serve_web(port: u16) -> Result<()> {
    println!("🚀 Starting Rust HTTP server...");

    // Detect if we're in workspace root or desktop-client directory
    let web_dir = if Path::new("desktop-client").exists() {
        // We're in workspace root
        Path::new("desktop-client/web").to_path_buf()
    } else if Path::new("web").exists() {
        // We're in desktop-client directory
        Path::new("web").to_path_buf()
    } else {
        return Err(anyhow::anyhow!(
            "Web build not found. Please run 'cargo xtask web-build --release' first."
        ));
    };

    println!(
        "   Serving directory: {}",
        web_dir.file_name().unwrap_or_default().to_string_lossy()
    );
    println!("   Port: {}", port);
    println!();

    // Check if required files exist
    let required_files = ["index.html", "iotcraft_web.js", "iotcraft_web_bg.wasm"];
    for file in &required_files {
        if !web_dir.join(file).exists() {
            return Err(anyhow::anyhow!(
                "Required file {} not found. Please run 'cargo xtask web-build --release' first.",
                file
            ));
        }
    }

    // Get network interfaces and display URLs
    let network_ip = get_network_ip();

    println!("📱 Access URLs:");
    println!("   Local:   http://localhost:{}", port);
    if let Some(ip) = &network_ip {
        println!("   Network: http://{}:{}", ip, port);
    }
    println!();

    // Generate and display QR code for mobile access
    if let Some(ip) = &network_ip {
        let network_url = format!("http://{}:{}", ip, port);
        println!("📱 QR Code for mobile access:");

        match qr2term::print_qr(&network_url) {
            Ok(_) => {
                println!("   Scan the QR code above with your phone's camera");
                println!("   or QR code reader app to open: {}", network_url);
            }
            Err(e) => {
                println!("   ⚠️  Failed to generate QR code: {}", e);
                println!("   Manual URL: {}", network_url);
            }
        }
        println!();
    }

    println!("📁 Serving files from: {}", web_dir.display());
    println!();
    println!("🌟 IoTCraft Web Server is ready!");
    println!("   Listening on 0.0.0.0:{}", port);
    println!("   Press Ctrl+C to stop the server");
    println!();

    // Use Python's built-in HTTP server with enhanced binding
    println!("🟢 Server starting on 0.0.0.0:{}...", port);
    let mut cmd = Command::new("python3");
    cmd.current_dir(&web_dir)
        .args(&["-m", "http.server", &port.to_string(), "--bind", "0.0.0.0"]);

    let status = cmd.status().context("Failed to start HTTP server")?;

    if !status.success() {
        // Fallback to Python 2 if Python 3 fails
        println!("⚠️  Python 3 failed, trying Python 2...");
        let mut cmd = Command::new("python");
        cmd.current_dir(&web_dir)
            .args(&["-m", "SimpleHTTPServer", &port.to_string()]);

        let status = cmd
            .status()
            .context("Failed to start HTTP server with Python 2")?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Failed to start HTTP server. Please ensure Python is installed."
            ));
        }
    }

    println!("💫 Server is running indefinitely - use Ctrl+C to stop");

    Ok(())
}

/// Get the first non-loopback IPv4 address for network access
fn get_network_ip() -> Option<String> {
    if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .filter_map(|iface| match iface.addr {
            if_addrs::IfAddr::V4(addr) if !addr.ip.is_loopback() && !addr.ip.is_link_local() => {
                Some(addr.ip.to_string())
            }
            _ => None,
        })
        .next()
}
