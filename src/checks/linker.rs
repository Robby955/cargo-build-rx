use crate::checks::Check;
use crate::context::{Os, ProjectContext};
use crate::finding::*;

pub struct LinkerCheck;

impl Check for LinkerCheck {
    fn name(&self) -> &'static str {
        "linker"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Check if a custom linker is already configured
        if has_custom_linker(ctx) {
            return findings;
        }

        match ctx.os {
            Os::Linux => {
                if ctx.installed_linkers.mold {
                    findings.push(Finding {
                        severity: Severity::Fix,
                        category: Category::Linker,
                        impact: Impact::High,
                        title: "Default system linker detected".into(),
                        description: "mold is installed and would be 2-5x faster for linking.".into(),
                        fix: Some(Fix {
                            description: "Use mold as the linker".into(),
                            kind: FixKind::CargoConfig {
                                key_path: "[target.x86_64-unknown-linux-gnu]".into(),
                                value: "linker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]".into(),
                            },
                        }),
                    });
                } else if ctx.installed_linkers.lld {
                    findings.push(Finding {
                        severity: Severity::Fix,
                        category: Category::Linker,
                        impact: Impact::High,
                        title: "Default system linker detected".into(),
                        description: "lld is installed and would be 2-3x faster for linking.".into(),
                        fix: Some(Fix {
                            description: "Use lld as the linker".into(),
                            kind: FixKind::CargoConfig {
                                key_path: "[target.x86_64-unknown-linux-gnu]".into(),
                                value: "linker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=lld\"]".into(),
                            },
                        }),
                    });
                } else {
                    findings.push(Finding {
                        severity: Severity::Warn,
                        category: Category::Linker,
                        impact: Impact::High,
                        title: "No fast linker installed".into(),
                        description: "Neither mold nor lld detected. Installing one can speed up linking 2-5x.".into(),
                        fix: Some(Fix {
                            description: "Install mold".into(),
                            kind: FixKind::ShellCommand("sudo apt install mold  # or: cargo install mold".into()),
                        }),
                    });
                }
            }
            Os::Macos => {
                // On macOS, recommend split-debuginfo for faster builds
                if !has_split_debuginfo(ctx) {
                    findings.push(Finding {
                        severity: Severity::Fix,
                        category: Category::Linker,
                        impact: Impact::Medium,
                        title: "split-debuginfo not enabled on macOS".into(),
                        description: "Using split-debuginfo=unpacked avoids bundling debug info during linking.".into(),
                        fix: Some(Fix {
                            description: "Enable split-debuginfo in dev profile".into(),
                            kind: FixKind::CargoToml {
                                section: "profile.dev".into(),
                                key: "split-debuginfo".into(),
                                value: "\"unpacked\"".into(),
                            },
                        }),
                    });
                }
            }
            _ => {}
        }

        findings
    }
}

fn has_custom_linker(ctx: &ProjectContext) -> bool {
    if let Some(config) = &ctx.cargo_config {
        // Check for any [target.*] linker setting
        if let Some(target) = config.get("target").and_then(|t| t.as_table()) {
            for (_target_name, target_config) in target {
                if let Some(table) = target_config.as_table() {
                    if table.contains_key("linker") {
                        return true;
                    }
                    // Also check rustflags for -fuse-ld
                    if let Some(flags) = table.get("rustflags").and_then(|f| f.as_array()) {
                        let flags_str: Vec<&str> = flags.iter().filter_map(|v| v.as_str()).collect();
                        if flags_str.iter().any(|f| f.contains("fuse-ld")) {
                            return true;
                        }
                    }
                }
            }
        }
        // Check [build] linker
        if let Some(build) = config.get("build").and_then(|b| b.as_table()) {
            if build.contains_key("linker") || build.contains_key("rustflags") {
                return true;
            }
        }
    }

    // Check RUSTFLAGS env
    if let Some(flags) = &ctx.env_vars.rustflags {
        if flags.contains("fuse-ld") {
            return true;
        }
    }

    false
}

fn has_split_debuginfo(ctx: &ProjectContext) -> bool {
    if let Some(profile) = ctx.cargo_toml.get("profile").and_then(|p| p.as_table()) {
        if let Some(dev) = profile.get("dev").and_then(|d| d.as_table()) {
            if dev.contains_key("split-debuginfo") {
                return true;
            }
        }
    }
    false
}
