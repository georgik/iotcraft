use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for IoTCraft Desktop Client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the web version of the application
    WebBuild {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
        /// Output directory
        #[arg(short, long, default_value = "dist")]
        output: String,
    },
    /// Serve the web version locally
    WebServe {
        /// Port to serve on
        #[arg(short, long, default_value = "8000")]
        port: u16,
        /// Directory to serve from
        #[arg(short, long, default_value = "dist")]
        dir: String,
    },
    /// Build and serve the web version
    WebDev {
        /// Port to serve on
        #[arg(short, long, default_value = "8000")]
        port: u16,
    },
    /// Format HTML files
    FormatHtml {
        /// Check formatting without modifying files
        #[arg(short, long)]
        check: bool,
        /// Path to HTML files or directory
        #[arg(default_value = "web")]
        path: String,
    },
    /// Run multiple client instances for testing
    MultiClient {
        /// Number of client instances to run
        #[arg(short, long, default_value = "2")]
        count: usize,
        /// MQTT server address override
        #[arg(short, long)]
        mqtt_server: Option<String>,
        /// Base directory for logs
        #[arg(short, long, default_value = "logs")]
        log_dir: String,
        /// Start MQTT server from ../mqtt-server
        #[arg(long)]
        with_mqtt_server: bool,
        /// Add MQTT observer using mosquitto_sub
        #[arg(long)]
        with_observer: bool,
        /// MQTT server port (default: 1883)
        #[arg(long, default_value = "1883")]
        mqtt_port: u16,
        /// Complete test environment (server + observer + clients)
        #[arg(long)]
        full_env: bool,
        /// Additional arguments to pass to each client
        #[arg(last = true)]
        client_args: Vec<String>,
    },
    /// Run tests with proper infrastructure
    Test {
        /// Test type to run
        #[command(subcommand)]
        test_type: TestType,
    },
}

#[derive(Subcommand)]
enum TestType {
    /// Run unit tests only (fast, no external dependencies)
    Unit,
    /// Run integration tests (requires MQTT server)
    Integration,
    /// Run MQTT-specific integration tests
    Mqtt,
    /// Run all available tests
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::WebBuild { release, output } => {
            web_build(*release, output).await?;
        }
        Commands::WebServe { port, dir } => {
            web_serve(*port, dir).await?;
        }
        Commands::WebDev { port } => {
            web_build(false, "dist").await?;
            web_serve(*port, "dist").await?;
        }
        Commands::FormatHtml { check, path } => {
            format_html(*check, path).await?;
        }
        Commands::MultiClient {
            count,
            mqtt_server,
            log_dir,
            with_mqtt_server,
            with_observer,
            mqtt_port,
            full_env,
            client_args,
        } => {
            multi_client_env(
                *count,
                mqtt_server.as_deref(),
                log_dir,
                *with_mqtt_server,
                *with_observer,
                *mqtt_port,
                *full_env,
                client_args,
            )
            .await?;
        }
        Commands::Test { test_type } => {
            run_tests(test_type).await?;
        }
    }

    Ok(())
}

async fn web_build(release: bool, output_dir: &str) -> Result<()> {
    println!("🔨 Building web version...");

    // Ensure we're in the project root
    let project_root = Path::new(".");
    std::env::set_current_dir(project_root).context("Failed to change to project directory")?;

    // Check if wasm-pack is installed
    if which::which("wasm-pack").is_err() {
        return Err(anyhow::anyhow!(
            "wasm-pack is not installed. Please install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
        ));
    }

    // Clean output directory
    let output_path = PathBuf::from(output_dir);
    if output_path.exists() {
        println!("🧹 Cleaning output directory...");
        fs::remove_dir_all(&output_path)
            .await
            .context("Failed to remove output directory")?;
    }
    fs::create_dir_all(&output_path)
        .await
        .context("Failed to create output directory")?;

    // Build with wasm-pack
    println!("📦 Building WASM package...");
    let mut cmd = Command::new("wasm-pack");
    cmd.args(&[
        "build",
        "--target",
        "web",
        "--out-dir",
        "pkg",
        "--out-name",
        "iotcraft_web",
    ]);

    if release {
        // Use standard release mode for now - our wasm-release profile will inherit from release anyway
        cmd.arg("--release");
        println!("   Building in release mode with size optimization...");
    } else {
        cmd.arg("--dev");
        println!("   Building in development mode...");
    }

    let status = cmd.status().context("Failed to execute wasm-pack")?;

    if !status.success() {
        return Err(anyhow::anyhow!("wasm-pack build failed"));
    }

    // Copy wasm files to output directory
    println!("📁 Copying WASM files...");
    copy_wasm_files(&output_path)
        .await
        .context("Failed to copy WASM files")?;

    // Generate HTML file
    println!("🌐 Generating HTML...");
    generate_html(&output_path, release)
        .await
        .context("Failed to generate HTML")?;

    // Copy additional HTML files (debug.html, etc.)
    println!("🌐 Copying additional HTML files...");
    copy_additional_html_files(&output_path)
        .await
        .context("Failed to copy additional HTML files")?;

    // Copy assets if they exist
    if Path::new("assets").exists() {
        println!("🎨 Copying assets...");
        copy_assets(&output_path)
            .await
            .context("Failed to copy assets")?;
    }

    println!("✅ Web build completed successfully!");
    println!("   Output directory: {}", output_path.display());

    Ok(())
}

async fn copy_wasm_files(output_path: &Path) -> Result<()> {
    let pkg_path = Path::new("pkg");

    // Copy essential WASM files
    let files_to_copy = [
        "iotcraft_web.js",
        "iotcraft_web_bg.wasm",
        "iotcraft_web_bg.wasm.d.ts",
        "iotcraft_web.d.ts",
    ];

    for file in files_to_copy {
        let src = pkg_path.join(file);
        let dst = output_path.join(file);

        if src.exists() {
            fs::copy(&src, &dst)
                .await
                .with_context(|| format!("Failed to copy {}", file))?;
        }
    }

    Ok(())
}

async fn generate_html(output_path: &Path, _is_release: bool) -> Result<()> {
    // Try to use template from web/index.html, otherwise generate default
    let template_path = Path::new("web/index.html");

    let html_content = if template_path.exists() {
        // Use existing template
        fs::read_to_string(template_path)
            .await
            .context("Failed to read web/index.html template")?
    } else {
        // Generate default HTML if template doesn't exist
        generate_default_html()
    };

    let html_path = output_path.join("index.html");
    fs::write(html_path, html_content)
        .await
        .context("Failed to write index.html")?;

    Ok(())
}

fn generate_default_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IoTCraft - Web Client</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            background: #000;
            font-family: 'Arial', sans-serif;
            overflow: hidden;
        }
        
        canvas {
            display: block;
            position: absolute;
            top: 0;
            left: 0;
            width: 100vw;
            height: 100vh;
            background: #000;
        }
        
        .loading {
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: white;
            font-size: 24px;
            z-index: 1000;
            text-align: center;
        }
        
        .loading::after {
            content: '';
            animation: dots 1.5s steps(4, end) infinite;
        }
        
        @keyframes dots {
            0%, 20% { content: ''; }
            40% { content: '.'; }
            60% { content: '..'; }
            80%, 100% { content: '...'; }
        }
        
        .error {
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: #ff4444;
            font-size: 18px;
            z-index: 1000;
            text-align: center;
            padding: 20px;
            background: rgba(0, 0, 0, 0.8);
            border-radius: 10px;
            max-width: 600px;
        }
    </style>
</head>
<body>
    <div id="loading" class="loading">Loading IoTCraft</div>
    <div id="error" class="error" style="display: none;">
        <h3>Failed to load IoTCraft</h3>
        <p id="error-message"></p>
        <p>Please refresh the page or check the browser console for more details.</p>
    </div>
    <canvas id="canvas"></canvas>

    <script type="module">
        import init, { main } from './iotcraft_web.js';
        
        async function run() {
            try {
                // Initialize the WASM module
                await init();
                
                // Hide loading indicator
                document.getElementById('loading').style.display = 'none';
                
                // Start the application
                main();
                
                console.log('IoTCraft Web Client started successfully');
            } catch (error) {
                console.error('Failed to start IoTCraft:', error);
                
                // Show error message
                document.getElementById('loading').style.display = 'none';
                document.getElementById('error').style.display = 'block';
                document.getElementById('error-message').textContent = error.message || 'Unknown error occurred';
            }
        }

        // Add some basic error handling for WASM loading
        window.addEventListener('error', (event) => {
            if (event.filename && event.filename.includes('.wasm')) {
                console.error('WASM loading error:', event.error);
                document.getElementById('loading').style.display = 'none';
                document.getElementById('error').style.display = 'block';
                document.getElementById('error-message').textContent = 'Failed to load WASM module: ' + (event.error?.message || 'Unknown WASM error');
            }
        });

        run();
    </script>
</body>
</html>"#.to_string()
}

async fn copy_additional_html_files(output_path: &Path) -> Result<()> {
    let web_dir = Path::new("web");

    // List of additional HTML files to copy (excluding index.html which is handled separately)
    let html_files = ["debug.html"];

    for file in html_files {
        let src = web_dir.join(file);
        let dst = output_path.join(file);

        if src.exists() {
            fs::copy(&src, &dst)
                .await
                .with_context(|| format!("Failed to copy {}", file))?;
            println!("   Copied {}", file);
        } else {
            println!("   Skipped {} (not found)", file);
        }
    }

    Ok(())
}

async fn copy_assets(output_path: &Path) -> Result<()> {
    let assets_src = Path::new("assets");
    let assets_dst = output_path.join("assets");

    fs::create_dir_all(&assets_dst).await?;

    copy_dir_recursively(assets_src, &assets_dst).await?;

    Ok(())
}

fn copy_dir_recursively<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(src).await?;

        while let Some(entry) = entries.next_entry().await? {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                fs::create_dir_all(&dst_path).await?;
                copy_dir_recursively(&src_path, &dst_path).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
            }
        }

        Ok(())
    })
}

async fn web_serve(port: u16, dir: &str) -> Result<()> {
    println!("🚀 Starting Rust HTTP server...");
    println!("   Serving directory: {}", dir);
    println!("   Port: {}", port);
    println!();

    // Get local IP for network access
    let local_ip = get_local_ip().unwrap_or_else(|| "localhost".to_string());
    let localhost_url = format!("http://localhost:{}", port);
    let network_url = format!("http://{}:{}", local_ip, port);

    println!("📱 Access URLs:");
    println!("   Local:   {}", localhost_url);
    println!("   Network: {}", network_url);
    println!();

    // Generate QR code for the network URL
    if local_ip != "localhost" {
        println!("📱 QR Code for mobile access:");
        generate_qr_code(&network_url);
        println!();
    }

    // Validate directory exists
    let serve_dir = Path::new(dir);
    if !serve_dir.exists() {
        return Err(anyhow::anyhow!("Directory '{}' does not exist", dir));
    }
    if !serve_dir.is_dir() {
        return Err(anyhow::anyhow!("'{}' is not a directory", dir));
    }

    // Convert to absolute path for better error reporting
    let absolute_dir =
        std::fs::canonicalize(serve_dir).context("Failed to resolve absolute path")?;

    println!("📁 Serving files from: {}", absolute_dir.display());
    println!();

    // Simplest possible static file server
    let routes = warp::fs::dir(absolute_dir.clone());

    println!("🌟 IoTCraft Web Server is ready!");
    println!("   Listening on 0.0.0.0:{}", port);
    println!("   Press Ctrl+C to stop the server");
    println!();

    // Start the server with proper async handling
    println!("🟢 Server starting on 0.0.0.0:{}...", port);

    // Spawn the server task
    let server_task = tokio::spawn(async move {
        let server = warp::serve(routes).run(([0, 0, 0, 0], port));

        server.await;
        println!("🔄 Server task completed");
    });

    println!("💫 Server is running indefinitely - use Ctrl+C to stop");
    println!();

    // Wait for Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    println!();
    println!("🛡️ Received Ctrl+C, shutting down...");

    // Abort the server task since warp doesn't support graceful shutdown in this version
    server_task.abort();

    println!("✅ Web server stopped successfully");

    Ok(())
}

/// Get the local IP address for network access
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    // Try to connect to a remote address to determine local IP
    // This doesn't actually send data, just determines routing
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

/// Generate and display a QR code for the given URL
fn generate_qr_code(url: &str) {
    match qr2term::print_qr(url) {
        Ok(_) => {
            println!("   Scan the QR code above with your phone's camera");
            println!("   or QR code reader app to open: {}", url);
        }
        Err(e) => {
            println!("   Failed to generate QR code: {}", e);
            println!("   Use this URL instead: {}", url);
        }
    }
}

/// Format HTML files using tidier
async fn format_html(check_only: bool, path_str: &str) -> Result<()> {
    let path = Path::new(path_str);

    if check_only {
        println!("🔍 Checking HTML formatting...");
    } else {
        println!("🎨 Formatting HTML files...");
    }

    let html_files = if path.is_file()
        && path
            .extension()
            .map_or(false, |ext| ext == "html" || ext == "htm")
    {
        vec![path.to_path_buf()]
    } else if path.is_dir() {
        find_html_files(path).await?
    } else {
        return Err(anyhow::anyhow!(
            "Path must be an HTML file or directory containing HTML files"
        ));
    };

    if html_files.is_empty() {
        println!("   No HTML files found in {}", path_str);
        return Ok(());
    }

    let mut files_processed = 0;
    let mut files_changed = 0;
    let mut errors = Vec::new();

    for html_file in html_files {
        match process_html_file(&html_file, check_only).await {
            Ok(changed) => {
                files_processed += 1;
                if changed {
                    files_changed += 1;
                    if check_only {
                        println!("   ❌ {}: formatting issues found", html_file.display());
                    } else {
                        println!("   ✅ {}: formatted", html_file.display());
                    }
                } else if !check_only {
                    println!("   ✅ {}: already formatted", html_file.display());
                }
            }
            Err(e) => {
                errors.push((html_file.display().to_string(), e));
            }
        }
    }

    println!();
    println!("📊 Summary:");
    println!("   Files processed: {}", files_processed);

    if check_only {
        if files_changed > 0 {
            println!("   ❌ Files with formatting issues: {}", files_changed);
            println!("   Run 'cargo xtask format-html' to fix formatting.");
        } else {
            println!("   ✅ All files are properly formatted.");
        }
    } else {
        println!("   Files formatted: {}", files_changed);
    }

    if !errors.is_empty() {
        println!("   ❌ Errors encountered: {}", errors.len());
        for (file, error) in errors {
            println!("      {}: {}", file, error);
        }
        return Err(anyhow::anyhow!("Some files could not be processed"));
    }

    if check_only && files_changed > 0 {
        return Err(anyhow::anyhow!("Formatting issues found"));
    }

    Ok(())
}

/// Find all HTML files in a directory recursively
async fn find_html_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut html_files = Vec::new();
    find_html_files_recursive(dir, &mut html_files).await?;
    Ok(html_files)
}

fn find_html_files_recursive<'a>(
    dir: &'a Path,
    html_files: &'a mut Vec<PathBuf>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(dir)
            .await
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                find_html_files_recursive(&path, html_files).await?;
            } else if let Some(ext) = path.extension() {
                if ext == "html" || ext == "htm" {
                    html_files.push(path);
                }
            }
        }

        Ok(())
    })
}

/// Process a single HTML file with pure Rust HTML5 parser
async fn process_html_file(file_path: &Path, check_only: bool) -> Result<bool> {
    let original_content = fs::read_to_string(file_path)
        .await
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Format the HTML using our pure Rust formatter
    let formatted_content = format_html_content(&original_content)
        .with_context(|| format!("Failed to format HTML file: {}", file_path.display()))?;

    let changed = original_content != formatted_content;

    if changed && !check_only {
        fs::write(file_path, formatted_content)
            .await
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
                writeln!(output, ">").map_err(|e| anyhow::anyhow!("Write error: {}", e))?;
            } else if !has_children {
                writeln!(output, "></{}>", tag_name)
                    .map_err(|e| anyhow::anyhow!("Write error: {}", e))?;
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

/// Run a single client instance with logging
async fn run_client_instance(
    client_num: usize,
    player_id: String,
    args: Vec<String>,
    log_file: PathBuf,
) -> Result<()> {
    // Create and open log file
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file)
        .await
        .with_context(|| format!("Failed to create log file: {}", log_file.display()))?;

    // Write header to log file
    let header = format!(
        "=== IoTCraft Client {} (Player: {}) ===\n\
         Started at: {}\n\
         Command: cargo {}\n\
         ==========================================\n\n",
        client_num,
        player_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        args.join(" ")
    );

    log_handle.write_all(header.as_bytes()).await?;
    log_handle.flush().await?;

    // Start the cargo process
    let mut child = TokioCommand::new("cargo")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start client {}", client_num))?;

    // Get stdout and stderr
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let log_file_clone = log_file.clone();
    let client_num_clone = client_num;

    // Spawn tasks to handle stdout and stderr
    let stdout_task = tokio::spawn(async move {
        handle_stdout_stream(
            stdout_reader,
            log_file_clone.clone(),
            client_num_clone,
            "STDOUT",
        )
        .await
    });

    let stderr_task = tokio::spawn(async move {
        handle_stderr_stream(stderr_reader, log_file, client_num, "STDERR").await
    });

    // Wait for the process to complete
    let exit_status = child
        .wait()
        .await
        .with_context(|| format!("Failed to wait for client {}", client_num))?;

    // Wait for output handling to complete
    let _ = tokio::try_join!(stdout_task, stderr_task);

    if exit_status.success() {
        println!(
            "✅ Client {} (Player: {}) exited successfully",
            client_num, player_id
        );
    } else {
        println!(
            "❌ Client {} (Player: {}) exited with code: {:?}",
            client_num,
            player_id,
            exit_status.code()
        );
        return Err(anyhow::anyhow!(
            "Client {} exited with non-zero status: {:?}",
            client_num,
            exit_status
        ));
    }

    Ok(())
}

/// Handle stdout stream from client process
async fn handle_stdout_stream(
    mut reader: BufReader<tokio::process::ChildStdout>,
    log_file: PathBuf,
    client_num: usize,
    stream_type: &str,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp and stream type
        let log_line = format!(
            "[{}] [{}] [Client-{}] {}",
            timestamp, stream_type, client_num, line
        );
        log_handle.write_all(log_line.as_bytes()).await?;

        // Also write to console with client prefix
        print!("[Client-{}] {}", client_num, line);

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Handle stderr stream from client process
async fn handle_stderr_stream(
    mut reader: BufReader<tokio::process::ChildStderr>,
    log_file: PathBuf,
    client_num: usize,
    stream_type: &str,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp and stream type
        let log_line = format!(
            "[{}] [{}] [Client-{}] {}",
            timestamp, stream_type, client_num, line
        );
        log_handle.write_all(log_line.as_bytes()).await?;

        // Also write to console with client prefix (stderr in red if supported)
        eprint!("[Client-{}] {}", client_num, line);

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Run tests with proper infrastructure setup
async fn run_tests(test_type: &TestType) -> Result<()> {
    match test_type {
        TestType::Unit => {
            println!("🧪 Running unit tests...");
            run_unit_tests().await
        }
        TestType::Integration => {
            println!("🔧 Running integration tests...");
            run_integration_tests().await
        }
        TestType::Mqtt => {
            println!("📡 Running MQTT integration tests...");
            run_mqtt_tests().await
        }
        TestType::All => {
            println!("🚀 Running all tests...");

            println!("\n📝 Step 1/3: Unit tests");
            run_unit_tests().await?;

            println!("\n📝 Step 2/3: Integration tests");
            run_integration_tests().await?;

            println!("\n📝 Step 3/3: MQTT tests");
            run_mqtt_tests().await?;

            println!("\n✅ All tests completed successfully!");
            Ok(())
        }
    }
}

/// Run unit tests (no external dependencies)
async fn run_unit_tests() -> Result<()> {
    println!("   Running cargo test for unit tests...");

    let status = Command::new("cargo")
        .args(&["test", "--lib", "--bins"])
        .status()
        .context("Failed to execute cargo test")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Unit tests failed"));
    }

    println!("   ✅ Unit tests passed");
    Ok(())
}

/// Run integration tests (with MQTT server infrastructure)
async fn run_integration_tests() -> Result<()> {
    println!("   Starting MQTT server for integration tests...");

    // Start MQTT server in background
    let mqtt_port = 1884; // Use different port to avoid conflicts
    let server_handle = tokio::spawn(async move {
        // Use a dummy log file for the test server
        let log_file = std::path::PathBuf::from("/tmp/test-mqtt-server.log");
        run_mqtt_server(log_file, mqtt_port).await
    });

    // Wait for server to be ready
    println!("   Waiting for MQTT server to start...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check if server is ready
    let timeout = get_mqtt_server_timeout();
    match wait_for_port("localhost", mqtt_port, timeout).await {
        Ok(_) => println!("   ✅ MQTT server ready on port {}", mqtt_port),
        Err(e) => {
            server_handle.abort();
            return Err(anyhow::anyhow!("MQTT server failed to start: {}", e));
        }
    }

    // Run integration tests
    println!("   Running integration tests...");
    let test_result = Command::new("cargo")
        .args(&[
            "test",
            "--test",
            "integration",
            "--features",
            "integration-tests",
        ])
        .env("MQTT_TEST_PORT", mqtt_port.to_string())
        .status()
        .context("Failed to execute integration tests");

    // Clean up server
    server_handle.abort();

    match test_result {
        Ok(status) if status.success() => {
            println!("   ✅ Integration tests passed");
            Ok(())
        }
        Ok(_) => Err(anyhow::anyhow!("Integration tests failed")),
        Err(e) => Err(e),
    }
}

/// Run MQTT-specific integration tests
async fn run_mqtt_tests() -> Result<()> {
    println!("   Starting MQTT server for MQTT tests...");

    // Start MQTT server in background
    let mqtt_port = 1885; // Use different port to avoid conflicts
    let server_handle = tokio::spawn(async move {
        // Use a dummy log file for the test server
        let log_file = std::path::PathBuf::from("/tmp/test-mqtt-server-mqtt.log");
        run_mqtt_server(log_file, mqtt_port).await
    });

    // Wait for server to be ready
    println!("   Waiting for MQTT server to start...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check if server is ready
    let timeout = get_mqtt_server_timeout();
    match wait_for_port("localhost", mqtt_port, timeout).await {
        Ok(_) => println!("   ✅ MQTT server ready on port {}", mqtt_port),
        Err(e) => {
            server_handle.abort();
            return Err(anyhow::anyhow!("MQTT server failed to start: {}", e));
        }
    }

    // Run MQTT tests
    println!("   Running MQTT-specific tests...");
    let test_result = Command::new("cargo")
        .args(&[
            "test",
            "--test",
            "integration",
            "--features",
            "integration-tests",
            "mqtt::",
        ])
        .env("MQTT_TEST_PORT", mqtt_port.to_string())
        .status()
        .context("Failed to execute MQTT tests");

    // Clean up server
    server_handle.abort();

    match test_result {
        Ok(status) if status.success() => {
            println!("   ✅ MQTT tests passed");
            Ok(())
        }
        Ok(_) => Err(anyhow::anyhow!("MQTT tests failed")),
        Err(e) => Err(e),
    }
}

/// Check if a TCP port is open and accepting connections
fn is_port_open(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;

    let addr_str = format!("{}:{}", host, port);

    // Use ToSocketAddrs to resolve hostname (including localhost) to actual IP addresses
    match addr_str.to_socket_addrs() {
        Ok(mut addrs) => {
            // Try to connect to the first resolved address
            if let Some(socket_addr) = addrs.next() {
                match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)) {
                    Ok(_) => {
                        // Successfully connected, close and return true
                        true
                    }
                    Err(_) => false,
                }
            } else {
                eprintln!("[DEBUG] No addresses resolved for: {}", addr_str);
                false
            }
        }
        Err(e) => {
            eprintln!("[DEBUG] Failed to resolve address {}: {}", addr_str, e);
            false
        }
    }
}

/// Wait for a port to become available with timeout
async fn wait_for_port(host: &str, port: u16, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if is_port_open(host, port) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Err(anyhow::anyhow!(
        "Port {}:{} did not become available within {} seconds",
        host,
        port,
        timeout_secs
    ))
}

/// Get appropriate timeout for MQTT server startup based on environment
fn get_mqtt_server_timeout() -> u64 {
    // Check if we're in CI environment
    let is_ci = std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("JENKINS_URL").is_ok();

    // Allow override via environment variable
    if let Ok(timeout_str) = std::env::var("MQTT_SERVER_TIMEOUT") {
        if let Ok(timeout) = timeout_str.parse::<u64>() {
            println!("   Using custom MQTT server timeout: {} seconds", timeout);
            return timeout;
        }
    }

    if is_ci {
        println!("   Detected CI environment, using extended timeout: 120 seconds");
        120 // 2 minutes for CI environments where build might be needed
    } else {
        println!("   Using standard timeout: 30 seconds");
        30 // 30 seconds for local development
    }
}

/// Enhanced multi-client runner with full environment support
async fn multi_client_env(
    count: usize,
    mqtt_server_override: Option<&str>,
    log_dir: &str,
    with_mqtt_server: bool,
    with_observer: bool,
    mqtt_port: u16,
    full_env: bool,
    client_args: &[String],
) -> Result<()> {
    // full_env is a shorthand for with_mqtt_server + with_observer
    let start_server = full_env || with_mqtt_server;
    let start_observer = full_env || with_observer;

    if count == 0 {
        return Err(anyhow::anyhow!("Client count must be greater than 0"));
    }

    // If no server override is provided and we're starting our own server, use localhost
    let effective_mqtt_server = if start_server && mqtt_server_override.is_none() {
        Some("localhost".to_string())
    } else {
        mqtt_server_override.map(|s| s.to_string())
    };

    println!("🚀 Starting IoTCraft test environment...");
    println!("   Client instances: {}", count);
    println!("   Log directory: {}", log_dir);
    if start_server {
        println!("   ✅ MQTT server: localhost:{}", mqtt_port);
    }
    if start_observer {
        println!("   ✅ MQTT observer: mosquitto_sub");
    }
    if let Some(ref server) = effective_mqtt_server {
        println!("   📡 MQTT server: {}", server);
    }
    println!();

    // Create timestamped log directory
    let log_path = Path::new(log_dir);
    fs::create_dir_all(log_path)
        .await
        .context("Failed to create log directory")?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let run_id = format!("{}", timestamp);
    let run_log_dir = log_path.join(&run_id);
    fs::create_dir_all(&run_log_dir)
        .await
        .context("Failed to create run log directory")?;

    println!(
        "📁 Session logs will be stored in: {}",
        run_log_dir.display()
    );
    println!();

    let mut handles = Vec::new();
    let mut abort_handles = Vec::new();
    let mut component_names = Vec::new();

    // Start MQTT server if requested
    if start_server {
        println!("🟢 Starting MQTT server...");
        let server_log_file = run_log_dir.join("mqtt-server.log");
        let server_handle = tokio::spawn(run_mqtt_server(server_log_file, mqtt_port));
        abort_handles.push(server_handle.abort_handle());
        handles.push(server_handle);
        component_names.push("MQTT Server".to_string());

        // Wait for server port to become available
        println!("   Waiting for MQTT server to open port {}...", mqtt_port);
        match wait_for_port("localhost", mqtt_port, 30).await {
            Ok(_) => println!(
                "   ✅ MQTT server is ready and listening on port {}",
                mqtt_port
            ),
            Err(e) => {
                println!("   ⚠️  Warning: {}", e);
                println!("   ⚠️  Proceeding anyway - server might still be starting up");
            }
        }
    }

    // Start MQTT observer if requested
    if start_observer {
        println!("🟢 Starting MQTT observer...");
        let observer_log_file = run_log_dir.join("mqtt-observer.log");
        let mqtt_host = if start_server {
            "localhost".to_string()
        } else if let Some(ref server) = effective_mqtt_server {
            server.split(':').next().unwrap_or("localhost").to_string()
        } else {
            "localhost".to_string()
        };

        let observer_handle =
            tokio::spawn(run_mqtt_observer(observer_log_file, mqtt_host, mqtt_port));
        abort_handles.push(observer_handle.abort_handle());
        handles.push(observer_handle);
        component_names.push("MQTT Observer".to_string());

        // Small delay to let observer connect
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    // Start clients
    for client_id in 0..count {
        let player_id = format!("player-{}", client_id + 1);
        let log_file = run_log_dir.join(format!("client-{}.log", client_id + 1));

        // Build command arguments
        let mut args = vec![
            "run".to_string(),
            "--".to_string(),
            "--player-id".to_string(),
            player_id.clone(),
        ];

        // Add MQTT server override if provided
        if let Some(ref server) = effective_mqtt_server {
            args.push("--mqtt-server".to_string());
            args.push(server.clone());
        }

        // Add any additional client arguments
        args.extend_from_slice(client_args);

        println!(
            "🟢 Starting client {} (Player ID: {})...",
            client_id + 1,
            player_id
        );
        println!("[DEBUG] Client {} command details:", client_id + 1);
        println!("[DEBUG]   Command: cargo {}", args.join(" "));
        println!(
            "[DEBUG]   Working directory: {} (current)",
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("unknown"))
                .display()
        );
        println!("[DEBUG]   Log file: {}", log_file.display());
        println!("[DEBUG]   Player ID: {}", player_id);

        let log_file_clone = log_file.clone();
        let args_clone = args.clone();
        let player_id_clone = player_id.clone();

        let handle = tokio::spawn(async move {
            run_client_instance(client_id + 1, player_id_clone, args_clone, log_file_clone).await
        });

        abort_handles.push(handle.abort_handle());
        handles.push(handle);
        component_names.push(format!("Client {}", client_id + 1));

        // Small delay between starting clients to avoid resource contention
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!();
    println!("✅ All components started successfully!");
    if start_server {
        println!("   🌐 MQTT Server: running on port {}", mqtt_port);
    }
    if start_observer {
        println!("   👁️  MQTT Observer: monitoring all topics");
    }
    println!("   🎮 Clients: {} instances running", count);
    println!();
    println!("💡 Monitoring all processes...");
    println!("   Press Ctrl+C to stop all components and exit");
    println!("   Logs are being written to: {}", run_log_dir.display());
    println!();

    // Wait for Ctrl+C or any component to exit
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!();
            println!("🛑 Received Ctrl+C, shutting down all components...");

            // Cancel all tasks using abort handles
            for abort_handle in abort_handles {
                abort_handle.abort();
            }

            println!("✅ All components terminated");
        }
        result = futures::future::try_join_all(handles) => {
            match result {
                Ok(results) => {
                    println!("📊 All components finished:");
                    for (i, result) in results.into_iter().enumerate() {
                        let fallback_name = format!("Component {}", i + 1);
                        let component_name = component_names.get(i)
                            .map(|s| s.as_str())
                            .unwrap_or(&fallback_name);
                        match result {
                            Ok(_) => println!("   {} ✅ Exited normally", component_name),
                            Err(e) => println!("   {} ❌ Failed: {}", component_name, e),
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error running components: {}", e);
                }
            }
        }
    }

    println!();
    println!("📋 Session Summary:");
    println!("   Run ID: {}", run_id);
    if start_server {
        println!("   MQTT Server: started on port {}", mqtt_port);
    }
    if start_observer {
        println!("   MQTT Observer: monitoring enabled");
    }
    println!("   Clients: {}", count);
    println!("   Logs location: {}", run_log_dir.display());
    println!();
    println!("🔍 To view logs:");
    if start_server {
        let server_log = run_log_dir.join("mqtt-server.log");
        println!("   tail -f {} # MQTT Server", server_log.display());
    }
    if start_observer {
        let observer_log = run_log_dir.join("mqtt-observer.log");
        println!("   tail -f {} # MQTT Observer", observer_log.display());
    }
    for client_id in 0..count {
        let log_file = run_log_dir.join(format!("client-{}.log", client_id + 1));
        println!(
            "   tail -f {} # Client {}",
            log_file.display(),
            client_id + 1
        );
    }

    Ok(())
}

/// Run the MQTT server from ../mqtt-server
async fn run_mqtt_server(log_file: PathBuf, port: u16) -> Result<()> {
    // Resolve and validate the server working directory
    let server_dir = std::fs::canonicalize("../mqtt-server").with_context(|| {
        "Failed to resolve ../mqtt-server. Are you running xtask from desktop-client?"
    })?;
    let config_path = server_dir.join("rumqttd.toml");
    let has_config = config_path.exists();

    // Check for pre-built binary first (CI optimization)
    let pre_built_binary = server_dir.join("target/release/iotcraft-mqtt-server");
    let use_prebuilt = pre_built_binary.exists() && std::env::var("CI").is_ok();

    let (command, args, working_dir) = if use_prebuilt {
        println!("[DEBUG] Using pre-built MQTT server binary (CI optimization)");
        (
            pre_built_binary.to_string_lossy().to_string(),
            vec!["--port".to_string(), port.to_string()],
            server_dir.clone(),
        )
    } else {
        println!("[DEBUG] Building and running MQTT server with cargo");
        (
            "cargo".to_string(),
            vec![
                "run".to_string(),
                "--release".to_string(),
                "--".to_string(),
                "--port".to_string(),
                port.to_string(),
            ],
            server_dir.clone(),
        )
    };

    println!("[DEBUG] Starting MQTT server with:");
    println!("[DEBUG]   Command: {} {}", command, args.join(" "));
    println!("[DEBUG]   Working directory: {}", working_dir.display());
    println!(
        "[DEBUG]   Config file: {} ({})",
        config_path.display(),
        if has_config { "found" } else { "MISSING" }
    );
    println!("[DEBUG]   Log file: {}", log_file.display());
    if use_prebuilt {
        println!("[DEBUG]   Using pre-built binary to avoid rebuild");
    }

    // Create and open log file
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file)
        .await
        .with_context(|| format!("Failed to create log file: {}", log_file.display()))?;

    // Write header to log file
    let header = format!(
        "=== MQTT Server ===\n\
         Started at: {}\n\
         Port: {}\n\
         Working directory: {}\n\
         Config file: {} ({})\n\
         Command: {} {}\n\
         Mode: {}\n\
         ===================\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        port,
        working_dir.display(),
        config_path.display(),
        if has_config { "found" } else { "MISSING" },
        command,
        args.join(" "),
        if use_prebuilt {
            "pre-built binary"
        } else {
            "cargo run"
        }
    );

    log_handle.write_all(header.as_bytes()).await?;
    log_handle.flush().await?;

    if !has_config {
        println!("[WARN] rumqttd.toml not found at {}. The server may fail to start if it requires this config.", config_path.display());
    }

    // Start the MQTT server process
    let mut child = TokioCommand::new("cargo")
        .args(&["run", "--release", "--", "--port", &port.to_string()])
        .current_dir(&server_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start MQTT server in {}", server_dir.display()))?;

    println!(
        "[DEBUG] MQTT server process started with PID: {:?}",
        child.id()
    );

    // Get stdout and stderr
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let log_file_clone = log_file.clone();

    // Spawn tasks to handle stdout and stderr
    let stdout_task = tokio::spawn(async move {
        handle_process_stdout_stream(stdout_reader, log_file_clone, "MQTT-Server", "STDOUT").await
    });

    let stderr_task = tokio::spawn(async move {
        handle_process_stderr_stream(stderr_reader, log_file, "MQTT-Server", "STDERR").await
    });

    // Wait for the process to complete
    let exit_status = child
        .wait()
        .await
        .context("Failed to wait for MQTT server")?;

    // Wait for output handling to complete
    let _ = tokio::try_join!(stdout_task, stderr_task);

    if exit_status.success() {
        println!("✅ MQTT Server exited successfully");
    } else {
        println!("❌ MQTT Server exited with code: {:?}", exit_status.code());
        return Err(anyhow::anyhow!(
            "MQTT Server exited with non-zero status: {:?}",
            exit_status
        ));
    }

    Ok(())
}

/// Run the MQTT observer using mosquitto_sub
async fn run_mqtt_observer(log_file: PathBuf, mqtt_host: String, mqtt_port: u16) -> Result<()> {
    println!("[DEBUG] Starting MQTT observer with:");
    println!(
        "[DEBUG]   Command: mosquitto_sub -h {} -p {} -t # -i sub",
        mqtt_host, mqtt_port
    );
    println!("[DEBUG]   Log file: {}", log_file.display());

    // Create and open log file
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file)
        .await
        .with_context(|| format!("Failed to create log file: {}", log_file.display()))?;

    // Write header to log file
    let header = format!(
        "=== MQTT Observer ===\n\
         Started at: {}\n\
         Host: {}:{}\n\
         Command: mosquitto_sub -h {} -p {} -t # -i sub\n\
         =====================\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        mqtt_host,
        mqtt_port,
        mqtt_host,
        mqtt_port
    );

    log_handle.write_all(header.as_bytes()).await?;
    log_handle.flush().await?;

    // Start the mosquitto_sub process
    let mut child = TokioCommand::new("mosquitto_sub")
        .args(&[
            "-h",
            &mqtt_host,
            "-p",
            &mqtt_port.to_string(),
            "-t",
            "#", // Subscribe to all topics
            "-i",
            "sub", // Client ID to avoid server rejection
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start mosquitto_sub (make sure mosquitto-clients is installed)")?;

    println!(
        "[DEBUG] MQTT observer process started with PID: {:?}",
        child.id()
    );

    // Get stdout and stderr
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let log_file_clone = log_file.clone();

    // Spawn tasks to handle stdout and stderr
    let stdout_task = tokio::spawn(async move {
        handle_process_stdout_stream(stdout_reader, log_file_clone, "MQTT-Observer", "STDOUT").await
    });

    let stderr_task = tokio::spawn(async move {
        handle_process_stderr_stream(stderr_reader, log_file, "MQTT-Observer", "STDERR").await
    });

    // Wait for the process to complete
    let exit_status = child
        .wait()
        .await
        .context("Failed to wait for MQTT observer")?;

    // Wait for output handling to complete
    let _ = tokio::try_join!(stdout_task, stderr_task);

    if exit_status.success() {
        println!("✅ MQTT Observer exited successfully");
    } else {
        println!(
            "❌ MQTT Observer exited with code: {:?}",
            exit_status.code()
        );
        return Err(anyhow::anyhow!(
            "MQTT Observer exited with non-zero status: {:?}",
            exit_status
        ));
    }

    Ok(())
}

/// Handle stdout stream for generic processes (server/observer)
async fn handle_process_stdout_stream(
    mut reader: BufReader<tokio::process::ChildStdout>,
    log_file: PathBuf,
    process_name: &str,
    stream_type: &str,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp and stream type
        let log_line = format!(
            "[{}] [{}] [{}] {}",
            timestamp, stream_type, process_name, line
        );
        log_handle.write_all(log_line.as_bytes()).await?;

        // Also write to console with process prefix
        print!("[{}] {}", process_name, line);

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Handle stderr stream for generic processes (server/observer)
async fn handle_process_stderr_stream(
    mut reader: BufReader<tokio::process::ChildStderr>,
    log_file: PathBuf,
    process_name: &str,
    stream_type: &str,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp and stream type
        let log_line = format!(
            "[{}] [{}] [{}] {}",
            timestamp, stream_type, process_name, line
        );
        log_handle.write_all(log_line.as_bytes()).await?;

        // Also write to console with process prefix (stderr in red if supported)
        eprint!("[{}] {}", process_name, line);

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}
