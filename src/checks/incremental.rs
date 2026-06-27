use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct IncrementalCheck;

impl Check for IncrementalCheck {
    fn name(&self) -> &'static str {
        "incremental"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        incremental_findings(ctx.env_vars.cargo_incremental.as_deref())
    }
}

/// Flag `CARGO_INCREMENTAL=0` in what looks like a local dev shell.
fn incremental_findings(cargo_incremental: Option<&str>) -> Vec<Finding> {
    if cargo_incremental != Some("0") {
        return Vec::new();
    }

    vec![Finding {
        severity: Severity::Warn,
        category: Category::Incremental,
        impact: Impact::Medium,
        title: "CARGO_INCREMENTAL=0 is set".into(),
        description: "Incremental compilation is disabled. This is appropriate for CI but hurts \
                      local iteration speed. If this is a local dev environment, unset this \
                      variable."
            .into(),
        fix: Some(Fix {
            description: "Enable incremental compilation".into(),
            kind: FixKind::ShellCommand(
                "unset CARGO_INCREMENTAL  # or remove from shell profile".into(),
            ),
        }),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_is_silent() {
        assert!(incremental_findings(None).is_empty());
    }

    #[test]
    fn enabled_is_silent() {
        assert!(incremental_findings(Some("1")).is_empty());
    }

    #[test]
    fn disabled_is_warned() {
        let findings = incremental_findings(Some("0"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
    }
}
