use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct IncrementalCheck;

impl Check for IncrementalCheck {
    fn name(&self) -> &'static str {
        "incremental"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        if let Some(val) = &ctx.env_vars.cargo_incremental {
            if val == "0" {
                findings.push(Finding {
                    severity: Severity::Warn,
                    category: Category::Incremental,
                    impact: Impact::Medium,
                    title: "CARGO_INCREMENTAL=0 is set".into(),
                    description: "Incremental compilation is disabled. This is appropriate for CI \
                                  but hurts local iteration speed. If this is a local dev environment, \
                                  unset this variable."
                        .into(),
                    fix: Some(Fix {
                        description: "Enable incremental compilation".into(),
                        kind: FixKind::ShellCommand("unset CARGO_INCREMENTAL  # or remove from shell profile".into()),
                    }),
                });
            }
        }

        findings
    }
}
