mod linker;
mod profile;
mod duplicates;
mod proc_macros;
mod build_scripts;
mod features;
mod dev_deps;
mod toolchain;
mod workspace;
mod incremental;

use crate::context::ProjectContext;
use crate::finding::Finding;

pub trait Check {
    fn name(&self) -> &'static str;
    fn run(&self, ctx: &ProjectContext) -> Vec<Finding>;
}

pub fn all_checks() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(linker::LinkerCheck),
        Box::new(profile::ProfileCheck),
        Box::new(duplicates::DuplicatesCheck),
        Box::new(proc_macros::ProcMacrosCheck),
        Box::new(build_scripts::BuildScriptsCheck),
        Box::new(features::FeaturesCheck),
        Box::new(dev_deps::DevDepsCheck),
        Box::new(toolchain::ToolchainCheck),
        Box::new(workspace::WorkspaceCheck),
        Box::new(incremental::IncrementalCheck),
    ]
}

pub fn run_checks(
    ctx: &ProjectContext,
    only: &[String],
    skip: &[String],
) -> Vec<Finding> {
    let checks = all_checks();
    let mut findings = Vec::new();

    for check in &checks {
        let name = check.name();
        if !only.is_empty() && !only.iter().any(|o| o == name) {
            continue;
        }
        if skip.iter().any(|s| s == name) {
            continue;
        }
        findings.extend(check.run(ctx));
    }

    // Sort: Fix > Warn > Info, then High > Medium > Low
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then(a.impact.cmp(&b.impact))
    });

    findings
}
