mod checks;
mod cli;
mod context;
mod finding;
mod output;

use anyhow::Result;
use clap::Parser;

use cli::{Cargo, Format};
use context::ProjectContext;

fn main() -> Result<()> {
    let Cargo::BuildRx(args) = Cargo::parse();

    let manifest_path = args.manifest_path.as_deref();
    let ctx = ProjectContext::gather(manifest_path)?;

    let mut findings = checks::run_checks(&ctx, &args.only, &args.skip);

    // Filter by minimum severity
    findings.retain(|f| args.min_severity.passes(f.severity));

    match args.format {
        Format::Terminal => output::render_terminal(ctx.project_name(), &findings),
        Format::Json => println!("{}", output::render_json(&findings)),
    }

    // Exit code 1 if any Fix-severity findings
    if findings.iter().any(|f| f.severity == finding::Severity::Fix) {
        std::process::exit(1);
    }

    Ok(())
}
