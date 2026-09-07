# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html),

## [0.1.1] - 2026-09-07

### Added

- add `kind = "sql"` support, loading test data from `INSERT INTO ... VALUES ...;` statements

### Changed

- rework the `README.md` examples around one realistic signup / account-validation domain expressed across every supported format

### Tests

- add realistic example suites (`tests/examples_scalars.rs`, `tests/examples_nested.rs`, `tests/examples_async.rs`) backed by data files in `tests/examples/`
- add panic-path tests for SQL root-table detection: empty input, cyclic references, and a non-`Option` parameter for an unfilled multi-table position

## [0.1.0] - 2025-01-21

### Added

- initial release of the `#[test_data_file(path = "...")]` attribute macro, calling the decorated test once per row/entry in a data file
- support for `csv`, `json`, `yaml`, `ron`, `toml`, and a space-separated `list` format, inferred from the file extension or set with `kind`
- scalar, `Option<T>`, nested struct, and `Vec<T>` parameters
- top-level arrays and named-key maps (named cases surface in test output)
- async test functions via `#[tokio::test]`

[Unreleased]: https://github.com/songokas/test-data-file/compare/0.1.1...HEAD
[0.1.1]: https://github.com/songokas/test-data-file/releases/tag/0.1.0...0.1.1
[0.1.0]: https://github.com/songokas/test-data-file/releases/tag/0.1.0
