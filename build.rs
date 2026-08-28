use std::fs;
use std::process::Command;

/// Capture build provenance for the footer. `sw-checklist` requires the footer
/// to name the build host, commit, and time, and those are only knowable here.
fn main() {
    track_rebuild_triggers();
    println!(
        "cargo:rustc-env=BUILD_SHA={}",
        capture("git", &["rev-parse", "--short", "HEAD"])
    );
    println!("cargo:rustc-env=BUILD_HOST={}", capture("hostname", &[]));
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
    );
}

/// Emitting any `rerun-if-changed` narrows cargo to exactly those paths, so
/// watching only build.rs would freeze the provenance at the first build and
/// leave every later rebuild reporting a stale commit. Watch the git ref too.
fn track_rebuild_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    let Ok(head) = fs::read_to_string(".git/HEAD") else {
        return;
    };
    if let Some(reference) = head.strip_prefix("ref: ") {
        println!("cargo:rerun-if-changed=.git/{}", reference.trim());
    }
}

fn capture(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".into())
}
