use crate::error::NixDocError;
use crate::OptionDoc;
use comrak::{markdown_to_html, ComrakOptions};

// Define CSS styles as a constant to keep the main function clean
const HTML_TEMPLATE_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>NixOS Module Options</title>
    <style>
        body { 
            font-family: system-ui, -apple-system, sans-serif; 
            margin: 40px auto; 
            max-width: 800px; 
            line-height: 1.6; 
            color: #333; 
            padding: 0 10px; 
        }
        h1 { margin-bottom: 1.5em; }
        .option { 
            margin-bottom: 2.5em; 
            padding-bottom: 1.5em; 
            border-bottom: 1px solid #eee; 
        }
        h2 { margin-top: 0; }
        .option-name { font-family: monospace; }
        a { color: #0366d6; text-decoration: none; }
        a:hover { text-decoration: underline; }
        pre { 
            background-color: #f6f8fa; 
            padding: 16px; 
            border-radius: 6px; 
            overflow: auto; 
        }
        code {
            font-family: ui-monospace, monospace;
            background-color: rgba(175, 184, 193, 0.2);
            padding: 0.2em 0.4em;
            border-radius: 3px;
        }
        pre code {
            background-color: transparent;
            padding: 0;
            border-radius: 0;
            font-family: inherit;
        }
        .metadata { margin-top: 1em; }
        .footer { 
            margin-top: 3em; 
            text-align: center; 
            color: #666; 
            font-size: 0.9em; 
        }
        .code-container {
            margin-top: 0.5em;
            margin-bottom: 0.5em;
        }
        .code-multiline {
            background-color: #f6f8fa;
            border-radius: 6px;
            padding: 1em;
            margin: 0;
            overflow: auto;
            font-family: ui-monospace, monospace;
        }
        .markdown-alert {
            padding: 0.5rem 1rem;
            margin-bottom: 16px;
            border-radius: 6px;
            border-left: 0.25rem solid #d0d7de;
            background-color: #f6f8fa;
        }
        .markdown-alert p {
            margin: 0.5rem 0;
        }
        .markdown-alert-title {
            font-weight: bold;
            margin-bottom: 0.5rem !important;
            text-transform: uppercase;
        }
        .markdown-alert-note {
            border-left-color: #1F6FEB;
            background-color: rgba(31, 111, 235, 0.1);
        }
        .markdown-alert-tip {
            border-left-color: #2DA44E;
            background-color: rgba(45, 164, 78, 0.1);
        }
        .markdown-alert-important {
            border-left-color: #8250DF;
            background-color: rgba(130, 80, 223, 0.1);
        }
        .markdown-alert-warning {
            border-left-color: #9A6700;
            background-color: rgba(154, 103, 0, 0.1);
        }
        .markdown-alert-caution {
            border-left-color: #CF222E;
            background-color: rgba(207, 34, 46, 0.1);
        }
    </style>
</head>
<body>
    <h1>NixOS Module Options</h1>
"#;

/// Formats a multiline code block for HTML output with proper syntax highlighting.
///
/// # Arguments
/// - `label`: The display label for the code block section.
/// - `content`: The code content to be displayed in the block.
///
/// # Returns
/// A formatted HTML string with proper escaping and CSS styling.
fn format_multiline_block(label: &str, content: &str) -> String {
    let escaped_content = html_escape::encode_text(content);
    format!(
        r#"        <div class="metadata">
            <strong>{label}:</strong>
            <div class="code-container">
                <pre class="code-multiline"><code>{escaped_content}</code></pre>
            </div>
        </div>
"#
    )
}

/// Formats a single line code reference for HTML output with inline styling.
///
/// # Arguments
/// - `label`: The display label for the code reference.
/// - `content`: The code content to be displayed inline.
///
/// # Returns
/// A formatted HTML string with proper escaping and CSS styling for inline code.
fn format_inline_code(label: &str, content: &str) -> String {
    let escaped_content = html_escape::encode_text(content);
    format!(
        r#"        <div class="metadata">
            <strong>{label}:</strong> <code>{escaped_content}</code>
        </div>
"#
    )
}

/// Generates an HTML document containing comprehensive documentation for NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries to render as HTML.
///
/// # Returns
/// A `Result` containing the complete HTML document with styling and navigation or an error.
pub fn generate_html(options: &[OptionDoc]) -> Result<String, NixDocError> {
    let mut output = String::with_capacity(options.len() * 800 + 500);
    output.push_str(HTML_TEMPLATE_HEADER);

    // Set up markdown rendering options
    let mut comrak_options = ComrakOptions::default();
    comrak_options.extension.strikethrough = true;
    comrak_options.extension.table = true;
    comrak_options.extension.autolink = true;
    comrak_options.extension.tasklist = true;
    comrak_options.extension.alerts = true;
    comrak_options.render.unsafe_ = true; // Allow HTML in markdown (if needed)

    // Generate option entries
    for option in options {
        // Create a slug for the option ID from the name
        let slug = option.name.replace(['.', ':'], "-");

        // Start option section
        output.push_str(&format!(
            r#"    <div class="option" id="{}">
        <h2><a href="{}#L{}" class="option-name">{}</a></h2>
"#,
            html_escape::encode_text(&slug),
            html_escape::encode_text(&option.file_path),
            option.line_number,
            html_escape::encode_text(&option.name)
        ));

        // Description with markdown conversion
        if let Some(description) = &option.description {
            let html_description = markdown_to_html(description, &comrak_options);
            output.push_str(&format!(
                r#"        <div class="metadata">
            {html_description}
        </div>
"#
            ));
        }

        // Type information
        if option.nix_type.contains('\n') || option.nix_type.len() > 72 {
            output.push_str(&format_multiline_block("Type", &option.nix_type));
        } else {
            output.push_str(&format_inline_code("Type", &option.nix_type));
        }

        // Default value if available
        if let Some(default) = &option.default_value {
            if default.contains('\n') || default.len() > 72 {
                output.push_str(&format_multiline_block("Default", default));
            } else {
                output.push_str(&format_inline_code("Default", default));
            }
        }

        // Example if available
        if let Some(example) = &option.example {
            if example.contains('\n') || example.len() > 72 {
                output.push_str(&format_multiline_block("Example", example));
            } else {
                output.push_str(&format_inline_code("Example", example));
            }
        }

        // Close option div
        output.push_str("    </div>\n\n");
    }

    // Add footer and close HTML
    output.push_str(&format!(
        r#"    <div class="footer">
        <p>Generated with <a href="{}">{}</a></p>
    </div>
</body>
</html>"#,
        option_env!("CARGO_PKG_REPOSITORY").unwrap_or(env!("CARGO_PKG_NAME")),
        env!("CARGO_PKG_NAME")
    ));

    Ok(output)
}
