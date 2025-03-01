pub use clap::{Parser, ValueEnum};
pub mod error;
pub mod generate;
pub mod types;

use crate::error::NixDocError;
use crate::types::NixType;
use rnix::{SyntaxKind, SyntaxNode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use walkdir::WalkDir;

#[cfg(test)]
mod tests {
    include!("tests/tests.rs");
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Markdown,
    Json,
    Html,
    Csv,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Local path or remote git repository URL to the nix configuration
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Path to the output file or 'stdout'
    #[arg(short, long, default_value = "nix-options.md")]
    pub out: String,

    /// Output format
    #[arg(short = 'f', long, default_value = "markdown")]
    pub format: OutputFormat,

    /// Whether the output names should be sorted
    #[arg(short, long)]
    pub sort: bool,

    /// Git branch or tag to use (if repository URL provided)
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Git commit depth (set to 1 for shallow clone)
    #[arg(short, long, default_value = "1")]
    pub depth: u32,

    /// Filter options by prefix (e.g. "services.nginx")
    #[arg(long)]
    pub prefix: Option<String>,

    /// Replace nix variable with the specified value in option paths
    /// (can be used multiple times)
    /// Format: --replace key=value
    #[arg(long, value_parser = parse_key_value)]
    pub replace: Vec<(String, String)>,

    /// Search in option names and descriptions
    #[arg(long)]
    pub search: Option<String>,

    /// Filter options by type (e.g. "bool", "string")
    #[arg(long)]
    pub type_filter: Option<String>,

    /// Only show options that have a default value
    #[arg(long)]
    pub has_default: bool,

    /// Only show options that have a description
    #[arg(long)]
    pub has_description: bool,

    /// Directories to exclude from processing (can be specified multiple times)
    #[arg(short = 'e', long, value_delimiter = ',')]
    pub exclude_dir: Vec<String>,

    /// Show progress bar
    #[arg(short = 'P', long)]
    pub progress: bool,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionDoc {
    pub name: String,
    pub description: Option<String>,
    pub nix_type: NixType,
    pub default_value: Option<String>,
    pub file_path: String,
    pub line_number: usize,
}

// Parse a key=value string into a tuple
// Used to replace dynamic variable definitions in nix files
fn parse_key_value(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 || parts[0].is_empty() {
        return Err(format!("Invalid key=value format: {}", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

// Parse and filter module options based on path, option type,
// default value, description
pub fn filter_options(options: &[OptionDoc], cli: &Cli) -> Vec<OptionDoc> {
    let mut filtered = options.to_vec();

    // Filter by prefix
    if let Some(ref prefix) = cli.prefix {
        filtered.retain(|opt| opt.name.starts_with(prefix));
    }

    // Filter by type
    if let Some(ref type_str) = cli.type_filter {
        filtered.retain(|opt| {
            let type_info = opt.nix_type.to_string().to_lowercase();
            type_info.contains(&type_str.to_lowercase())
        });
    }

    // Filter by search text
    if let Some(ref search) = cli.search {
        let search_lower = search.to_lowercase();
        filtered.retain(|opt| {
            opt.name.to_lowercase().contains(&search_lower)
                || opt
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
        });
    }

    // Filter by having default value
    if cli.has_default {
        filtered.retain(|opt| opt.default_value.is_some());
    }

    // Filter by having description
    if cli.has_description {
        filtered.retain(|opt| opt.description.is_some());
    }

    filtered
}

// Clone repository to temporary directory or return local path
pub fn prepare_path(cli: &Cli) -> Result<(PathBuf, Option<TempDir>), NixDocError> {
    let path = &cli.path;

    if Path::new(path).exists() {
        return Ok((PathBuf::from(path), None));
    };

    let temp_dir = TempDir::new()?;
    let mut fo = git2::FetchOptions::new();
    fo.depth(cli.depth as i32)
        .download_tags(git2::AutotagOption::None);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    match builder.clone(path, temp_dir.path()) {
        Ok(repo) => {
            let temp_path = repo.workdir().ok_or(NixDocError::NoWorkDir)?;

            // Checkout specific branch/tag if requested
            if let Some(ref branch) = cli.branch {
                let obj = repo
                    .revparse_single(&format!("origin/{}", branch))
                    .or_else(|_| repo.revparse_single(branch))?;

                let mut checkout_builder = git2::build::CheckoutBuilder::new();
                checkout_builder.force();

                repo.checkout_tree(&obj, Some(&mut checkout_builder))?;
                repo.set_head_detached(obj.id())?;
            }

            Ok((temp_path.to_path_buf(), Some(temp_dir)))
        }
        Err(e) => match e.code() {
            git2::ErrorCode::Auth => Err(NixDocError::GitClone(
                path.to_string(),
                e.message().to_string(),
            )),
            _ => Err(NixDocError::InvalidPath(path.to_string())),
        },
    }
}

// Walk through the directory structure and
// parse AST for every .nix file for module options.
pub fn collect_options(
    dir: &Path,
    exclude_dirs: &[String],
    replacements: &HashMap<String, String>,
    verbose: bool,
    show_progress: bool,
) -> Result<Vec<OptionDoc>, NixDocError> {
    let mut options = Vec::new();

    if verbose && !replacements.is_empty() {
        eprintln!("Using variable replacements:");
        for (key, value) in replacements {
            eprintln!("  ${{{0}}} => {1}", key, value);
        }
    }

    // Collect list of directories and paths to be excluded
    // from the generated documentation
    let exclude_paths: Vec<PathBuf> = exclude_dirs
        .iter()
        .map(|s| {
            let p = PathBuf::from(s);
            if p.is_absolute() {
                p
            } else {
                dir.join(p)
            }
        })
        .collect();

    if verbose && !exclude_paths.is_empty() {
        eprintln!("Excluding directories:");
        for path in &exclude_paths {
            eprintln!("  {}", path.display());
        }
    }

    // Collect all .nix files first
    let mut nix_files = Vec::new();
    for entry in WalkDir::new(dir).follow_links(true).into_iter() {
        let entry = entry?;

        // Check if this path is in an excluded directory
        let should_exclude = exclude_paths
            .iter()
            .any(|excl| entry.path().starts_with(excl));

        if should_exclude {
            if verbose {
                eprintln!("Skipping excluded path: {}", entry.path().display());
            }
            continue;
        }

        if is_hidden(&entry)
            || !entry.file_type().is_file()
            || entry.path().extension().is_none_or(|ext| ext != "nix")
        {
            continue;
        }
        nix_files.push(entry.path().to_path_buf());
    }

    // Set up progress bar if requested, mostly pointless
    // since Rust is quick enough to never see the progress,
    // but you never know
    let progress_bar = if show_progress {
        Some(indicatif::ProgressBar::new(nix_files.len() as u64))
    } else {
        None
    };

    // Process files with progress reporting
    for file_path in nix_files {
        if let Some(ref pb) = progress_bar {
            pb.inc(1);
            if let Some(file_name) = file_path.file_name() {
                pb.set_message(format!("Processing {}", file_name.to_string_lossy()));
            }
        }

        if verbose {
            eprintln!("Processing file: {}", file_path.display());
        }

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let parse = rnix::Root::parse(&content);
                let relative_path = file_path.strip_prefix(dir)?.to_string_lossy().into_owned();

                visit_node(
                    &parse.syntax(),
                    &relative_path,
                    &mut options,
                    "",
                    replacements,
                    &content,
                )?;
            }
            Err(e) => {
                if verbose {
                    eprintln!("  Error reading file: {}", e);
                }
                return Err(NixDocError::Io(e));
            }
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Processing complete");
    }

    if verbose {
        eprintln!("Total options found: {}", options.len());
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
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if node.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
        let key = node
            .children()
            .find(|n| n.kind() == SyntaxKind::NODE_ATTRPATH)
            .as_ref()
            .map(|n| parse_attrpath(n, replacements));

        if let Some(value_node) = node.children().nth(1) {
            if let Some(key) = key {
                let new_prefix = if prefix.is_empty() {
                    key
                } else {
                    format!("{}.{}", prefix, key)
                };
                parse_attrset(
                    &value_node,
                    file_path,
                    options,
                    &new_prefix,
                    replacements,
                    source_text,
                )?;
            }
        }
    } else {
        // Visit all children for other node types
        for child in node.children() {
            visit_node(
                &child,
                file_path,
                options,
                prefix,
                replacements,
                source_text,
            )?;
        }
    }

    Ok(())
}

// Parse attribute path for option names.
fn parse_attrpath(node: &SyntaxNode, replacements: &HashMap<String, String>) -> String {
    let mut path = Vec::new();
    for child in node.children() {
        if child.kind() == SyntaxKind::NODE_IDENT {
            path.push(child.text().to_string());
        } else if child.kind() == SyntaxKind::NODE_DYNAMIC {
            // This is an interpolation node like ${namespace}
            let interpol_text = child.text().to_string();

            // Extract the variable name from ${...}
            let var_name = interpol_text
                .trim_start_matches("${")
                .trim_end_matches("}")
                .trim();

            // Look up the replacement value or keep the original
            let replacement = replacements
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| format!("${{{}}}", var_name));

            path.push(replacement);
        }
    }
    path.join(".")
}

// Parse line number for each option definition.
// Will be formatted in the output to point to the
// exact definition in file.
fn get_line_number(node: &SyntaxNode, source_text: &str) -> usize {
    // Get the text range of this node
    let text_range = node.text_range();
    let start_offset: usize = text_range.start().into();

    // Count newlines up to this position
    let line_count = source_text[..start_offset]
        .chars()
        .filter(|&c| c == '\n')
        .count();

    // Line numbers are 1-based
    line_count + 1
}

// Parse attribute set and generate option documentation.
fn parse_attrset(
    node: &SyntaxNode,
    file_path: &str,
    options: &mut Vec<OptionDoc>,
    current_prefix: &str,
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match node.kind() {
        // Nested attributes
        SyntaxKind::NODE_ATTR_SET => {
            for child in node.children() {
                visit_node(
                    &child,
                    file_path,
                    options,
                    current_prefix,
                    replacements,
                    source_text,
                )?;
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
                        .map(|n| {
                            let desc_text = n.text().to_string().trim_matches('"').to_string();
                            // Apply replacements to description
                            replacements.iter().fold(desc_text, |acc, (key, value)| {
                                acc.replace(&format!("${{{}}}", key), value)
                            })
                        });

                    options.retain(|opt| opt.name != current_prefix);
                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        nix_type: NixType::Bool,
                        default_value: Some(String::from("false")),
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                Some("mkOption") => {
                    let mut nix_type = NixType::Unknown("any".to_string());
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
                                        let type_str = v.text().to_string();
                                        nix_type = NixType::from_nix_str(&type_str);
                                    }
                                    (Some("description"), Some(v)) => {
                                        let desc_text =
                                            v.text().to_string().trim_matches('"').to_string();
                                        description = Some(replacements.iter().fold(
                                            desc_text,
                                            |acc, (key, value)| {
                                                acc.replace(&format!("${{{}}}", key), value)
                                            },
                                        ));
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
                        nix_type,
                        default_value,
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}

// Generate output document in the specified format
pub fn generate_doc(
    options: &[OptionDoc],
    format: OutputFormat,
    sorted: bool,
) -> Result<String, NixDocError> {
    let mut options_copy = options.to_vec();
    if sorted {
        options_copy.sort_by(|a, b| a.name.cmp(&b.name));
    }

    match format {
        OutputFormat::Markdown => Ok(generate::generate_markdown(&options_copy)?),
        OutputFormat::Json => generate::generate_json(&options_copy),
        OutputFormat::Html => generate::generate_html(&options_copy),
        OutputFormat::Csv => generate::generate_csv(&options_copy),
    }
}
