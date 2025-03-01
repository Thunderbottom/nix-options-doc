use clap::Parser;
use nix_options_doc::{collect_options, filter_options, generate_doc, prepare_path, Cli};
use std::collections::HashMap;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("Starting {}", env!("CARGO_PKG_NAME"));
        eprintln!("Input path: {}", cli.path);
        eprintln!("Output: {}", cli.out);
    }

    let (path, _temp_dir) = prepare_path(&cli)?;

    if cli.verbose {
        eprintln!("Using path: {}", path.display());
        eprintln!("Collecting options...");
    }

    // Get replacements for any dynamic variables if defined
    let replacements: HashMap<String, String> = cli.replace.clone().into_iter().collect();
    let options = collect_options(
        &path,
        &cli.exclude_dir,
        &replacements,
        cli.verbose,
        cli.progress,
    )?;

    if options.is_empty() {
        eprintln!("No NixOS options found in the specified path");
        return Ok(());
    }

    // Apply module filters if specified
    let filtered_options = filter_options(&options, &cli);

    if filtered_options.is_empty() {
        eprintln!("No options match the specified filters");
        return Ok(());
    }

    if cli.verbose {
        eprintln!("Generating documentation...");
    }

    let output = generate_doc(&filtered_options, cli.format, cli.sort)?;

    // Output to stdout or file path
    if cli.out == "stdout" {
        println!("{}", output);
    } else {
        fs::write(&cli.out, &output)?;
        println!(
            "Found {} options (filtered from {} total).\nDocumentation generated in: {}",
            filtered_options.len(),
            options.len(),
            cli.out
        );
    }

    if cli.verbose {
        eprintln!("Documentation generation complete");
    }

    Ok(())
}
