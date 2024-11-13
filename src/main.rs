use anyhow::Result;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

mod markdown;
use markdown::render_markdown_file;

// Server configuration
const PORT: u16 = 3000;
const HOST: &str = "127.0.0.1";
const CONTENT_DIR: &str = "doc";
const OUTPUT_DIR: &str = "_dist";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize directories
    let content_dir = PathBuf::from(CONTENT_DIR);
    let output_dir = PathBuf::from(OUTPUT_DIR);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir)?;

    // Initial render of all markdown files
    render_all_markdown_files(&content_dir, &output_dir)?;

    // Set up broadcast channel for file changes
    let (tx, _) = broadcast::channel::<PathBuf>(16);
    let tx = Arc::new(tx);

    // Set up file watcher
    let watcher_tx = tx.clone();
    let watcher_output_dir = output_dir.clone();
    setup_file_watcher(content_dir, watcher_output_dir, watcher_tx)?;

    // Build router with static file serving and SSE endpoint
    let app = Router::new()
        .route("/events", get(sse_handler))
        .nest_service("/", ServeDir::new(output_dir))
        .with_state(tx);

    // Create server address
    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    println!("Server starting at http://{}:{}", HOST, PORT);

    // Open browser
    if let Err(e) = webbrowser::open(&format!("http://{}:{}", HOST, PORT)) {
        eprintln!("Failed to open browser: {}", e);
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Set up file watcher for markdown files
fn setup_file_watcher(
    content_dir: PathBuf,
    output_dir: PathBuf,
    tx: Arc<broadcast::Sender<PathBuf>>,
) -> Result<()> {
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                for path in event.paths {
                    if path.extension().map_or(false, |ext| ext == "md") {
                        // Render markdown to HTML
                        if let Err(e) = render_markdown_file(&path, &output_dir) {
                            eprintln!("Error rendering markdown: {}", e);
                        }
                        // Notify clients
                        let _ = tx.send(path);
                    }
                }
            }
        },
        Config::default(),
    )?;

    // Start watching content directory
    watcher.watch(&content_dir, RecursiveMode::Recursive)?;

    // Keep watcher alive by moving it into a spawned task
    tokio::spawn(async move {
        let _watcher = watcher;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    Ok(())
}

// SSE handler for live reload
async fn sse_handler(
    State(tx): State<Arc<broadcast::Sender<PathBuf>>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let mut rx = tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(_) => {
                    yield Ok(Event::default().data("reload"));
                }
                Err(e) => {
                    eprintln!("SSE error: {}", e);
                    break;
                }
            }
        }
    };

    Sse::new(stream)
}

// Render all markdown files and create an index
fn render_all_markdown_files(content_dir: &Path, output_dir: &Path) -> Result<()> {
    let mut markdown_files = Vec::new();
    collect_markdown_files(content_dir, content_dir, &mut markdown_files)?;

    // Render each markdown file
    for path in &markdown_files {
        render_markdown_file(path, output_dir)?;
    }

    // Generate index.html
    generate_index_html(output_dir, &markdown_files, content_dir)?;

    Ok(())
}

// Recursively collect markdown files
fn collect_markdown_files(
    current_dir: &Path,
    base_dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries = std::fs::read_dir(current_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(path);
        } else if path.is_dir() {
            collect_markdown_files(&path, base_dir, files)?;
        }
    }
    
    Ok(())
}

// Generate index.html with links to all rendered files
fn generate_index_html(output_dir: &Path, markdown_files: &[PathBuf], content_dir: &Path) -> Result<()> {
    let mut html_content = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Markdown Documentation</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Arial, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            color: #333;
        }
        h1 {
            border-bottom: 2px solid #eaecef;
            padding-bottom: 0.3em;
        }
        .file-list {
            list-style: none;
            padding: 0;
        }
        .file-list li {
            margin: 0.5em 0;
            padding: 0.5em;
            background: #f6f8fa;
            border-radius: 3px;
        }
        .file-list li:hover {
            background: #eaecef;
        }
        a {
            color: #0366d6;
            text-decoration: none;
            display: block;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
    <script>
        // Set up SSE for live reload
        const events = new EventSource('/events');
        events.onmessage = (e) => {
            if (e.data === 'reload') {
                window.location.reload();
            }
        };
    </script>
</head>
<body>
    <h1>Documentation Index</h1>
    <ul class="file-list">
"#,
    );

    // Sort files for consistent ordering
    let mut sorted_files = markdown_files.to_vec();
    sorted_files.sort();

    // Add links to each file
    for path in sorted_files {
        if let (Some(file_stem), Some(rel_path)) = (
            path.file_stem().and_then(|s| s.to_str()),
            path.strip_prefix(content_dir).ok(),
        ) {
            let html_path = rel_path.with_file_name(file_stem).with_extension("html");
            let display_name = rel_path.display().to_string()
                .trim_end_matches(".md")
                .replace('_', " ");
            
            html_content.push_str(&format!(
                "        <li><a href=\"{}\">{}</a></li>\n",
                html_path.display(),
                display_name
            ));
        }
    }

    html_content.push_str(
        r#"    </ul>
</body>
</html>"#,
    );

    // Write index.html to output directory
    let index_path = output_dir.join("index.html");
    std::fs::write(index_path, html_content)?;

    Ok(())
}
