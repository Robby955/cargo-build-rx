//! `cargo-build-rx`: a compile-time diagnostic and prescription tool for Rust projects.
//!
//! The tool reads a project's `cargo metadata`, its `Cargo.toml`, its
//! `.cargo/config.toml`, and a few environment variables, then runs a set of
//! pure-function checks over that gathered [`context::ProjectContext`]. Each
//! check returns [`finding::Finding`]s with a severity, an estimated impact,
//! and (usually) a concrete fix. No part of the target project is compiled.
//!
//! The ten checks are:
//!
//! 1. `linker`: recommends a fast linker (mold/lld) on Linux, or split
//!    debug info on macOS.
//! 2. `profile`: flags `debug = 2`, `opt-level > 0` in dev, and a missing
//!    `build-override` opt-level for proc-macros.
//! 3. `duplicates`: the same crate compiled in several distinct versions.
//! 4. `proc-macros`: the `syn` 1.x/2.x split and a high proc-macro count.
//! 5. `build-scripts`: an inventory of `build.rs` crates, flagging native
//!    `links`.
//! 6. `features`: heavy default feature sets (e.g. `tokio` `full`).
//! 7. `dev-deps`: heavy dev-dependencies such as `criterion` or `proptest`.
//! 8. `toolchain`: the installed toolchain versus the project's MSRV.
//! 9. `workspace`: large workspaces without a `workspace-hack` crate.
//! 10. `incremental`: `CARGO_INCREMENTAL=0` set in a local dev shell.
//!
//! # Library usage
//!
//! The check engine is exposed so it can be embedded or tested directly:
//!
//! ```no_run
//! use cargo_build_rx::{checks, context::ProjectContext};
//!
//! let ctx = ProjectContext::gather(None)?;
//! let findings = checks::run_checks(&ctx, &[], &[]);
//! println!("{} findings", findings.len());
//! # Ok::<(), anyhow::Error>(())
//! ```

#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::module_name_repetitions
)]

pub mod checks;
pub mod cli;
pub mod context;
pub mod finding;
pub mod output;
