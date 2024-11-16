use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;

use crate::markdown::render_markdown_file;

/// Sets up a file watcher for markdown files in the content directory
pub fn setup_file_watcher(
    content_dir: PathBuf,
    output_dir: PathBuf,
    tx: Arc<broadcast::Sender<PathBuf>>,
) -> Result<()> {
    let mut watcher =
        create_watcher(output_dir, tx.clone()).context("Failed to create file watcher")?;

    // Start watching content directory
    watcher
        .watch(&content_dir, RecursiveMode::Recursive)
        .context("Failed to start watching content directory")?;

    // Keep watcher alive by moving it into a spawned task
    tokio::spawn(async move {
        let _watcher = watcher;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    Ok(())
}

/// Creates a new file watcher with the specified configuration
fn create_watcher(
    output_dir: PathBuf,
    tx: Arc<broadcast::Sender<PathBuf>>,
) -> Result<RecommendedWatcher> {
    let config = Config::default()
        .with_compare_contents(true) // Detect content changes
        .with_poll_interval(Duration::from_secs(1));

    RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            handle_fs_event(res, &output_dir, &tx);
        },
        config,
    )
    .context("Failed to create watcher with config")
}

/// Handles file system events for markdown files
fn handle_fs_event(
    res: Result<Event, notify::Error>,
    output_dir: &Path,
    tx: &Arc<broadcast::Sender<PathBuf>>,
) {
    match res {
        Ok(event) => {
            // Filter events to only handle relevant ones
            if !is_relevant_event(&event) {
                return;
            }

            for path in event.paths {
                if path.extension().map_or(false, |ext| ext == "md") {
                    // Render markdown to HTML
                    if let Err(e) = render_markdown_file(&path, output_dir) {
                        eprintln!("Error rendering markdown: {}", e);
                    }
                    // Notify clients
                    if let Err(e) = tx.send(path) {
                        eprintln!("Error broadcasting change: {}", e);
                    }
                }
            }
        }
        Err(e) => eprintln!("Watch error: {}", e),
    }
}

/// Determines if a file system event is relevant for processing
fn is_relevant_event(event: &Event) -> bool {
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    matches!(
        event.kind,
        EventKind::Create(CreateKind::File)
            | EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Remove(RemoveKind::File)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_watcher_file_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&content_dir)?;
        fs::create_dir_all(&output_dir)?;

        let (tx, mut rx) = broadcast::channel(16);
        let tx = Arc::new(tx);

        // Setup watcher
        setup_file_watcher(content_dir.clone(), output_dir.clone(), tx)?;

        // Create a new markdown file
        let test_file = content_dir.join("test.md");
        fs::write(&test_file, "# Test")?;

        // Wait for the watcher to process the file
        let received_path = tokio::select! {
            _ = sleep(Duration::from_secs(2)) => {
                panic!("Timeout waiting for file change event");
            }
            result = rx.recv() => {
                result.expect("Failed to receive file change event")
            }
        };

        assert_eq!(received_path.canonicalize()?, test_file.canonicalize()?);

        // Check if HTML was generated
        let html_file = output_dir.join("test.html");
        assert!(html_file.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_watcher_file_modification() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&content_dir)?;
        fs::create_dir_all(&output_dir)?;

        let (tx, mut rx) = broadcast::channel(16);
        let tx = Arc::new(tx);

        // setup watcher
        setup_file_watcher(content_dir.clone(), output_dir.clone(), tx.clone())?;

        // create initial file and ensure it's synced to disk
        let test_file = content_dir.join("test.md");
        fs::write(&test_file, "# initial content")?;

        // wait for initial file creation to be processed
        let _ = rx.recv().await;

        // add delay to ensure initial rendering completes
        sleep(Duration::from_millis(100)).await;

        // modify the file and ensure it's synced to disk
        fs::write(&test_file, "# modified content")?;

        // wait for the modification event
        let received_path = tokio::select! {
            _ = sleep(Duration::from_secs(2)) => {
                panic!("timeout waiting for file modification event");
            }
            result = rx.recv() => {
                result.expect("failed to receive file modification event")
            }
        };

        assert_eq!(received_path.canonicalize()?, test_file.canonicalize()?);

        // add delay to ensure modification rendering completes
        sleep(Duration::from_millis(100)).await;
        // verify html content was updated
        let html_content = fs::read_to_string(output_dir.join("test.html"))?;
        assert!(html_content.contains("modified content"));

        Ok(())
    }

    #[test]
    fn test_is_relevant_event() {
        use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

        // Test create events
        assert!(is_relevant_event(&Event::new(EventKind::Create(
            CreateKind::File
        ))));
        assert!(!is_relevant_event(&Event::new(EventKind::Create(
            CreateKind::Folder
        ))));

        // Test modify events
        assert!(is_relevant_event(&Event::new(EventKind::Modify(
            ModifyKind::Data(notify::event::DataChange::Content)
        ))));
        assert!(is_relevant_event(&Event::new(EventKind::Modify(
            ModifyKind::Name(notify::event::RenameMode::From)
        ))));

        // Test remove events
        assert!(is_relevant_event(&Event::new(EventKind::Remove(
            RemoveKind::File
        ))));
        assert!(!is_relevant_event(&Event::new(EventKind::Remove(
            RemoveKind::Folder
        ))));

        // Test irrelevant events
        assert!(!is_relevant_event(&Event::new(EventKind::Access(
            AccessKind::Read
        ))));
    }
}
