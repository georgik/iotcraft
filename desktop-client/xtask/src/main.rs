use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

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
