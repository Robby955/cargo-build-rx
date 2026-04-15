use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct ToolchainCheck;

/// Approximate latest stable minor version. Update this periodically.
const LATEST_STABLE_MINOR: u32 = 87;

impl Check for ToolchainCheck {
    fn name(&self) -> &'static str {
        "toolchain"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        let version = match &ctx.rustc_version {
            Some(v) => v,
            None => {
                findings.push(Finding {
                    severity: Severity::Warn,
                    category: Category::Toolchain,
                    impact: Impact::Low,
                    title: "Could not determine rustc version".into(),
                    description: "Unable to parse `rustc --version` output.".into(),
                    fix: None,
                });
                return findings;
            }
        };

        if version.major == 1 && version.minor + 2 < LATEST_STABLE_MINOR {
            let behind = LATEST_STABLE_MINOR - version.minor;
            findings.push(Finding {
                severity: if behind >= 4 { Severity::Fix } else { Severity::Warn },
                category: Category::Toolchain,
                impact: Impact::Medium,
                title: format!("rustc {}.{}.{} is {} minor versions behind stable", version.major, version.minor, version.patch, behind),
                description: format!(
                    "Newer Rust versions include compiler performance improvements. \
                     Current: {}, latest stable: ~1.{LATEST_STABLE_MINOR}.0.",
                    version.raw
                ),
                fix: Some(Fix {
                    description: "Update Rust toolchain".into(),
                    kind: FixKind::ShellCommand("rustup update stable".into()),
                }),
            });
        }

        findings
    }
}
