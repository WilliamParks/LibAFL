//! This is a 'meta-package' for libafl that exposes a consistent URL and commit hash for the
//! [`SymCC` fork](https://github.com/AFLplusplus/symcc).
#![allow(clippy::module_name_repetitions)]

/// The URL of the `LibAFL` `SymCC` fork.
pub const SYMCC_REPO_URL: &str = "https://github.com/AFLplusplus/symcc.git";
/// The commit of the `LibAFL` `SymCC` fork.
pub const SYMCC_REPO_COMMIT: &str = "d3870f3f6e4b8d1c01516e76537547d980b3cbed";

#[cfg(feature = "clone")]
mod clone {
    use std::{
        io::{stdout, Write},
        path::Path,
        process::Command,
    };

    use which::which;

    use crate::{SYMCC_REPO_COMMIT, SYMCC_REPO_URL};

    /// Checks out the repository into the given directory with the given URL and commit hash.
    /// Any errors will trigger a panic.
    pub fn clone_symcc_at_version(path: &Path, url: &str, commit: &str) {
        assert!(
            which("git").is_ok(),
            "ERROR: unable to find git. Git is required to download SymCC."
        );

        let mut cmd = Command::new("git");
        cmd.arg("clone").arg(url).arg(path);
        let output = cmd.output().expect("failed to execute git clone");
        if output.status.success() {
            let mut cmd = Command::new("git");
            cmd.arg("checkout").arg(commit).current_dir(path);
            let output = cmd.output().expect("failed to execute git checkout");
            if !output.status.success() {
                eprintln!("failed to checkout symcc git repository commit:");
                let mut stdout = stdout();
                stdout
                    .write_all(&output.stderr)
                    .expect("failed to write git error message to stdout");
                panic!();
            }
        } else {
            eprintln!("failed to clone symcc git repository:");
            let mut stdout = stdout();
            stdout
                .write_all(&output.stderr)
                .expect("failed to write git error message to stdout");
            panic!();
        }
    }

    /// Checks out the repository into the given directory.
    /// Any errors will trigger a panic.
    pub fn clone_symcc(path: &Path) {
        clone_symcc_at_version(path, SYMCC_REPO_URL, SYMCC_REPO_COMMIT);
    }
}

#[cfg(feature = "clone")]
pub use clone::clone_symcc;

#[cfg(feature = "build")]
mod build {
    use std::path::{Path, PathBuf};

    /// Builds `SymCC` at the given directory using [`cmake`](https://crates.io/crates/cmake).
    /// Returns the build artifact directory.
    #[must_use]
    pub fn build_symcc(path: &Path) -> PathBuf {
        use cmake::Config;

        Config::new(path)
            .define("Z3_TRUST_SYSTEM_VERSION", "ON")
            .no_build_target(true)
            .build()
            .join("build")
    }
}

#[cfg(feature = "build")]
pub use build::build_symcc;
