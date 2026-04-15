use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct ProcMacrosCheck;

impl Check for ProcMacrosCheck {
    fn name(&self) -> &'static str {
        "proc-macros"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        let mut proc_macro_count = 0;
        let mut has_syn_v1 = false;
        let mut has_syn_v2 = false;

        for package in &ctx.metadata.packages {
            // Count proc-macro crates
            if package.targets.iter().any(|t| t.is_proc_macro()) {
                proc_macro_count += 1;
            }

            // Track syn versions
            if package.name == "syn" {
                let major = package.version.major;
                if major == 1 {
                    has_syn_v1 = true;
                } else if major >= 2 {
                    has_syn_v2 = true;
                }
            }
        }

        // syn v1 + v2 split
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
                    kind: FixKind::ShellCommand("cargo update  # then check: cargo tree -d -p syn".into()),
                }),
            });
        }

        // High proc-macro count
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
}
