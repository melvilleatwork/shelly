# Shelly 🐚

**Ask your shell questions in plain English. Get the right command.**

Shelly translates natural language into shell commands using AI, explains what it's about to run, and keeps you safe with a built-in risk check.

```text
$ shelly "what's my wifi ip address?"

  Command: ipconfig getifaddr en0
  Reason:  Gets the IP address of the Wi-Fi interface on macOS

192.168.1.42
```

## Why Shelly?

- **No more memorizing flags.** "how much disk space do I have?" just works.
- **OS-aware.** Knows whether you're on macOS, Linux, or Windows and picks the right command.
- **Safe by default.** Read-only commands run automatically; anything destructive (`rm`, `sudo`, `kill`, ...) requires explicit confirmation.
- **Fast and tiny.** A single static binary written in Rust.

## Installation

### Homebrew (macOS / Linux)

```bash
brew install shelly
```

### Cargo

```bash
cargo install shellysh
```

> The crate is published as `shellysh` (the `shelly` name was taken), but the binary you run is still `shelly`.

### Prebuilt binaries

Download a binary for your platform from the [releases page](https://github.com/graememelville/shelly/releases).

### Build from source

```bash
git clone https://github.com/graememelville/shelly
cd shelly
cargo install --path .
```

## Configuration

Shelly needs an [OpenAI API key](https://platform.openai.com/api-keys). Two ways to provide it:

**Option 1 — environment variable (recommended):**

```bash
export SHELLY_API_KEY="sk-..."   # OPENAI_API_KEY also works
```

**Option 2 — config file:**

On first run, Shelly creates a config file at:

| OS      | Path                                               |
|---------|----------------------------------------------------|
| macOS   | `~/Library/Application Support/shelly/config.toml` |
| Linux   | `~/.config/shelly/config.toml`                     |
| Windows | `%APPDATA%\shelly\config.toml`                     |

Edit it:

```toml
[openai]
api_key = "sk-..."
# model = "gpt-4o-mini"   # optional, defaults to gpt-4o-mini
```

## Usage

```bash
shelly "how much disk space do I have?"
shelly "what's my public IP?"
shelly "what process is using port 3000?"
shelly "how many local time machine snapshots do I have?"
```

### Options

| Flag         | Description                            |
|--------------|----------------------------------------|
| `-d, --dry-run` | Show the command without executing it |
| `-V, --version` | Print version                        |

### Safety

Shelly classifies every command before running it:

- **Safe** (read-only, non-destructive) runs automatically.
- **Risky** (`rm`, `sudo`, `kill`, `dd`, `shutdown`, ...) shows a confirmation prompt first.
- Not sure? Use `--dry-run` to preview without executing.

## Development

```bash
cargo build          # build
cargo test           # run tests
cargo clippy         # lint
cargo fmt            # format
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
