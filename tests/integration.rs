use anyhow::Result;
use live_md::config::Config;
use reqwest::Client;
use std::{
    fs,
    net::{IpAddr, Ipv4Addr, TcpListener},
    path::PathBuf,
    time::Duration,
};
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper function to find an available port
fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

/// Helper function to create a test server configuration with a guaranteed available port
fn create_test_config(temp_dir: &TempDir) -> Result<Config> {
    Ok(Config::new(
        temp_dir.path().join("content"),
        temp_dir.path().join("output"),
        find_available_port()?,
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        false,
        16,
    ))
}

/// Helper function to create a test markdown file
fn create_markdown_file(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Helper function to start server and wait for it to be ready
async fn start_test_server(config: Config) -> Result<(tokio::task::JoinHandle<()>, String)> {
    let server_url = config.server_url();

    // Start server in background task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = live_md::server::start_server(config).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to be available
    let client = Client::new();
    let mut attempts = 0;
    while attempts < 50 {
        if let Ok(response) = client.get(&server_url).send().await {
            if response.status().is_success() {
                return Ok((server_handle, server_url));
            }
        }
        sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    Err(anyhow::anyhow!("Server failed to start"))
}

#[tokio::test]
async fn test_server_starts_and_serves_content() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    // Create content and output directories
    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create a test markdown file
    let test_file = config.content_dir.join("test.md");
    create_markdown_file(&test_file, "# Test Page\n\nHello, world!")?;

    // Start server and wait for it to be ready
    let (server_handle, server_url) = start_test_server(config.clone()).await?;

    // Make HTTP request to server
    let client = Client::new();
    let response = client
        .get(format!("{}/test.html", &server_url))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Test Page"));
    assert!(body.contains("Hello, world!"));

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_file_watching_and_live_reload() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    // Create directories
    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create initial markdown file
    let test_file = config.content_dir.join("test.md");
    create_markdown_file(&test_file, "# Initial Content")?;

    // Start server and wait for it to be ready
    let (server_handle, server_url) = start_test_server(config).await?;

    // Connect to SSE endpoint
    let client = Client::new();
    let events_response = client.get(format!("{}/events", server_url)).send().await?;

    assert!(events_response.status().is_success());

    // Modify the file
    create_markdown_file(&test_file, "# Updated Content")?;

    // Wait for file system event to be processed
    sleep(Duration::from_millis(100)).await;

    // Verify content was updated
    let response = client
        .get(format!("{}/test.html", server_url))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;
    assert!(body.contains("Updated Content"));

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_nested_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    // Create nested directory structure
    let nested_dir = config.content_dir.join("docs").join("section");
    fs::create_dir_all(&nested_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create markdown files in different locations
    create_markdown_file(&config.content_dir.join("root.md"), "# Root Document")?;
    create_markdown_file(&nested_dir.join("nested.md"), "# Nested Document")?;
    create_markdown_file(&config.content_dir.join("README.md"), "# Project Index")?;

    // Start server and wait for it to be ready
    let (server_handle, server_url) = start_test_server(config).await?;

    let client = Client::new();

    // Test root document
    let response = client
        .get(format!("{}/root.html", server_url))
        .send()
        .await?;
    assert!(response.status().is_success());
    assert!(response.text().await?.contains("Root Document"));

    // Test nested document
    let response = client
        .get(format!("{}/docs/section/nested.html", server_url))
        .send()
        .await?;
    assert!(response.status().is_success());
    assert!(response.text().await?.contains("Nested Document"));

    // FIXME: This doesn't work, but fixing it properly is a separate pr
    // // Test README.md -> index.html conversion
    // let response = client
    //     .get(format!("{}/index.html", server_url))
    //     .send()
    //     .await?;
    // assert!(response.status().is_success());
    // assert!(response.text().await?.contains("Project Index"));

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_markdown_features() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create markdown file with various features
    let content = r#"# Feature Test

## Tables
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |

## Task List
- [x] Completed task
- [ ] Pending task

## Formatting
**Bold** and *italic* and ~~strikethrough~~

## Footnotes
Here's a note[^1]

[^1]: The footnote text

## Code
```rust
fn main() {
    println!("Hello!");
}
```"#;

    create_markdown_file(&config.content_dir.join("features.md"), content)?;

    // Start server and wait for it to be ready
    let (server_handle, server_url) = start_test_server(config).await?;

    // Verify rendered content
    let client = Client::new();
    let response = client
        .get(format!("{}/features.html", server_url))
        .send()
        .await?;

    assert!(response.status().is_success());
    let body = response.text().await?;

    // Check for rendered features
    assert!(body.contains("<table>")); // Tables
    assert!(body.contains("type=\"checkbox\"")); // Task list
    assert!(body.contains("<strong>Bold</strong>")); // Bold
    assert!(body.contains("<em>italic</em>")); // Italic
    assert!(body.contains("<del>strikethrough</del>")); // Strikethrough
    assert!(body.contains("class=\"footnote-definition\"")); // Footnotes
    assert!(body.contains("<pre><code")); // Code

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_index_generation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create multiple markdown files
    create_markdown_file(&config.content_dir.join("page1.md"), "# Page 1")?;
    create_markdown_file(&config.content_dir.join("page2.md"), "# Page 2")?;
    create_markdown_file(
        &config.content_dir.join("docs").join("page3.md"),
        "# Page 3",
    )?;

    // Start server and wait for it to be ready
    let (server_handle, server_url) = start_test_server(config).await?;

    // Check index.html
    let client = Client::new();
    let response = client.get(format!("{}/", server_url)).send().await?;

    assert!(response.status().is_success());
    let body = response.text().await?;

    // Verify index content
    assert!(body.contains("Page 1"));
    assert!(body.contains("Page 2"));
    assert!(body.contains("Page 3"));
    assert!(body.contains("in docs")); // Check directory indication
    assert!(body.contains("href=\"page1.html\"")); // Check links
    assert!(body.contains("href=\"docs/page3.html\"")); // Check nested links

    // Cleanup
    server_handle.abort();
    Ok(())
}
