use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist_path = PathBuf::from(&manifest_dir).join("../frontend/dist");

    if dist_path.exists() {
        println!("cargo:rerun-if-changed={}", dist_path.display());
    }
}
