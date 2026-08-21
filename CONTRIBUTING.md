# Contributing to Shelly

Thanks for your interest in contributing!

## Getting started

```bash
git clone https://github.com/graememelville/shelly
cd shelly
cargo build
cargo test
```

## Before opening a PR

- `cargo fmt --check` passes
- `cargo clippy -- -D warnings` is clean
- `cargo test` passes
- Add tests for new behavior where practical
- Update `CHANGELOG.md` under **Unreleased**

## Reporting bugs

Open an issue with:

1. Your OS and shell
2. The exact command you ran
3. Expected vs. actual behavior

## Security

If you find a security issue (especially anything that could cause Shelly to execute unintended commands), please open a private security advisory rather than a public issue.
