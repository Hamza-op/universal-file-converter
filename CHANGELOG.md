# Changelog

All notable changes to MediaForge are documented here. Versions follow [Semantic Versioning](https://semver.org/).

## [1.3.0] - 2026-08-22

### Added

- Stage conversions beside their destination and commit them only after successful completion.
- Show cancelled files separately from failures and aggregate concurrent file progress accurately.
- Show the conversion engine's actionable error directly beneath each failed file.
- Prefer specific unsupported-codec, sample-rate, path, and permission diagnostics over generic FFmpeg exit messages.
- Exercise every advertised output format plus failure cleanup, overwrite preservation, and queued cancellation with generated fixtures and bundled FFmpeg.
- Test macOS and Linux targets in CI in addition to the Windows release pipeline.
- Block pull requests on known RustSec vulnerabilities in the locked dependency graph.
- Publish bundled FFmpeg provenance, license, source, and hashes with each Windows release.

### Fixed

- Use container-compatible video/audio codecs for AVI, WMV, and MPEG output; MPEG no longer fails with an unsupported AAC stream.
- Encode OPUS at its required 48 kHz rate instead of failing with the default 44.1 kHz setting.
- Encode ICO images with the required RGBA payload so generated icons remain readable.
- Route AVIF input through FFmpeg because the native image backend is encode-only for AVIF in this build.
- Prevent same-named batch entries from sharing a destination even when overwrite mode is enabled.
- Prevent same-format conversions from selecting the source file as their output.
- Remove partial staging files after failed and cancelled jobs, preserving an existing destination on failure.
- Verify extracted FFmpeg binaries byte-for-byte instead of trusting file size alone.
- Preserve partial/older configuration files with defaults and sanitize unsafe out-of-range values on load.
- Upgrade the GUI, dialog, image, and browser-opening dependency stack and replace the vulnerable XML-based notification backend with platform-native commands.

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

[1.3.0]: https://github.com/Hamza-op/universal-file-converter/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Hamza-op/universal-file-converter/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/Hamza-op/universal-file-converter/releases/tag/v1.1.0
[1.0.1]: https://github.com/Hamza-op/universal-file-converter/releases/tag/v1.0.1
