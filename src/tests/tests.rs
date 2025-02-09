use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_test_file(dir: &Path, filename: &str, content: &str) -> Result<(), std::io::Error> {
    fs::write(dir.join(filename), content)
}

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

    let options = collect_options(temp_dir.path())?;
    println!("{:?}", options);
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].name, "options.test.simple.enable");
    assert_eq!(options[0].type_info, "boolean");
    assert_eq!(
        options[0].description,
        Some("Simple test option".to_string())
    );
    assert_eq!(options[0].default_value, Some("false".to_string()));

    Ok(())
}

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

    let options = collect_options(temp_dir.path())?;
    assert_eq!(options.len(), 2);

    let string_opt = options
        .iter()
        .find(|o| o.name == "options.test.complex.stringOpt")
        .unwrap();
    assert_eq!(string_opt.type_info, "lib.types.str");
    assert_eq!(string_opt.description, Some("A string option".to_string()));
    assert_eq!(string_opt.default_value, Some("\"test\"".to_string()));

    let nested_opt = options
        .iter()
        .find(|o| o.name == "options.test.complex.nested.value")
        .unwrap();
    assert_eq!(nested_opt.type_info, "lib.types.int");
    assert_eq!(
        nested_opt.description,
        Some("A nested number option".to_string())
    );
    assert_eq!(nested_opt.default_value, None);

    Ok(())
}

#[test]
fn test_markdown_generation() -> Result<(), Box<dyn std::error::Error>> {
    let options = vec![
        OptionDoc {
            name: "options.test.opt1".to_string(),
            description: Some("Test option 1".to_string()),
            type_info: "boolean".to_string(),
            default_value: Some("false".to_string()),
            file_path: "test.nix".to_string(),
        },
        OptionDoc {
            name: "options.test.opt2".to_string(),
            description: Some("Test option 2".to_string()),
            type_info: "lib.types.str".to_string(),
            default_value: None,
            file_path: "test.nix".to_string(),
        },
    ];

    // Test unsorted output
    let markdown = generate_markdown(&options, false)?;
    println!("{:?}", markdown);
    assert!(markdown.contains("| [`options.test.opt1`](test.nix)"));
    assert!(markdown.contains("| [`options.test.opt2`](test.nix)"));
    assert!(markdown.contains("Test option 1"));
    assert!(markdown.contains("Test option 2"));

    // Test sorted output
    let markdown_sorted = generate_markdown(&options, true)?;
    let opt1_pos = markdown_sorted.find("options.test.opt1").unwrap();
    let opt2_pos = markdown_sorted.find("options.test.opt2").unwrap();
    assert!(opt1_pos < opt2_pos);

    Ok(())
}

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

    let options = collect_options(temp_dir.path())?;
    assert_eq!(options.len(), 0);

    Ok(())
}

#[test]
fn test_cli_args() {
    use clap::Parser;

    let args = Cli::parse_from(&["program", "--path", "/test/path"]);
    assert_eq!(args.path, "/test/path");
    assert_eq!(args.out, "nix-options.md"); // default value
    assert!(!args.sort); // default false

    let args = Cli::parse_from(&["program", "--out", "stdout", "--sort"]);
    assert_eq!(args.path, "."); // default value
    assert_eq!(args.out, "stdout");
    assert!(args.sort);
}

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

    let options = collect_options(temp_dir.path())?;
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
