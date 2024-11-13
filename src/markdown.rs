use anyhow::Result;
use pulldown_cmark::{html, Options, Parser};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Renders a markdown file to HTML and saves it to the output directory
pub fn render_markdown_file(markdown_path: &Path, output_dir: &Path) -> Result<()> {
    // Read markdown content
    let markdown_content = fs::read_to_string(markdown_path)?;

    // Generate HTML content
    let html_content = markdown_to_html(&markdown_content);

    // Generate full HTML document
    let final_html = wrap_with_template(&html_content, markdown_path);

    // Determine output path
    let output_path = get_output_path(markdown_path, output_dir);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write HTML file
    fs::write(output_path, final_html)?;

    Ok(())
}

/// Converts markdown text to HTML
fn markdown_to_html(markdown: &str) -> String {
    // Set up options for markdown parsing
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Parse markdown
    let parser = Parser::new_ext(markdown, options);

    // Convert to HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

/// Wraps HTML content in a complete HTML document with necessary styles and scripts
fn wrap_with_template(content: &str, source_path: &Path) -> String {
    let title = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Markdown Preview")
        .replace('_', " ");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 1rem;
            color: #333;
        }}
        pre, code {{
            background-color: #f6f8fa;
            border-radius: 3px;
            padding: 0.2em 0.4em;
            font-family: SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace;
        }}
        pre code {{
            padding: 0;
        }}
        pre {{
            padding: 16px;
            overflow: auto;
        }}
        blockquote {{
            margin: 0;
            padding-left: 1em;
            border-left: 4px solid #ddd;
            color: #666;
        }}
        img {{
            max-width: 100%;
            height: auto;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
        }}
        th, td {{
            border: 1px solid #ddd;
            padding: 8px;
            text-align: left;
        }}
        th {{
            background-color: #f6f8fa;
        }}
        h1, h2, h3, h4, h5, h6 {{
            margin-top: 24px;
            margin-bottom: 16px;
            font-weight: 600;
            line-height: 1.25;
        }}
        a {{
            color: #0366d6;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
    </style>
    <script>
        // Set up SSE for live reload
        const events = new EventSource('/events');
        events.onmessage = (e) => {{
            if (e.data === 'reload') {{
                window.location.reload();
            }}
        }};
    </script>
</head>
<body>
    {content}
</body>
</html>"#
    )
}

/// Determines the output HTML path for a given markdown path
fn get_output_path(markdown_path: &Path, output_dir: &Path) -> PathBuf {
    // Get the file stem (file name without extension)
    let file_stem = markdown_path.file_stem().unwrap_or_default();

    // Handle README.md as a special case
    if markdown_path.file_name().unwrap_or_default() == "README.md" {
        return output_dir.join("index.html");
    }

    // Get the relative directory structure
    if let Some(Ok(content_dir)) = markdown_path
        .parent()
        .map(|p| p.strip_prefix(Path::new(crate::CONTENT_DIR)))
    {
        // Preserve directory structure
        if !content_dir.as_os_str().is_empty() {
            let output_path = output_dir.join(content_dir);
            fs::create_dir_all(&output_path).unwrap_or_default();
            output_path.join(file_stem).with_extension("html")
        } else {
            // File is in the root content directory
            output_dir.join(file_stem).with_extension("html")
        }
    } else {
        // Fallback: just put the file in the output directory
        output_dir.join(file_stem).with_extension("html")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_markdown_to_html() {
        let markdown = "# Hello\n\nThis is a **test**";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<h1>"));
        assert!(html.contains("<strong>"));
    }

    #[test]
    fn test_get_output_path() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        // Test README.md -> index.html
        let readme_path = Path::new("content/README.md");
        let output_path = get_output_path(readme_path, output_dir);
        assert_eq!(output_path.file_name().unwrap(), "index.html");

        // Test regular markdown file
        let markdown_path = Path::new("content/test.md");
        let output_path = get_output_path(markdown_path, output_dir);
        assert_eq!(output_path.file_name().unwrap(), "test.html");
    }
}
