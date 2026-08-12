use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=LOBSTER_BUILD_GIT_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    let git_sha = std::env::var("LOBSTER_BUILD_GIT_SHA")
        .ok()
        .filter(|value| is_commit_sha(value))
        .or_else(git_head_sha)
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=LOBSTER_BUILD_GIT_SHA={git_sha}");
}

fn git_head_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    is_commit_sha(&value).then_some(value)
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
