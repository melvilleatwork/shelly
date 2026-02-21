# Shelly 🐚

Translate natural language into shell commands using AI.

```
$ shelly "what's my wifi ip address?"
▶ ipconfig getifaddr en0
ℹ Gets the IP address of the Wi-Fi interface on macOS
192.168.1.42
```

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) (1.75+)
- An [OpenAI API key](https://platform.openai.com/api-keys)

### Build from source

```bash
git clone <repo-url>
cd shelly
cargo install --path .
```

## Configuration

On first run, Shelly creates a config file at:

| OS      | Path                                          |
|---------|-----------------------------------------------|
| macOS   | `~/Library/Application Support/shelly/config.toml` |
| Linux   | `~/.config/shelly/config.toml`                |
| Windows | `%APPDATA%\shelly\config.toml`                |

Edit the file and add your OpenAI API key:

```toml
[openai]
api_key = "sk-..."
# model = "gpt-4o-mini"   # optional, defaults to gpt-4o-mini
```

## Usage

```bash
shelly "how much disk space do I have?"
shelly "what's my public IP?"
shelly "how many local time machine snapshots do I have?"
shelly "what process is using port 3000?"
```

### Safety

- **Safe commands** (read-only, non-destructive) run automatically.
- **Risky commands** (rm, sudo, kill, etc.) show a confirmation prompt before executing.

## License

MIT
