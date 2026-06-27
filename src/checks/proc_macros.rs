use cargo_metadata::Target;

use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct ProcMacrosCheck;

impl Check for ProcMacrosCheck {
    fn name(&self) -> &'static str {
        "proc-macros"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut proc_macro_count = 0usize;
        let mut has_syn_v1 = false;
        let mut has_syn_v2 = false;

        for package in &ctx.metadata.packages {
            if package.targets.iter().any(Target::is_proc_macro) {
                proc_macro_count += 1;
            }

            if package.name.as_str() == "syn" {
                match package.version.major {
                    1 => has_syn_v1 = true,
                    n if n >= 2 => has_syn_v2 = true,
                    _ => {}
                }
            }
        }

        proc_macro_findings(has_syn_v1, has_syn_v2, proc_macro_count)
    }
}

/// Decide findings from the summarized proc-macro facts. Pure, so it can be
/// unit-tested without constructing `cargo metadata` output.
fn proc_macro_findings(has_syn_v1: bool, has_syn_v2: bool, proc_macro_count: usize) -> Vec<Finding> {
    let mut findings = Vec::new();

    if has_syn_v1 && has_syn_v2 {
        findings.push(Finding {
            severity: Severity::Warn,
            category: Category::ProcMacros,
            impact: Impact::Medium,
            title: "syn v1 and v2 both present".into(),
            description: "Both syn 1.x and syn 2.x are compiled. syn is the most expensive \
                          proc-macro dependency. Updating all dependents to use syn 2.x can \
                          eliminate a full extra compilation of syn."
                .into(),
            fix: Some(Fix {
                description: "Update dependencies that still pull syn 1.x".into(),
                kind: FixKind::ShellCommand(
                    "cargo update  # then check: cargo tree -d -p syn".into(),
                ),
            }),
        });
    }

    if proc_macro_count > 15 {
        findings.push(Finding {
            severity: Severity::Info,
            category: Category::ProcMacros,
            impact: Impact::Low,
            title: format!("{proc_macro_count} proc-macro crates in dependency tree"),
            description: "Proc-macro crates must be compiled for the host before dependent \
                          crates can proceed. A high count increases the critical path."
                .into(),
            fix: None,
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syn_split_is_warned() {
        let findings = proc_macro_findings(true, true, 3);
        assert!(findings.iter().any(|f| f.title == "syn v1 and v2 both present"));
    }

    #[test]
    fn single_syn_major_is_silent() {
        assert!(proc_macro_findings(false, true, 3).is_empty());
    }

    #[test]
    fn high_proc_macro_count_is_info() {
        let findings = proc_macro_findings(false, false, 20);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn boundary_count_of_15_is_silent() {
        assert!(proc_macro_findings(false, false, 15).is_empty());
    }
}
