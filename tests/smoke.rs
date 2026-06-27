//! End-to-end tests: build the binary and run it against the crate itself and
//! against the fixture projects under `tests/fixtures/`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args(["run", "--quiet", "--"]);
    cmd
}

/// A fixture project copied into a unique temp dir for the duration of one test.
///
/// The fixtures ship with their manifest named `Cargo.toml.fixture` rather than
/// `Cargo.toml`. That rename is deliberate: a directory containing a real
/// `Cargo.toml` is a nested crate, and `cargo package` strips nested crates from
/// the published `.crate` tarball. With the manifest renamed, the fixture files
/// travel with the package, and each test materializes a working copy here.
struct StagedFixture {
    dir: PathBuf,
}

impl StagedFixture {
    fn manifest(&self) -> PathBuf {
        self.dir.join("Cargo.toml")
    }
}

impl Drop for StagedFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Recursively copy a directory tree from `src` to `dst`.
fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create staged dir");
    for entry in std::fs::read_dir(src).expect("read fixture dir") {
        let entry = entry.expect("read fixture entry");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&from, &to);
        } else {
            std::fs::copy(&from, &to).expect("copy fixture file");
        }
    }
}

/// Stage the named fixture into a fresh temp dir, renaming `Cargo.toml.fixture`
/// back to `Cargo.toml` so cargo treats it as a real project at test time.
fn stage_fixture(name: &str) -> StagedFixture {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);

    // A unique, collision-resistant directory under the system temp dir. The
    // process-wide counter guarantees uniqueness across parallel test threads,
    // which share a pid and can read the same nanosecond clock.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = format!(
        "cargo-build-rx-{name}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    let dir = std::env::temp_dir().join(unique);
    copy_dir(&src, &dir);

    let staged_manifest = dir.join("Cargo.toml.fixture");
    std::fs::rename(&staged_manifest, dir.join("Cargo.toml"))
        .expect("materialize fixture Cargo.toml");

    StagedFixture { dir }
}

/// Run `cargo build-rx --format json` against a manifest and parse the array.
fn run_json(manifest: &PathBuf) -> Vec<serde_json::Value> {
    let output = cargo_bin()
        .args(["build-rx", "--manifest-path"])
        .arg(manifest)
        .args(["--format", "json"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"))
}

fn titles(findings: &[serde_json::Value]) -> Vec<String> {
    findings
        .iter()
        .map(|f| f["title"].as_str().unwrap_or_default().to_string())
        .collect()
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
        .args(["build-rx", "--format", "json"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
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
        .args([
            "build-rx",
            "--skip",
            "linker,profile,duplicates,proc-macros,build-scripts,features,dev-deps,toolchain,workspace",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput: {stdout}"));
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

#[test]
fn default_exit_code_is_zero_without_deny() {
    // Without --deny, the tool must not fail CI even when it has findings.
    let fixture = stage_fixture("bloated");
    let status = cargo_bin()
        .args(["build-rx", "--manifest-path"])
        .arg(fixture.manifest())
        .args(["--format", "json"])
        .status()
        .expect("failed to run");
    assert!(status.success(), "default run must exit 0");
}

#[test]
fn bloated_fixture_reports_profile_findings() {
    let fixture = stage_fixture("bloated");
    let findings = run_json(&fixture.manifest());
    let titles = titles(&findings);
    assert!(
        titles.iter().any(|t| t == "Full debuginfo in dev profile"),
        "bloated fixture should flag debug = 2. Got: {titles:?}"
    );
    assert!(
        titles.iter().any(|t| t.starts_with("opt-level = 2")),
        "bloated fixture should flag opt-level = 2. Got: {titles:?}"
    );
}

#[test]
fn clean_fixture_has_no_profile_findings() {
    let fixture = stage_fixture("clean");
    let findings = run_json(&fixture.manifest());
    let profile: Vec<_> = findings
        .iter()
        .filter(|f| f["category"].as_str() == Some("Profile"))
        .collect();
    assert!(
        profile.is_empty(),
        "clean fixture should have no Profile findings. Got: {profile:?}"
    );
}

#[test]
fn deny_warn_exits_nonzero_on_bloated_fixture() {
    // The bloated fixture has Warn-level profile findings; --deny warn must fail.
    let fixture = stage_fixture("bloated");
    let status = cargo_bin()
        .args(["build-rx", "--manifest-path"])
        .arg(fixture.manifest())
        .args(["--deny", "warn", "--format", "json"])
        .status()
        .expect("failed to run");
    assert!(!status.success(), "--deny warn must exit non-zero here");
}
