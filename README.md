# cargo-build-rx

Compile-time diagnostic and prescription tool for Rust projects.

Rust compile times are the #1 developer complaint. **42% of developers don't know the easy fixes.** `cargo-build-rx` scans your project in under 2 seconds — no compilation required — and produces a ranked list of prescriptions to speed up your builds.

## Installation

```bash
cargo install cargo-build-rx
```

## Usage

```bash
# Run all checks
cargo build-rx

# JSON output (for CI or tooling)
cargo build-rx --format json

# Run only specific checks
cargo build-rx --only linker,profile

# Skip specific checks
cargo build-rx --skip dev-deps

# Only show fixes (hide warnings and info)
cargo build-rx --min-severity fix
```

## Example output

```
cargo-build-rx — 5 prescriptions for my-web-app

   FIX [High] Default system linker detected
       mold is installed and would be 2-5x faster for linking.

       -> Add to .cargo/config.toml:
         [target.x86_64-unknown-linux-gnu]
         linker = "clang"
         rustflags = ["-C", "link-arg=-fuse-ld=mold"]

   FIX [Medium] Missing build-override opt-level for proc-macros
       Proc-macros and build scripts run at opt-level 0 by default.
       Compiling them with opt-level 3 makes them run faster during builds.

       -> In Cargo.toml [profile.dev.build-override]:
         opt-level = 3

  WARN [Medium] syn v1 and v2 both present
       Both syn 1.x and syn 2.x are compiled. syn is the most expensive
       proc-macro dependency. Updating all dependents to use syn 2.x can
       eliminate a full extra compilation of syn.

       -> Run: cargo update  # then check: cargo tree -d -p syn

  WARN [Medium] tokio uses "full" feature
       The "full" feature pulls in many sub-features, increasing compile time.
       Only enable the features you actually use (e.g., rt-multi-thread, macros, io-util).

  INFO [Low] 12 crates with build scripts
       12 crates have build.rs scripts.

Summary: 2 fixes, 2 warnings, 1 info
```

Exit code is **1** if any Fix-severity findings are present, **0** otherwise — useful for CI gates.

## Checks

| # | Check | What it detects |
|---|-------|-----------------|
| 1 | **linker** | Default ld on Linux (recommends mold/lld). macOS split-debuginfo. |
| 2 | **profile** | debuginfo=2, opt-level>0 in dev, missing build-override for proc-macros |
| 3 | **duplicates** | Multiple semver-incompatible versions (escalates for syn, serde, tokio) |
| 4 | **proc-macros** | syn v1+v2 split, total proc-macro count >15 |
| 5 | **build-scripts** | Inventory of build.rs crates, flags those with native `links` |
| 6 | **features** | tokio/full, reqwest/default-tls, and other heavy default features |
| 7 | **dev-deps** | criterion, proptest, and other heavy dev-dependencies |
| 8 | **toolchain** | rustc >2 minor versions behind stable |
| 9 | **workspace** | Multi-crate workspace without workspace-hack crate |
| 10 | **incremental** | CARGO_INCREMENTAL=0 set in local dev environment |

## How it works

`cargo-build-rx` runs `cargo metadata` and reads your `Cargo.toml`, `.cargo/config.toml`, and environment. All checks are pure functions over this data — **no compilation of your project is required**. Typical runtime is under 2 seconds.

## CI usage

```yaml
# GitHub Actions example
- name: Check build hygiene
  run: |
    cargo install cargo-build-rx
    cargo build-rx --min-severity fix
```

The exit code is 1 when fixable issues are found, so it naturally fails the CI step.

## JSON output

```bash
cargo build-rx --format json
```

Returns an array of findings, each with `severity`, `category`, `impact`, `title`, `description`, and optional `fix` with structured `kind` (CargoConfig, CargoToml, ShellCommand, or Manual).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
