//! Rendering: turn a slice of [`Finding`]s into terminal text or JSON.

use std::io::IsTerminal;

use anyhow::{Context, Result};

use crate::cli::ColorChoice;
use crate::finding::{Finding, Fix, FixKind, Severity};

/// ANSI escape codes, blanked out when color is disabled.
///
/// Every renderer formats against a `Palette` so that disabling color is a
/// matter of substituting empty strings, not branching at each call site.
struct Palette {
    reset: &'static str,
    bold: &'static str,
    red: &'static str,
    yellow: &'static str,
    cyan: &'static str,
    dim: &'static str,
}

impl Palette {
    const COLOR: Palette = Palette {
        reset: "\x1b[0m",
        bold: "\x1b[1m",
        red: "\x1b[31m",
        yellow: "\x1b[33m",
        cyan: "\x1b[36m",
        dim: "\x1b[2m",
    };

    const PLAIN: Palette = Palette {
        reset: "",
        bold: "",
        red: "",
        yellow: "",
        cyan: "",
        dim: "",
    };

    fn severity_color(&self, s: Severity) -> &'static str {
        match s {
            Severity::Fix => self.red,
            Severity::Warn => self.yellow,
            Severity::Info => self.cyan,
        }
    }
}

/// Decide whether to colorize, honoring `--color`, TTY status, and `NO_COLOR`.
fn use_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stdout().is_terminal() && !no_color_set(),
    }
}

/// `NO_COLOR` is honored when present and non-empty (per the informal standard).
fn no_color_set() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

/// Render findings as human-readable text to stdout.
pub fn render_terminal(project_name: &str, findings: &[Finding], color: ColorChoice) {
    let p = if use_color(color) {
        &Palette::COLOR
    } else {
        &Palette::PLAIN
    };
    let (reset, bold, dim) = (p.reset, p.bold, p.dim);

    if findings.is_empty() {
        println!(
            "\n{bold}cargo-build-rx{reset} — no prescriptions for {project_name}. Looks healthy!\n"
        );
        return;
    }

    println!(
        "\n{bold}cargo-build-rx{reset} — {} prescription{} for {project_name}\n",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" }
    );

    for finding in findings {
        let color = p.severity_color(finding.severity);
        println!(
            "  {color}{bold}{:4}{reset} {dim}[{}]{reset} {bold}{}{reset}",
            finding.severity.label(),
            finding.impact,
            finding.title,
        );

        for line in finding.description.lines() {
            println!("       {dim}{line}{reset}");
        }

        if let Some(fix) = &finding.fix {
            println!();
            render_fix(fix, p);
        }

        println!();
    }

    render_summary(findings, p);
}

fn render_summary(findings: &[Finding], p: &Palette) {
    let (reset, bold) = (p.reset, p.bold);
    let fixes = findings.iter().filter(|f| f.severity == Severity::Fix).count();
    let warns = findings.iter().filter(|f| f.severity == Severity::Warn).count();
    let infos = findings.iter().filter(|f| f.severity == Severity::Info).count();

    let mut parts = Vec::new();
    if fixes > 0 {
        parts.push(format!(
            "{}{fixes} fix{}{reset}",
            p.red,
            if fixes == 1 { "" } else { "es" }
        ));
    }
    if warns > 0 {
        parts.push(format!(
            "{}{warns} warning{}{reset}",
            p.yellow,
            if warns == 1 { "" } else { "s" }
        ));
    }
    if infos > 0 {
        parts.push(format!("{}{infos} info{reset}", p.cyan));
    }
    println!("{bold}Summary:{reset} {}", parts.join(", "));
    println!();
}

fn render_fix(fix: &Fix, p: &Palette) {
    let (reset, bold, dim) = (p.reset, p.bold, p.dim);
    match &fix.kind {
        FixKind::CargoConfig { key_path, value } => {
            println!("       {bold}->{reset} Add to .cargo/config.toml:");
            println!("         {dim}{key_path}{reset}");
            for line in value.lines() {
                println!("         {line}");
            }
        }
        FixKind::CargoToml {
            section,
            key,
            value,
        } => {
            println!("       {bold}->{reset} In Cargo.toml [{section}]:");
            println!("         {key} = {value}");
        }
        FixKind::ShellCommand(cmd) => {
            println!("       {bold}->{reset} Run: {dim}{cmd}{reset}");
        }
        FixKind::Manual(desc) => {
            println!("       {bold}->{reset} {desc}");
        }
    }
}

/// Render findings as a pretty-printed JSON array.
///
/// # Errors
///
/// Returns an error if the findings cannot be serialized to JSON.
pub fn render_json(findings: &[Finding]) -> Result<String> {
    serde_json::to_string_pretty(findings).context("Failed to serialize findings to JSON")
}
