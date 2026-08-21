# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-21

### Added

- Natural language to shell command translation via OpenAI.
- OS and shell detection for platform-appropriate commands.
- Risk classification with confirmation prompt for destructive commands.
- `--dry-run` flag to preview commands without executing them.
- API key support via `SHELLY_API_KEY` / `OPENAI_API_KEY` environment variables.
- Config file support (`config.toml`) with automatic creation on first run.
