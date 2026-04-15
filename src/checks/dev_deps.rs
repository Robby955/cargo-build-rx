use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct DevDepsCheck;

/// Known heavy dev-dependency crates.
const HEAVY_DEV_DEPS: &[(&str, &str)] = &[
    ("criterion", "heavyweight benchmarking framework"),
    ("proptest", "property-based testing (includes regex, bitset, etc.)"),
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
        let mut findings = Vec::new();

        let dev_deps = match ctx.cargo_toml.get("dev-dependencies").and_then(|d| d.as_table()) {
            Some(d) => d,
            None => return findings,
        };

        let mut heavy: Vec<(&str, &str)> = Vec::new();
        for (name, _spec) in dev_deps {
            for &(crate_name, desc) in HEAVY_DEV_DEPS {
                if name == crate_name {
                    heavy.push((crate_name, desc));
                }
            }
        }

        if !heavy.is_empty() {
            let list = heavy
                .iter()
                .map(|(name, desc)| format!("  - {name}: {desc}"))
                .collect::<Vec<_>>()
                .join("\n");

            findings.push(Finding {
                severity: Severity::Info,
                category: Category::DevDeps,
                impact: Impact::Low,
                title: format!("{} heavy dev-dependenc{}", heavy.len(), if heavy.len() == 1 { "y" } else { "ies" }),
                description: format!(
                    "These dev-dependencies add to compile time even when not running tests:\n{list}\n\
                     Consider feature-gating heavy test harnesses behind a feature flag."
                ),
                fix: None,
            });
        }

        findings
    }
}
