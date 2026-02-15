# Kozmotic

**AI agent-friendly, fast and portable CLI tools written
in Rust**

Kozmotic provides a collection of command-line tools
designed to be easily consumed by AI agents, automation
scripts, and other programmatic interfaces. All tools
output structured data (JSON) by default, with
human-readable options available.

The project dogfoods its own tools — kozmotic CLI tools
are used within the project's own development workflow.

## Features

- **Structured Output**: JSON output by default for easy
  parsing
- **Agent-Friendly**: Designed for consumption by AI agents
  and automation
- **Modular Tools**: Each tool focuses on a specific task
- **Fast & Reliable**: Built in Rust for performance and
  safety
- **Consistent Interface**: Uniform command structure across
  all tools

## Installation

```bash
# From crates.io (when published)
cargo install kozmotic

# From source
git clone https://github.com/yourusername/kozmotic.git
cd kozmotic
cargo install --path .
```

## Usage

```bash
# Get help
kozmotic --help

# Run a tool
kozmotic <tool-name> [OPTIONS]
```

## Tools

Coming soon! This project is under active development.

## Output Format

All tools output JSON by default:

```json
{
  "status": "success",
  "data": { ... },
  "metadata": {
    "timestamp": "2026-02-15T20:00:00Z",
    "tool": "example",
    "version": "0.1.0"
  }
}
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run locally
cargo run -- --help
```

## Contributing

Contributions are welcome! Please feel free to submit a
Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Roadmap

- [ ] Core CLI framework
- [ ] File system operations tool
- [ ] Process management tool
- [ ] Network utilities
- [ ] Git operations tool
- [ ] Data transformation tools
- [ ] CI/CD integrations

## Why Kozmotic?

Traditional CLI tools are designed for human interaction,
which can make them difficult for agents to parse:
- Inconsistent output formats
- Mixed structured and unstructured data
- Varying exit codes and error reporting
- Different conventions across tools

Kozmotic solves this by providing a consistent, structured
interface across all tools, making automation and AI agent
integration seamless.
