use std::process::Command;

fn cargo_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["run", "--quiet", "--"]);
    cmd
}

#[test]
fn help_flag() {
    let output = cargo_bin()
        .arg("build-rx")
        .arg("--help")
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Diagnose compile-time issues"),
        "Help text should contain description. Got: {stdout}"
    );
}

#[test]
fn runs_on_self_terminal() {
    let output = cargo_bin()
        .arg("build-rx")
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cargo-build-rx"),
        "Should contain tool name in output. Got: {stdout}"
    );
}

#[test]
fn runs_on_self_json() {
    let output = cargo_bin()
        .arg("build-rx")
        .arg("--format")
        .arg("json")
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON (array)
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"));
    assert!(parsed.is_array(), "JSON output should be an array");
}

#[test]
fn only_flag_filters() {
    let output = cargo_bin()
        .args(["build-rx", "--only", "incremental", "--format", "json"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"));
    // All findings (if any) should be Incremental category
    for finding in &parsed {
        assert_eq!(
            finding["category"].as_str().unwrap(),
            "Incremental",
            "--only incremental should only produce Incremental findings"
        );
    }
}

#[test]
fn skip_flag_filters() {
    let output = cargo_bin()
        .args(["build-rx", "--skip", "linker,profile,duplicates,proc-macros,build-scripts,features,dev-deps,toolchain,workspace", "--format", "json"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"));
    // Should only have incremental findings (if any)
    for finding in &parsed {
        assert_eq!(
            finding["category"].as_str().unwrap(),
            "Incremental",
            "Skipping all but incremental should only produce Incremental findings"
        );
    }
}

#[test]
fn min_severity_filters() {
    // With --min-severity fix, only Fix-level findings should appear
    let output = cargo_bin()
        .args(["build-rx", "--min-severity", "fix", "--format", "json"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"));
    for finding in &parsed {
        assert_eq!(
            finding["severity"].as_str().unwrap(),
            "Fix",
            "--min-severity fix should only show Fix findings"
        );
    }
}
