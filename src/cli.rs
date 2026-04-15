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

#[derive(Debug, Parser)]
#[command(version, about = "Compile-time diagnostic and prescription tool")]
pub struct Args {
    /// Output format.
    #[arg(long, default_value = "terminal")]
    pub format: Format,

    /// Run only these checks (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Skip these checks (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<String>,

    /// Minimum severity to display.
    #[arg(long, default_value = "info")]
    pub min_severity: SeverityFilter,

    /// Path to the project (default: current directory).
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Terminal,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SeverityFilter {
    Fix,
    Warn,
    Info,
}

impl SeverityFilter {
    pub fn passes(&self, severity: Severity) -> bool {
        match self {
            SeverityFilter::Info => true,
            SeverityFilter::Warn => matches!(severity, Severity::Fix | Severity::Warn),
            SeverityFilter::Fix => matches!(severity, Severity::Fix),
        }
    }
}
