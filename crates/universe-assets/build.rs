use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-env-changed=UNIVERSE_PORTAL_DIST");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let built_dist = manifest_dir.join("../../packages/portal/build");
    // Headless v0.1.0 fallback (decision-11): when the SvelteKit SSG output has not been
    // built, embed the committed placeholder so the crate compiles from a fresh checkout
    // without a Bun/SvelteKit toolchain. The real SSG output always wins when present.
    let placeholder_dist = manifest_dir.join("placeholder");

    println!("cargo:rerun-if-changed={}", built_dist.display());
    println!("cargo:rerun-if-changed={}", placeholder_dist.display());

    let source_dir = match env::var("UNIVERSE_PORTAL_DIST") {
        Ok(custom) => PathBuf::from(custom),
        Err(_) if built_dist.join("index.html").is_file() => built_dist,
        // Keep the original so the panic message below can still name the placeholder path.
        Err(_) => placeholder_dist.clone(),
    };

    let staged = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("portal-dist");
    if staged.exists() {
        fs::remove_dir_all(&staged).expect("clean staged portal-dist");
    }

    if source_dir.join("index.html").is_file() {
        copy_dir_recursive(&source_dir, &staged).expect("stage built portal assets");
    } else {
        panic!(
            "Missing portal assets at '{}' and placeholder at '{}'. Run `pixi run build` first.",
            source_dir.display(),
            placeholder_dist.display()
        );
    }
}
