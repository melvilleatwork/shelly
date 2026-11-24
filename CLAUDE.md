# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Shelly is a Rust CLI that sends a natural-language question to an LLM, gets back one shell command, and runs it. The crate is `shellysh` (name collision on crates.io); the binary is `shelly`.

## Commands

These match CI, which runs on Linux, macOS and Windows with stable Rust:

```bash
cargo fmt --all -- --check
cargo build --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings   # --all-targets is needed to lint test modules
cargo test --all-features

cargo test parses_fenced_json     # single test
cargo test llm::tests::           # one module's tests
cargo run -- -d "question"        # dry run: still calls the LLM, does not execute the command
```

Running the binary creates a default config in the real user config directory if none exists. To run it without touching that (macOS/Linux), set a throwaway home: `env -i PATH="$PATH" HOME=/some/tmp/dir target/debug/shelly ...`. `env -i` also drops API keys, so pass `SHELLY_API_KEY=...` explicitly.

Unit tests live inline in each module and need no network or config file, though the `executor.rs` tests spawn real `sh`/`cmd` processes. There are no integration tests; the confirmation prompt, the file handling in `Config::load` and the HTTP calls are untested. Tests cannot set environment variables (`set_var` is unsafe in edition 2024 and `unsafe_code` is denied), so inject the lookup as `Config::resolve` does.

Update `CHANGELOG.md` under **Unreleased** before opening a PR. Pushing a `v*` tag builds release binaries and then runs `cargo publish`.

## Lint regime

`Cargo.toml` sets `warnings = "deny"`, so every lint is a hard error, including those listed as `warn` (all of `clippy::pedantic`). Plain `cargo build` also fails on rustc warnings such as dead code.

- Non-test code cannot use `unwrap`, `expect`, `panic!`, `todo!`, indexing, string slicing, `as` casts, or arithmetic operators that can overflow. Use `.get()`/`.first()`, `strip_prefix`, `From`/`TryFrom` and checked/saturating ops.
- `#[allow]` is rejected. Suppress with a narrow `#[expect(lint, reason = "...")]`, and in production code only for the cases allowlisted in the comment above `[lints.rust]` in `Cargo.toml`.
- Test modules that unwrap carry `#[expect(clippy::unwrap_used, reason = "test code")]`. Adding it to a module that does not unwrap fails as an unfulfilled expectation, which is why `prompt.rs` has none.

Fallible functions return `Result<_, String>` with a user-facing message; `main` prints it and exits 1. There is no error crate.

## Architecture

`main` → `Config::load` (config.rs) → `ask_llm` (llm.rs, using the system prompt from prompt.rs) → `parse_response` → `executor::execute`.

**The JSON contract spans three files.** The prompt in `prompt.rs` asks for `command`, `explanation` and `risk`; `ShellCommand` in `llm.rs` deserializes it (all three fields required, markdown fences tolerated); `executor.rs` acts on it. Change them together. An empty `command` means "can't be done": the explanation is printed and nothing runs.

**The risk gate is the model's own label.** `ShellCommand::is_risky` fails closed: only `risk` equal to `"safe"` (case-insensitive) runs without confirmation. There is no local inspection of the command, so a destructive command the model labels `"safe"` still runs unprompted. Treat changes to `is_risky`, the prompt's risk definitions, or the confirmation flow in `execute` as security-sensitive.

**Providers.** `ProviderKind` has two variants: `OpenAi`, which covers every OpenAI-compatible endpoint (Groq, DeepSeek, Mistral, OpenRouter, Ollama), and `Anthropic`, which uses the native Messages API. A new OpenAI-compatible service needs no code. A new wire format needs a variant, its `match` arms in `config.rs`, an `ask_*` function and `with_auth` arm in `llm.rs`, and the README provider table.

**Config resolution** (`Config::load` reads the file, `Config::resolve` is the pure logic):
- Provider kind is decided first: explicit `provider`, else `base_url` containing `"anthropic"`, else Anthropic if neither is set and `ANTHROPIC_API_KEY` is the only key available anywhere, else OpenAI.
- The key is then `SHELLY_API_KEY`, else the provider's env var, else the config file. Blank values and the default-config placeholder count as not set at every step.
- A missing key is an error only for default endpoints. With a custom `base_url`, `api_key` is `None` and `with_auth` in `llm.rs` sends no auth header (this is how Ollama works); a 401/403 without a key gets a hint from `missing_key_hint`.
- `base_url` has the endpoint path appended verbatim, so a trailing slash produces a double slash.

**Execution.** Commands always run through `executor::SHELL` (`sh -c`, or `cmd /C` on Windows), never the user's `$SHELL`. `prompt.rs` reports the same constant to the model and mentions the login shell only as context. Shelly exits with the command's exit code (1 if killed by a signal); dry runs, cancelled commands and empty commands exit 0.
