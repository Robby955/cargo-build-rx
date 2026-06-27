use crate::checks::Check;
use crate::context::ProjectContext;
use crate::finding::{Category, Finding, Fix, FixKind, Impact, Severity};

pub struct FeaturesCheck;

/// Known heavy default-feature sets.
const HEAVY_DEFAULTS: &[(&str, &str, &str)] = &[
    (
        "tokio",
        "full",
        "Only enable the features you actually use (e.g., rt-multi-thread, macros, io-util)",
    ),
    (
        "reqwest",
        "default-tls",
        "Consider using rustls-tls instead, or disable default features",
    ),
    (
        "aws-sdk-s3",
        "default",
        "Disable default features and pick only what you need",
    ),
    (
        "openssl",
        "default",
        "Consider using rustls as a TLS backend to avoid native compilation",
    ),
];

impl Check for FeaturesCheck {
    fn name(&self) -> &'static str {
        "features"
    }

    fn run(&self, ctx: &ProjectContext) -> Vec<Finding> {
        feature_findings(&ctx.cargo_toml)
    }
}

/// Inspect the direct dependencies declared in the manifest for heavy default
/// feature sets. Pure over the parsed manifest.
fn feature_findings(cargo_toml: &toml::Table) -> Vec<Finding> {
    let mut findings = Vec::new();
    let deps_sections = ["dependencies", "dev-dependencies", "build-dependencies"];

    for section in &deps_sections {
        let Some(deps) = cargo_toml.get(*section).and_then(toml::Value::as_table) else {
            continue;
        };

        for (name, spec) in deps {
            let features = extract_features(spec);
            let default_features_enabled = !is_default_features_false(spec);

            for &(crate_name, feature_flag, advice) in HEAVY_DEFAULTS {
                if name != crate_name {
                    continue;
                }

                if features.iter().any(|f| f == feature_flag) {
                    findings.push(Finding {
                        severity: Severity::Warn,
                        category: Category::Features,
                        impact: Impact::Medium,
                        title: format!("{name} uses \"{feature_flag}\" feature"),
                        description: format!(
                            "The \"{feature_flag}\" feature pulls in many sub-features, \
                             increasing compile time. {advice}."
                        ),
                        fix: Some(Fix {
                            description: format!("Reduce {name} features"),
                            kind: FixKind::Manual(advice.to_string()),
                        }),
                    });
                } else if default_features_enabled && feature_flag == "default" {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: Category::Features,
                        impact: Impact::Low,
                        title: format!("{name} has default features enabled"),
                        description: format!("{advice}."),
                        fix: Some(Fix {
                            description: format!("Disable default features for {name}"),
                            kind: FixKind::Manual(format!(
                                "Set default-features = false for {name} and enable only needed features"
                            )),
                        }),
                    });
                }
            }
        }
    }

    findings
}

fn extract_features(spec: &toml::Value) -> Vec<String> {
    match spec {
        toml::Value::Table(t) => t
            .get("features")
            .and_then(toml::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn is_default_features_false(spec: &toml::Value) -> bool {
    match spec {
        toml::Value::Table(t) => !t
            .get("default-features")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> toml::Table {
        s.parse().unwrap()
    }

    #[test]
    fn tokio_full_is_warned() {
        let findings = feature_findings(&parse(
            "[dependencies]\ntokio = { version = \"1\", features = [\"full\"] }\n",
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
        assert_eq!(findings[0].title, "tokio uses \"full\" feature");
    }

    #[test]
    fn tokio_without_full_is_silent() {
        let findings = feature_findings(&parse(
            "[dependencies]\ntokio = { version = \"1\", features = [\"macros\"] }\n",
        ));
        assert!(findings.is_empty());
    }

    #[test]
    fn openssl_default_features_is_info() {
        let findings = feature_findings(&parse("[dependencies]\nopenssl = \"0.10\"\n"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn openssl_default_features_false_is_silent() {
        let findings = feature_findings(&parse(
            "[dependencies]\nopenssl = { version = \"0.10\", default-features = false }\n",
        ));
        assert!(findings.is_empty());
    }

    #[test]
    fn unknown_crate_is_silent() {
        let findings = feature_findings(&parse(
            "[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\n",
        ));
        assert!(findings.is_empty());
    }
}
