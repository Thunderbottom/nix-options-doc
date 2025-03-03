use crate::error::NixDocError;
use crate::OptionDoc;
use std::fmt::Write;

/// Generates a Markdown formatted string documenting NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
///
/// # Returns
/// A `Result` containing the generated Markdown string or an error.
pub fn generate_markdown(options: &[OptionDoc]) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::with_capacity(options.len() * 100 + 200); // Pre-allocate approximate size
    output.push_str("# NixOS Module Options\n\n");
    output.push_str("| Option | Type | Default | Description |\n");
    output.push_str("|--------|------|---------|-------------|\n");

    for option in options {
        // Escape pipe characters in fields
        let name = option.name.replace('|', "\\|");
        let file_path = option.file_path.replace('|', "\\|");
        let type_info = option.nix_type.to_string().replace('|', "\\|");
        let default = option
            .default_value
            .as_deref()
            .map(|v| v.replace('|', "\\|"))
            .unwrap_or_else(|| "-".to_string());
        let description = option
            .description
            .as_deref()
            .map(|d| d.replace('|', "\\|"))
            .unwrap_or_else(|| "-".to_string());

        writeln!(
            output,
            "| [`{}`]({}#L{}) | `{}` | `{}` | {} |",
            name, file_path, option.line_number, type_info, default, description
        )?;
    }

    writeln!(
        output,
        "\n\n*Generated with [{}]({})*",
        env!("CARGO_PKG_NAME"),
        option_env!("CARGO_PKG_REPOSITORY").unwrap_or(env!("CARGO_PKG_NAME"))
    )?;

    Ok(output)
}

/// Generates a pretty-printed JSON string documenting NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
///
/// # Returns
/// A `Result` containing the JSON string or a serialization error.
pub fn generate_json(options: &[OptionDoc]) -> Result<String, NixDocError> {
    serde_json::to_string_pretty(options).map_err(|e| NixDocError::Serialization(e.to_string()))
}

/// Generates an HTML document containing a table of NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
///
/// # Returns
/// A `Result` containing the generated HTML string or an error.
pub fn generate_html(options: &[OptionDoc]) -> Result<String, NixDocError> {
    let mut output = String::from(
        "<!DOCTYPE html>
<html>
<head>
    <meta charset=\"UTF-8\">
    <title>NixOS Module Options</title>
    <style>
        body { font-family: sans-serif; margin: 40px; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        tr:nth-child(even) { background-color: #f9f9f9; }
    </style>
</head>
<body>
    <h1>NixOS Module Options</h1>
    <table>
        <tr>
            <th>Option</th>
            <th>Type</th>
            <th>Default</th>
            <th>Description</th>
        </tr>\n",
    );

    for option in options {
        let name = html_escape::encode_text(&option.name);
        let file_path = html_escape::encode_text(&option.file_path);
        let type_string = option.nix_type.to_string();
        let type_info = html_escape::encode_text(&type_string);
        let default = option
            .default_value
            .as_deref()
            .map(html_escape::encode_text)
            .unwrap_or_else(|| "-".into());
        let description = option
            .description
            .as_deref()
            .map(html_escape::encode_text)
            .unwrap_or_else(|| "-".into());

        output.push_str(&format!(
            "        <tr>\n            <td><a href=\"{}#L{}\">{}</a></td>\n            <td><code>{}</code></td>\n            <td><code>{}</code></td>\n            <td>{}</td>\n        </tr>\n",
            file_path, option.line_number, name, type_info, default, description
        ));
    }

    output.push_str(&format!(
        "    </table>\n    <p><em>Generated with <a href=\"{}\">{}</a></em></p>\n</body>\n</html>",
        env!("CARGO_PKG_NAME"),
        option_env!("CARGO_PKG_REPOSITORY").unwrap_or(env!("CARGO_PKG_NAME"))
    ));

    Ok(output)
}

/// Generates a CSV formatted string documenting NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
///
/// # Returns
/// A `Result` containing the CSV string or an error.
pub fn generate_csv(options: &[OptionDoc]) -> Result<String, NixDocError> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);

    // Write header
    wtr.write_record(["Option", "Type", "Default", "Description", "FilePath"])?;

    for option in options {
        let default = option.default_value.as_deref().unwrap_or("-");
        let description = option.description.as_deref().unwrap_or("-");

        wtr.write_record([
            &option.name,
            &option.nix_type.to_string(),
            default,
            description,
            &option.file_path,
            &option.line_number.to_string(),
        ])?;
    }

    let data = wtr.into_inner()?;
    String::from_utf8(data).map_err(|e| NixDocError::Serialization(e.to_string()))
}
