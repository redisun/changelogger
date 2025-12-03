# Contributing to Changelogger

Thanks for your interest in contributing to changelogger! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/changelogger.git`
3. Create a branch for your changes: `git checkout -b your-feature-name`
4. Make your changes
5. Test your changes
6. Submit a pull request

## Development Setup

Make sure you have Rust installed (1.70 or later). Then:

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the linter
cargo clippy

# Format code
cargo fmt
```

## Code Style

- Follow Rust conventions and use `cargo fmt` to format your code
- Run `cargo clippy` before submitting to catch common issues
- Write clear, descriptive commit messages following conventional commit format (see below)

## Commit Messages

Please follow the conventional commit format that changelogger itself uses:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test additions or changes
- `chore:` - Maintenance tasks

Examples:
- `feat: add support for custom commit prefixes`
- `fix: handle edge case in tag parsing`
- `docs: update installation instructions`

## Testing

- Add tests for new features and bug fixes
- Make sure all existing tests pass: `cargo test`
- Test your changes manually with various git repositories

## Pull Request Process

1. Make sure your code compiles and all tests pass
2. Update documentation if needed
3. Write a clear description of your changes in the PR
4. Reference any related issues
5. Be responsive to feedback and questions

## Reporting Bugs

When reporting bugs, please include:

- A clear description of the bug
- Steps to reproduce the issue
- Expected behavior
- Actual behavior
- Your environment (OS, Rust version, git version)
- Any relevant error messages or logs

## Feature Requests

Feature requests are welcome! Please open an issue describing:

- The feature you'd like to see
- Why it would be useful
- Any ideas you have for how it might work

## Questions?

Feel free to open an issue for any questions you have about contributing.

