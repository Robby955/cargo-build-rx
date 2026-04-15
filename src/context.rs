use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::path::Path;
use std::process::Command;

/// All project data gathered up front. Checks are pure functions over this.
pub struct ProjectContext {
    pub metadata: Metadata,
    pub cargo_config: Option<toml::Table>,
    pub cargo_toml: toml::Table,
    pub rustc_version: Option<RustcVersion>,
    pub installed_linkers: InstalledLinkers,
    pub env_vars: EnvVars,
    pub os: Os,
}

#[derive(Debug)]
pub struct RustcVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub raw: String,
}

#[derive(Debug, Default)]
pub struct InstalledLinkers {
    pub mold: bool,
    pub lld: bool,
}

#[derive(Debug, Default)]
pub struct EnvVars {
    pub cargo_incremental: Option<String>,
    pub rustflags: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
    Windows,
    Other,
}

impl ProjectContext {
    pub fn gather(manifest_path: Option<&Path>) -> Result<Self> {
        let mut cmd = MetadataCommand::new();
        if let Some(path) = manifest_path {
            cmd.manifest_path(path);
        }
        let metadata = cmd.exec().context("Failed to run `cargo metadata`")?;

        let project_root = metadata
            .workspace_root
            .as_std_path()
            .to_path_buf();

        let cargo_toml = read_toml(&project_root.join("Cargo.toml"))?;
        let cargo_config = read_cargo_config(&project_root);
        let rustc_version = parse_rustc_version();
        let installed_linkers = detect_linkers();
        let env_vars = gather_env_vars();
        let os = detect_os();

        Ok(Self {
            metadata,
            cargo_config,
            cargo_toml,
            rustc_version,
            installed_linkers,
            env_vars,
            os,
        })
    }

    /// Get the project name from metadata.
    pub fn project_name(&self) -> &str {
        self.metadata
            .root_package()
            .map(|p| p.name.as_str())
            .unwrap_or("project")
    }

    /// Check if this is a workspace with multiple members.
    pub fn is_workspace(&self) -> bool {
        self.metadata.workspace_members.len() > 1
    }
}

fn read_toml(path: &Path) -> Result<toml::Table> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    content
        .parse::<toml::Table>()
        .with_context(|| format!("Failed to parse {}", path.display()))
}

fn read_cargo_config(root: &Path) -> Option<toml::Table> {
    let candidates = [
        root.join(".cargo/config.toml"),
        root.join(".cargo/config"),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return Some(table);
            }
        }
    }
    None
}

fn parse_rustc_version() -> Option<RustcVersion> {
    let output = Command::new("rustc").arg("--version").output().ok()?;
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // "rustc 1.78.0 (9b00956e5 2024-04-29)"
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
        mold: which("mold"),
        lld: which("lld") || which("ld.lld"),
    }
}

fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
