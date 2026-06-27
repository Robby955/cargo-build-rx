use crate::checks::Check;
use crate::context::{InstalledLinkers, Os, ProjectContext};
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct LinkerCheck;

/// Used only when the host triple cannot be read from `rustc`.
const FALLBACK_LINUX_TRIPLE: &str = "x86_64-unknown-linux-gnu";

impl Check for LinkerCheck {
    fn name(&self) -> &'static str {
        "linker"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        // A linker is already configured; nothing to recommend.
        if has_custom_linker(ctx) {
            return Vec::new();
        }

        match ctx.os {
            Os::Linux => linux_findings(&ctx.installed_linkers, ctx.host_triple.as_deref()),
            Os::Macos => macos_findings(&ctx.cargo_toml),
            Os::Windows | Os::Other => Vec::new(),
        }
    }
}

/// The `.cargo/config.toml` section for the host, e.g.
/// `[target.aarch64-unknown-linux-gnu]`. Falls back to the common x86-64 triple
/// when the host could not be determined, so the advice is never aimed at a
/// target the machine does not build for.
fn target_section(host_triple: Option<&str>) -> String {
    format!("[target.{}]", host_triple.unwrap_or(FALLBACK_LINUX_TRIPLE))
}

fn linux_findings(linkers: &InstalledLinkers, host_triple: Option<&str>) -> Vec<Finding> {
    let key_path = target_section(host_triple);

    if linkers.mold {
        return vec![Finding {
            severity: Severity::Fix,
            category: Category::Linker,
            impact: Impact::High,
            title: "Default system linker detected".into(),
            description: "mold is installed and would be 2-5x faster for linking.".into(),
            fix: Some(Fix {
                description: "Use mold as the linker".into(),
                kind: FixKind::CargoConfig {
                    key_path,
                    value: "linker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=mold\"]"
                        .into(),
                },
            }),
        }];
    }

    if linkers.lld {
        return vec![Finding {
            severity: Severity::Fix,
            category: Category::Linker,
            impact: Impact::High,
            title: "Default system linker detected".into(),
            description: "lld is installed and would be 2-3x faster for linking.".into(),
            fix: Some(Fix {
                description: "Use lld as the linker".into(),
                kind: FixKind::CargoConfig {
                    key_path,
                    value: "linker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=lld\"]"
                        .into(),
                },
            }),
        }];
    }

    vec![Finding {
        severity: Severity::Warn,
        category: Category::Linker,
        impact: Impact::High,
        title: "No fast linker installed".into(),
        description: "Neither mold nor lld detected. Installing one can speed up linking 2-5x."
            .into(),
        fix: Some(Fix {
            description: "Install mold".into(),
            kind: FixKind::ShellCommand("sudo apt install mold  # or: cargo install mold".into()),
        }),
    }]
}

fn macos_findings(cargo_toml: &toml::Table) -> Vec<Finding> {
    if has_split_debuginfo(cargo_toml) {
        return Vec::new();
    }
    // Recent macOS toolchains already default dev to unpacked, so this is
    // surfaced as Info rather than a Fix that nags an already-applied default.
    vec![Finding {
        severity: Severity::Info,
        category: Category::Linker,
        impact: Impact::Low,
        title: "split-debuginfo not set on macOS".into(),
        description: "Setting split-debuginfo = \"unpacked\" avoids bundling debug info during \
                      linking. Recent toolchains may already default to this for dev."
            .into(),
        fix: Some(Fix {
            description: "Set split-debuginfo in dev profile".into(),
            kind: FixKind::CargoToml {
                section: "profile.dev".into(),
                key: "split-debuginfo".into(),
                value: "\"unpacked\"".into(),
            },
        }),
    }]
}

fn has_custom_linker(ctx: &ProjectContext) -> bool {
    if let Some(config) = &ctx.cargo_config {
        // Any [target.*] linker or -fuse-ld in rustflags.
        if let Some(target) = config.get("target").and_then(toml::Value::as_table) {
            for target_config in target.values() {
                if let Some(table) = target_config.as_table() {
                    if table.contains_key("linker") {
                        return true;
                    }
                    if let Some(flags) = table.get("rustflags").and_then(toml::Value::as_array) {
                        if flags
                            .iter()
                            .filter_map(toml::Value::as_str)
                            .any(|f| f.contains("fuse-ld"))
                        {
                            return true;
                        }
                    }
                }
            }
        }
        // [build] linker or rustflags.
        if let Some(build) = config.get("build").and_then(toml::Value::as_table) {
            if build.contains_key("linker") || build.contains_key("rustflags") {
                return true;
            }
        }
    }

    // RUSTFLAGS env.
    if let Some(flags) = &ctx.env_vars.rustflags {
        if flags.contains("fuse-ld") {
            return true;
        }
    }

    false
}

fn has_split_debuginfo(cargo_toml: &toml::Table) -> bool {
    cargo_toml
        .get("profile")
        .and_then(toml::Value::as_table)
        .and_then(|p| p.get("dev"))
        .and_then(toml::Value::as_table)
        .is_some_and(|dev| dev.contains_key("split-debuginfo"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_section_uses_host_triple() {
        assert_eq!(
            target_section(Some("aarch64-unknown-linux-gnu")),
            "[target.aarch64-unknown-linux-gnu]"
        );
    }

    #[test]
    fn target_section_falls_back_when_host_unknown() {
        assert_eq!(target_section(None), "[target.x86_64-unknown-linux-gnu]");
    }

    #[test]
    fn linux_mold_fix_targets_the_real_host() {
        let linkers = InstalledLinkers {
            mold: true,
            lld: false,
        };
        let findings = linux_findings(&linkers, Some("aarch64-unknown-linux-gnu"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Fix);
        match &findings[0].fix.as_ref().unwrap().kind {
            FixKind::CargoConfig { key_path, .. } => {
                assert_eq!(key_path, "[target.aarch64-unknown-linux-gnu]");
            }
            other => panic!("expected CargoConfig, got {other:?}"),
        }
    }

    #[test]
    fn linux_no_linker_is_warn() {
        let linkers = InstalledLinkers::default();
        let findings = linux_findings(&linkers, None);
        assert_eq!(findings[0].severity, Severity::Warn);
    }

    #[test]
    fn macos_split_debuginfo_present_is_silent() {
        let toml: toml::Table = "[profile.dev]\nsplit-debuginfo = \"unpacked\"\n"
            .parse()
            .unwrap();
        assert!(macos_findings(&toml).is_empty());
    }

    #[test]
    fn macos_missing_split_debuginfo_is_info_not_fix() {
        let toml = toml::Table::new();
        let findings = macos_findings(&toml);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }
}
