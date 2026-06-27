//! The check registry and the [`Check`] trait every diagnostic implements.

mod build_scripts;
mod dev_deps;
mod duplicates;
mod features;
mod incremental;
mod linker;
mod proc_macros;
mod profile;
mod toolchain;
mod workspace;

use crate::context::ProjectContext;
use crate::finding::Finding;

/// A single diagnostic over a gathered [`ProjectContext`].
///
/// Implementations are pure: [`Check::run`] reads the context and returns
/// findings without mutating anything or compiling the target project.
pub trait Check {
    /// The check's stable identifier, used by `--only` and `--skip`.
    fn name(&self) -> &'static str;
    /// Produce findings for the given context.
    fn run(&self, ctx: &ProjectContext) -> Vec<Finding>;
}

/// All checks, in their default reporting order.
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

/// Run the selected checks and return findings sorted by severity then impact.
///
/// `only` restricts to the named checks (when non-empty); `skip` removes named
/// checks. Names match [`Check::name`].
pub fn run_checks(ctx: &ProjectContext, only: &[String], skip: &[String]) -> Vec<Finding> {
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

    // Sort: Fix > Warn > Info, then High > Medium > Low.
    findings.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.impact.cmp(&b.impact)));

    findings
}
