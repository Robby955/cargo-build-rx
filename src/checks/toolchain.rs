use crate::checks::Check;
use crate::context::{ProjectContext, RustcVersion};
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct ToolchainCheck;

/// A static reference point for "recent" stable Rust.
///
/// A network-free tool cannot know the true latest stable, so this is only a
/// dated snapshot used for a soft, informational hint. The authoritative,
/// always-correct comparison is against the project's declared MSRV. Update the
/// snapshot here, in one place, when convenient.
const REFERENCE_STABLE_MINOR: u32 = 96;
const REFERENCE_STABLE_AS_OF: &str = "2026-05";

/// How far behind the reference snapshot before emitting the soft Info hint.
const STALE_MINOR_GAP: u32 = 6;

impl Check for ToolchainCheck {
    fn name(&self) -> &'static str {
        "toolchain"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        toolchain_findings(ctx.rustc_version.as_ref(), ctx.declared_msrv())
    }
}

/// Compare the installed toolchain against the declared MSRV (authoritative)
/// and, as a soft hint, a dated reference snapshot of latest stable.
fn toolchain_findings(rustc: Option<&RustcVersion>, msrv: Option<(u32, u32)>) -> Vec<Finding> {
    let Some(version) = rustc else {
        return vec![Finding {
            severity: Severity::Warn,
            category: Category::Toolchain,
            impact: Impact::Low,
            title: "Could not determine rustc version".into(),
            description: "Unable to parse `rustc -vV` output.".into(),
            fix: None,
        }];
    };

    let installed = (version.major, version.minor);

    // Authoritative: the installed toolchain is older than the project requires.
    if let Some((msrv_major, msrv_minor)) = msrv {
        if installed < (msrv_major, msrv_minor) {
            return vec![Finding {
                severity: Severity::Fix,
                category: Category::Toolchain,
                impact: Impact::High,
                title: format!(
                    "Installed rustc {}.{} is older than the project MSRV {msrv_major}.{msrv_minor}",
                    version.major, version.minor
                ),
                description: format!(
                    "The project declares rust-version = \"{msrv_major}.{msrv_minor}\" but the \
                     installed toolchain is {}. Builds may fail until the toolchain is updated.",
                    version.raw
                ),
                fix: Some(Fix {
                    description: "Update the Rust toolchain".into(),
                    kind: FixKind::ShellCommand("rustup update stable".into()),
                }),
            }];
        }
    }

    // Soft, dated hint: well behind the last known stable snapshot.
    if version.major == 1 && version.minor + STALE_MINOR_GAP < REFERENCE_STABLE_MINOR {
        return vec![Finding {
            severity: Severity::Info,
            category: Category::Toolchain,
            impact: Impact::Low,
            title: format!(
                "rustc {}.{}.{} may be behind current stable",
                version.major, version.minor, version.patch
            ),
            description: format!(
                "Newer Rust versions include compiler performance improvements. Installed: {}. \
                 As of {REFERENCE_STABLE_AS_OF}, the latest stable was ~1.{REFERENCE_STABLE_MINOR}.0 \
                 (this reference is static and may be out of date).",
                version.raw
            ),
            fix: Some(Fix {
                description: "Update the Rust toolchain".into(),
                kind: FixKind::ShellCommand("rustup update stable".into()),
            }),
        }];
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rustc(minor: u32) -> RustcVersion {
        RustcVersion {
            major: 1,
            minor,
            patch: 0,
            raw: format!("rustc 1.{minor}.0 (test)"),
        }
    }

    #[test]
    fn missing_version_warns() {
        let findings = toolchain_findings(None, None);
        assert_eq!(findings[0].severity, Severity::Warn);
    }

    #[test]
    fn older_than_msrv_is_fix() {
        let findings = toolchain_findings(Some(&rustc(70)), Some((1, 80)));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Fix);
        assert!(findings[0].title.contains("older than the project MSRV"));
    }

    #[test]
    fn meets_msrv_and_recent_is_silent() {
        assert!(toolchain_findings(Some(&rustc(96)), Some((1, 80))).is_empty());
    }

    #[test]
    fn far_behind_reference_is_info_not_fix() {
        let findings = toolchain_findings(Some(&rustc(70)), None);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn current_toolchain_with_no_msrv_is_silent() {
        assert!(toolchain_findings(Some(&rustc(REFERENCE_STABLE_MINOR)), None).is_empty());
    }
}
