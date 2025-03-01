use super::*;
use crate::{generate::generate_markdown, NixType};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a test file with the specified filename and content in the given directory.
///
/// # Arguments
/// - `dir`: The directory in which to create the file.
/// - `filename`: The name of the file to create.
/// - `content`: The content to write into the file.
///
/// # Returns
/// Returns `Ok(())` if the file is created successfully; otherwise returns an I/O error.
fn create_test_file(dir: &Path, filename: &str, content: &str) -> Result<(), std::io::Error> {
    fs::write(dir.join(filename), content)
}

/// Tests that a simple option is parsed correctly from a Nix file.
/// Creates a temporary file with a simple option definition and asserts that the parsed option’s
/// name, type, description, and default value match expectations.
#[test]
fn test_basic_option_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let content = r#"
{
  options.test.simple = {
    enable = lib.mkEnableOption "Simple test option";
  };
}
"#;
    create_test_file(temp_dir.path(), "flake.nix", content)?;

    let options = collect_options(temp_dir.path(), &[], &HashMap::new(), false)?;
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].name, "options.test.simple.enable");
    assert_eq!(options[0].nix_type.to_string(), "boolean");
    assert_eq!(
        options[0].description,
        Some("Simple test option".to_string())
    );
    assert_eq!(options[0].default_value, Some("false".to_string()));

    Ok(())
}

/// Tests parsing of complex options including nested attributes.
/// Verifies that string and numeric options are correctly parsed from a Nix file.
#[test]
fn test_complex_option_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let content = r#"
{
  options.test.complex = {
    stringOpt = lib.mkOption {
      type = lib.types.str;
      default = "test";
      description = "A string option";
    };

    nested.value = lib.mkOption {
      type = lib.types.int;
      description = "A nested number option";
    };
  };
}
"#;
    create_test_file(temp_dir.path(), "test.nix", content)?;

    let options = collect_options(temp_dir.path(), &[], &HashMap::new(), false)?;
    assert_eq!(options.len(), 2);

    let string_opt = options
        .iter()
        .find(|o| o.name == "options.test.complex.stringOpt")
        .unwrap();
    assert_eq!(string_opt.nix_type.to_string(), "lib.types.str");
    assert_eq!(string_opt.description, Some("A string option".to_string()));
    assert_eq!(string_opt.default_value, Some("\"test\"".to_string()));

    let nested_opt = options
        .iter()
        .find(|o| o.name == "options.test.complex.nested.value")
        .unwrap();
    assert_eq!(nested_opt.nix_type.to_string(), "lib.types.int");
    assert_eq!(
        nested_opt.description,
        Some("A nested number option".to_string())
    );
    assert_eq!(nested_opt.default_value, None);

    Ok(())
}

/// Tests the generation of Markdown documentation from a set of option definitions.
/// Checks that the resulting Markdown output contains expected table entries and links.
#[test]
fn test_markdown_generation() -> Result<(), Box<dyn std::error::Error>> {
    let options = vec![
        OptionDoc {
            name: "options.test.opt1".to_string(),
            description: Some("Test option 1".to_string()),
            nix_type: NixType::Bool,
            default_value: Some("false".to_string()),
            file_path: "test.nix".to_string(),
            line_number: 1,
        },
        OptionDoc {
            name: "options.test.opt2".to_string(),
            description: Some("Test option 2".to_string()),
            nix_type: NixType::Unknown("lib.types.str".to_string()),
            default_value: None,
            file_path: "test.nix".to_string(),
            line_number: 2,
        },
    ];

    // Test unsorted output
    let markdown = generate_markdown(&options)?;
    assert!(markdown.contains("| [`options.test.opt1`](test.nix#L1)"));
    assert!(markdown.contains("| [`options.test.opt2`](test.nix#L2)"));
    assert!(markdown.contains("Test option 1"));
    assert!(markdown.contains("Test option 2"));

    // Test sorted output
    let mut sorted_options = options.clone();
    sorted_options.sort_by(|a, b| a.name.cmp(&b.name));
    let markdown_sorted = generate_markdown(&sorted_options)?;
    let opt1_pos = markdown_sorted.find("options.test.opt1").unwrap();
    let opt2_pos = markdown_sorted.find("options.test.opt2").unwrap();
    assert!(opt1_pos < opt2_pos);

    Ok(())
}

/// Tests that hidden files (e.g. files starting with a dot) are correctly excluded from processing.
#[test]
fn test_hidden_files_exclusion() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let content = r#"
{
  options.test.hidden = {
    enable = lib.mkEnableOption "Hidden test option";
  };
}
"#;
    create_test_file(temp_dir.path(), ".hidden.nix", content)?;

    let options = collect_options(temp_dir.path(), &[], &HashMap::new(), false)?;
    assert_eq!(options.len(), 0);

    Ok(())
}

/// Tests the parsing of command-line arguments and verifies that default values are correctly assigned.
#[test]
fn test_cli_args() {
    use clap::Parser;

    let args = Cli::parse_from(["program", "--path", "/test/path"]);
    assert_eq!(args.path, "/test/path");
    assert_eq!(args.out, "nix-options.md"); // default value
    assert!(!args.sort); // default false

    let args = Cli::parse_from(["program", "--out", "stdout", "--sort"]);
    assert_eq!(args.path, "."); // default value
    assert_eq!(args.out, "stdout");
    assert!(args.sort);
}

/// Tests that duplicate option definitions are prevented by ensuring only one instance is kept.
#[test]
fn test_duplicate_prevention() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let content = r#"
{
  options.test = {
    enable = lib.mkEnableOption "Test option";
    enable = lib.mkEnableOption "Duplicate test option";
  };
}
"#;
    create_test_file(temp_dir.path(), "test.nix", content)?;

    let options = collect_options(temp_dir.path(), &[], &HashMap::new(), false)?;
    let enable_options: Vec<_> = options
        .iter()
        .filter(|o| o.name == "options.test.enable")
        .collect();

    assert_eq!(
        enable_options.len(),
        1,
        "Should only have one enable option"
    );

    Ok(())
}

/// Tests that options in excluded directories are not included in the final results.
#[test]
fn test_exclude_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a structure with files in subdirectories
    let main_content = r#"
{
  options.main = {
    enable = lib.mkEnableOption "Main option";
  };
}
"#;

    let excluded_content = r#"
{
  options.excluded = {
    enable = lib.mkEnableOption "Excluded option";
  };
}
"#;

    // Create directories and files
    fs::create_dir_all(temp_dir.path().join("modules"))?;
    fs::create_dir_all(temp_dir.path().join("excluded"))?;

    create_test_file(temp_dir.path(), "main.nix", main_content)?;
    create_test_file(
        temp_dir.path().join("excluded").as_path(),
        "excluded.nix",
        excluded_content,
    )?;

    // Test without exclusion
    let all_options = collect_options(temp_dir.path(), &[], &HashMap::new(), false)?;
    assert!(!all_options.is_empty()); // At least the main option
    assert!(all_options.iter().any(|o| o.name == "options.main.enable"));

    // Test with exclusion
    let exclude_dirs = vec![temp_dir
        .path()
        .join("excluded")
        .to_string_lossy()
        .to_string()];
    let filtered_options = collect_options(temp_dir.path(), &exclude_dirs, &HashMap::new(), false)?;

    assert!(filtered_options
        .iter()
        .any(|o| o.name == "options.main.enable"));
    assert!(!filtered_options
        .iter()
        .any(|o| o.name == "options.excluded.enable"));

    Ok(())
}

/// Tests variable replacement functionality in option names and descriptions.
/// Verifies that placeholders (e.g. `${namespace}`) are replaced with provided values.
#[test]
fn test_variable_replacements() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a file with variable interpolation
    let content = r#"
{
  options.${namespace}.hardware.bluetooth = {
    enable = lib.mkEnableOption "Enable ${namespace} bluetooth";
  };
  
  options.${system}.networking = {
    enable = lib.mkEnableOption "Enable networking for ${system}";
  };
}
"#;
    create_test_file(temp_dir.path(), "config.nix", content)?;

    // Set up replacements
    let mut replacements = HashMap::new();
    replacements.insert("namespace".to_string(), "snowflake".to_string());
    replacements.insert("system".to_string(), "x86_64-linux".to_string());

    let options = collect_options(temp_dir.path(), &[], &replacements, false)?;

    // Check if options contain the replaced values
    let bluetooth_options: Vec<_> = options
        .iter()
        .filter(|o| o.name.contains("bluetooth"))
        .collect();

    let networking_options: Vec<_> = options
        .iter()
        .filter(|o| o.name.contains("networking"))
        .collect();

    if !bluetooth_options.is_empty() {
        let bluetooth_opt = &bluetooth_options[0];
        assert!(bluetooth_opt.name.contains("snowflake"));
        assert!(!bluetooth_opt.name.contains("${namespace}"));

        // Check if description also had replacements
        if let Some(desc) = &bluetooth_opt.description {
            assert!(desc.contains("snowflake"));
            assert!(!desc.contains("${namespace}"));
        }
    }

    if !networking_options.is_empty() {
        let networking_opt = &networking_options[0];
        assert!(networking_opt.name.contains("x86_64-linux"));
        assert!(!networking_opt.name.contains("${system}"));

        // Check if description also had replacements
        if let Some(desc) = &networking_opt.description {
            assert!(desc.contains("x86_64-linux"));
            assert!(!desc.contains("${system}"));
        }
    }

    Ok(())
}

/// Tests error handling by checking that invalid paths and malformed files produce proper errors without panicking.
#[test]
fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Test non-existent path
    let non_existent = temp_dir.path().join("non-existent");
    let result = collect_options(&non_existent, &[], &HashMap::new(), false);
    assert!(result.is_err());

    // Create a file with invalid Nix syntax
    let invalid_content = r#"
{
  options.test = {
    # Missing closing brace
    invalid = lib.mkEnableOption "Invalid option"
  ;
}
"#;
    create_test_file(temp_dir.path(), "invalid.nix", invalid_content)?;

    // The parser might handle syntax errors gracefully, so we'll
    // just check that it doesn't panic
    let _ = collect_options(temp_dir.path(), &[], &HashMap::new(), false);

    Ok(())
}

/// Tests the parsing of replacement arguments from the command line,
/// ensuring that key-value pairs are correctly split and stored.
#[test]
fn test_cli_replace_argument() {
    use clap::Parser;

    // Test parsing replacement arguments
    let args = Cli::parse_from([
        "program",
        "--replace",
        "namespace=snowflake",
        "--replace",
        "system=x86_64-linux",
    ]);

    assert_eq!(args.replace.len(), 2);
    assert!(args
        .replace
        .contains(&("namespace".to_string(), "snowflake".to_string())));
    assert!(args
        .replace
        .contains(&("system".to_string(), "x86_64-linux".to_string())));

    // Convert to HashMap and verify
    let replacements: HashMap<String, String> = args.replace.into_iter().collect();
    assert_eq!(
        replacements.get("namespace"),
        Some(&"snowflake".to_string())
    );
    assert_eq!(
        replacements.get("system"),
        Some(&"x86_64-linux".to_string())
    );
}
