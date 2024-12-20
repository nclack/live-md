use anyhow::{Context, Result};
use pulldown_cmark::{html, Options, Parser};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Configuration for markdown parsing
#[derive(Debug, Clone, Default)]
pub struct MarkdownOptions {
    pub disable_tables: bool,
    pub disable_footnotes: bool,
    pub disable_strikethrough: bool,
    pub disable_tasklists: bool,
    pub disable_smart_punctuation: bool,
}

impl MarkdownOptions {
    fn to_parser_options(&self) -> Options {
        let mut options = Options::all();
        if self.disable_tables {
            options.remove(Options::ENABLE_TABLES);
        }
        if self.disable_footnotes {
            options.remove(Options::ENABLE_FOOTNOTES);
        }
        if self.disable_strikethrough {
            options.remove(Options::ENABLE_STRIKETHROUGH);
        }
        if self.disable_tasklists {
            options.remove(Options::ENABLE_TASKLISTS);
        }
        if self.disable_smart_punctuation {
            options.remove(Options::ENABLE_SMART_PUNCTUATION);
        }
        options
    }
}

/// Renders a markdown file to HTML and saves it to the output directory
pub fn render_markdown_file(markdown_path: &Path, output_dir: &Path) -> Result<PathBuf> {
    // Read markdown content
    let markdown_content = fs::read_to_string(markdown_path)
        .with_context(|| format!("Failed to read markdown file: {}", markdown_path.display()))?;

    // Generate HTML content with default options
    let html_content = markdown_to_html(&markdown_content, &MarkdownOptions::default());

    // Generate full HTML document
    let final_html = wrap_html_template(&html_content, markdown_path)?;

    // Determine output path"
    let output_path = get_output_path(markdown_path, output_dir)?;

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Write HTML file
    fs::write(&output_path, final_html)
        .with_context(|| format!("Failed to write HTML file: {}", output_path.display()))?;

    Ok(output_path)
}

/// Converts markdown text to HTML with specified options
pub fn markdown_to_html(markdown: &str, options: &MarkdownOptions) -> String {
    let parser = Parser::new_ext(markdown, options.to_parser_options());
    let mut html_output = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}

/// Wraps HTML content in a complete HTML document with styling
fn wrap_html_template(content: &str, source_path: &Path) -> Result<String> {
    let title = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Markdown Preview")
        .replace('_', " ");

    Ok(crate::wrap_html_template(content, &title))
}

/// Determines the output HTML path for a given markdown path
fn get_output_path(markdown_path: &Path, output_dir: &Path) -> Result<PathBuf> {
    let file_stem = markdown_path
        .file_stem()
        .with_context(|| format!("Invalid markdown path: {}", markdown_path.display()))?;

    // Handle README.md as a special case
    if markdown_path.file_name().unwrap_or_default() == "README.md" {
        return Ok(output_dir.join("index.html"));
    }

    // Get the parent directory of the markdown file
    if let Some(parent) = markdown_path.parent() {
        // Try to find common root directory by walking up the parent directories
        let mut current = parent;
        while let Some(parent_dir) = current.parent() {
            if parent_dir
                .file_name()
                .map_or(false, |name| name == "content" || name == "doc")
            {
                // Found the content root, now we can get the relative path
                if let Ok(rel_path) = parent.strip_prefix(parent_dir) {
                    let output_path = output_dir.join(rel_path);
                    fs::create_dir_all(&output_path)?;
                    return Ok(output_path.join(file_stem).with_extension("html"));
                }
            }
            current = parent_dir;
        }
    }

    // If no content root found or other issues, place in output root
    Ok(output_dir.join(file_stem).with_extension("html"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_markdown_to_html_basic() {
        let options = MarkdownOptions::default();
        let markdown = "# Hello\n\nThis is a **test**";
        let html = markdown_to_html(markdown, &options);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>test</strong>"));
    }

    #[test]
    fn test_markdown_options() {
        let options = MarkdownOptions::default();
        let markdown = "| Header |\n|--------|\n| Cell   |";
        let html = markdown_to_html(markdown, &options);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Header</th>"));
        assert!(html.contains("<td>Cell</td>"));
    }

    #[test]
    fn test_render_markdown_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&content_dir)?;
        fs::create_dir_all(&output_dir)?;

        // Create a test markdown file
        let markdown_path = content_dir.join("test.md");
        fs::write(&markdown_path, "# Test Heading\n\nTest content")?;

        // Render the file
        let output_path = render_markdown_file(&markdown_path, &output_dir)?;

        // Verify the output
        assert!(output_path.exists());
        let html_content = fs::read_to_string(output_path)?;
        assert!(html_content.contains("<h1>Test Heading</h1>"));
        assert!(html_content.contains("Test content"));
        assert!(html_content.contains("<!DOCTYPE html>"));

        Ok(())
    }

    #[test]
    fn test_readme_special_case() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&content_dir)?;
        fs::create_dir_all(&output_dir)?;

        // Create a README.md file
        let readme_path = content_dir.join("README.md");
        fs::write(&readme_path, "# Project README")?;

        // Render the file
        let output_path = render_markdown_file(&readme_path, &output_dir)?;

        // Verify it was rendered as index.html
        assert_eq!(output_path.file_name().unwrap(), "index.html");
        assert!(output_path.exists());

        Ok(())
    }

    #[test]
    fn test_nested_directory_structure() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let content_dir = temp_dir.path().join("content");
        let output_dir = temp_dir.path().join("output");
        let nested_dir = content_dir.join("docs").join("section");

        fs::create_dir_all(&nested_dir)?;
        std::env::set_current_dir(&temp_dir)?;

        // Create a nested markdown file
        let markdown_path = nested_dir.join("nested.md");
        fs::write(&markdown_path, "# Nested Content")?;

        // Render the file
        let output_path = render_markdown_file(&markdown_path, &output_dir)?;

        // Verify directory structure is preserved
        assert!(output_path.starts_with(&output_dir));
        assert!(output_path.to_string_lossy().contains("docs"));
        assert!(output_path.to_string_lossy().contains("section"));
        assert_eq!(output_path.file_name().unwrap(), "nested.html");

        Ok(())
    }

    #[test]
    fn test_markdown_special_features() {
        let mut options = MarkdownOptions::default();

        // Test tables
        let table = "| Header |\n|--------|\n| Cell   |";
        assert!(markdown_to_html(table, &options).contains("<table>"));

        // Test footnotes
        let footnote = "Text with a footnote[^1]\n\n[^1]: The footnote text";
        assert!(markdown_to_html(footnote, &options).contains("class=\"footnote-definition\""));

        // Test strikethrough
        let strike = "~~struck through~~";
        assert!(markdown_to_html(strike, &options).contains("<del>"));

        // Test tasklists
        let task = "- [ ] Unchecked\n- [x] Checked";
        let html = markdown_to_html(task, &options);
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked"));

        // Test with features disabled
        options.disable_tables = true;
        assert!(!markdown_to_html(table, &options).contains("<table>"));
    }

    #[test]
    fn test_code_block_rendering() {
        let options = MarkdownOptions::default();
        let markdown = "\n```rust\nfn main() {}\n```";
        let html = dbg!(markdown_to_html(markdown, &options));
        assert!(html.contains("<pre><code"));
        assert!(html.contains("class=\"language-rust\""));
    }
}
