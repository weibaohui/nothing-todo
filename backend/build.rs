use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist_path = PathBuf::from(&manifest_dir).join("../frontend/dist");

    if dist_path.exists() {
        println!("cargo:rerun-if-changed={}", dist_path.display());
    }

    // Inject git hash from env (set by CI) or try to get from git
    let git_hash = env::var("NTD_GIT_HASH")
        .ok()
        .or_else(|| {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()?;
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=NTD_GIT_HASH={}", git_hash);
}
