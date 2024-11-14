pub mod config;
pub mod markdown;
pub mod server;
pub mod watcher;

use anyhow::Result;
use std::path::PathBuf;

/// Renders all markdown files in the content directory to HTML files in the output directory
pub fn render_all_markdown_files(
    content_dir: &std::path::Path,
    output_dir: &std::path::Path,
) -> Result<Vec<PathBuf>> {
    let mut markdown_files = Vec::new();
    collect_markdown_files(content_dir, content_dir, &mut markdown_files)?;

    // Render each markdown file
    for path in &markdown_files {
        markdown::render_markdown_file(path, output_dir)?;
    }

    // Generate index.html
    generate_index_html(output_dir, &markdown_files, content_dir)?;

    Ok(markdown_files)
}

/// Recursively collect markdown files from a directory
pub fn collect_markdown_files(
    current_dir: &std::path::Path,
    _base_dir: &std::path::Path,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    let entries = std::fs::read_dir(current_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(path);
        } else if path.is_dir() {
            collect_markdown_files(&path, _base_dir, files)?;
        }
    }

    Ok(())
}

/// Generate index.html with links to all rendered files
pub fn generate_index_html(
    output_dir: &std::path::Path,
    markdown_files: &[PathBuf],
    content_dir: &std::path::Path,
) -> Result<()> {
    let mut html_content = String::from(include_str!("templates/index-start.html"));

    // Sort files for consistent ordering
    let mut sorted_files = markdown_files.to_vec();
    sorted_files.sort();

    // Add links to each file
    for path in sorted_files {
        if let Ok(rel_path) = path.strip_prefix(content_dir) {
            let file_stem = rel_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let parent = rel_path.parent().and_then(|p| p.to_str()).unwrap_or("");

            // Create the HTML path, preserving directory structure
            let html_path = if !parent.is_empty() {
                format!("{}/{}.html", parent, file_stem)
            } else {
                format!("{}.html", file_stem)
            };

            // Create a display name, handling both file name and path
            let display_name = file_stem.replace('_', " ");
            let display_path = if !parent.is_empty() {
                format!("<span class=\"path\">in {}</span>", parent)
            } else {
                String::new()
            };

            html_content.push_str(&format!(
                "        <li><a href=\"{}\">{}{}</a></li>\n",
                html_path, display_name, display_path
            ));
        }
    }

    html_content.push_str(include_str!("templates/index-end.html"));

    // Write index.html to output directory
    let index_path = output_dir.join("index.html");
    std::fs::write(index_path, html_content)?;

    Ok(())
}

/// Sets up an HTML template with live reload capability
pub fn wrap_html_template(content: &str, title: &str) -> String {
    format!(
        "{}{}{}",
        include_str!("templates/page-start.html"),
        content,
        include_str!("templates/page-end.html")
    )
    .replace("{{title}}", title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_markdown_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path();

        // Create some test files
        fs::write(base_path.join("test1.md"), "# Test 1")?;
        fs::write(base_path.join("test2.md"), "# Test 2")?;
        fs::create_dir(base_path.join("subdir"))?;
        fs::write(base_path.join("subdir").join("test3.md"), "# Test 3")?;

        let mut files = Vec::new();
        collect_markdown_files(base_path, base_path, &mut files)?;

        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|p| p.extension().unwrap() == "md"));
        Ok(())
    }

    #[test]
    fn test_generate_index_html() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path();
        let output_dir = temp_dir.path();

        // Create test markdown files
        let files = vec![content_dir.join("test1.md"), content_dir.join("test2.md")];

        generate_index_html(output_dir, &files, content_dir)?;

        let index_content = fs::read_to_string(output_dir.join("index.html"))?;
        assert!(index_content.contains("test1"));
        assert!(index_content.contains("test2"));

        Ok(())
    }

    #[test]
    fn test_wrap_html_template() {
        let content = "<p>Test content</p>";
        let title = "Test Title";
        let result = wrap_html_template(content, title);

        assert!(result.contains(content));
        assert!(result.contains(title));
        assert!(result.contains("<!DOCTYPE html>"));
        assert!(result.contains("</html>"));
    }
}
