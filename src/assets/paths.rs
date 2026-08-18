//! Resolve pack-relative paths so `cargo run` works from any cwd.

use std::path::{Path, PathBuf};

/// Look next to the crate, then cwd, then the executable.
pub fn resolve_asset(rel: impl AsRef<Path>) -> PathBuf {
    let rel = rel.as_ref();
    if rel.is_absolute() && rel.exists() {
        return rel.to_path_buf();
    }

    let from_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    if from_manifest.exists() {
        return from_manifest;
    }

    if rel.exists() {
        return rel.to_path_buf();
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_exe = dir.join(rel);
            if next_to_exe.exists() {
                return next_to_exe;
            }
            let up_one = dir.join("..").join(rel);
            if up_one.exists() {
                return up_one;
            }
        }
    }

    from_manifest
}
