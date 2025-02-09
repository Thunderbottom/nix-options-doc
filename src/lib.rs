pub use clap::Parser;
use git2::FetchOptions;
use rnix::{SyntaxKind, SyntaxNode};
use rowan::ast::AstNode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

#[cfg(test)]
mod tests {
    include!("tests/tests.rs");
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Local path or remote git repository URL to the nix configuration
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Path to output file or 'stdout'
    #[arg(short, long, default_value = "nix-options.md")]
    pub out: String,

    /// Whether the output names should be sorted
    #[arg(short, long)]
    pub sort: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionDoc {
    pub name: String,
    pub description: Option<String>,
    pub type_info: String,
    pub default_value: Option<String>,
    pub file_path: String,
}

// Clone repository to temporary directory or return local path
pub fn prepare_path(path: &str) -> Result<(PathBuf, Option<TempDir>), Box<dyn std::error::Error>> {
    if Path::new(path).exists() {
        return Ok((PathBuf::from(path), None));
    };

    let temp_dir = TempDir::new()?;
    let mut fo = FetchOptions::new();
    fo.depth(1).download_tags(git2::AutotagOption::None);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);
    match builder.clone(path, temp_dir.path()) {
        Ok(repo) => {
            let temp_path = repo
                .workdir()
                .ok_or("Repository work directory not found")?;
            Ok((temp_path.to_path_buf(), Some(temp_dir)))
        }
        Err(e) => match e.code() {
            git2::ErrorCode::Auth => {
                Err(format!("Failed to clone repository: {}, {}", path, e.message()).into())
            }
            _ => Err(format!("Not a valid local path or git repository: {}", path).into()),
        },
    }
}

// Walk through the directory structure and
// parse AST for every .nix file for module options.
pub fn collect_options(dir: &Path) -> Result<Vec<OptionDoc>, Box<dyn std::error::Error>> {
    let mut options = Vec::new();

    for entry in WalkDir::new(dir).follow_links(true).into_iter() {
        let entry = entry?;
        if is_hidden(&entry)
            || !entry.file_type().is_file()
            || entry.path().extension().is_none_or(|ext| ext != "nix")
        {
            continue;
        }

        let content = fs::read_to_string(entry.path())?;
        let parse = rnix::Root::parse(&content).ok()?;

        let relative_path = entry
            .path()
            .strip_prefix(&dir)?
            .to_string_lossy()
            .into_owned();

        visit_node(parse.syntax(), &relative_path, &mut options, "")?;
    }

    Ok(options)
}

// Check if the directory is a hidden directory.
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

// Recursively parse through the AST attributes
// and find children nodes to parse for option values.
fn visit_node(
    node: &SyntaxNode,
    file_path: &str,
    options: &mut Vec<OptionDoc>,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if node.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
        let key = node
            .children()
            .find(|n| n.kind() == SyntaxKind::NODE_ATTRPATH)
            .as_ref()
            .map(parse_attrpath);

        if let Some(value_node) = node.children().nth(1) {
            if let Some(key) = key {
                let new_prefix = if prefix.is_empty() {
                    key
                } else {
                    format!("{}.{}", prefix, key)
                };
                parse_attrset(&value_node, file_path, options, &new_prefix)?;
            }
        }
    } else {
        // Visit all children for other node types
        for child in node.children() {
            visit_node(&child, file_path, options, prefix)?;
        }
    }

    Ok(())
}

// Parse attribute path for option names.
fn parse_attrpath(node: &SyntaxNode) -> String {
    let mut path = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::NODE_IDENT {
            path.push(child.text().to_string());
        }
    }
    path.join(".")
}

// Parse attribute set and generate option documentation.
fn parse_attrset(
    node: &SyntaxNode,
    file_path: &str,
    options: &mut Vec<OptionDoc>,
    current_prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match node.kind() {
        // Nested attributes
        SyntaxKind::NODE_ATTR_SET => {
            for child in node.children() {
                visit_node(&child, file_path, options, current_prefix)?;
            }
        }
        // Child node, parse for mkOption or mkEnableOption
        SyntaxKind::NODE_APPLY => {
            let fn_name = node
                .children()
                .find(|n| n.kind() == SyntaxKind::NODE_SELECT)
                .and_then(|n| n.children().last())
                .map(|n| n.text().to_string());

            match fn_name.as_deref() {
                Some("mkEnableOption") => {
                    let description = node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::NODE_STRING)
                        .map(|n| n.text().to_string().trim_matches('"').to_string());

                    options.retain(|opt| opt.name != current_prefix);
                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        type_info: String::from("boolean"),
                        default_value: Some(String::from("false")),
                        file_path: file_path.to_string(),
                    });
                }
                Some("mkOption") => {
                    let mut type_info = String::from("any");
                    let mut description = None;
                    let mut default_value = None;

                    if let Some(attr_set) = node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::NODE_ATTR_SET)
                    {
                        for attr in attr_set.children() {
                            if attr.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                                let attr_key = attr
                                    .children()
                                    .find(|n| n.kind() == SyntaxKind::NODE_ATTRPATH)
                                    .and_then(|n| n.children().next())
                                    .map(|n| n.text().to_string());

                                let attr_value = attr.children().nth(1);

                                match (attr_key.as_deref(), attr_value) {
                                    (Some("type"), Some(v)) => {
                                        type_info = v.text().to_string();
                                    }
                                    (Some("description"), Some(v)) => {
                                        description = Some(
                                            v.text().to_string().trim_matches('"').to_string(),
                                        );
                                    }
                                    (Some("default"), Some(v)) => {
                                        default_value = Some(v.text().to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    options.retain(|opt| opt.name != current_prefix);
                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        type_info,
                        default_value,
                        file_path: file_path.to_string(),
                    });
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}

// Generate a markdown table with all the option values
pub fn generate_markdown(
    options: &[OptionDoc],
    sorted: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::from("# NixOS Module Options\n\n");

    // Set markdown table headers
    output.push_str("| Option | Type | Default | Description |\n");
    output.push_str("|--------|------|---------|-------------|\n");

    // Sort options by name for better readability
    let mut nix_options = options.to_vec();
    if sorted {
        nix_options.sort_by(|a, b| a.name.cmp(&b.name));
    }

    for option in nix_options {
        // Escape any pipe characters in the fields to avoid breaking table formatting
        let name = option.name.replace('|', "\\|");
        let file_path = option.file_path.replace('|', "\\|");
        let type_info = option.type_info.replace('|', "\\|");
        let default = option
            .default_value
            .map(|v| v.replace('|', "\\|"))
            .unwrap_or_else(|| "-".to_string());
        let description = option
            .description
            .map(|d| d.replace('|', "\\|"))
            .unwrap_or_else(|| "-".to_string());

        output.push_str(&format!(
            "| [`{}`]({}) | `{}` | `{}` | {} |\n",
            name, file_path, type_info, default, description
        ));
    }

    // Add a note about the source files
    output.push_str("\n\n*Generated from NixOS module declarations*\n");

    Ok(output)
}
