//! Binary entry point for `cargo build-rx`. The check engine lives in the
//! library crate; this file only wires arguments to output and an exit code.

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use cargo_build_rx::cli::{Cargo, Format};
use cargo_build_rx::context::ProjectContext;
use cargo_build_rx::{checks, output};

fn main() -> Result<ExitCode> {
    let Cargo::BuildRx(args) = Cargo::parse();

    let manifest_path = args.manifest_path.as_deref();
    let ctx = ProjectContext::gather(manifest_path)?;

    let mut findings = checks::run_checks(&ctx, &args.only, &args.skip);

    // Filter by minimum severity.
    findings.retain(|f| args.min_severity.passes(f.severity));

    match args.format {
        Format::Terminal => {
            output::render_terminal(ctx.project_name(), &findings, args.color);
        }
        Format::Json => println!("{}", output::render_json(&findings)?),
    }

    // Exit non-zero only when the caller opted into a deny threshold.
    if findings.iter().any(|f| args.deny.triggers(f.severity)) {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
