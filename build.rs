use std::process::Command;

/// Capture build provenance for the footer. `sw-checklist` requires the footer
/// to name the build host, commit, and time, and those are only knowable here.
fn main() {
    println!(
        "cargo:rustc-env=BUILD_SHA={}",
        capture("git", &["rev-parse", "--short", "HEAD"])
    );
    println!("cargo:rustc-env=BUILD_HOST={}", capture("hostname", &[]));
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        capture("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
    );
    println!("cargo:rerun-if-changed=build.rs");
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
