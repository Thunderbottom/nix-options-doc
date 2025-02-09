use clap::Parser;
use nix_options_doc::{collect_options, generate_markdown, prepare_path, Cli};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let (path, _temp_dir) = prepare_path(&cli.path)?;
    let options = collect_options(&path)?;
    if options.is_empty() {
        eprintln!("No NixOS options found in the specified path");
        return Ok(());
    }

    let markdown = generate_markdown(&options, cli.sort)?;

    // Output to stdout or file path
    if cli.out == "stdout" {
        println!("{}", markdown);
    } else {
        fs::write(&cli.out, &markdown)?;
        println!(
            "Found {} options. Documentation generated in: {}",
            options.len(),
            cli.out
        );
    }
    Ok(())
}
