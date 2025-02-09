<h1 align="center">nix-options-doc</h1>

A command-line tool that generates documentation for NixOS module options by parsing Nix files and producing formatted Markdown table.

A live preview of the generated documentation can be found at: [Thunderbottom/flakes](https://github.com/Thunderbottom/flakes/blob/main/options.md)

## Features

- Generates comprehensive Markdown documentation for NixOS module options
- Recursively scans directories for `.nix` files
- Supports both local paths and remote Git repositories
- Extracts option names, types, default values, and descriptions
- Optional alphabetical sorting of options
- Links file paths in generated documentation

## Installation

### Using Cargo

```bash
$ cargo install nix-options-doc
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
# Stores the generated documentation as nix-options.md
$ nix-options-doc

# Generate documentation for a specific path
$ nix-options-doc --path ./nixos/modules

# Generate sorted documentation
$ nix-options-doc --path ./nixos/modules --sort

# Output to stdout instead of file
$ nix-options-doc --out stdout
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
Options:
  -p, --path <PATH>  Local path or remote git repository URL to the nix configuration [default: .]
  -o, --out <OUT>    Path to output file or 'stdout' [default: nix-options.md]
  -s, --sort         Whether the output names should be sorted
  -h, --help         Print help
  -V, --version      Print version
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
| [services.nginx.enable](modules/nginx/default.nix) | boolean | `false` | Whether to enable nginx |
| [services.nginx.port](modules/nginx/default.nix) | number | `80` | Port to listen on |
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
- rnix: Nix parser
- git2: Git repository handling
- rowan: Syntax tree manipulation
- serde: Serialization/deserialization
- walkdir: Directory traversal
- tempfile: Temporary directory management

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
