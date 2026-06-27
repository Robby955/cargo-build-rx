use std::fmt::Write as _;

use cargo_metadata::Target;

use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Impact, Severity};

pub struct BuildScriptsCheck;

impl Check for BuildScriptsCheck {
    fn name(&self) -> &'static str {
        "build-scripts"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut build_script_crates: Vec<String> = Vec::new();
        let mut native_link_crates: Vec<String> = Vec::new();

        for package in &ctx.metadata.packages {
            if package.targets.iter().any(Target::is_custom_build) {
                let name = format!("{} {}", package.name, package.version);
                if package.links.is_some() {
                    native_link_crates.push(name);
                } else {
                    build_script_crates.push(name);
                }
            }
        }

        build_script_findings(&build_script_crates, &native_link_crates)
    }
}

/// Summarize build-script usage. Pure over the two crate lists.
fn build_script_findings(build_script_crates: &[String], native_link_crates: &[String]) -> Vec<Finding> {
    let total = build_script_crates.len() + native_link_crates.len();
    if total == 0 {
        return Vec::new();
    }

    let mut desc = format!(
        "{total} crate{} ha{} build.rs scripts.",
        if total == 1 { "" } else { "s" },
        if total == 1 { "s" } else { "ve" },
    );

    if !native_link_crates.is_empty() {
        let _ = write!(
            desc,
            "\nNative linking (may invoke C compiler): {}",
            native_link_crates.join(", ")
        );
    }

    vec![Finding {
        severity: Severity::Info,
        category: Category::BuildScripts,
        impact: if native_link_crates.is_empty() {
            Impact::Low
        } else {
            Impact::Medium
        },
        title: format!(
            "{total} crate{} with build scripts",
            if total == 1 { "" } else { "s" }
        ),
        description: desc,
        fix: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_build_scripts_is_silent() {
        assert!(build_script_findings(&[], &[]).is_empty());
    }

    #[test]
    fn plain_build_scripts_are_low_impact() {
        let findings = build_script_findings(&["foo 1.0.0".into()], &[]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].impact, Impact::Low);
        assert_eq!(findings[0].title, "1 crate with build scripts");
    }

    #[test]
    fn native_linking_escalates_impact_and_lists_crate() {
        let findings = build_script_findings(&[], &["openssl-sys 0.9.0".into()]);
        assert_eq!(findings[0].impact, Impact::Medium);
        assert!(findings[0].description.contains("openssl-sys 0.9.0"));
    }
}
