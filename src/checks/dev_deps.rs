use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Impact, Severity};

pub struct DevDepsCheck;

/// Known heavy dev-dependency crates.
const HEAVY_DEV_DEPS: &[(&str, &str)] = &[
    ("criterion", "heavyweight benchmarking framework"),
    (
        "proptest",
        "property-based testing (includes regex, bitset, etc.)",
    ),
    ("insta", "snapshot testing with serde support"),
    ("trybuild", "compile-fail test harness"),
    ("trycmd", "CLI integration test harness"),
    ("rstest", "parameterized testing with proc-macros"),
];

impl Check for DevDepsCheck {
    fn name(&self) -> &'static str {
        "dev-deps"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        dev_dep_findings(&ctx.cargo_toml)
    }
}

/// Flag heavy dev-dependencies declared in the manifest. Pure over the manifest.
fn dev_dep_findings(cargo_toml: &toml::Table) -> Vec<Finding> {
    let Some(dev_deps) = cargo_toml
        .get("dev-dependencies")
        .and_then(toml::Value::as_table)
    else {
        return Vec::new();
    };

    let mut heavy: Vec<(&str, &str)> = Vec::new();
    for name in dev_deps.keys() {
        for &(crate_name, desc) in HEAVY_DEV_DEPS {
            if name == crate_name {
                heavy.push((crate_name, desc));
            }
        }
    }

    if heavy.is_empty() {
        return Vec::new();
    }

    let list = heavy
        .iter()
        .map(|(name, desc)| format!("  - {name}: {desc}"))
        .collect::<Vec<_>>()
        .join("\n");

    vec![Finding {
        severity: Severity::Info,
        category: Category::DevDeps,
        impact: Impact::Low,
        title: format!(
            "{} heavy dev-dependenc{}",
            heavy.len(),
            if heavy.len() == 1 { "y" } else { "ies" }
        ),
        description: format!(
            "These dev-dependencies add to compile time even when not running tests:\n{list}\n\
             Consider feature-gating heavy test harnesses behind a feature flag."
        ),
        fix: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> toml::Table {
        s.parse().unwrap()
    }

    #[test]
    fn no_dev_deps_is_silent() {
        assert!(dev_dep_findings(&parse("")).is_empty());
    }

    #[test]
    fn light_dev_deps_are_silent() {
        assert!(dev_dep_findings(&parse("[dev-dependencies]\nserde = \"1\"\n")).is_empty());
    }

    #[test]
    fn heavy_dev_deps_are_counted() {
        let findings = dev_dep_findings(&parse(
            "[dev-dependencies]\ncriterion = \"0.5\"\nproptest = \"1\"\n",
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "2 heavy dev-dependencies");
    }

    #[test]
    fn single_heavy_dev_dep_uses_singular() {
        let findings = dev_dep_findings(&parse("[dev-dependencies]\ncriterion = \"0.5\"\n"));
        assert_eq!(findings[0].title, "1 heavy dev-dependency");
    }
}
