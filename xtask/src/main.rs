use anyhow::{Context, Result};
use chrono;
use clap::{Parser, Subcommand};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use if_addrs;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use qr2term;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use which;

#[derive(clap::ValueEnum, Clone, Debug)]
enum TestMode {
    /// Run all test types (unit, integration, scenario)
    All,
    /// Run only unit tests
    Unit,
    /// Run only integration tests
    Integration,
    /// Run scenario-based tests with mcplay
    Scenario,
    /// Run WASM-specific tests
    Wasm,
    /// Check test compilation without running
    Check,
}

#[derive(Subcommand)]
enum GithubAction {
    /// List recent workflow runs
    Runs {
        /// Number of runs to show (default: 10)
        #[arg(short, long, default_value = "10")]
        limit: u32,
        /// Filter by workflow name
        #[arg(short, long)]
        workflow: Option<String>,
        /// Filter by branch
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// Watch the latest workflow run in real-time
    Watch {
        /// Workflow run ID to watch (defaults to latest)
        #[arg(short, long)]
        run_id: Option<String>,
        /// Refresh interval in seconds (default: 60)
        #[arg(long, default_value = "60")]
        interval: u64,
    },
    /// Show detailed status of a workflow run
    Status {
        /// Workflow run ID (defaults to latest)
        #[arg(short, long)]
        run_id: Option<String>,
        /// Show logs for failed jobs
        #[arg(short, long)]
        logs: bool,
    },
    /// Trigger a workflow manually
    Trigger {
        /// Workflow name or file name
        #[arg(short, long)]
        workflow: Option<String>,
        /// Branch to run on (default: current branch)
        #[arg(short, long)]
        branch: Option<String>,
        /// List available workflows instead of triggering
        #[arg(short, long)]
        list: bool,
    },
}

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Workspace-level build automation for IoTCraft")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run comprehensive tests across workspace members
    Test {
        /// Test mode to run
        #[arg(short, long, default_value = "all")]
        mode: TestMode,
        /// Generate test reports (JSON/HTML format)
        #[arg(long)]
        report: bool,
        /// Use nextest instead of cargo test
        #[arg(long)]
        nextest: bool,
        /// Generate code coverage report
        #[arg(long)]
        coverage: bool,
        /// Output directory for test results
        #[arg(short, long, default_value = "test-results")]
        output: String,
        /// Specific component to test (desktop-client, mcplay, etc.)
        #[arg(long)]
        component: Option<String>,
        /// Run tests with virtual display for GUI testing
        #[arg(long)]
        virtual_display: bool,
        /// Enable verbose test output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Format all workspace members (Rust code and HTML)
    Format {
        /// Check formatting without modifying files
        #[arg(short, long)]
        check: bool,
        /// Format HTML files in addition to Rust code
        #[arg(long)]
        html: bool,
        /// Path to HTML files or directory (for HTML formatting)
        #[arg(long, default_value = "web")]
        html_path: String,
    },
    /// Build the web version using wasm-pack
    WebBuild {
        /// Build with release optimizations
        #[arg(short, long)]
        release: bool,
        /// Output directory for the web build
        #[arg(short, long, default_value = "web")]
        output: String,
    },
    /// Serve the web version locally for testing
    WebServe {
        /// Port to serve on (default: 8000)
        #[arg(short, long, default_value = "8000")]
        port: u16,
        /// Directory to serve from
        #[arg(short, long, default_value = "web")]
        dir: String,
    },
    /// Build and serve the web version
    WebDev {
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
    /// GitHub CLI integration for CI/CD workflow monitoring
    Github {
        #[command(subcommand)]
        action: GithubAction,
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
        Commands::Test {
            mode,
            report,
            nextest,
            coverage,
            output,
            component,
            virtual_display,
            verbose,
        } => {
            run_tests(
                mode,
                *report,
                *nextest,
                *coverage,
                output,
                component.as_deref(),
                *virtual_display,
                *verbose,
            )?;
        }
        Commands::Format {
            check,
            html,
            html_path,
        } => {
            format_workspace_members(*check, *html, html_path)?;
        }
        Commands::WebBuild { release, output } => {
            build_web(*release, output)?;
        }
        Commands::WebServe { port, dir } => {
            serve_web(*port, dir)?;
        }
        Commands::WebDev { port } => {
            build_web(false, "web")?;
            serve_web(*port, "web")?;
        }
        Commands::Cleanup { check } => {
            cleanup_source_files(*check)?;
        }
        Commands::Github { action } => {
            handle_github_action(action)?;
        }
    }

    Ok(())
}

/// Run comprehensive tests across workspace components
fn run_tests(
    mode: &TestMode,
    generate_reports: bool,
    use_nextest: bool,
    generate_coverage: bool,
    output_dir: &str,
    component: Option<&str>,
    virtual_display: bool,
    verbose: bool,
) -> Result<()> {
    println!("🧪 Running comprehensive tests...");
    println!("Mode: {:?}", mode);

    if generate_reports {
        println!("Test reports will be saved to: {}", output_dir);
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create output directory: {}", output_dir))?;
    }

    if use_nextest {
        check_nextest_installed()?;
        println!("⚡ Using nextest for faster test execution");
    }

    if generate_coverage {
        check_coverage_tools()?;
        println!("📊 Code coverage analysis enabled");
    }

    if virtual_display {
        println!("🖥️ Virtual display mode enabled for GUI testing");
        setup_virtual_display()?;
    }

    let components = if let Some(comp) = component {
        vec![comp.to_string()]
    } else {
        vec!["desktop-client".to_string(), "mcplay".to_string()]
    };

    let mut failed_components = Vec::new();
    let mut total_tests_run = 0;

    for comp in &components {
        println!();
        println!("📦 Testing component: {}", comp);
        println!("{}=", "=".repeat(50 + comp.len()));

        let component_result = match comp.as_str() {
            "desktop-client" => run_desktop_client_tests(
                mode,
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                virtual_display,
                verbose,
            ),
            "mcplay" => run_mcplay_tests(
                mode,
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                verbose,
            ),
            _ => {
                println!("⚠️ Unknown component: {}, skipping...", comp);
                continue;
            }
        };

        match component_result {
            Ok(tests_run) => {
                println!("✅ {} tests completed successfully", comp);
                total_tests_run += tests_run;
            }
            Err(e) => {
                println!("❌ {} tests failed: {}", comp, e);
                failed_components.push(comp.clone());
            }
        }
    }

    println!();
    println!("📊 Test Summary:");
    println!("=================");
    println!("Total test suites run: {}", total_tests_run);
    println!("Components tested: {}", components.len());
    println!("Successful: {}", components.len() - failed_components.len());
    println!("Failed: {}", failed_components.len());

    if generate_reports {
        println!("📊 Reports saved to: {}/", output_dir);
    }

    if failed_components.is_empty() {
        println!();
        println!("🎉 All tests passed! Your code is ready for commit.");
        Ok(())
    } else {
        println!();
        println!("❌ Failed components:");
        for comp in &failed_components {
            println!("  • {}", comp);
        }
        Err(anyhow::anyhow!(
            "Tests failed in {} components",
            failed_components.len()
        ))
    }
}

/// Check if nextest is installed
fn check_nextest_installed() -> Result<()> {
    if which::which("cargo-nextest").is_err() {
        println!("⚠️ nextest not found. Installing...");
        let status = Command::new("cargo")
            .args(&["install", "cargo-nextest", "--locked"])
            .status()
            .context("Failed to install cargo-nextest")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to install cargo-nextest"));
        }
        println!("✅ nextest installed successfully");
    }
    Ok(())
}

/// Check if coverage tools are available
fn check_coverage_tools() -> Result<()> {
    // Check for cargo-llvm-cov first (preferred)
    if which::which("cargo-llvm-cov").is_err() {
        println!("⚠️ cargo-llvm-cov not found. Installing...");
        let status = Command::new("cargo")
            .args(&["install", "cargo-llvm-cov"])
            .status()
            .context("Failed to install cargo-llvm-cov")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to install cargo-llvm-cov"));
        }
        println!("✅ cargo-llvm-cov installed successfully");
    }
    Ok(())
}

/// Setup virtual display for GUI testing
fn setup_virtual_display() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Check if Xvfb is available
        if which::which("Xvfb").is_err() {
            return Err(anyhow::anyhow!(
                "Xvfb not found. Install it with: sudo apt-get install xvfb"
            ));
        }

        // Set up virtual display environment
        std::env::set_var("DISPLAY", ":99");

        // Start Xvfb in background
        let _xvfb = Command::new("Xvfb")
            .args(&[":99", "-screen", "0", "1024x768x24"])
            .spawn()
            .context("Failed to start Xvfb")?;

        // Wait a moment for Xvfb to start
        std::thread::sleep(std::time::Duration::from_secs(2));

        println!("✅ Virtual display set up on :99");
    }

    #[cfg(target_os = "macos")]
    {
        println!("⚠️ Virtual display on macOS - using headless mode");
        // On macOS, we'll rely on Bevy's headless rendering
        std::env::set_var("RUN_WGPU_BACKEND", "vulkan");
    }

    #[cfg(target_os = "windows")]
    {
        println!("⚠️ Virtual display on Windows - using headless mode");
        // On Windows, we'll rely on Bevy's headless rendering
    }

    Ok(())
}

/// Run desktop-client tests
fn run_desktop_client_tests(
    mode: &TestMode,
    generate_reports: bool,
    use_nextest: bool,
    generate_coverage: bool,
    output_dir: &str,
    virtual_display: bool,
    verbose: bool,
) -> Result<usize> {
    let desktop_client_path = Path::new("desktop-client");

    if !desktop_client_path.exists() {
        return Err(anyhow::anyhow!("desktop-client directory not found"));
    }

    let mut tests_run = 0;

    match mode {
        TestMode::All => {
            tests_run += run_rust_tests(
                desktop_client_path,
                "unit",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                virtual_display,
                verbose,
            )?;
            tests_run += run_rust_tests(
                desktop_client_path,
                "integration",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                virtual_display,
                verbose,
            )?;
            tests_run +=
                run_wasm_tests(desktop_client_path, generate_reports, output_dir, verbose)?;
        }
        TestMode::Unit => {
            tests_run += run_rust_tests(
                desktop_client_path,
                "unit",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                virtual_display,
                verbose,
            )?;
        }
        TestMode::Integration => {
            tests_run += run_rust_tests(
                desktop_client_path,
                "integration",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                virtual_display,
                verbose,
            )?;
        }
        TestMode::Wasm => {
            tests_run +=
                run_wasm_tests(desktop_client_path, generate_reports, output_dir, verbose)?;
        }
        TestMode::Check => {
            tests_run += check_compilation(desktop_client_path, verbose)?;
        }
        TestMode::Scenario => {
            println!("⚠️ Scenario tests for desktop-client are handled by mcplay component");
        }
    }

    Ok(tests_run)
}

/// Run mcplay tests
fn run_mcplay_tests(
    mode: &TestMode,
    generate_reports: bool,
    use_nextest: bool,
    generate_coverage: bool,
    output_dir: &str,
    verbose: bool,
) -> Result<usize> {
    let mcplay_path = Path::new("mcplay");

    if !mcplay_path.exists() {
        return Err(anyhow::anyhow!("mcplay directory not found"));
    }

    let mut tests_run = 0;

    match mode {
        TestMode::All => {
            tests_run += run_rust_tests(
                mcplay_path,
                "unit",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                false,
                verbose,
            )?;
            tests_run += run_scenario_tests(mcplay_path, verbose)?;
        }
        TestMode::Unit => {
            tests_run += run_rust_tests(
                mcplay_path,
                "unit",
                generate_reports,
                use_nextest,
                generate_coverage,
                output_dir,
                false,
                verbose,
            )?;
        }
        TestMode::Scenario => {
            tests_run += run_scenario_tests(mcplay_path, verbose)?;
        }
        TestMode::Check => {
            tests_run += check_compilation(mcplay_path, verbose)?;
        }
        TestMode::Integration | TestMode::Wasm => {
            println!("⚠️ {:?} tests not applicable for mcplay, skipping...", mode);
        }
    }

    Ok(tests_run)
}

/// Run Rust tests (unit or integration)
fn run_rust_tests(
    component_path: &Path,
    test_type: &str,
    generate_reports: bool,
    use_nextest: bool,
    generate_coverage: bool,
    output_dir: &str,
    virtual_display: bool,
    verbose: bool,
) -> Result<usize> {
    println!("🧪 Running {} tests...", test_type);

    let mut cmd = if generate_coverage {
        let mut coverage_cmd = Command::new("cargo");
        coverage_cmd.current_dir(component_path).args(&["llvm-cov"]);

        if use_nextest {
            coverage_cmd.args(&["nextest"]);
        } else {
            coverage_cmd.args(&["test"]);
        }

        coverage_cmd
    } else if use_nextest {
        let mut nextest_cmd = Command::new("cargo");
        nextest_cmd
            .current_dir(component_path)
            .args(&["nextest", "run"]);
        nextest_cmd
    } else {
        let mut test_cmd = Command::new("cargo");
        test_cmd.current_dir(component_path).args(&["test"]);
        test_cmd
    };

    // Add test-specific arguments
    match test_type {
        "unit" => {
            cmd.args(&["--lib", "--bins"]);
        }
        "integration" => {
            cmd.args(&["--tests"]);
        }
        _ => {}
    }

    if verbose {
        cmd.args(&["--verbose"]);
    }

    // Add environment variables for GUI testing
    if virtual_display {
        cmd.env("RUST_LOG", "debug")
            .env("BEVY_DISABLE_AUDIO", "1")
            .env("WGPU_BACKEND", "vulkan");
    }

    // Add report generation
    if generate_reports {
        if use_nextest {
            let report_path = format!("{}/{}-nextest-report.xml", output_dir, test_type);
            cmd.args(&["--junit-output", &report_path]);
        } else {
            let json_path = format!("{}/{}-results.json", output_dir, test_type);
            cmd.args(&["--", "--format", "json", "--show-output"])
                .stdout(
                    std::fs::File::create(&json_path)
                        .context("Failed to create JSON output file")?,
                );
        }
    }

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run {} tests", test_type))?;

    if status.success() {
        println!("✅ {} tests passed", test_type);
        Ok(1)
    } else {
        Err(anyhow::anyhow!("{} tests failed", test_type))
    }
}

/// Run WASM-specific tests
fn run_wasm_tests(
    component_path: &Path,
    _generate_reports: bool,
    _output_dir: &str,
    verbose: bool,
) -> Result<usize> {
    println!("🕸️ Running WASM tests...");

    // Check WASM target is installed
    let target_status = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()
        .context("Failed to list installed targets")?;

    let targets = String::from_utf8_lossy(&target_status.stdout);
    if !targets.contains("wasm32-unknown-unknown") {
        println!("Installing WASM target...");
        let install_status = Command::new("rustup")
            .args(&["target", "add", "wasm32-unknown-unknown"])
            .status()
            .context("Failed to install WASM target")?;

        if !install_status.success() {
            return Err(anyhow::anyhow!("Failed to install WASM target"));
        }
    }

    // Run WASM compilation check
    let mut cmd = Command::new("cargo");
    cmd.current_dir(component_path)
        .args(&["test", "--target", "wasm32-unknown-unknown"]);

    if verbose {
        cmd.args(&["--verbose"]);
    }

    let status = cmd.status().context("Failed to run WASM tests")?;

    if status.success() {
        println!("✅ WASM tests passed");
        Ok(1)
    } else {
        Err(anyhow::anyhow!("WASM tests failed"))
    }
}

/// Run scenario-based tests with mcplay
fn run_scenario_tests(mcplay_path: &Path, verbose: bool) -> Result<usize> {
    println!("🎭 Running scenario tests...");

    // First validate all scenarios
    let mut cmd = Command::new("cargo");
    cmd.current_dir(mcplay_path)
        .args(&["run", "--", "--validate-all"]);

    if verbose {
        cmd.args(&["--verbose"]);
    }

    let status = cmd.status().context("Failed to validate scenarios")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Scenario validation failed"));
    }

    println!("✅ All scenarios are valid");

    // Run a quick scenario test
    println!("Running quick scenario test...");
    let test_cmd = Command::new("cargo")
        .current_dir(mcplay_path)
        .args(&["run", "scenarios/comprehensive_fast_test.ron"])
        .status()
        .context("Failed to run test scenario")?;

    if test_cmd.success() {
        println!("✅ Scenario tests passed");
        Ok(1)
    } else {
        Err(anyhow::anyhow!("Scenario tests failed"))
    }
}

/// Check compilation without running tests
fn check_compilation(component_path: &Path, verbose: bool) -> Result<usize> {
    println!("🔧 Checking compilation...");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(component_path)
        .args(&["check", "--all-targets"]);

    if verbose {
        cmd.args(&["--verbose"]);
    }

    let status = cmd.status().context("Failed to check compilation")?;

    if status.success() {
        println!("✅ Compilation check passed");
        Ok(1)
    } else {
        Err(anyhow::anyhow!("Compilation check failed"))
    }
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
fn format_workspace_members(check_only: bool, include_html: bool, html_path: &str) -> Result<()> {
    let members = read_workspace_members().context("Failed to read workspace members")?;

    if check_only {
        println!("🔍 Checking code formatting for all workspace members...");
        if include_html {
            println!("   Including HTML formatting checks in: {}", html_path);
        }
    } else {
        println!("🎨 Formatting code for all workspace members...");
        if include_html {
            println!("   Including HTML formatting in: {}", html_path);
        }
    }
    println!("   Found {} workspace members", members.len());
    println!();

    // Check for ESP-IDF C projects (DISABLED temporarily)
    let c_projects = find_c_projects()?;
    if !c_projects.is_empty() {
        if check_only {
            println!(
                "⏭️  Skipping C code formatting check for {} ESP-IDF projects (disabled)...",
                c_projects.len()
            );
        } else {
            println!(
                "⏭️  Skipping C code formatting for {} ESP-IDF projects (disabled)...",
                c_projects.len()
            );
        }
        println!();
    }

    let mut failed_members = Vec::new();
    let mut processed_members = 0;
    let mut failed_c_projects: Vec<String> = Vec::new(); // Explicitly typed (unused when C formatting disabled)
    let mut processed_c_projects = 0;
    let mut failed_html_files = 0;
    let mut processed_html_files = 0;

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

    // Format C projects (ESP-IDF) - DISABLED temporarily
    // TODO: Re-enable when C files are stable and always present
    /*
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
    */

    // Format HTML files if requested
    if include_html {
        print!("🌐 Processing HTML files in {}... ", html_path);
        match format_html_files(html_path, check_only) {
            Ok((processed, failed)) => {
                processed_html_files = processed;
                failed_html_files = failed;
                if failed == 0 {
                    if check_only {
                        println!("✅ {} files properly formatted", processed);
                    } else {
                        println!("✅ {} files formatted successfully", processed);
                    }
                } else {
                    if check_only {
                        println!(
                            "❌ {} files have formatting issues out of {}",
                            failed, processed
                        );
                    } else {
                        println!("❌ {} files failed to format out of {}", failed, processed);
                    }
                }
            }
            Err(e) => {
                println!("❌ HTML formatting failed: {}", e);
                failed_html_files = 1;
            }
        }
    }

    println!();
    println!("📊 Summary:");
    println!("   Rust members processed: {}", processed_members);
    if !c_projects.is_empty() {
        println!("   C projects processed: {}", processed_c_projects);
    }
    if include_html {
        println!("   HTML files processed: {}", processed_html_files);
    }

    let total_failures = failed_members.len() + failed_c_projects.len() + failed_html_files;

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
fn build_web(release: bool, output_dir: &str) -> Result<()> {
    println!("🔨 Building IoTCraft for web (WASM)...");
    println!("   Output directory: {}", output_dir);

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
fn serve_web(port: u16, serve_dir_name: &str) -> Result<()> {
    println!("🚀 Starting HTTP server...");
    println!("   Serving directory: {}", serve_dir_name);
    println!("   Port: {}", port);

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

/// Format HTML files in a directory
fn format_html_files(path_str: &str, check_only: bool) -> Result<(usize, usize)> {
    let path = Path::new(path_str);

    let html_files = if path.is_file()
        && path
            .extension()
            .map_or(false, |ext| ext == "html" || ext == "htm")
    {
        vec![path.to_path_buf()]
    } else if path.is_dir() {
        find_html_files(path)?
    } else {
        return Ok((0, 0)); // No HTML files found
    };

    if html_files.is_empty() {
        return Ok((0, 0));
    }

    let mut files_processed = 0;
    let mut files_failed = 0;

    for html_file in html_files {
        match process_html_file(&html_file, check_only) {
            Ok(changed) => {
                files_processed += 1;
                if changed && !check_only {
                    // File was changed during formatting
                }
            }
            Err(_) => {
                files_failed += 1;
            }
        }
    }

    Ok((files_processed, files_failed))
}

/// Find all HTML files in a directory recursively
fn find_html_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut html_files = Vec::new();
    find_html_files_recursive(dir, &mut html_files)?;
    Ok(html_files)
}

fn find_html_files_recursive(dir: &Path, html_files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir() {
            find_html_files_recursive(&path, html_files)?;
        } else if let Some(ext) = path.extension() {
            if ext == "html" || ext == "htm" {
                html_files.push(path);
            }
        }
    }
    Ok(())
}

/// Process a single HTML file with pure Rust HTML5 parser
fn process_html_file(file_path: &Path, check_only: bool) -> Result<bool> {
    let original_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Format the HTML using our pure Rust formatter
    let formatted_content = format_html_content(&original_content)
        .with_context(|| format!("Failed to format HTML file: {}", file_path.display()))?;

    let changed = original_content != formatted_content;

    if changed && !check_only {
        fs::write(file_path, formatted_content)
            .with_context(|| format!("Failed to write formatted file: {}", file_path.display()))?;
    }

    Ok(changed)
}

/// Pure Rust HTML formatter using html5ever
fn format_html_content(content: &str) -> Result<String> {
    // Parse the HTML document
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut content.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to parse HTML: {}", e))?;

    // Format the DOM back to HTML
    let mut output = Vec::new();
    serialize_node(&dom.document, &mut output, 0)?;

    let formatted = String::from_utf8(output)
        .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in formatted HTML: {}", e))?;

    Ok(formatted)
}

/// Serialize an HTML node to properly formatted output
fn serialize_node(handle: &Handle, output: &mut Vec<u8>, indent_level: usize) -> Result<()> {
    let indent = "    ".repeat(indent_level); // 4 spaces per indent level

    match &handle.data {
        NodeData::Document => {
            // Process children without adding any content for the document node
            for child in handle.children.borrow().iter() {
                serialize_node(child, output, indent_level)?
            }
        }
        NodeData::Doctype { name, .. } => writeln!(output, "<!DOCTYPE {}>", name)
            .map_err(|e| anyhow::anyhow!("Write error: {}", e))?,
        NodeData::Text { contents } => {
            let text = contents.borrow();
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                // Only add indentation if we're not already on a line with content
                if output.last().map_or(true, |&b| b == b'\n') {
                    write!(output, "{}", indent)
                        .map_err(|e| anyhow::anyhow!("Write error: {}", e))?
                }
                write!(output, "{}", trimmed).map_err(|e| anyhow::anyhow!("Write error: {}", e))?
            }
        }
        NodeData::Comment { contents } => writeln!(output, "{}<!--{}-->", indent, contents)
            .map_err(|e| anyhow::anyhow!("Write error: {}", e))?,
        NodeData::Element { name, attrs, .. } => {
            let tag_name = &name.local;

            // Write opening tag with proper indentation
            write!(output, "{}<{}", indent, tag_name)
                .map_err(|e| anyhow::anyhow!("Write error: {}", e))?;

            // Add attributes
            for attr in attrs.borrow().iter() {
                write!(output, " {}=\"{}\"", attr.name.local, attr.value)
                    .map_err(|e| anyhow::anyhow!("Write error: {}", e))?;
            }

            let children = handle.children.borrow();
            let has_children = !children.is_empty();

            // Handle void elements (self-closing tags)
            let void_elements = [
                "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                "param", "source", "track", "wbr",
            ];

            if void_elements.contains(&tag_name.as_ref()) {
                writeln!(output, ">").map_err(|e| anyhow::anyhow!("Write error: {}", e))?
            } else if !has_children {
                writeln!(output, "></{}>", tag_name)
                    .map_err(|e| anyhow::anyhow!("Write error: {}", e))?
            } else {
                writeln!(output, ">").map_err(|e| anyhow::anyhow!("Write error: {}", e))?;

                // Handle special formatting for style and script tags
                if tag_name.as_ref() == "style" || tag_name.as_ref() == "script" {
                    // For style/script, preserve internal formatting but ensure proper indentation
                    for child in children.iter() {
                        if let NodeData::Text { contents } = &child.data {
                            let text = contents.borrow();
                            let lines: Vec<&str> = text.lines().collect();
                            for line in lines {
                                if !line.trim().is_empty() {
                                    writeln!(output, "{}    {}", indent, line.trim())
                                        .map_err(|e| anyhow::anyhow!("Write error: {}", e))?
                                }
                            }
                        } else {
                            serialize_node(child, output, indent_level + 1)?
                        }
                    }
                } else {
                    // Regular content - process children with increased indentation
                    for child in children.iter() {
                        serialize_node(child, output, indent_level + 1)?
                    }
                }

                // Close tag
                writeln!(output, "{}</{}>", indent, tag_name)
                    .map_err(|e| anyhow::anyhow!("Write error: {}", e))?
            }
        }
        _ => {
            // Handle other node types if needed
        }
    }

    Ok(())
}

/// Handle GitHub CLI actions for workflow monitoring
fn handle_github_action(action: &GithubAction) -> Result<()> {
    match action {
        GithubAction::Runs {
            limit,
            workflow,
            branch,
        } => github_list_runs(*limit, workflow.as_deref(), branch.as_deref()),
        GithubAction::Watch { run_id, interval } => github_watch_run(run_id.as_deref(), *interval),
        GithubAction::Status { run_id, logs } => github_show_status(run_id.as_deref(), *logs),
        GithubAction::Trigger {
            workflow,
            branch,
            list,
        } => {
            if *list {
                github_list_workflows()
            } else {
                github_trigger_workflow(workflow.as_deref(), branch.as_deref())
            }
        }
    }
}

/// List recent workflow runs using GitHub CLI
fn github_list_runs(limit: u32, workflow: Option<&str>, branch: Option<&str>) -> Result<()> {
    println!("📋 Listing recent GitHub Actions workflow runs...");

    let mut cmd = Command::new("gh");
    cmd.args(["run", "list", "--limit", &limit.to_string()]);

    if let Some(wf) = workflow {
        cmd.args(["--workflow", wf]);
    }

    if let Some(br) = branch {
        cmd.args(["--branch", br]);
    }

    let output = cmd.output().with_context(|| {
        "Failed to execute 'gh run list'. Make sure GitHub CLI is installed and authenticated."
    })?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!(
            "❌ GitHub CLI error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow::anyhow!("GitHub CLI command failed"));
    }

    Ok(())
}

/// Watch a workflow run in real-time with configurable refresh interval
fn github_watch_run(run_id: Option<&str>, refresh_interval_secs: u64) -> Result<()> {
    println!(
        "👀 Watching GitHub Actions workflow run (refreshing every {} seconds)...",
        refresh_interval_secs
    );
    println!("💡 Press Ctrl+C to stop watching\n");

    let target_run_id = if let Some(id) = run_id {
        id.to_string()
    } else {
        // Get the latest run ID
        let output = Command::new("gh")
            .args(["run", "list", "--limit", "1", "--json", "databaseId"])
            .output()
            .with_context(|| "Failed to get latest run ID")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch latest run: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let json_output = String::from_utf8_lossy(&output.stdout);
        // Simple parsing for the ID - in a real implementation, you'd use serde_json
        if let Some(start) = json_output.find("\"databaseId\":") {
            if let Some(id_start) = json_output[start..].find(':') {
                if let Some(id_end) = json_output[start + id_start + 1..].find([',', '}']) {
                    let id_str =
                        json_output[start + id_start + 1..start + id_start + 1 + id_end].trim();
                    id_str.to_string()
                } else {
                    return Err(anyhow::anyhow!(
                        "Could not parse run ID from GitHub response"
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Could not parse run ID from GitHub response"
                ));
            }
        } else {
            return Err(anyhow::anyhow!("No runs found"));
        }
    };

    let mut previous_status = String::new();

    loop {
        // Get current status
        let output = Command::new("gh")
            .args(["run", "view", &target_run_id])
            .output()
            .with_context(|| "Failed to get run status")?;

        if !output.status.success() {
            eprintln!(
                "❌ Failed to get run status: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            std::thread::sleep(std::time::Duration::from_secs(refresh_interval_secs));
            continue;
        }

        let current_output = String::from_utf8_lossy(&output.stdout);

        // Only print if status changed or it's the first check
        if current_output != previous_status {
            println!(
                "🔄 Status update at {}",
                chrono::Utc::now().format("%H:%M:%S UTC")
            );
            println!("{}", current_output);
            println!("{}", "=".repeat(80));

            // Check if run is complete
            if current_output.contains("completed") {
                println!("✅ Workflow run completed!");
                break;
            }

            previous_status = current_output.to_string();
        } else {
            print!(
                "⏱️  Still running... (checked at {})",
                chrono::Utc::now().format("%H:%M:%S")
            );
            std::io::stdout().flush().unwrap();
            println!(""); // New line
        }

        std::thread::sleep(std::time::Duration::from_secs(refresh_interval_secs));
    }

    Ok(())
}

/// Show detailed status of a workflow run
fn github_show_status(run_id: Option<&str>, show_logs: bool) -> Result<()> {
    println!("🔍 Showing GitHub Actions workflow status...");

    let mut cmd = Command::new("gh");
    if let Some(id) = run_id {
        cmd.args(["run", "view", id]);
    } else {
        cmd.args(["run", "view"]);
    }

    let output = cmd.output().with_context(|| {
        "Failed to execute 'gh run view'. Make sure GitHub CLI is installed and authenticated."
    })?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));

        if show_logs {
            println!("\n📄 Fetching logs for failed jobs...");
            github_show_logs(run_id)?;
        }
    } else {
        eprintln!(
            "❌ GitHub CLI error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow::anyhow!("GitHub CLI view command failed"));
    }

    Ok(())
}

/// Show logs for a workflow run (typically for failed jobs)
fn github_show_logs(run_id: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("gh");
    if let Some(id) = run_id {
        cmd.args(["run", "view", id, "--log-failed"]);
    } else {
        cmd.args(["run", "view", "--log-failed"]);
    }

    let output = cmd.output().with_context(|| {
        "Failed to execute 'gh run view --log-failed'. Make sure GitHub CLI is installed."
    })?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!(
            "❌ GitHub CLI logs error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!("💡 Logs may not be available or the run may still be in progress");
    }

    Ok(())
}

/// List available workflows
fn github_list_workflows() -> Result<()> {
    println!("📝 Listing available GitHub Actions workflows...");

    let cmd = Command::new("gh")
        .args(["workflow", "list"])
        .output()
        .with_context(|| "Failed to execute 'gh workflow list'. Make sure GitHub CLI is installed and authenticated.")?;

    if cmd.status.success() {
        println!("{}", String::from_utf8_lossy(&cmd.stdout));
    } else {
        eprintln!(
            "❌ GitHub CLI error: {}",
            String::from_utf8_lossy(&cmd.stderr)
        );
        return Err(anyhow::anyhow!("GitHub CLI workflow list failed"));
    }

    Ok(())
}

/// Trigger a workflow manually
fn github_trigger_workflow(workflow: Option<&str>, branch: Option<&str>) -> Result<()> {
    println!("🚀 Triggering GitHub Actions workflow...");

    let mut cmd = Command::new("gh");
    cmd.args(["workflow", "run"]);

    if let Some(wf) = workflow {
        cmd.arg(wf);
    } else {
        return Err(anyhow::anyhow!("Workflow name or file is required for triggering. Use --list to see available workflows."));
    }

    if let Some(br) = branch {
        cmd.args(["--ref", br]);
    }

    let output = cmd.output().with_context(|| {
        "Failed to execute 'gh workflow run'. Make sure GitHub CLI is installed and authenticated."
    })?;

    if output.status.success() {
        println!("✅ Workflow triggered successfully!");
        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    } else {
        eprintln!(
            "❌ GitHub CLI error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow::anyhow!("GitHub CLI workflow run command failed"));
    }

    Ok(())
}
