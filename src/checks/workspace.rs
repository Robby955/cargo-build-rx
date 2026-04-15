use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct WorkspaceCheck;

impl Check for WorkspaceCheck {
    fn name(&self) -> &'static str {
        "workspace"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        if !ctx.is_workspace() {
            return findings;
        }

        let member_count = ctx.metadata.workspace_members.len();

        // Check if there's a workspace-hack or hakari crate
        let has_hack = ctx.metadata.packages.iter().any(|p| {
            p.name.contains("hack")
                || p.name.contains("hakari")
                || p.name.contains("workspace-deps")
        });

        if !has_hack && member_count >= 4 {
            findings.push(Finding {
                severity: Severity::Warn,
                category: Category::Workspace,
                impact: Impact::Medium,
                title: format!("Workspace with {member_count} members, no workspace-hack crate"),
                description: "In large workspaces, duplicate dependency compilations across members \
                              can be significant. A workspace-hack crate (via cargo-hakari) unifies \
                              feature resolution and avoids redundant builds."
                    .into(),
                fix: Some(Fix {
                    description: "Set up cargo-hakari".into(),
                    kind: FixKind::ShellCommand("cargo install cargo-hakari && cargo hakari init".into()),
                }),
            });
        }

        findings
    }
}
