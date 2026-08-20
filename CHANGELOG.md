# Changelog

All notable changes to MediaForge are documented here. Versions follow [Semantic Versioning](https://semver.org/).

## [1.2.0] - 2026-08-20

### Fixed

- Reserve output names before concurrent batch work begins, preventing same-named inputs from overwriting or contending for one destination.
- Keep the Windows executable version metadata synchronized with the Cargo package version.
- Expose the package version through the standard `--version` command-line flag.

### Changed

- Validate formatting, Clippy lints, tests, and release builds on pull requests and main-branch pushes.
- Publish stable releases only from semantic-version tags, with a SHA-256 checksum alongside the executable.
- Correct source-build and release links in the README.

## [1.1.0] - 2025-06-01

- Reduced the default eframe feature set for a smaller portable build.

## [1.0.1] - 2025-05-31

- Added automated Windows executable packaging.

[1.2.0]: https://github.com/Hamza-op/universal-file-converter/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/Hamza-op/universal-file-converter/releases/tag/v1.1.0
[1.0.1]: https://github.com/Hamza-op/universal-file-converter/releases/tag/v1.0.1
