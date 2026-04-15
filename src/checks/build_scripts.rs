use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct BuildScriptsCheck;

impl Check for BuildScriptsCheck {
    fn name(&self) -> &'static str {
        "build-scripts"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        let mut build_script_crates: Vec<String> = Vec::new();
        let mut native_link_crates: Vec<String> = Vec::new();

        for package in &ctx.metadata.packages {
            let has_build = package.targets.iter().any(|t| t.is_custom_build());

            if has_build {
                let name = format!("{} {}", package.name, package.version);
                if package.links.is_some() {
                    native_link_crates.push(name);
                } else {
                    build_script_crates.push(name);
                }
            }
        }

        let total = build_script_crates.len() + native_link_crates.len();
        if total == 0 {
            return findings;
        }

        let mut desc = format!("{total} crate{} ha{} build.rs scripts.",
            if total == 1 { "" } else { "s" },
            if total == 1 { "s" } else { "ve" },
        );

        if !native_link_crates.is_empty() {
            desc.push_str(&format!(
                "\nNative linking (may invoke C compiler): {}",
                native_link_crates.join(", ")
            ));
        }

        findings.push(Finding {
            severity: Severity::Info,
            category: Category::BuildScripts,
            impact: if !native_link_crates.is_empty() { Impact::Medium } else { Impact::Low },
            title: format!("{total} crate{} with build scripts", if total == 1 { "" } else { "s" }),
            description: desc,
            fix: None,
        });

        findings
    }
}
