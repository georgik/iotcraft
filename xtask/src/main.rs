use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use if_addrs;
use qr2term;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
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
    /// Clean up problematic lines that break formatting
    Cleanup {
        /// Only check for issues without fixing them
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
        Commands::WebBuild { release } => {
            build_web(*release)?;
        }
        Commands::WebServe { port } => {
            serve_web(*port)?;
        }
        Commands::Cleanup { check } => {
            cleanup_source_files(*check)?;
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

    // Check for ESP-IDF C projects
    let c_projects = find_c_projects()?;
    if !c_projects.is_empty() {
        if check_only {
            println!(
                "🔍 Also checking C code formatting for {} ESP-IDF projects...",
                c_projects.len()
            );
        } else {
            println!(
                "🎨 Also formatting C code for {} ESP-IDF projects...",
                c_projects.len()
            );
        }
        println!();
    }

    let mut failed_members = Vec::new();
    let mut processed_members = 0;
    let mut failed_c_projects = Vec::new();
    let mut processed_c_projects = 0;

    // Format Rust workspace members
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

    // Format C projects (ESP-IDF)
    for c_project in &c_projects {
        print!("🔧 Processing {} (C)... ", c_project.display());

        let format_result = if check_only {
            format_c_project(c_project, true)
        } else {
            format_c_project(c_project, false)
        };

        match format_result {
            Ok(()) => {
                if check_only {
                    println!("✅ properly formatted");
                } else {
                    println!("✅ formatted successfully");
                }
                processed_c_projects += 1;
            }
            Err(e) => {
                if check_only {
                    println!("❌ formatting issues found: {}", e);
                } else {
                    println!("❌ formatting failed: {}", e);
                }
                failed_c_projects.push(c_project.display().to_string());
            }
        }
    }

    println!();
    println!("📊 Summary:");
    println!("   Rust members processed: {}", processed_members);
    if !c_projects.is_empty() {
        println!("   C projects processed: {}", processed_c_projects);
    }

    let total_failures = failed_members.len() + failed_c_projects.len();

    if total_failures == 0 {
        if check_only {
            println!("   ✅ All projects have proper formatting");
        } else {
            println!("   ✅ All projects formatted successfully");
        }
        println!();
        println!("🎉 Formatting complete! Your code is ready for commit.");
    } else {
        println!("   ❌ Projects with issues: {}", total_failures);

        if !failed_members.is_empty() {
            println!("   Rust members:");
            for member in &failed_members {
                if check_only {
                    println!("      • {} (needs formatting)", member);
                } else {
                    println!("      • {} (failed to format)", member);
                }
            }
        }

        if !failed_c_projects.is_empty() {
            println!("   C projects:");
            for c_project in &failed_c_projects {
                if check_only {
                    println!("      • {} (needs formatting)", c_project);
                } else {
                    println!("      • {} (failed to format)", c_project);
                }
            }
        }
        println!();

        if check_only {
            println!("💡 Run 'cargo xtask format' (without --check) to fix formatting issues.");
            return Err(anyhow::anyhow!(
                "Formatting issues found in {} projects",
                total_failures
            ));
        } else {
            return Err(anyhow::anyhow!(
                "Failed to format {} projects",
                total_failures
            ));
        }
    }

    Ok(())
}

/// Find C projects (ESP-IDF) in the workspace
fn find_c_projects() -> Result<Vec<PathBuf>> {
    let mut c_projects = Vec::new();

    // Common patterns for ESP-IDF projects
    let esp_idf_patterns = ["iotcraft-gateway", "esp32-*"];

    // Look for directories with ESP-IDF project structure
    for entry in fs::read_dir(".").context("Failed to read current directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check if it matches ESP-IDF patterns
            let matches_pattern = esp_idf_patterns.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    dir_name.starts_with(prefix)
                } else {
                    dir_name == *pattern
                }
            });

            if matches_pattern {
                // Verify it's actually an ESP-IDF project
                if is_esp_idf_project(&path) {
                    c_projects.push(path);
                }
            }
        }
    }

    Ok(c_projects)
}

/// Check if a directory is an ESP-IDF project
fn is_esp_idf_project(path: &Path) -> bool {
    // ESP-IDF projects typically have:
    // - CMakeLists.txt in the root
    // - main/ directory with source files
    // - sdkconfig or sdkconfig.defaults files

    let has_cmake = path.join("CMakeLists.txt").exists();
    let has_main_dir = path.join("main").exists();
    let has_sdkconfig = path.join("sdkconfig").exists()
        || path.join("sdkconfig.defaults").exists()
        || path.join("sdkconfig.defaults.esp-box-3").exists();

    has_cmake && has_main_dir && has_sdkconfig
}

/// Format a C project using clang-format
fn format_c_project(project_path: &Path, check_only: bool) -> Result<()> {
    // Check if clang-format is available
    if which::which("clang-format").is_err() {
        return Err(anyhow::anyhow!(
            "clang-format not found. Please install it with: brew install clang-format (macOS) or your package manager"
        ));
    }

    // Create default .clang-format if it doesn't exist
    let clang_format_path = project_path.join(".clang-format");
    if !clang_format_path.exists() {
        create_default_clang_format(&clang_format_path)?;
    }

    // Find all C/C++ files in main/ and components/ directories
    let c_files = find_c_files(project_path)?;

    if c_files.is_empty() {
        return Ok(());
    }

    let mut failed_files = Vec::new();

    for c_file in c_files {
        let mut cmd = Command::new("clang-format");
        cmd.current_dir(project_path);

        if check_only {
            // Check if file needs formatting
            cmd.args(&["--dry-run", "--Werror"]);
        } else {
            // Format in-place
            cmd.arg("-i");
        }

        cmd.arg(&c_file);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to run clang-format on {}", c_file.display()))?;

        if !status.success() {
            failed_files.push(c_file);
        }
    }

    if !failed_files.is_empty() {
        return Err(anyhow::anyhow!(
            "{} files need formatting",
            failed_files.len()
        ));
    }

    Ok(())
}

/// Find all C/C++ source files in an ESP-IDF project
fn find_c_files(project_path: &Path) -> Result<Vec<PathBuf>> {
    let mut c_files = Vec::new();

    // Search in main/ directory
    let main_dir = project_path.join("main");
    if main_dir.exists() {
        find_c_files_in_dir(&main_dir, &mut c_files)?;
    }

    // Search in components/ directory
    let components_dir = project_path.join("components");
    if components_dir.exists() {
        find_c_files_in_dir(&components_dir, &mut c_files)?;
    }

    Ok(c_files)
}

/// Recursively find C/C++ files in a directory
fn find_c_files_in_dir(dir: &Path, c_files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            find_c_files_in_dir(&path, c_files)?;
        } else if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension {
                "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hxx" => {
                    c_files.push(path);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Create a default .clang-format configuration suitable for ESP-IDF projects
fn create_default_clang_format(clang_format_path: &Path) -> Result<()> {
    let config = r#"# ESP-IDF C/C++ Code Style Configuration
# Based on ESP-IDF coding standards and common practices

BasedOnStyle: Google

# Indentation
IndentWidth: 4
TabWidth: 4
UseTab: Never
ContinuationIndentWidth: 8

# Line length
ColumnLimit: 120

# Braces
BreakBeforeBraces: Linux

# Spaces
SpaceBeforeParens: ControlStatements
SpaceInEmptyParentheses: false
SpacesBeforeTrailingComments: 2
SpacesInAngles: false
SpacesInContainerLiterals: false
SpacesInCStyleCastParentheses: false
SpacesInParentheses: false
SpacesInSquareBrackets: false

# Alignment
AlignAfterOpenBracket: Align
AlignConsecutiveAssignments: false
AlignConsecutiveDeclarations: false
AlignOperands: true
AlignTrailingComments: true

# Function declarations
AllowShortFunctionsOnASingleLine: None
AllowShortIfStatementsOnASingleLine: false
AllowShortLoopsOnASingleLine: false

# Include sorting
SortIncludes: true
IncludeBlocks: Preserve

# Other formatting options
KeepEmptyLinesAtTheStartOfBlocks: false
MaxEmptyLinesToKeep: 2
PointerAlignment: Right
"#;

    fs::write(clang_format_path, config)
        .with_context(|| format!("Failed to create {}", clang_format_path.display()))?;

    println!("   📝 Created default .clang-format configuration");
    Ok(())
}

/// Build the web version using wasm-pack
fn build_web(release: bool) -> Result<()> {
    println!("🌐 Building IoTCraft for web (WASM)...");

    // Detect if we're in workspace root or desktop-client directory
    let (_workspace_root, desktop_client_dir) = if Path::new("desktop-client").exists() {
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

/// Clean up problematic lines in source files that can break formatting
fn cleanup_source_files(check_only: bool) -> Result<()> {
    let members = read_workspace_members().context("Failed to read workspace members")?;

    if check_only {
        println!("🔍 Checking for problematic lines in source files...");
    } else {
        println!("🧹 Cleaning up problematic lines in source files...");
    }

    let mut total_issues = 0;
    let mut total_files_checked = 0;

    for member in &members {
        let member_path = Path::new(member);

        // Skip members that don't exist or don't have a src directory
        if !member_path.exists() || !member_path.join("src").exists() {
            continue;
        }

        print!("📦 Processing {}... ", member);

        let src_dir = member_path.join("src");
        let (files_checked, issues_found) = cleanup_rust_files_in_dir(&src_dir, check_only)?;

        total_files_checked += files_checked;
        total_issues += issues_found;

        if issues_found > 0 {
            if check_only {
                println!(
                    "⚠️  {} issues found in {} files",
                    issues_found, files_checked
                );
            } else {
                println!(
                    "✅ {} issues fixed in {} files",
                    issues_found, files_checked
                );
            }
        } else {
            println!("✅ clean ({} files)", files_checked);
        }
    }

    println!();
    println!("📊 Summary:");
    println!("   Files checked: {}", total_files_checked);
    println!("   Issues found: {}", total_issues);

    if total_issues == 0 {
        println!("   ✅ All source files are clean");
    } else if check_only {
        println!("   ⚠️  Run 'cargo xtask cleanup' (without --check) to fix these issues.");
        return Err(anyhow::anyhow!("Found {} problematic lines", total_issues));
    } else {
        println!("   ✅ All issues have been fixed");
    }

    Ok(())
}

/// Recursively clean up Rust files in a directory
fn cleanup_rust_files_in_dir(dir: &Path, check_only: bool) -> Result<(usize, usize)> {
    let mut files_checked = 0;
    let mut total_issues = 0;

    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory {}", dir.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            let (sub_files, sub_issues) = cleanup_rust_files_in_dir(&path, check_only)?;
            files_checked += sub_files;
            total_issues += sub_issues;
        } else if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            if extension == "rs" {
                files_checked += 1;
                let issues = cleanup_rust_file(&path, check_only)?;
                total_issues += issues;
            }
        }
    }

    Ok((files_checked, total_issues))
}

/// Clean up a single Rust file
fn cleanup_rust_file(file_path: &Path, check_only: bool) -> Result<usize> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file {}", file_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut cleaned_lines = Vec::new();
    let mut issues_found = 0;

    for line in lines {
        // Check for problematic patterns that should cause entire line removal
        let should_remove_line =
            // Bell characters
            line.contains('\x07') ||
            // Other control characters that might appear from terminal output
            line.chars().any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r') ||
            // Suspiciously long lines that might be terminal output
            (line.len() > 500 && line.contains("INFO") && line.contains("DEBUG")) ||
            // Lines that look like terminal command artifacts
            line.trim().starts_with("[1]") && line.contains("killed") ||
            line.trim().starts_with("[2]") && line.contains("interrupt");

        // Check for trailing whitespace
        let has_trailing_whitespace = line.len() != line.trim_end().len();

        if should_remove_line {
            issues_found += 1;
            // Skip the problematic line (don't add it to cleaned_lines)
        } else if has_trailing_whitespace {
            issues_found += 1;
            // Remove trailing whitespace but keep the line
            cleaned_lines.push(line.trim_end());
        } else {
            cleaned_lines.push(line);
        }
    }

    // Write the cleaned content if we found issues and we're not in check-only mode
    if issues_found > 0 && !check_only {
        let cleaned_content = cleaned_lines.join("\n");
        // Add final newline if the original content had one
        let final_content = if content.ends_with('\n') {
            format!("{}\n", cleaned_content)
        } else {
            cleaned_content
        };
        if final_content != content {
            fs::write(file_path, &final_content)
                .with_context(|| format!("Failed to write cleaned file {}", file_path.display()))?;
        }
    }

    Ok(issues_found)
}
