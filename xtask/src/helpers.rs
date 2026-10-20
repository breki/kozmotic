use std::path::PathBuf;

/// Resolve the workspace root (the parent of the xtask
/// crate directory).
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate always lives under workspace root")
        .to_path_buf()
}

/// This project's settings for the shared `rustbase-xtask` gates:
/// the name sets `KOZMOTIC_DEP_AGE_ALLOW`, `KOZMOTIC_DEP_AGE_STRICT`,
/// the user agent and `crates/kozmotic/Cargo.toml` as the version
/// manifest. Override a limit on the returned value, e.g.
/// `cfg.thresholds.coverage_file = 80.0;`.
pub fn tasks_config() -> rustbase_xtask::Config {
    rustbase_xtask::Config::for_project("kozmotic", workspace_root())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_contains_cargo_toml() {
        let root = workspace_root();
        assert!(
            root.join("Cargo.toml").is_file(),
            "workspace root should contain Cargo.toml: {}",
            root.display()
        );
    }
}
