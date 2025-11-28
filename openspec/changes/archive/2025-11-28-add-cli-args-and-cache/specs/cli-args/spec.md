# CLI Arguments and XDG Support

## ADDED Requirements

### Requirement: CLI Argument Parsing
The application MUST parse command-line arguments using `clap`.

#### Scenario: Help Command
When the user runs `cli --help`, the application SHOULD display usage information, including available flags and options.

#### Scenario: Version Command
When the user runs `cli --version`, the application SHOULD display the current version.

### Requirement: XDG Directory Support
The application MUST use XDG base directories for resolving file paths.

#### Scenario: Cache Directory Usage
The application MUST use the XDG cache directory (e.g., `~/.cache/cryptodash` on Linux) for storing temporary data and, for this iteration, primary data files.

#### Scenario: Config Directory Resolution
The application SHOULD be able to resolve configuration files from the XDG config directory.

### Requirement: Environment Variable Overrides
The application MUST allow overriding standard directories via environment variables.

#### Scenario: Cache Directory Override
When the `CRYPTODASH_CACHE_DIR` environment variable is set, the application MUST use its value as the cache directory instead of the XDG default.

#### Scenario: Data Directory Override
When the `CRYPTODASH_DATA_DIR` environment variable is set, the application MUST use its value as the data directory.
