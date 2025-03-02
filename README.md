<h1 align="center">nix-options-doc</h1>

A command-line tool that generates multi-format Nix modules documentation.

A live version of the generated documentation can be found at: [Thunderbottom/flakes](https://github.com/Thunderbottom/flakes/blob/main/options.md)

## Why?

I was always fascinated by various Nix projects that showcased their Nix module documentation, yet I failed to find any tool to do so for my own projects. And so I wrote one. This has also served me as an exercise in learning Rust.

## Features

- Recursively scans directories for `.nix` files (with directory exclusion support)
- Supports both local paths and remote Git repositories (with branch/tag selection)
- Extracts option names, types, default values, descriptions, and line numbers
- Handles variable interpolation (e.g., `${namespace}`) with configurable replacements
- Robust error handling with detailed error messages
- Multiple output formats (Markdown, JSON, HTML, CSV)
- Line numbers in file references for direct linking
- Optional alphabetical sorting of options
- Links file paths in generated documentation for easy reference

## Installation

### Pre-built Binary

Pre-built binaries for ARM and x86 based GNU/Linux systems are available under [releases](/releases).

### Using Cargo

```bash
$ cargo install --git https://github.com/Thunderbottom/nix-options-doc
```

Or build from source:

```bash
$ git clone https://github.com/Thunderbottom/nix-options-doc.git
$ cd nix-options-doc
$ cargo build --release
```

### Using Nix

```bash
$ nix build github:Thunderbottom/nix-options-doc
$ ./results/bin/nix-options-doc --path /etc/nixos --out stdout  
```

## Usage

### Basic Usage

```bash
# Generate documentation for current directory
# Prints the generated documentation to stdout
$ nix-options-doc

# Generate documentation for a specific path
$ nix-options-doc --path ./nixos/modules

# Generate sorted documentation
$ nix-options-doc --path ./nixos/modules --sort

# Output to stdout instead of file
$ nix-options-doc --out stdout

# Exclude specific directories
$ nix-options-doc --exclude-dir templates,tests

# Replace variables like ${namespace} in paths
$ nix-options-doc --replace namespace=snowflake --replace system=x86_64-linux
```

### Working with Git Repositories

The tool supports all Git URL formats:

```bash
# GitHub HTTPS
$ nix-options-doc --path https://github.com/user/repo.git

# GitHub SSH
$ nix-options-doc --path git@github.com:user/repo.git

# Other Git URLs
$ nix-options-doc --path git://example.com/repo.git
$ nix-options-doc --path ssh://git@example.com/repo.git
```

### Command Line Options

```
Usage: nix-options-doc [OPTIONS]

Options:
  -p, --path <PATH>                Local path or remote git repository URL to the nix configuration [default: .]
  -o, --out <OUT>                  Path to the output file or 'stdout' [default: stdout]
  -f, --format <FORMAT>            Output format [default: markdown] [possible values: markdown, json, html, csv]
  -s, --sort                       Whether the output names should be sorted
  -b, --branch <BRANCH>            Git branch or tag to use (if repository URL provided)
  -d, --depth <DEPTH>              Git commit depth (set to 1 for shallow clone) [default: 1]
      --prefix <PREFIX>            Filter options by prefix (e.g. "services.nginx")
      --replace <REPLACE>          Replace nix variable with the specified value in option paths (can be used multiple times) Format: --replace key=value
      --search <SEARCH>            Search in option names and descriptions
      --type-filter <TYPE_FILTER>  Filter options by type (e.g. "bool", "string")
      --has-default                Only show options that have a default value
      --has-description            Only show options that have a description
  -e, --exclude-dir <EXCLUDE_DIR>  Directories to exclude from processing (can be specified multiple times)
      --follow-symlinks            Enable traversing through symbolic links
  -P, --progress                   Show progress bar
  -h, --help                       Print help
  -V, --version                    Print version
```

## Output Format

The generated documentation includes a Markdown table with the following columns:

| Column | Description |
|--------|-------------|
| Option | The full option definition path |
| Type | The option's type (e.g., boolean, string, etc.) |
| Default | The default value, if any |
| Description | Documentation for the option |

Example output:

```markdown
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| [services.nginx.enable](modules/nginx/default.nix#L25) | boolean | `false` | Whether to enable nginx |
| [services.nginx.port](modules/nginx/default.nix#L32) | number | `80` | Port to listen on |
```

## Development

### Prerequisites

- Rust 1.70 or later
- Git (for repository cloning features)
- OpenSSL-dev and pkg-config

### Building

```bash
# Using Cargo
$ cargo build

# Using nix develop
$ nix develop
(nix shell) $ cargo build --reelease
```

### Running Tests

```bash
$ cargo test
```

### Dependencies

- clap: Command-line argument parsing
- csv: CSV file generation
- env_logger: Log configuration through environment variables
- gix: Git repository handling
- html-escape: HTML entity escaping
- indicatif: Progress bar
- log: Simple logging utility
- rnix: Nix parser
- serde_json: JSON serialization
- serde: Serialization/deserialization
- tempfile: Temporary directory management
- thiserror: Error handling
- walkdir: Directory traversal

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
