use anyhow::Result;
use live_md::config::Config;
use reqwest::Client;
use std::{
    fs,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    time::Duration,
};
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper function to create a test server configuration
fn create_test_config(temp_dir: &TempDir) -> Config {
    Config::new(
        temp_dir.path().join("content"),
        temp_dir.path().join("output"),
        0, // Port 0 means OS will assign a random available port
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        false, // Don't open browser in tests
        16,
    )
}

/// Helper function to create a test markdown file
fn create_markdown_file(path: &PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[tokio::test]
async fn test_server_starts_and_serves_content() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir);
    
    // Create content and output directories
    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create a test markdown file
    let test_file = config.content_dir.join("test.md");
    create_markdown_file(&test_file, "# Test Page\n\nHello, world!")?;

    // Start server in background task
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        live_md::server::start_server(server_config).await
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Make HTTP request to server
    let client = Client::new();
    let response = client
        .get(config.server_url())
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
    let config = create_test_config(&temp_dir);
    
    // Create directories
    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create initial markdown file
    let test_file = config.content_dir.join("test.md");
    create_markdown_file(&test_file, "# Initial Content")?;

    // Start server
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        live_md::server::start_server(server_config).await
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Connect to SSE endpoint
    let client = Client::new();
    let events_response = client
        .get(format!("{}/events", config.server_url()))
        .send()
        .await?;

    assert!(events_response.status().is_success());

    // Modify the file
    create_markdown_file(&test_file, "# Updated Content")?;

    // Wait for file system event to be processed
    sleep(Duration::from_millis(100)).await;

    // Verify content was updated
    let response = client
        .get(format!("{}/test.html", config.server_url()))
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
    let config = create_test_config(&temp_dir);
    
    // Create nested directory structure
    let nested_dir = config.content_dir.join("docs").join("section");
    fs::create_dir_all(&nested_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create markdown files in different locations
    create_markdown_file(
        &config.content_dir.join("root.md"),
        "# Root Document",
    )?;
    create_markdown_file(
        &nested_dir.join("nested.md"),
        "# Nested Document",
    )?;
    create_markdown_file(
        &config.content_dir.join("README.md"),
        "# Project Index",
    )?;

    // Start server
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        live_md::server::start_server(server_config).await
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    let client = Client::new();

    // Test root document
    let response = client
        .get(format!("{}/root.html", config.server_url()))
        .send()
        .await?;
    assert!(response.status().is_success());
    assert!(response.text().await?.contains("Root Document"));

    // Test nested document
    let response = client
        .get(format!("{}/docs/section/nested.html", config.server_url()))
        .send()
        .await?;
    assert!(response.status().is_success());
    assert!(response.text().await?.contains("Nested Document"));

    // Test README.md -> index.html conversion
    let response = client
        .get(format!("{}/index.html", config.server_url()))
        .send()
        .await?;
    assert!(response.status().is_success());
    assert!(response.text().await?.contains("Project Index"));

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_markdown_features() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir);
    
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

    // Start server
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        live_md::server::start_server(server_config).await
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Verify rendered content
    let client = Client::new();
    let response = client
        .get(format!("{}/features.html", config.server_url()))
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
    assert!(body.contains("<code>")) ; // Code blocks

    // Cleanup
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_index_generation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir);
    
    fs::create_dir_all(&config.content_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    // Create multiple markdown files
    create_markdown_file(&config.content_dir.join("page1.md"), "# Page 1")?;
    create_markdown_file(&config.content_dir.join("page2.md"), "# Page 2")?;
    create_markdown_file(&config.content_dir.join("docs").join("page3.md"), "# Page 3")?;

    // Start server
    let server_config = config.clone();
    let server_handle = tokio::spawn(async move {
        live_md::server::start_server(server_config).await
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Check index.html
    let client = Client::new();
    let response = client
        .get(format!("{}/", config.server_url()))
        .send()
        .await?;

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
