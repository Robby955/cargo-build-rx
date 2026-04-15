use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::*;

pub struct ProfileCheck;

impl Check for ProfileCheck {
    fn name(&self) -> &'static str {
        "profile"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let profiles = ctx.cargo_toml.get("profile").and_then(|p| p.as_table());

        // Check dev profile debuginfo level
        if let Some(dev) = profiles.and_then(|p| p.get("dev")).and_then(|d| d.as_table()) {
            if let Some(debug) = dev.get("debug") {
                if debug.as_integer() == Some(2) || debug.as_bool() == Some(true) {
                    findings.push(Finding {
                        severity: Severity::Warn,
                        category: Category::Profile,
                        impact: Impact::Medium,
                        title: "Full debuginfo in dev profile".into(),
                        description: "debug = 2 (full) slows compilation. Consider debug = 1 (line tables only) \
                                      unless you need full variable inspection.".into(),
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
        }

        // Check for opt-level > 0 in dev profile
        if let Some(dev) = profiles.and_then(|p| p.get("dev")).and_then(|d| d.as_table()) {
            if let Some(opt) = dev.get("opt-level") {
                let opt_val = opt.as_integer().unwrap_or(0);
                if opt_val > 0 {
                    findings.push(Finding {
                        severity: Severity::Warn,
                        category: Category::Profile,
                        impact: Impact::Medium,
                        title: format!("opt-level = {} in dev profile", opt_val),
                        description: "Optimization in dev slows compile times significantly. \
                                      Consider using opt-level = 0 for dev builds.".into(),
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
            }
        }

        // Check for build-override opt-level (proc-macro compilation speed)
        let has_build_override = profiles
            .and_then(|p| p.get("dev"))
            .and_then(|d| d.as_table())
            .and_then(|d| d.get("build-override"))
            .and_then(|b| b.as_table())
            .and_then(|b| b.get("opt-level"))
            .is_some();

        if !has_build_override {
            findings.push(Finding {
                severity: Severity::Fix,
                category: Category::Profile,
                impact: Impact::Medium,
                title: "Missing build-override opt-level for proc-macros".into(),
                description: "Proc-macros and build scripts run at opt-level 0 by default. \
                              Compiling them with opt-level 3 makes them run faster during builds.".into(),
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
}
