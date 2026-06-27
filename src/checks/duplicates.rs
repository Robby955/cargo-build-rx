use std::collections::HashMap;

use cargo_metadata::semver::Version;

use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct DuplicatesCheck;

/// Crates where duplicates are especially costly.
const HIGH_IMPACT_CRATES: &[&str] = &["syn", "serde", "tokio", "rand", "itertools", "http"];

impl Check for DuplicatesCheck {
    fn name(&self) -> &'static str {
        "duplicates"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let entries: Vec<(&str, Version)> = ctx
            .metadata
            .packages
            .iter()
            .map(|p| (p.name.as_str(), p.version.clone()))
            .collect();
        duplicate_findings(&entries)
    }
}

/// Group `(name, version)` pairs and flag any crate compiled in more than one
/// distinct version. Owns `semver::Version` values so they sort by precedence
/// rather than lexicographically.
fn duplicate_findings(entries: &[(&str, Version)]) -> Vec<Finding> {
    let mut versions_by_name: HashMap<&str, Vec<Version>> = HashMap::new();
    for (name, version) in entries {
        versions_by_name
            .entry(name)
            .or_default()
            .push(version.clone());
    }

    let mut findings = Vec::new();
    for (name, mut versions) in versions_by_name {
        versions.sort();
        versions.dedup();
        if versions.len() <= 1 {
            continue;
        }

        let is_high_impact = HIGH_IMPACT_CRATES.contains(&name);
        let (severity, impact) = if is_high_impact {
            (Severity::Warn, Impact::Medium)
        } else {
            (Severity::Info, Impact::Low)
        };

        let version_list = versions
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        findings.push(Finding {
            severity,
            category: Category::Dependencies,
            impact,
            title: format!(
                "{name} present in {count} distinct compiled versions",
                count = versions.len()
            ),
            description: format!(
                "Versions: {version_list}. Each distinct version is compiled separately."
            ),
            fix: Some(Fix {
                description: format!("Try updating dependents to unify on one version of {name}"),
                kind: FixKind::ShellCommand(format!("cargo update -p {name}")),
            }),
        });
    }

    // Stable output regardless of HashMap iteration order.
    findings.sort_by(|a, b| a.title.cmp(&b.title));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn single_version_is_not_flagged() {
        let entries = [("serde", v("1.0.1")), ("anyhow", v("1.0.0"))];
        assert!(duplicate_findings(&entries).is_empty());
    }

    #[test]
    fn duplicate_versions_are_sorted_by_precedence_not_lexically() {
        let entries = [("foo", v("1.9.0")), ("foo", v("1.10.0"))];
        let findings = duplicate_findings(&entries);
        assert_eq!(findings.len(), 1);
        // Lexical order would put "1.10.0" before "1.9.0"; semver order is correct.
        assert!(findings[0].description.contains("1.9.0, 1.10.0"));
        assert!(findings[0].title.contains("distinct compiled versions"));
    }

    #[test]
    fn high_impact_crate_escalates_to_warn() {
        let entries = [("syn", v("1.0.109")), ("syn", v("2.0.117"))];
        let findings = duplicate_findings(&entries);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
    }

    #[test]
    fn low_impact_crate_stays_info() {
        let entries = [("widget", v("0.1.0")), ("widget", v("0.2.0"))];
        let findings = duplicate_findings(&entries);
        assert_eq!(findings[0].severity, Severity::Info);
    }
}
