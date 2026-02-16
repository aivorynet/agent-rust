# Contributing to AIVory Monitor Rust Agent

Thank you for your interest in contributing to the AIVory Monitor Rust Agent. Contributions of all kinds are welcome -- bug reports, feature requests, documentation improvements, and code changes.

## How to Contribute

- **Bug reports**: Open an issue at [GitHub Issues](https://github.com/aivorynet/agent-rust/issues) with a clear description, steps to reproduce, and your environment details (Rust version, OS, target triple).
- **Feature requests**: Open an issue describing the use case and proposed behavior.
- **Pull requests**: See the Pull Request Process below.

## Development Setup

### Prerequisites

- Rust 1.70 or later (install via [rustup](https://rustup.rs/))

### Build and Test

```bash
cd monitor-agents/agent-rust
cargo build
cargo test
```

### Running the Agent

Add the agent as a dependency in your `Cargo.toml` and call the initialization function at application startup. See the README for integration details.

## Coding Standards

- Follow the existing code style in the repository.
- Write tests for all new features and bug fixes.
- Run `cargo clippy` and resolve all warnings before submitting.
- Run `cargo fmt` to ensure consistent formatting.
- Panic hooks must be safe and must not introduce undefined behavior.
- Keep `unsafe` usage to an absolute minimum and document the safety invariants.

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.
2. Make your changes and write tests.
3. Ensure all tests pass (`cargo test`) and clippy is clean (`cargo clippy`).
4. Submit a pull request on [GitHub](https://github.com/aivorynet/agent-rust) or GitLab.
5. All pull requests require at least one review before merge.

## Reporting Bugs

Use [GitHub Issues](https://github.com/aivorynet/agent-rust/issues). Include:

- Rust version (`rustc --version`) and OS
- Agent version
- Backtrace or error output
- Minimal reproduction steps

## Security

Do not open public issues for security vulnerabilities. Report them to **security@aivory.net**. See [SECURITY.md](SECURITY.md) for details.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
