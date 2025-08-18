use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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
    println!("🚀 Starting web server...");
    println!("   Serving directory: {}", dir);
    println!("   Port: {}", port);
    println!("   URL: http://localhost:{}", port);
    println!();
    println!("Press Ctrl+C to stop the server");

    // Use Python's built-in HTTP server for simplicity and portability
    let mut cmd = if which::which("python3").is_ok() {
        let mut cmd = Command::new("python3");
        cmd.args(&["-m", "http.server", &port.to_string()]);
        cmd
    } else if which::which("python").is_ok() {
        let mut cmd = Command::new("python");
        cmd.args(&["-m", "http.server", &port.to_string()]);
        cmd
    } else {
        return Err(anyhow::anyhow!(
            "Python is not installed. Please install Python or use a different HTTP server."
        ));
    };

    cmd.current_dir(dir);

    let status = cmd.status().context("Failed to start HTTP server")?;

    if !status.success() {
        return Err(anyhow::anyhow!("HTTP server failed"));
    }

    Ok(())
}
