use clap::Parser;
use nix_options_doc::{collect_options, filter_options, generate_doc, prepare_path, Cli};
use std::collections::HashMap;
use std::fs;
use std::io::Write;

/// Entry point of the application.
///
/// Parses command line arguments, prepares the working directory (or clones a repository),
/// collects NixOS module options from the specified path, applies filtering and variable replacements,
/// generates documentation in the desired format, and outputs the result to stdout or a file.
///
/// # Returns
/// Returns `Ok(())` if the application completes successfully; otherwise returns an error with details.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let cli = Cli::parse();

    log::info!("Starting {}", env!("CARGO_PKG_NAME"));
    log::debug!("Input path: {}", cli.io.path);
    log::debug!("Output: {}", cli.io.out);

    let (path, _temp_dir) = prepare_path(&cli)?;

    log::debug!("Using path: {}", path.display());
    log::debug!("Collecting options...");

    // Get replacements for any dynamic variables if defined
    let replacements: HashMap<String, String> = cli.filter.replace.clone().into_iter().collect();
    let options = collect_options(
        &path,
        &cli.util.exclude_dir,
        &replacements,
        cli.util.progress,
        cli.util.follow_symlinks,
    )?;

    if options.is_empty() {
        log::warn!("No NixOS options found in the specified path");
        return Ok(());
    }

    // Apply module filters if specified
    let filtered_options = filter_options(&options, &cli);

    if filtered_options.is_empty() {
        log::warn!(
            "No options match the specified filters (from {} total options)",
            options.len()
        );
        return Ok(());
    }

    log::debug!("Generating documentation...");

    let output = generate_doc(&filtered_options, cli.io.format, cli.io.sort)?;

    // Output to stdout or file path
    if cli.io.out == "stdout" {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        handle.write_all(output.as_bytes())?;
    } else {
        fs::write(&cli.io.out, &output)?;
        log::info!(
            "Found {} options (filtered from {} total). Documentation generated in: {}",
            filtered_options.len(),
            options.len(),
            cli.io.out
        );
    }

    Ok(())
}
