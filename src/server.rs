use crate::{config::Config, render_all_markdown_files, watcher::setup_file_watcher};
use anyhow::Result;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

/// Server state containing the broadcast channel for file changes
#[derive(Clone)]
pub struct ServerState {
    tx: Arc<broadcast::Sender<PathBuf>>,
}

/// Start the live-md server with the given configuration
pub async fn start_server(config: Config) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&config.output_dir)?;

    // Initial render of all markdown files
    render_all_markdown_files(&config.content_dir, &config.output_dir)?;

    // Set up broadcast channel for file changes
    let (tx, _) = broadcast::channel::<PathBuf>(config.broadcast_capacity);
    let tx = Arc::new(tx);

    // Set up file watcher
    let watcher_tx = tx.clone();
    let watcher_output_dir = config.output_dir.clone();
    setup_file_watcher(config.content_dir.clone(), watcher_output_dir, watcher_tx)?;

    // Build router with static file serving and SSE endpoint
    let app = Router::new()
        .route("/events", get(sse_handler))
        .nest_service("/", ServeDir::new(&config.output_dir))
        .with_state(ServerState { tx });

    // Create server address
    let addr = config.socket_addr();
    println!("Server starting at {}", config.server_url());

    // Open browser if configured
    if config.open_browser {
        if let Err(e) = webbrowser::open(&config.server_url()) {
            eprintln!("Failed to open browser: {}", e);
        }
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// SSE handler for live reload functionality
async fn sse_handler(
    State(state): State<ServerState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let mut rx = state.tx.subscribe();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::Duration;
    use futures_util::{FutureExt, StreamExt};
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_sse_handler() {
        let (tx, _) = broadcast::channel(16);
        let tx = Arc::new(tx);
        let state = ServerState { tx: tx.clone() };

        // Spawn SSE handler
        let _sse = sse_handler(State(state));

        // Send a test event
        tx.send(PathBuf::from("test.md")).unwrap();

        // Sleep briefly to allow event processing
        sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_server_setup() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");

        std::fs::create_dir_all(&content_dir)?;

        let config = Config::new(
            content_dir,
            output_dir,
            0, // Use port 0 for automatic port assignment
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            false, // Don't open browser in test
            16,
        );

        // Start server in background task
        let server_handle = tokio::spawn(start_server(config.clone()));

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Try to connect to the server
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://127.0.0.1:{}", config.port))
            .send()
            .await;

        // Cleanup
        server_handle.abort();

        // Check if server responded
        assert!(response.is_ok());

        Ok(())
    }
}
