use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct ProfileCheck;

impl Check for ProfileCheck {
    fn name(&self) -> &'static str {
        "profile"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        analyze_profile(&ctx.cargo_toml)
    }
}

/// Inspect `[profile.dev]` for settings that slow dev compiles.
///
/// Pure over the parsed manifest, so it is unit-testable without `cargo metadata`.
fn analyze_profile(cargo_toml: &toml::Table) -> Vec<Finding> {
    let mut findings = Vec::new();
    let dev = cargo_toml
        .get("profile")
        .and_then(toml::Value::as_table)
        .and_then(|p| p.get("dev"))
        .and_then(toml::Value::as_table);

    // Full debuginfo in the dev profile.
    if let Some(debug) = dev.and_then(|d| d.get("debug")) {
        if debug.as_integer() == Some(2) || debug.as_bool() == Some(true) {
            findings.push(Finding {
                severity: Severity::Warn,
                category: Category::Profile,
                impact: Impact::Medium,
                title: "Full debuginfo in dev profile".into(),
                description: "debug = 2 (full) slows compilation. Consider debug = 1 (line tables \
                              only) unless you need full variable inspection."
                    .into(),
                fix: Some(Fix {
                    description: "Reduce debuginfo level".into(),
                    kind: FixKind::CargoToml {
                        section: "profile.dev".into(),
                        key: "debug".into(),
                        value: "1".into(),
                    },
                }),
            });
        }
    }

    // Optimization in the dev profile (integer or string opt-level).
    if let Some(display) = dev.and_then(|d| d.get("opt-level")).and_then(opt_level_nonzero) {
        findings.push(Finding {
            severity: Severity::Warn,
            category: Category::Profile,
            impact: Impact::Medium,
            title: format!("opt-level = {display} in dev profile"),
            description: "Optimization in dev slows compile times significantly. Consider using \
                          opt-level = 0 for dev builds."
                .into(),
            fix: Some(Fix {
                description: "Remove or reduce opt-level in dev".into(),
                kind: FixKind::CargoToml {
                    section: "profile.dev".into(),
                    key: "opt-level".into(),
                    value: "0".into(),
                },
            }),
        });
    }

    // Missing build-override opt-level for proc-macros. This is sound advice but
    // applies to almost every project, so it is a Warn, not a Fix (it must not
    // alone fail a `--deny fix` gate).
    let has_build_override = dev
        .and_then(|d| d.get("build-override"))
        .and_then(toml::Value::as_table)
        .and_then(|b| b.get("opt-level"))
        .is_some();

    if !has_build_override {
        findings.push(Finding {
            severity: Severity::Warn,
            category: Category::Profile,
            impact: Impact::Medium,
            title: "Missing build-override opt-level for proc-macros".into(),
            description: "Proc-macros and build scripts run at opt-level 0 by default. Compiling \
                          them with opt-level 3 makes them run faster during builds."
                .into(),
            fix: Some(Fix {
                description: "Optimize proc-macro compilation".into(),
                kind: FixKind::CargoToml {
                    section: "profile.dev.build-override".into(),
                    key: "opt-level".into(),
                    value: "3".into(),
                },
            }),
        });
    }

    findings
}

/// If an `opt-level` value is non-zero, return how to display it.
///
/// Handles both the integer form (`1`, `2`, `3`) and the string form (`"s"`,
/// `"z"`), the latter of which the previous integer-only path silently treated
/// as zero.
fn opt_level_nonzero(val: &toml::Value) -> Option<String> {
    if let Some(i) = val.as_integer() {
        return (i > 0).then(|| i.to_string());
    }
    if let Some(s) = val.as_str() {
        return (s != "0").then(|| s.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Severity;

    fn parse(toml_str: &str) -> toml::Table {
        toml_str.parse().unwrap()
    }

    fn titles(findings: &[Finding]) -> Vec<&str> {
        findings.iter().map(|f| f.title.as_str()).collect()
    }

    #[test]
    fn clean_profile_only_flags_build_override() {
        let findings = analyze_profile(&parse(""));
        // No [profile.dev] at all still means no build-override opt-level.
        assert_eq!(
            titles(&findings),
            vec!["Missing build-override opt-level for proc-macros"]
        );
        assert!(findings.iter().all(|f| f.severity != Severity::Fix));
    }

    #[test]
    fn full_debuginfo_is_warned() {
        let findings = analyze_profile(&parse("[profile.dev]\ndebug = 2\n"));
        assert!(titles(&findings).contains(&"Full debuginfo in dev profile"));
    }

    #[test]
    fn integer_opt_level_is_flagged() {
        let findings = analyze_profile(&parse("[profile.dev]\nopt-level = 2\n"));
        assert!(findings
            .iter()
            .any(|f| f.title == "opt-level = 2 in dev profile"));
    }

    #[test]
    fn string_opt_level_s_is_flagged() {
        let findings = analyze_profile(&parse("[profile.dev]\nopt-level = \"s\"\n"));
        assert!(
            findings
                .iter()
                .any(|f| f.title == "opt-level = s in dev profile"),
            "string opt-level \"s\" must not be treated as zero"
        );
    }

    #[test]
    fn opt_level_zero_is_not_flagged() {
        let findings = analyze_profile(&parse("[profile.dev]\nopt-level = 0\n"));
        assert!(!findings
            .iter()
            .any(|f| f.title.starts_with("opt-level")));
    }

    #[test]
    fn present_build_override_suppresses_finding() {
        let findings =
            analyze_profile(&parse("[profile.dev.build-override]\nopt-level = 3\n"));
        assert!(!findings
            .iter()
            .any(|f| f.title.contains("build-override")));
    }
}
