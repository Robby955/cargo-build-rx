# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-06-27

### Fixed

- **Test fixtures now ship with the published crate.** The `clean` and `bloated`
  fixture projects under `tests/fixtures/` each carried a `Cargo.toml`, which made
  cargo treat them as nested crates and strip the directories from the packaged
  `.crate` tarball. A downstream user who unpacked the crate and ran `cargo test`
  hit failures because the fixtures the `tests/smoke.rs` integration test depends
  on were missing. Each fixture manifest is now committed as `Cargo.toml.fixture`,
  so cargo packages the directory; `tests/smoke.rs` copies a fixture into a temp
  dir and restores `Cargo.toml` before invoking the tool. `cargo package --list`
  now contains the fixture files and downstream `cargo test` passes from the
  published crate.

## [0.2.0] - 2026-06-26

### Changed

- **Severity recalibration.** The "missing build-override opt-level" check is
  now `Warn` instead of `Fix`, and the macOS `split-debuginfo` check is now
  `Info` (recent toolchains may already default to unpacked). These fired on
  nearly every project, so `Fix` is reserved for changes that are almost always
  correct.
- **Exit code is opt-in.** The tool exits `0` by default. Use the new
  `--deny <none|fix|warn|info>` flag to exit non-zero for CI gating. Previously
  any `Fix` finding always forced exit `1`.
- **Linker fix targets the real host.** The recommended `.cargo/config.toml`
  block now uses the host triple parsed from `rustc -vV` (e.g.
  `aarch64-apple-darwin`) instead of a hardcoded `x86_64-unknown-linux-gnu`.
- **Toolchain check is MSRV-aware.** It compares the installed toolchain against
  the project's declared `rust-version` (authoritative) and only emits a soft,
  dated `Info` hint when far behind a static reference snapshot.
- **Duplicate versions sort by precedence.** The duplicates check holds
  `semver::Version` values, so `1.9.0` sorts before `1.10.0`, and the wording is
  now "distinct compiled versions" rather than "incompatible versions".

### Added

- `--color <auto|always|never>` flag; terminal color now honors TTY detection
  and the `NO_COLOR` environment variable.
- A library target (`cargo_build_rx`) exposing the check engine, with crate- and
  item-level documentation and a runnable doctest.
- Per-check unit tests, property tests for the severity filter and deny
  threshold, JSON-serialization tests for every `FixKind`, and `clean`/`bloated`
  fixture projects under `tests/fixtures/` with end-to-end assertions.
- `rust-version` (MSRV 1.85, verified against the committed lockfile) plus
  `homepage`, `documentation`, `readme`, and `authors` in `Cargo.toml`.

### Fixed

- Removed an intentional memory leak: the duplicates check no longer leaks a
  `String` to `&'static str` per package via `.leak()`.
- The profile check now recognizes string opt-levels (`"s"`, `"z"`), which were
  previously treated as `0` and skipped.
- Linker detection scans `PATH` directly instead of spawning `which`, which is
  portable and avoids a subprocess per linker.
- JSON output surfaces serialization errors instead of silently emitting `[]`.

## [0.1.0]

### Added

- Initial release: ten checks (linker, profile, duplicates, proc-macros,
  build-scripts, features, dev-deps, toolchain, workspace, incremental) over a
  single gathered project context, with terminal and JSON output.
