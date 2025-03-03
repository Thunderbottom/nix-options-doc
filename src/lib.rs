pub mod error;
pub mod generate;
pub mod parser;
pub mod types;
pub mod utils;

use crate::{error::NixDocError, types::NixType};
use clap::{command, ArgGroup, Args, Parser};
use gix::{progress::Discard, remote::fetch::Shallow};
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
    #[command(flatten)]
    pub io: IoOptions,

    #[command(flatten)]
    pub git: GitOptions,

    #[command(flatten)]
    pub filter: FilterOptions,

    #[command(flatten)]
    pub util: UtilityOptions,
}

/// IO Command Options
#[derive(Args)]
#[command(group(ArgGroup::new("io")))]
pub struct IoOptions {
    /// Local path or remote git repository URL to the nix configuration
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Path to the output file or 'stdout'
    #[arg(short, long, default_value = "stdout")]
    pub out: String,

    /// Output format
    #[arg(short = 'f', long, default_value = "markdown")]
    pub format: OutputFormat,

    /// Whether the output should be sorted (asc.)
    #[arg(short, long)]
    pub sort: bool,
}

/// Git Options
#[derive(Args)]
#[command(group(ArgGroup::new("git")))]
pub struct GitOptions {
    /// Git branch or tag to use (if repository URL provided)
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Git commit depth (set to 1 for shallow clone)
    #[arg(short, long, default_value = "1")]
    pub depth: u32,
}

/// Filter Options
#[derive(Args)]
#[command(group(ArgGroup::new("filter")))]
pub struct FilterOptions {
    /// Filter options by prefix (e.g. "services.nginx")
    #[arg(long, value_name = "PREFIX")]
    pub filter_by_prefix: Option<String>,

    /// Filter options by type (e.g. "bool", "string")
    #[arg(long, value_name = "NIX_TYPE")]
    pub filter_by_type: Option<String>,

    /// Search in option names and descriptions
    #[arg(long, value_name = "OPTION")]
    pub search: Option<String>,

    /// Only show options that have a default value
    #[arg(long)]
    pub has_default: bool,

    /// Only show options that have a description
    #[arg(long)]
    pub has_description: bool,

    /// Replace nix variables in the generated
    /// document with the specified value
    /// (can be used multiple times)
    #[arg(long, value_parser = parse_key_value)]
    #[arg(value_name = "KEY=VALUE")]
    pub replace: Vec<(String, String)>,
}

/// Utility Options
#[derive(Args)]
#[command(group(ArgGroup::new("utility")))]
pub struct UtilityOptions {
    /// Directories to exclude from processing (can be specified multiple times)
    #[arg(short = 'e', long, value_delimiter = ',')]
    pub exclude_dir: Vec<String>,

    /// Enable traversing through symbolic links
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Show progress bar
    #[arg(long)]
    pub progress: bool,
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

// Parses a string for a given key and replaces with the specified value.
// Used to interpolate variables in nix module definition as nix does not
// replace interpolated variables until evaluated.
//
// # Arguments
// - `string`: A string in the format key=value
//
// # Returns
// A Result containing two strings as separate key and value
fn parse_key_value(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 || parts[0].is_empty() {
        return Err(format!("Invalid key=value format: {}", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Filters the list of option documentation entries based on CLI parameters.
/// Applies filters such as prefix, type, search term, and presence of a default value or description.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
/// - `cli`: The CLI arguments containing filter criteria.
///
/// # Returns
/// A vector of options that match the specified filters.
pub fn filter_options(options: &[OptionDoc], cli: &Cli) -> Vec<OptionDoc> {
    let mut filtered = options.to_vec();

    // Filter by prefix
    if let Some(ref prefix) = cli.filter.filter_by_prefix {
        filtered.retain(|opt| opt.name.starts_with(prefix));
    }

    // Filter by type
    if let Some(ref type_str) = cli.filter.filter_by_type {
        filtered.retain(|opt| {
            let type_info = opt.nix_type.to_string().to_lowercase();
            type_info.contains(&type_str.to_lowercase())
        });
    }

    // Filter by search text
    if let Some(ref search) = cli.filter.search {
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
    if cli.filter.has_default {
        filtered.retain(|opt| opt.default_value.is_some());
    }

    // Filter by having description
    if cli.filter.has_description {
        filtered.retain(|opt| opt.description.is_some());
    }

    filtered
}

/// Prepares a local directory for processing Nix files.
/// If the specified path exists locally, it is used directly; otherwise, if a Git URL is provided,
/// the repository is cloned (using optional branch and depth settings) into a temporary directory.
///
/// # Arguments
/// - `cli`: The CLI arguments containing the path, branch, depth, etc.
///
/// # Returns
/// A tuple containing the path to the working directory and an optional `TempDir` (for cleanup).
pub fn prepare_path(cli: &Cli) -> Result<(PathBuf, Option<TempDir>), NixDocError> {
    // Check if the path is a local directory
    let path = Path::new(&cli.io.path);
    if path.exists() {
        log::debug!("Found local path: {}", path.to_string_lossy());
        return Ok((path.to_path_buf(), None));
    }

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Attempt to fetch git repository
    // Initialize interrupt handler.
    unsafe {
        gix::interrupt::init_handler(1, || {}).map_err(|e| {
            NixDocError::GitOperation(format!("Failed to initialize interrupt handler: {}", e))
        })?;
    }

    let url = gix::url::parse(cli.io.path.as_bytes().into())
        .map_err(|e| NixDocError::InvalidPath(format!("Invalid git URL: {}", e)))?;

    // Prepare the clone builder
    let mut prepare_clone = gix::prepare_clone(url, temp_path).map_err(|e| {
        let err_msg = e.to_string();
        if err_msg.contains("auth") || err_msg.contains("credentials") {
            NixDocError::GitClone(cli.io.path.clone(), err_msg)
        } else {
            NixDocError::GitOperation(format!("Failed to prepare clone: {}", e))
        }
    })?;

    // Configure shallow clone with the provided depth (defaults to 1)
    let shallow = Shallow::DepthAtRemote(
        std::num::NonZeroU32::new(cli.git.depth)
            .unwrap_or_else(|| std::num::NonZeroU32::new(1).unwrap()),
    );

    if let Some(ref branch) = cli.git.branch {
        prepare_clone = prepare_clone.with_ref_name(Some(branch)).unwrap();
    }
    let (mut prepare_checkout, _) = prepare_clone
        .with_shallow(shallow)
        .fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| NixDocError::GitClone(cli.io.path.clone(), e.to_string()))?;

    let (repo, _) = prepare_checkout
        .main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(|e| NixDocError::GitOperation(format!("Failed to checkout worktree: {}", e)))?;

    let work_dir = repo.work_dir().ok_or(NixDocError::NoWorkDir)?;
    Ok((work_dir.to_path_buf(), Some(temp_dir)))
}

/// Recursively collects NixOS module options from all .nix files in the specified directory,
/// excluding specified directories and applying variable replacements.
///
/// # Arguments
/// - `dir`: The base directory to search for Nix files.
/// - `exclude_dirs`: A list of directory paths to exclude from processing.
/// - `replacements`: A map of variable replacements for dynamic parts in option definitions.
/// - `show_progress`: Displays a progress bar if set to true.
///
/// # Returns
/// A `Result` containing a vector of option documentation entries or an error.
pub fn collect_options(
    dir: &Path,
    exclude_dirs: &[String],
    replacements: &HashMap<String, String>,
    show_progress: bool,
    follow_symlinks: bool,
) -> Result<Vec<OptionDoc>, NixDocError> {
    let mut options = Vec::new();

    if !replacements.is_empty() {
        log::debug!("Using variable replacements:");
        for (key, value) in replacements {
            log::debug!("\t${{{0}}} => {1}", key, value);
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

    if !exclude_paths.is_empty() {
        log::debug!("Excluding directories:");
        for path in &exclude_paths {
            log::debug!("\t{}", path.display());
        }
    }

    // Collect all .nix files first
    let mut nix_files = Vec::new();

    // Filter function to check if a path should be excluded
    let is_excluded = move |entry: &walkdir::DirEntry| -> bool {
        let path = entry.path();

        // Check if this path is an excluded directory
        let should_exclude = exclude_paths
            .clone()
            .iter()
            .any(|excl| entry.path().starts_with(excl));

        if should_exclude {
            log::debug!("Skipping excluded path: {}", path.display());
            return true;
        }

        // Exclude hidden files and directories
        is_hidden(entry)
    };

    for dir_entry in WalkDir::new(dir)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_entry(|e| !is_excluded(e))
    {
        let entry = match dir_entry {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("An error occurred, skipping directory: {}", e);
                continue;
            }
        };

        // Only collect nix files
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "nix") {
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

        log::debug!(
            "Processing file: {}",
            file_path.strip_prefix(dir)?.to_string_lossy()
        );

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let parse = rnix::Root::parse(&content);
                let relative_path = match file_path.strip_prefix(dir) {
                    Ok(rel_path) => rel_path.to_string_lossy().into_owned(),
                    Err(e) => {
                        log::warn!(
                            "Error getting relative path for {}: {}",
                            file_path.display(),
                            e
                        );
                        file_path.to_string_lossy().into_owned()
                    }
                };

                parser::visit_node(
                    &parse.syntax(),
                    &relative_path,
                    &mut options,
                    "",
                    replacements,
                    &content,
                )?;
            }
            Err(e) => {
                log::error!("Error reading file: {}", e);
                log::warn!("Skipping file: {}", file_path.display());
                continue;
            }
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_with_message("Processing complete");
    }

    log::debug!("Total options found: {}", options.len());

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

/// Generates documentation for the given options in the specified output format.
/// Optionally sorts the options alphabetically.
///
/// # Arguments
/// - `options`: A slice of option documentation entries.
/// - `format`: The desired output format (Markdown, JSON, HTML, or CSV).
/// - `sorted`: If true, sorts the options by name.
///
/// # Returns
/// A `Result` containing the generated documentation string or an error.
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
