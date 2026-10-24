//! Build script for `chickenscratch-core`.
//!
//! Windows only: link `advapi32` explicitly. `libgit2-sys 0.17` (via `git2
//! 0.19`) calls the registry (`RegOpenKeyExW`), CryptoAPI (`Crypt*`) and
//! `GetNamedSecurityInfoW` from its C sources without declaring the import
//! library; it used to link only because Rust's standard library pulled
//! `advapi32.lib` in for its own use. Rust 1.99 switched std to raw-dylib
//! bindings for that DLL, so every binary that links this crate (the core
//! test runner first) failed with `LNK1120: 11 unresolved externals` on the
//! Windows CI job (run 40071873531, 2026-10-24). Upgrading `git2` is the
//! long-term fix; this keeps the pinned dependency building meanwhile.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
