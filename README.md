# cargo-build-rx

Compile-time diagnostic and prescription tool for Rust projects.

Rust compile times are the #1 developer complaint. 42% of developers don't know the easy fixes. `cargo-build-rx` scans your project in under 2 seconds (no compilation required) and produces a ranked prescription list to speed up your builds.

## Installation

```bash
cargo install cargo-build-rx
```

## Usage

```bash
cargo build-rx                        # run all checks, terminal output
cargo build-rx --format json          # JSON output for CI/tooling
cargo build-rx --only linker,profile  # run specific checks only
cargo build-rx --skip dev-deps        # skip specific checks
cargo build-rx --min-severity fix     # only show actionable fixes
```

## Proof: real output

### Running on itself

This is the actual output of `cargo build-rx` run on its own source tree:

```
cargo-build-rx — 3 prescriptions for cargo-build-rx

  FIX  [Medium] split-debuginfo not enabled on macOS
       Using split-debuginfo=unpacked avoids bundling debug info during linking.

       -> In Cargo.toml [profile.dev]:
         split-debuginfo = "unpacked"

  FIX  [Medium] Missing build-override opt-level for proc-macros
       Proc-macros and build scripts run at opt-level 0 by default.
       Compiling them with opt-level 3 makes them run faster during builds.

       -> In Cargo.toml [profile.dev.build-override]:
         opt-level = 3

  INFO [Low] 9 crates with build scripts
       9 crates have build.rs scripts.

Summary: 2 fixes, 1 info
```

### Running on a bloated project

A test project with tokio (full features), old serde pulling syn v1, openssl, criterion, proptest, debug=2, and opt-level=1 in dev:

```
cargo-build-rx — 10 prescriptions for test-bloated-project

  FIX  [Medium] split-debuginfo not enabled on macOS
       Using split-debuginfo=unpacked avoids bundling debug info during linking.

       -> In Cargo.toml [profile.dev]:
         split-debuginfo = "unpacked"

  FIX  [Medium] Missing build-override opt-level for proc-macros
       Proc-macros and build scripts run at opt-level 0 by default.
       Compiling them with opt-level 3 makes them run faster during builds.

       -> In Cargo.toml [profile.dev.build-override]:
         opt-level = 3

  WARN [Medium] Full debuginfo in dev profile
       debug = 2 (full) slows compilation. Consider debug = 1 (line tables only)
       unless you need full variable inspection.

       -> In Cargo.toml [profile.dev]:
         debug = 1

  WARN [Medium] opt-level = 1 in dev profile
       Optimization in dev slows compile times significantly.
       Consider using opt-level = 0 for dev builds.

       -> In Cargo.toml [profile.dev]:
         opt-level = 0

  WARN [Medium] syn present in 2 incompatible versions
       Versions: 1.0.109, 2.0.117. Each distinct version is compiled separately.

       -> Run: cargo update -p syn

  WARN [Medium] syn v1 and v2 both present
       Both syn 1.x and syn 2.x are compiled. syn is the most expensive
       proc-macro dependency. Updating all dependents to use syn 2.x can
       eliminate a full extra compilation of syn.

       -> Run: cargo update  # then check: cargo tree -d -p syn

  WARN [Medium] tokio uses "full" feature
       The "full" feature pulls in many sub-features, increasing compile time.
       Only enable the features you actually use (e.g., rt-multi-thread, macros, io-util).

  INFO [Medium] 21 crates with build scripts
       21 crates have build.rs scripts.
       Native linking (may invoke C compiler): openssl-sys 0.9.113,
       rayon-core 1.13.0, wasm-bindgen-shared 0.2.118

  INFO [Low] openssl has default features enabled
       Consider using rustls as a TLS backend to avoid native compilation.

  INFO [Low] 2 heavy dev-dependencies
       criterion: heavyweight benchmarking framework
       proptest: property-based testing (includes regex, bitset, etc.)

Summary: 2 fixes, 5 warnings, 3 info
```

All 10 findings are correct. The 3 checks that stayed silent (toolchain, workspace, incremental) are also correct: rustc is current, it's not a workspace, and CARGO_INCREMENTAL is not set to 0.

Exit code is **1** if any Fix-severity findings exist, **0** otherwise. Useful for CI gates.

## The 10 checks

| # | Check | What it detects |
|---|-------|-----------------|
| 1 | **linker** | Default ld on Linux (recommends mold/lld). macOS split-debuginfo. |
| 2 | **profile** | debuginfo=2, opt-level>0 in dev, missing build-override for proc-macros |
| 3 | **duplicates** | Multiple semver-incompatible versions (escalates for syn, serde, tokio) |
| 4 | **proc-macros** | syn v1+v2 split, total proc-macro count >15 |
| 5 | **build-scripts** | Inventory of build.rs crates, flags those with native `links` |
| 6 | **features** | tokio/full, reqwest/default-tls, other heavy default features |
| 7 | **dev-deps** | criterion, proptest, other heavy dev-dependencies |
| 8 | **toolchain** | rustc more than 2 minor versions behind stable |
| 9 | **workspace** | Multi-crate workspace without workspace-hack crate |
| 10 | **incremental** | CARGO_INCREMENTAL=0 set in local dev environment |

## How it works

`cargo-build-rx` runs `cargo metadata` and reads your `Cargo.toml`, `.cargo/config.toml`, and environment variables. All checks are pure functions over this data. No compilation of your project happens. Typical runtime is under 2 seconds.

## CI usage

```yaml
- name: Check build hygiene
  run: |
    cargo install cargo-build-rx
    cargo build-rx --min-severity fix
```

Exit code 1 when fixable issues are found, so the CI step fails naturally.

## JSON output

```bash
cargo build-rx --format json
```

Returns an array of findings with `severity`, `category`, `impact`, `title`, `description`, and optional `fix` with structured `kind` (CargoConfig, CargoToml, ShellCommand, or Manual).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
