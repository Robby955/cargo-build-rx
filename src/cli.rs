//! Command-line surface: the `cargo build-rx` argument parser.

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use crate::finding::Severity;

/// Cargo subcommand wrapper so `cargo build-rx` works.
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum Cargo {
    /// Diagnose compile-time issues and prescribe fixes.
    BuildRx(Args),
}

/// Parsed arguments for the `build-rx` subcommand.
#[derive(Debug, Parser)]
#[command(version, about = "Compile-time diagnostic and prescription tool")]
pub struct Args {
    /// Output format.
    #[arg(long, default_value = "terminal")]
    pub format: Format,

    /// When to use ANSI color in terminal output.
    #[arg(long, default_value = "auto")]
    pub color: ColorChoice,

    /// Run only these checks (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Skip these checks (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<String>,

    /// Minimum severity to display.
    #[arg(long, default_value = "info")]
    pub min_severity: SeverityFilter,

    /// Exit non-zero when a finding at or above this severity is present.
    ///
    /// Defaults to `none`, so the tool exits 0. Pass `--deny fix` (or `warn`)
    /// to turn build-rx into a failing CI gate.
    #[arg(long, default_value = "none")]
    pub deny: DenyLevel,

    /// Path to the project (default: current directory).
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

/// Output format for the report.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    /// Human-readable text for a terminal.
    Terminal,
    /// A JSON array of findings, for CI and tooling.
    Json,
}

/// When to emit ANSI color escapes in terminal output.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorChoice {
    /// Color only when stdout is a terminal and `NO_COLOR` is unset.
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

/// The lowest severity a `--min-severity` filter keeps.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SeverityFilter {
    /// Show only `Fix` findings.
    Fix,
    /// Show `Fix` and `Warn` findings.
    Warn,
    /// Show everything.
    Info,
}

impl SeverityFilter {
    /// Returns `true` if a finding of `severity` passes this filter.
    ///
    /// The relation is monotone: anything `Fix` passes keeps passing at
    /// `Warn` and `Info`, so `fix ⊆ warn ⊆ info`.
    ///
    /// ```
    /// use cargo_build_rx::cli::SeverityFilter;
    /// use cargo_build_rx::finding::Severity;
    /// assert!(SeverityFilter::Fix.passes(Severity::Fix));
    /// assert!(!SeverityFilter::Fix.passes(Severity::Warn));
    /// assert!(SeverityFilter::Info.passes(Severity::Info));
    /// ```
    #[must_use]
    pub fn passes(self, severity: Severity) -> bool {
        match self {
            SeverityFilter::Info => true,
            SeverityFilter::Warn => matches!(severity, Severity::Fix | Severity::Warn),
            SeverityFilter::Fix => matches!(severity, Severity::Fix),
        }
    }
}

/// Threshold at which the process exits non-zero (for CI gating).
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DenyLevel {
    /// Never exit non-zero on findings.
    None,
    /// Exit non-zero if any `Fix` finding is present.
    Fix,
    /// Exit non-zero if any `Fix` or `Warn` finding is present.
    Warn,
    /// Exit non-zero if any finding is present.
    Info,
}

impl DenyLevel {
    /// Returns `true` if `severity` should trigger a non-zero exit.
    ///
    /// ```
    /// use cargo_build_rx::cli::DenyLevel;
    /// use cargo_build_rx::finding::Severity;
    /// assert!(!DenyLevel::None.triggers(Severity::Fix));
    /// assert!(DenyLevel::Fix.triggers(Severity::Fix));
    /// assert!(!DenyLevel::Fix.triggers(Severity::Warn));
    /// assert!(DenyLevel::Warn.triggers(Severity::Fix));
    /// assert!(DenyLevel::Info.triggers(Severity::Info));
    /// ```
    #[must_use]
    pub fn triggers(self, severity: Severity) -> bool {
        match self {
            DenyLevel::None => false,
            DenyLevel::Fix => severity == Severity::Fix,
            DenyLevel::Warn => matches!(severity, Severity::Fix | Severity::Warn),
            DenyLevel::Info => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Severity; 3] = [Severity::Fix, Severity::Warn, Severity::Info];

    #[test]
    fn severity_filter_is_monotone() {
        // fix ⊆ warn ⊆ info: anything a stricter filter keeps, a looser one keeps.
        for s in ALL {
            if SeverityFilter::Fix.passes(s) {
                assert!(SeverityFilter::Warn.passes(s));
            }
            if SeverityFilter::Warn.passes(s) {
                assert!(SeverityFilter::Info.passes(s));
            }
        }
    }

    #[test]
    fn info_filter_passes_everything() {
        assert!(ALL.into_iter().all(|s| SeverityFilter::Info.passes(s)));
    }

    #[test]
    fn deny_none_never_triggers() {
        assert!(ALL.into_iter().all(|s| !DenyLevel::None.triggers(s)));
    }

    #[test]
    fn deny_is_monotone_with_severity() {
        // A stricter deny threshold triggers on a superset of severities.
        for s in ALL {
            if DenyLevel::Fix.triggers(s) {
                assert!(DenyLevel::Warn.triggers(s));
            }
            if DenyLevel::Warn.triggers(s) {
                assert!(DenyLevel::Info.triggers(s));
            }
        }
    }
}
