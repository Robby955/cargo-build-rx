//! Gathers everything the checks need, once, up front.
//!
//! [`ProjectContext::gather`] runs `cargo metadata`, reads the manifest and
//! `.cargo/config.toml`, queries `rustc`, probes for fast linkers, and reads a
//! couple of environment variables. Every check is then a pure function over
//! the resulting [`ProjectContext`].

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::path::Path;
use std::process::Command;

/// All project data gathered up front. Checks are pure functions over this.
pub struct ProjectContext {
    /// Output of `cargo metadata` for the project.
    pub metadata: Metadata,
    /// Parsed `.cargo/config.toml`, if present.
    pub cargo_config: Option<toml::Table>,
    /// Parsed workspace-root `Cargo.toml`.
    pub cargo_toml: toml::Table,
    /// Parsed `rustc` version, if it could be determined.
    pub rustc_version: Option<RustcVersion>,
    /// The host target triple reported by `rustc -vV`, e.g.
    /// `aarch64-apple-darwin`. `None` if it could not be parsed.
    pub host_triple: Option<String>,
    /// Which fast linkers were found on `PATH`.
    pub installed_linkers: InstalledLinkers,
    /// Build-relevant environment variables.
    pub env_vars: EnvVars,
    /// The operating system build-rx is running on.
    pub os: Os,
}

/// A parsed `rustc` version (`major.minor.patch`) plus the raw version line.
#[derive(Debug)]
pub struct RustcVersion {
    /// Major version (always `1` for current stable Rust).
    pub major: u32,
    /// Minor version, e.g. `96` for `1.96.0`.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
    /// The unparsed first line of `rustc -vV`.
    pub raw: String,
}

/// Which fast linkers were detected on `PATH`.
#[derive(Debug, Default)]
pub struct InstalledLinkers {
    /// `mold` is available.
    pub mold: bool,
    /// `lld` (or `ld.lld`) is available.
    pub lld: bool,
}

/// Build-relevant environment variables read at startup.
#[derive(Debug, Default)]
pub struct EnvVars {
    /// Value of `CARGO_INCREMENTAL`, if set.
    pub cargo_incremental: Option<String>,
    /// Value of `RUSTFLAGS`, if set.
    pub rustflags: Option<String>,
}

/// The operating system build-rx is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// Linux.
    Linux,
    /// macOS.
    Macos,
    /// Windows.
    Windows,
    /// Anything else.
    Other,
}

impl ProjectContext {
    /// Gather all project data.
    ///
    /// `manifest_path` selects the project; `None` uses the current directory.
    pub fn gather(manifest_path: Option<&Path>) -> Result<Self> {
        let mut cmd = MetadataCommand::new();
        if let Some(path) = manifest_path {
            cmd.manifest_path(path);
        }
        let metadata = cmd.exec().context("Failed to run `cargo metadata`")?;

        let project_root = metadata.workspace_root.as_std_path().to_path_buf();

        let cargo_toml = read_toml(&project_root.join("Cargo.toml"))?;
        let cargo_config = read_cargo_config(&project_root);
        let RustcInfo {
            version: rustc_version,
            host_triple,
        } = parse_rustc_info();
        let installed_linkers = detect_linkers();
        let env_vars = gather_env_vars();
        let os = detect_os();

        Ok(Self {
            metadata,
            cargo_config,
            cargo_toml,
            rustc_version,
            host_triple,
            installed_linkers,
            env_vars,
            os,
        })
    }

    /// Get the project name from metadata.
    pub fn project_name(&self) -> &str {
        self.metadata
            .root_package()
            .map_or("project", |p| p.name.as_str())
    }

    /// Check if this is a workspace with multiple members.
    pub fn is_workspace(&self) -> bool {
        self.metadata.workspace_members.len() > 1
    }

    /// The project's declared minimum supported Rust version (`rust-version`),
    /// as `(major, minor)`, read from `[package]` or `[workspace.package]`.
    ///
    /// Returns `None` when no MSRV is declared.
    pub fn declared_msrv(&self) -> Option<(u32, u32)> {
        let from_package = self
            .cargo_toml
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("rust-version"))
            .and_then(toml::Value::as_str);

        let from_workspace = self
            .cargo_toml
            .get("workspace")
            .and_then(|w| w.as_table())
            .and_then(|w| w.get("package"))
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("rust-version"))
            .and_then(toml::Value::as_str);

        let raw = from_package.or(from_workspace)?;
        parse_major_minor(raw)
    }
}

/// Parse a `major.minor` (or `major.minor.patch`) version string.
fn parse_major_minor(raw: &str) -> Option<(u32, u32)> {
    let mut parts = raw.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn read_toml(path: &Path) -> Result<toml::Table> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    content
        .parse::<toml::Table>()
        .with_context(|| format!("Failed to parse {}", path.display()))
}

fn read_cargo_config(root: &Path) -> Option<toml::Table> {
    let candidates = [root.join(".cargo/config.toml"), root.join(".cargo/config")];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return Some(table);
            }
        }
    }
    None
}

/// What we extract from a single `rustc -vV` call.
struct RustcInfo {
    version: Option<RustcVersion>,
    host_triple: Option<String>,
}

fn parse_rustc_info() -> RustcInfo {
    let Some(output) = Command::new("rustc").arg("-vV").output().ok() else {
        return RustcInfo {
            version: None,
            host_triple: None,
        };
    };
    let text = String::from_utf8_lossy(&output.stdout);

    // First line: "rustc 1.96.0 (ac68faa20 2026-05-25)"
    let version = text.lines().next().and_then(parse_version_line);

    // A later line: "host: aarch64-apple-darwin"
    let host_triple = text
        .lines()
        .find_map(|line| line.strip_prefix("host:"))
        .map(|t| t.trim().to_string());

    RustcInfo {
        version,
        host_triple,
    }
}

fn parse_version_line(line: &str) -> Option<RustcVersion> {
    let raw = line.trim().to_string();
    let version_part = raw.strip_prefix("rustc ")?.split_whitespace().next()?;
    let mut parts = version_part.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some(RustcVersion {
        major,
        minor,
        patch,
        raw,
    })
}

fn detect_linkers() -> InstalledLinkers {
    InstalledLinkers {
        mold: on_path("mold"),
        lld: on_path("lld") || on_path("ld.lld"),
    }
}

/// Returns `true` if an executable named `name` is found on `PATH`.
///
/// Scans `PATH` directly rather than shelling out to `which`/`where`, so it is
/// portable and spawns no subprocess.
fn on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(name);
        candidate.is_file() || (cfg!(windows) && candidate.with_extension("exe").is_file())
    })
}

fn gather_env_vars() -> EnvVars {
    EnvVars {
        cargo_incremental: std::env::var("CARGO_INCREMENTAL").ok(),
        rustflags: std::env::var("RUSTFLAGS").ok(),
    }
}

fn detect_os() -> Os {
    if cfg!(target_os = "linux") {
        Os::Linux
    } else if cfg!(target_os = "macos") {
        Os::Macos
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else {
        Os::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_version_line() {
        let v = parse_version_line("rustc 1.96.0 (ac68faa20 2026-05-25)").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (1, 96, 0));
    }

    #[test]
    fn rejects_non_version_line() {
        assert!(parse_version_line("host: aarch64-apple-darwin").is_none());
    }

    #[test]
    fn parses_msrv_two_and_three_component() {
        assert_eq!(parse_major_minor("1.74"), Some((1, 74)));
        assert_eq!(parse_major_minor("1.74.1"), Some((1, 74)));
        assert_eq!(parse_major_minor("nightly"), None);
    }
}
