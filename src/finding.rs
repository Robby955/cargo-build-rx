//! The data model every check produces: [`Finding`] and its parts.
//!
//! A check returns a `Vec<Finding>`. Each finding carries a [`Severity`]
//! (how strongly the tool recommends acting), a [`Category`] (which check
//! produced it), an [`Impact`] (rough size of the build-time win), and an
//! optional [`Fix`] describing the concrete change to make.

use serde::Serialize;

/// How strongly the tool recommends acting on a finding.
///
/// Variants are ordered from most to least actionable, so the derived
/// [`Ord`] sorts `Fix` before `Warn` before `Info`.
///
/// ```
/// use cargo_build_rx::finding::Severity;
/// assert!(Severity::Fix < Severity::Warn);
/// assert!(Severity::Warn < Severity::Info);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    /// Nearly always correct to change; the strongest recommendation.
    Fix,
    /// Worth reviewing; the win is real but depends on the project.
    Warn,
    /// Informational; surfaced for awareness, not necessarily action.
    Info,
}

impl Severity {
    /// The fixed-width uppercase label shown in terminal output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Severity::Fix => "FIX",
            Severity::Warn => "WARN",
            Severity::Info => "INFO",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Which check produced a finding. Used for filtering and JSON consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Category {
    /// Linker selection and debug-info packaging.
    Linker,
    /// `[profile.*]` settings that affect dev compile time.
    Profile,
    /// The same crate compiled in several distinct versions.
    Dependencies,
    /// Procedural-macro crates on the build critical path.
    ProcMacros,
    /// Crates that ship a `build.rs` script.
    BuildScripts,
    /// Heavy default feature sets on direct dependencies.
    Features,
    /// Heavy dev-dependencies that add to compile time.
    DevDeps,
    /// The installed Rust toolchain.
    Toolchain,
    /// Workspace-wide feature unification.
    Workspace,
    /// Incremental-compilation configuration.
    Incremental,
}

/// Rough size of the build-time win from acting on a finding.
///
/// Ordered most to least impactful for sorting alongside [`Severity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Impact {
    /// A large, broadly applicable build-time win.
    High,
    /// A moderate win.
    Medium,
    /// A small or situational win.
    Low,
}

impl Impact {
    /// The capitalized label shown in terminal output (e.g. `"High"`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Impact::High => "High",
            Impact::Medium => "Medium",
            Impact::Low => "Low",
        }
    }
}

impl std::fmt::Display for Impact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// One diagnostic produced by a check.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// How strongly the tool recommends acting.
    pub severity: Severity,
    /// Which check produced this finding.
    pub category: Category,
    /// Rough size of the build-time win.
    pub impact: Impact,
    /// One-line summary of the issue.
    pub title: String,
    /// Longer explanation, possibly multiple lines.
    pub description: String,
    /// The concrete change to make, when one applies.
    pub fix: Option<Fix>,
}

/// A concrete, actionable change attached to a [`Finding`].
#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    /// Human-readable summary of what the fix does.
    pub description: String,
    /// The structured form of the change, for tooling.
    pub kind: FixKind,
}

/// The structured shape of a [`Fix`], so consumers can act on it programmatically.
#[derive(Debug, Clone, Serialize)]
pub enum FixKind {
    /// A key to add under `.cargo/config.toml`.
    CargoConfig {
        /// The TOML key path, e.g. `[target.aarch64-apple-darwin]`.
        key_path: String,
        /// The value(s) to place under that key.
        value: String,
    },
    /// A key to add under a `Cargo.toml` section.
    CargoToml {
        /// The section, e.g. `profile.dev`.
        section: String,
        /// The key to set within the section.
        key: String,
        /// The value to set.
        value: String,
    },
    /// A shell command to run.
    ShellCommand(String),
    /// A manual step described in prose.
    Manual(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders_fix_before_warn_before_info() {
        let mut v = [Severity::Info, Severity::Fix, Severity::Warn];
        v.sort();
        assert_eq!(v, [Severity::Fix, Severity::Warn, Severity::Info]);
    }

    #[test]
    fn every_fixkind_variant_serializes() {
        let variants = [
            FixKind::CargoConfig {
                key_path: "[target.aarch64-apple-darwin]".into(),
                value: "linker = \"clang\"".into(),
            },
            FixKind::CargoToml {
                section: "profile.dev".into(),
                key: "debug".into(),
                value: "1".into(),
            },
            FixKind::ShellCommand("cargo update".into()),
            FixKind::Manual("do the thing".into()),
        ];
        for kind in variants {
            let json = serde_json::to_value(&kind).expect("FixKind must serialize");
            // Externally-tagged enum: exactly one variant key, and it round-trips
            // back to a string for the tag.
            assert!(json.is_object());
            assert_eq!(json.as_object().unwrap().len(), 1);
        }
    }

    #[test]
    fn finding_serializes_with_expected_fields() {
        let finding = Finding {
            severity: Severity::Warn,
            category: Category::Profile,
            impact: Impact::Medium,
            title: "t".into(),
            description: "d".into(),
            fix: None,
        };
        let json = serde_json::to_value(&finding).unwrap();
        for key in ["severity", "category", "impact", "title", "description", "fix"] {
            assert!(json.get(key).is_some(), "missing field {key}");
        }
        assert_eq!(json["severity"], "Warn");
    }
}
