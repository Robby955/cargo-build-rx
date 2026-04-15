use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    Fix,
    Warn,
    Info,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Fix => "FIX",
            Severity::Warn => "WARN",
            Severity::Info => "INFO",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Category {
    Linker,
    Profile,
    Dependencies,
    ProcMacros,
    BuildScripts,
    Features,
    DevDeps,
    Toolchain,
    Workspace,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Impact {
    High,
    Medium,
    Low,
}

impl Impact {
    pub fn label(&self) -> &'static str {
        match self {
            Impact::High => "High",
            Impact::Medium => "Medium",
            Impact::Low => "Low",
        }
    }
}

impl std::fmt::Display for Impact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub category: Category,
    pub impact: Impact,
    pub title: String,
    pub description: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    pub description: String,
    pub kind: FixKind,
}

#[derive(Debug, Clone, Serialize)]
pub enum FixKind {
    CargoConfig { key_path: String, value: String },
    CargoToml { section: String, key: String, value: String },
    ShellCommand(String),
    Manual(String),
}
