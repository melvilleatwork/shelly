# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Bring-your-own-provider support: any OpenAI-compatible endpoint via `base_url`, plus native Anthropic support.
- Provider selection via `provider` config field or inference from `base_url`.
- `ANTHROPIC_API_KEY` environment variable support.
- Local model support via Ollama (no API key required).

### Changed

- Config file format moved from `[openai]` to `[provider]` section with `base_url` and `model` fields.
- Shelly now exits with the exit code of the command it ran, instead of always exiting 0.
- `ANTHROPIC_API_KEY` selects Anthropic when no provider, `base_url` or other API key is set.
- A missing API key is only an error for default endpoints; with a custom `base_url` the request is sent without an auth header.

### Fixed

- Ollama and other keyless endpoints no longer need a dummy API key.
- An empty `SHELLY_API_KEY` no longer hides the provider-specific environment variables.
- The model is told which shell commands actually run in (`sh` or `cmd`), not the user's login shell.

### Security

- Commands now require confirmation unless the model explicitly labels them `safe`. Previously, any risk label other than `risky` ran without confirmation.

## [0.1.0] - 2026-08-21

### Added

- Natural language to shell command translation via OpenAI.
- OS and shell detection for platform-appropriate commands.
- Risk classification with confirmation prompt for destructive commands.
- `--dry-run` flag to preview commands without executing them.
- API key support via `SHELLY_API_KEY` / `OPENAI_API_KEY` environment variables.
- Config file support (`config.toml`) with automatic creation on first run.
