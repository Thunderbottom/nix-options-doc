<h1 align="center">nix-options-doc</h1>

A command-line tool that generates comprehensive, multi-format documentation for NixOS module options.

A live example of the generated documentation can be found at: [Thunderbottom/flakes](https://github.com/Thunderbottom/flakes/blob/main/options.md)

## Why?

NixOS configurations can be complex, with numerous modules and options that need clear documentation. While many Nix projects showcase elegant module documentation, I couldn't find a dedicated tool to generate such documentation for my own projects. This tool fills that gap, while also serving as my exercise in learning Rust.

## Features

- **Multiple Output Formats**: Generate documentation in Markdown, HTML, JSON, or CSV
- **Rich Documentation**: Captures option names, types, default values, examples, descriptions, and source references
- **Improved Type Detection**: Intelligent parsing of complex Nix types with human-friendly output
- **Repository Support**: Works with both local paths and remote Git repositories (with branch/tag selection)
- **Variable Interpolation**: Handles `${namespace}` style variables with configurable replacements
- **Admonition Support**: Renders warning, note, and important blocks in both Markdown and HTML output
- **Filtering Capabilities**: Filter by prefix, type, search term, or other criteria
- **Robust Error Handling**: Detailed error messages and graceful recovery from parsing issues
- **Parallel Processing**: Fast performance with multi-threaded file processing
- **Progress Visibility**: Optional progress bar for monitoring documentation generation

## Installation

### Pre-built Binary

Pre-built binaries for ARM and x86 based GNU/Linux systems are available under [releases](https://github.com/Thunderbottom/nix-options-doc/releases).

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
$ ./result/bin/nix-options-doc --path /etc/nixos --out nixos-options.md
```

## Usage

### Basic Usage

```bash
# Generate documentation for current directory, output to stdout
$ nix-options-doc

# Generate documentation for a specific path
$ nix-options-doc --path ./nixos/modules --out modules-doc.md

# Generate sorted documentation
$ nix-options-doc --path ./nixos/modules --sort

# Generate HTML documentation
$ nix-options-doc --format html --out modules.html

# Show progress bar during generation
$ nix-options-doc --progress
```

### Advanced Usage

```bash
# Filter options by prefix
$ nix-options-doc --filter-by-prefix services.nginx

# Exclude specific directories
$ nix-options-doc --exclude-dir templates,tests

# Replace variables in Nix modules
$ nix-options-doc --replace namespace=snowflake --replace system=x86_64-linux

# Only include options with descriptions
$ nix-options-doc --has-description

# Strip common prefix from option names
$ nix-options-doc --strip-prefix options.services
```

### Working with Git Repositories

```bash
# Clone and document a GitHub repository (HTTPS)
$ nix-options-doc --path https://github.com/user/repo.git

# Use specific branch or tag
$ nix-options-doc --path git@github.com:user/repo.git --branch feature-branch

# Shallow clone with custom depth
$ nix-options-doc --path git://example.com/repo.git --depth 5
```

### Command Line Options

```
Usage: nix-options-doc [OPTIONS]

Options:
  -p, --path <PATH>                Local path or remote git repository URL [default: .]
  -o, --out <OUT>                  Path to output file or 'stdout' [default: stdout]
  -f, --format <FORMAT>            Output format [default: markdown] [possible values: markdown, json, html, csv]
  -s, --sort                       Sort options alphabetically
  -b, --branch <BRANCH>            Git branch or tag to use (for remote repositories)
  -d, --depth <DEPTH>              Git commit depth for shallow clones [default: 1]
      --filter-by-prefix <PREFIX>  Filter options by prefix (e.g. "services.nginx")
      --filter-by-type <NIX_TYPE>  Filter options by type (e.g. "bool", "string")
      --search <OPTION>            Search in option names and descriptions
      --has-default                Only show options that have a default value
      --has-description            Only show options that have a description
      --replace <KEY=VALUE>        Replace variables in Nix modules (can be used multiple times)
      --strip-prefix [<PREFIX>]    Remove the specified prefix from output [default: options.]
  -e, --exclude-dir <EXCLUDE_DIR>  Directories to exclude from processing
      --follow-symlinks            Enable traversing through symbolic links
      --progress                   Show progress bar
  -h, --help                       Print help
  -V, --version                    Print version
```

## Output Examples

### Markdown Format

The Markdown output uses a heading-based structure for each option:

```markdown
## [`services.nginx.enable`](modules/nginx/default.nix#L25)

Whether to enable the Nginx web server.

**Type:** `boolean`

**Default:** `false`

**Example:** `true`
```

### Admonition Support

The tool properly renders admonition blocks in Nix module descriptions:

```nix
# In your Nix file:
description = ''
  Regular description text.
  
  ::: {.warning}
  This setting can impact system security.
  :::
'';
```

Will be rendered in Markdown as:

```markdown
Regular description text.

> [!WARNING]  
> This setting can impact system security.
```

And in HTML with proper styling.

## Development

### Prerequisites

- Rust 1.70 or later
- Git (for repository cloning features)

### Building and Testing

```bash
# Build the project
$ cargo build

# Run tests
$ cargo test

# Run with debug logging
$ RUST_LOG=debug cargo run -- --path /path/to/nixos/modules
```

### Project Structure

- `src/generate/` - Output format generators (Markdown, HTML, JSON, CSV)
- `src/parser.rs` - Nix file parser using rnix syntax tree
- `src/types.rs` - NixOS type definitions and formatting
- `src/utils.rs` - Helper functions for file processing and text manipulation
- `src/error.rs` - Error type definitions and handling
- `src/lib.rs` - Core functions and CLI structure
- `src/main.rs` - Command-line interface

## Contributing

Contributions are welcome! Feel free to submit a Pull Request or open an issue.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
