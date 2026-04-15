use std::collections::HashMap;

use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct DuplicatesCheck;

/// Crates where duplicates are especially costly.
const HIGH_IMPACT_CRATES: &[&str] = &["syn", "serde", "tokio", "rand", "itertools", "http"];

impl Check for DuplicatesCheck {
    fn name(&self) -> &'static str {
        "duplicates"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Group packages by name, track distinct major.minor versions
        let mut versions_by_name: HashMap<&str, Vec<&str>> = HashMap::new();
        for package in &ctx.metadata.packages {
            versions_by_name
                .entry(package.name.as_str())
                .or_default()
                .push(package.version.to_string().leak());
        }

        for (name, versions) in &versions_by_name {
            if versions.len() <= 1 {
                continue;
            }

            // Deduplicate — cargo metadata can list the same version twice in workspaces
            let mut unique: Vec<&str> = versions.clone();
            unique.sort();
            unique.dedup();
            if unique.len() <= 1 {
                continue;
            }

            let is_high_impact = HIGH_IMPACT_CRATES.contains(name);
            let (severity, impact) = if is_high_impact {
                (Severity::Warn, Impact::Medium)
            } else {
                (Severity::Info, Impact::Low)
            };

            let version_list = unique.join(", ");
            findings.push(Finding {
                severity,
                category: Category::Dependencies,
                impact,
                title: format!("{name} present in {count} incompatible versions", count = unique.len()),
                description: format!("Versions: {version_list}. Each distinct version is compiled separately."),
                fix: Some(Fix {
                    description: format!("Try updating dependents to unify on one version of {name}"),
                    kind: FixKind::ShellCommand(format!("cargo update -p {name}")),
                }),
            });
        }

        findings
    }
}
