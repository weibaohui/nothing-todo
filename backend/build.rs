use std::env;
use std::path::PathBuf;
use vergen_gitcl::{Emitter, GitclBuilder};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist_path = PathBuf::from(&manifest_dir).join("../frontend/dist");

    if dist_path.exists() {
        println!("cargo:rerun-if-changed={}", dist_path.display());
    }

    let gitcl = GitclBuilder::default()
        .sha(true)
        .describe(true, true, None)
        .build()
        .expect("Failed to build gitcl config");

    if let Err(e) = Emitter::default()
        .add_instructions(&gitcl)
        .expect("Failed to add vergen instructions")
        .emit()
    {
        println!("cargo:warning=vergen: {}", e);
    }
}
