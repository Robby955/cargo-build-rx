use crate::finding::{Finding, Fix, FixKind, Severity};

// ANSI helpers — no color crate needed.
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";

fn severity_color(s: Severity) -> &'static str {
    match s {
        Severity::Fix => RED,
        Severity::Warn => YELLOW,
        Severity::Info => CYAN,
    }
}

pub fn render_terminal(project_name: &str, findings: &[Finding]) {
    if findings.is_empty() {
        println!(
            "\n{BOLD}cargo-build-rx{RESET} — no prescriptions for {project_name}. Looks healthy!\n"
        );
        return;
    }

    println!(
        "\n{BOLD}cargo-build-rx{RESET} — {} prescription{} for {project_name}\n",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" }
    );

    for finding in findings {
        let color = severity_color(finding.severity);
        println!(
            "  {color}{BOLD}{:4}{RESET} {DIM}[{}]{RESET} {BOLD}{}{RESET}",
            finding.severity.label(),
            finding.impact,
            finding.title,
        );

        for line in finding.description.lines() {
            println!("       {DIM}{line}{RESET}");
        }

        if let Some(fix) = &finding.fix {
            println!();
            render_fix(fix);
        }

        println!();
    }

    // Summary line
    let fixes = findings.iter().filter(|f| f.severity == Severity::Fix).count();
    let warns = findings.iter().filter(|f| f.severity == Severity::Warn).count();
    let infos = findings.iter().filter(|f| f.severity == Severity::Info).count();

    let mut parts = Vec::new();
    if fixes > 0 {
        parts.push(format!("{RED}{fixes} fix{}{RESET}", if fixes == 1 { "" } else { "es" }));
    }
    if warns > 0 {
        parts.push(format!("{YELLOW}{warns} warning{}{RESET}", if warns == 1 { "" } else { "s" }));
    }
    if infos > 0 {
        parts.push(format!("{CYAN}{infos} info{RESET}"));
    }
    println!("{BOLD}Summary:{RESET} {}", parts.join(", "));
    println!();
}

fn render_fix(fix: &Fix) {
    match &fix.kind {
        FixKind::CargoConfig { key_path, value } => {
            println!("       {BOLD}->{RESET} Add to .cargo/config.toml:");
            println!("         {DIM}{key_path}{RESET}");
            for line in value.lines() {
                println!("         {line}");
            }
        }
        FixKind::CargoToml { section, key, value } => {
            println!("       {BOLD}->{RESET} In Cargo.toml [{section}]:");
            println!("         {key} = {value}");
        }
        FixKind::ShellCommand(cmd) => {
            println!("       {BOLD}->{RESET} Run: {DIM}{cmd}{RESET}");
        }
        FixKind::Manual(desc) => {
            println!("       {BOLD}->{RESET} {desc}");
        }
    }
}

pub fn render_json(findings: &[Finding]) -> String {
    serde_json::to_string_pretty(findings).unwrap_or_else(|_| "[]".to_string())
}
