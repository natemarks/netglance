// Build script to embed git information at compile time

use std::process::Command;

fn main() {
    // Get git commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    // Check if git working directory is clean
    let is_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    let git_hash_with_dirty = if is_dirty {
        format!("{}-dirty", git_hash)
    } else {
        git_hash
    };

    // Embed as environment variables for use in the binary
    println!("cargo:rustc-env=GIT_HASH={}", git_hash_with_dirty);
    println!(
        "cargo:rustc-env=BUILD_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );

    // Fail build if git is dirty (for release builds only)
    if std::env::var("PROFILE").unwrap_or_default() == "release"
        && is_dirty
        && std::env::var("NETGLANCE_ALLOW_DIRTY").is_err()
    {
        panic!(
            "\n\n\
            ╔═══════════════════════════════════════════════════════════════╗\n\
            ║              BUILD FAILED: UNCLEAN GIT STATE                 ║\n\
            ╚═══════════════════════════════════════════════════════════════╝\n\
            \n\
            Release builds require a clean git working directory.\n\
            \n\
            You have uncommitted changes:\n\
            \n\
            Run 'git status' to see changes.\n\
            \n\
            To build with uncommitted changes (not recommended):\n\
              NETGLANCE_ALLOW_DIRTY=1 cargo build --release\n\
            \n\
            For releases, commit or stash your changes first.\n\
            "
        );
    }

    // Re-run if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
