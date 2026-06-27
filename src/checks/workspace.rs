use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct WorkspaceCheck;

impl Check for WorkspaceCheck {
    fn name(&self) -> &'static str {
        "workspace"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        if !ctx.is_workspace() {
            return Vec::new();
        }

        let member_count = ctx.metadata.workspace_members.len();
        let has_hack = ctx.metadata.packages.iter().any(|p| {
            let name = p.name.as_str();
            name.contains("hack") || name.contains("hakari") || name.contains("workspace-deps")
        });

        workspace_findings(member_count, has_hack)
    }
}

/// Decide whether a large workspace would benefit from a workspace-hack crate.
fn workspace_findings(member_count: usize, has_hack: bool) -> Vec<Finding> {
    if has_hack || member_count < 4 {
        return Vec::new();
    }

    vec![Finding {
        severity: Severity::Warn,
        category: Category::Workspace,
        impact: Impact::Medium,
        title: format!("Workspace with {member_count} members, no workspace-hack crate"),
        description: "In large workspaces, duplicate dependency compilations across members can be \
                      significant. A workspace-hack crate (via cargo-hakari) unifies feature \
                      resolution and avoids redundant builds."
            .into(),
        fix: Some(Fix {
            description: "Set up cargo-hakari".into(),
            kind: FixKind::ShellCommand("cargo install cargo-hakari && cargo hakari init".into()),
        }),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_workspace_is_silent() {
        assert!(workspace_findings(3, false).is_empty());
    }

    #[test]
    fn large_workspace_without_hack_is_warned() {
        let findings = workspace_findings(6, false);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
        assert!(findings[0].title.contains("6 members"));
    }

    #[test]
    fn large_workspace_with_hack_is_silent() {
        assert!(workspace_findings(6, true).is_empty());
    }
}
