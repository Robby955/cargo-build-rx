# cargo-build-rx

Compile-time diagnostic and prescription tool for Rust projects.

Slow Rust builds usually trace back to a short list of fixable causes: a slow
default linker, an over-broad feature set, a few crates compiled in several
versions, or a dev profile tuned for the wrong thing. `cargo-build-rx` reads
your project's metadata and configuration (it does not compile your code) and
prints a ranked list of concrete changes, each with the exact edit to make.

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
cargo build-rx --min-severity fix     # only show the strongest recommendations
cargo build-rx --color never          # disable ANSI color (also honors NO_COLOR)
cargo build-rx --deny warn            # exit non-zero if any Warn/Fix finding exists
```

## Severity levels

- **FIX**: nearly always correct to change (e.g. a fast linker is installed but
  unused).
- **WARN**: a real win that depends on the project; worth reviewing.
- **INFO**: surfaced for awareness.

By default the tool always exits `0`. Pass `--deny <fix|warn|info>` to turn it
into a failing gate (see [CI usage](#ci-usage)).

## Proof: real output

These are the actual outputs of the current build, reproducible by anyone.

### Running on itself

```
cargo-build-rx — 3 prescriptions for cargo-build-rx

  WARN [Medium] Missing build-override opt-level for proc-macros
       Proc-macros and build scripts run at opt-level 0 by default. Compiling them with opt-level 3 makes them run faster during builds.

       -> In Cargo.toml [profile.dev.build-override]:
         opt-level = 3

  INFO [Low] split-debuginfo not set on macOS
       Setting split-debuginfo = "unpacked" avoids bundling debug info during linking. Recent toolchains may already default to this for dev.

       -> In Cargo.toml [profile.dev]:
         split-debuginfo = "unpacked"

  INFO [Low] 9 crates with build scripts
       9 crates have build.rs scripts.

Summary: 1 warning, 2 info
```

### Running on a fixture with a slow dev profile

The repository ships `tests/fixtures/bloated`, a crate whose dev profile sets
`opt-level = 2` and `debug = 2`. Run `cargo build-rx --manifest-path
tests/fixtures/bloated/Cargo.toml`:

```
cargo-build-rx — 4 prescriptions for bloated-fixture

  WARN [Medium] Full debuginfo in dev profile
       debug = 2 (full) slows compilation. Consider debug = 1 (line tables only) unless you need full variable inspection.

       -> In Cargo.toml [profile.dev]:
         debug = 1

  WARN [Medium] opt-level = 2 in dev profile
       Optimization in dev slows compile times significantly. Consider using opt-level = 0 for dev builds.

       -> In Cargo.toml [profile.dev]:
         opt-level = 0

  WARN [Medium] Missing build-override opt-level for proc-macros
       Proc-macros and build scripts run at opt-level 0 by default. Compiling them with opt-level 3 makes them run faster during builds.

       -> In Cargo.toml [profile.dev.build-override]:
         opt-level = 3

  INFO [Low] split-debuginfo not set on macOS
       Setting split-debuginfo = "unpacked" avoids bundling debug info during linking. Recent toolchains may already default to this for dev.

       -> In Cargo.toml [profile.dev]:
         split-debuginfo = "unpacked"

Summary: 3 warnings, 1 info
```

Linker and toolchain findings depend on the host, so output varies by machine.

## The 10 checks

| # | Check | What it detects |
|---|-------|-----------------|
| 1 | **linker** | Default `ld` on Linux (recommends mold/lld, targeting the real host triple). On macOS, an unset dev `split-debuginfo`. |
| 2 | **profile** | `debug = 2`, `opt-level > 0` in dev (integer or `"s"`/`"z"`), missing `build-override` opt-level for proc-macros. |
| 3 | **duplicates** | The same crate compiled in several distinct versions (escalates for syn, serde, tokio, ...). |
| 4 | **proc-macros** | `syn` 1.x and 2.x both present; a high total proc-macro count. |
| 5 | **build-scripts** | Inventory of `build.rs` crates, flagging those with native `links`. |
| 6 | **features** | `tokio`/`full`, `reqwest`/`default-tls`, and other heavy default feature sets. |
| 7 | **dev-deps** | `criterion`, `proptest`, and other heavy dev-dependencies. |
| 8 | **toolchain** | Installed `rustc` older than the project's `rust-version` (MSRV); a dated hint when far behind stable. |
| 9 | **workspace** | Multi-crate workspace without a `workspace-hack` crate. |
| 10 | **incremental** | `CARGO_INCREMENTAL=0` set in the local dev environment. |

## How it works

`cargo-build-rx` runs `cargo metadata` and reads your `Cargo.toml`,
`.cargo/config.toml`, and a few environment variables. Every check is a pure
function over this gathered data. No compilation of your project happens, so a
run finishes in roughly a second on a typical crate.

The check engine is also exposed as a library (`cargo_build_rx`) for embedding
or testing; see the crate docs.

## CI usage

```yaml
- name: Check build hygiene
  run: |
    cargo install cargo-build-rx
    cargo build-rx --deny fix
```

`--deny fix` exits non-zero only when a `Fix`-severity finding exists, so the
step fails just for the strongest recommendations. Use `--deny warn` for a
stricter gate. Without `--deny`, the tool reports and exits `0`.

## JSON output

```bash
cargo build-rx --format json
```

Returns an array of findings with `severity`, `category`, `impact`, `title`,
`description`, and an optional `fix` carrying a structured `kind`
(`CargoConfig`, `CargoToml`, `ShellCommand`, or `Manual`).

## Minimum supported Rust version

1.85, verified against the committed `Cargo.lock`. Newer dependency versions may
raise the effective floor when the lockfile is not used.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
