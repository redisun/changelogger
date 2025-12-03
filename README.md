# Changelogger

A Rust command-line tool to automatically generate or update `CHANGELOG.md` files from git commits.

## Features

- **Automatic commit classification** - Categorizes commits based on conventional commit messages
- **Semantic versioning** - Computes version bumps based on commit types
- **Git integration** - Generates links to commits, issues, and releases when remote info is available
- **Interactive mode** - Prompts you to classify commits it doesn't recognize
- **Flexible** - Supports custom version tags, output files, and starting points

## Installation

### From Source

```bash
git clone <repository-url>
cd changelogger
cargo build --release
```

The binary will be available at `target/release/changelogger`.

### Using Cargo

```bash
cargo install --path .
```

## Usage

### Basic Usage

Generate a changelog from commits since the latest semver tag:

```bash
changelogger
```

### Specify a Custom Repository

```bash
changelogger --repo /path/to/repo
```

### Dry Run (Preview)

Preview the changelog without writing to file:

```bash
changelogger --dry-run
```

### Specify a Version

Manually specify the new version:

```bash
changelogger --new-version 1.2.3
```

### Start from a Specific Tag

Generate changelog from a specific tag:

```bash
changelogger --from-tag v1.0.0
```

### Non-Interactive Mode

Automatically classify unrecognized commits as patch releases:

```bash
changelogger --non-interactive
```

### Custom Output File

Write to a different file:

```bash
changelogger --output HISTORY.md
```

## Commit Message Conventions

Changelogger uses conventional commit message prefixes to automatically classify commits. Commits should follow the format:

```
type: subject
```

or

```
type(scope): subject
```

### Supported Prefixes

#### Major (Breaking Changes)
- `breaking:` - Breaking changes that require a major version bump
- `major:` - Major changes

#### Minor (New Features)
- `feat:` - New features
- `minor:` - Minor changes

#### Patch (Bug Fixes)
- `fix:` - Bug fixes
- `fixes:` - Bug fixes
- `perf:` - Performance improvements
- `refactor:` - Code refactoring
- `patch:` - Patch-level changes
- `tweak:` - Small tweaks
- `tweaks:` - Small tweaks

#### Ignored (Not in Changelog)
- `docs:` - Documentation changes
- `doc:` - Documentation changes
- `style:` - Code style changes
- `chore:` - Maintenance tasks
- `test:` - Test changes

### Examples

```bash
# Major version bump
git commit -m "breaking: change API signature"

# Minor version bump
git commit -m "feat: add user authentication"

# Patch version bump
git commit -m "fix: resolve memory leak in parser"

# Ignored (won't appear in changelog)
git commit -m "docs: update README"
```

### Release Messages

Commits with the format `-> v1.2.3` or `-> 1.2.3` are treated as release markers and are ignored.

## Command-Line Options

```
Usage: changelogger [OPTIONS]

Options:
      --repo <REPO>                Path to the repository, defaults to current directory [default: .]
      --new-version <NEW_VERSION>  Optional new version, otherwise computed from commits
      --from-tag <FROM_TAG>        Optional tag to start from, otherwise latest semver tag is used
      --output <OUTPUT>            File to write the changelog to [default: CHANGELOG.md]
      --dry-run                    Dry run, print to stdout instead of writing file
      --non-interactive            Do not ask interactive questions, unknown commits become patch by default
  -h, --help                       Print help
  -V, --version                    Print version
```

## How It Works

1. **Find Version Tags**: Searches for semantic version tags (e.g., `v1.2.3`) in the repository
2. **Collect Commits**: Retrieves all commits since the latest tag (or from a specified tag)
3. **Classify Commits**: Automatically categorizes commits based on their message prefixes
4. **Interactive Classification**: Prompts for unrecognized commits (unless `--non-interactive` is used)
5. **Compute Version**: Determines the new version based on commit categories:
   - Major commits → major version bump
   - Minor commits → minor version bump
   - Patch commits → patch version bump
6. **Generate Changelog**: Creates a markdown-formatted changelog section with:
   - Version header with date
   - Grouped commits by category (Breaking changes, New features, Bug fixes)
   - Links to commits and issues (if remote info is available)
   - Comparison links between versions

## Example Output

```markdown
## [Version 1.2.0](https://github.com/user/repo/releases/tag/v1.2.0) (2024-01-15)

### New features

* Add user authentication system [`abc1234`](https://github.com/user/repo/commit/abc1234) ([#42](https://github.com/user/repo/issues/42))
* Implement dark mode toggle [`def5678`](https://github.com/user/repo/commit/def5678)

### Bug fixes

* Fix memory leak in parser [`ghi9012`](https://github.com/user/repo/commit/ghi9012) ([#38](https://github.com/user/repo/issues/38))

[...full changes](https://github.com/user/repo/compare/v1.1.0...v1.2.0)
```

## Requirements

- Rust 1.70 or later
- Git repository with semantic version tags (optional, but recommended)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

