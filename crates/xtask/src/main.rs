//! `cargo xtask` — build orchestrator for SpotledgerCore.
//!
//! Commands:
//!   cargo xtask build            — host workspace (debug) + all WASM apps (debug)
//!   cargo xtask build --release  — both in release mode
//!   cargo xtask wasm             — WASM apps only
//!   cargo xtask host             — host workspace only
//!   cargo xtask check            — cargo check --workspace (fast, no WASM)
//!
//! After building, WASM files are copied to `target/plugins/` so the server
//! can load them from a single, predictable location.
//!
//! The WASM target used is `wasm32-wasip1`.  If it is not installed you will
//! get a clear error; install with:
//!   rustup target add wasm32-wasip1

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

// ── WASM apps — any sub-directory of `apps/` that contains a Cargo.toml ────
const WASM_TARGET: &str = "wasm32-wasip1";

fn main() -> ExitCode {
    let mut args = env::args().skip(1); // skip "xtask"
    let cmd = args.next().unwrap_or_else(|| "build".to_owned());
    let release = args.any(|a| a == "--release");

    let workspace_root = workspace_root();

    match cmd.as_str() {
        "build" => {
            if !build_wasm_apps(&workspace_root, release) { return ExitCode::FAILURE; }
            if !build_host(&workspace_root, release) { return ExitCode::FAILURE; }
        }
        "wasm" => {
            if !build_wasm_apps(&workspace_root, release) { return ExitCode::FAILURE; }
        }
        "host" => {
            if !build_host(&workspace_root, release) { return ExitCode::FAILURE; }
        }
        "check" => {
            if !run_cargo(&workspace_root, &["check", "--workspace"]) { return ExitCode::FAILURE; }
        }
        other => {
            eprintln!("Unknown command: {other}");
            eprintln!("Usage: cargo xtask [build|wasm|host|check] [--release]");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

// ─────────────────────────────────────────────────────────────────────────────

fn build_host(root: &Path, release: bool) -> bool {
    println!("\n==> Building host workspace …");
    let mut args = vec!["build", "--workspace",
                        "--exclude", "xtask"];   // xtask doesn't need to rebuild itself
    if release { args.push("--release"); }
    run_cargo(root, &args)
}

fn build_wasm_apps(root: &Path, release: bool) -> bool {
    let apps_dir = root.join("apps");
    let Ok(entries) = fs::read_dir(&apps_dir) else {
        println!("No apps/ directory found — skipping WASM build.");
        return true;
    };

    let mut any = false;
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.exists() { continue; }

        let app_name = entry.file_name().to_string_lossy().to_string();
        println!("\n==> Building WASM app: {app_name}");
        any = true;

        let manifest_str = manifest.to_string_lossy().to_string();
        let mut args = vec![
            "build",
            "--manifest-path", &manifest_str,
            "--target", WASM_TARGET,
        ];
        if release { args.push("--release"); }

        if !run_cargo(root, &args) {
            return false;
        }

        // Copy wasm from app-local target/ → workspace target/plugins/
        // Each app has its own target/ because it's outside the workspace.
        let profile = if release { "release" } else { "debug" };
        let app_wasm = entry.path()
            .join("target")
            .join(WASM_TARGET)
            .join(profile)
            .join(format!("{}.wasm", app_name));
        if app_wasm.exists() {
            let dst_dir = root.join("target").join("plugins");
            fs::create_dir_all(&dst_dir).ok();
            let dst = dst_dir.join(format!("{}.wasm", app_name));
            match fs::copy(&app_wasm, &dst) {
                Ok(_) => println!("  → target/plugins/{}.wasm", app_name),
                Err(e) => eprintln!("  WARN: copy failed: {e}"),
            }
        }
    }

    if !any {
        println!("No apps with Cargo.toml found in apps/ — skipping WASM build.");
    }
    true
}

// ─────────────────────────────────────────────────────────────────────────────

fn run_cargo(root: &Path, args: &[&str]) -> bool {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(&cargo)
        .args(args)
        .current_dir(root)
        .status();
    match status {
        Ok(s) if s.success() => true,
        Ok(s) => {
            eprintln!("cargo {} failed with code {:?}", args[0], s.code());
            false
        }
        Err(e) => {
            eprintln!("Failed to run cargo: {e}");
            false
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../crates/xtask   →   ../../  = workspace root
    // Use canonicalize to resolve symlinks, then strip UNC prefix if present
    // so Cargo doesn't reject \\?\ prefixed paths on Windows.
    let raw = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("Cannot resolve workspace root");
    // Strip \\?\ UNC prefix that canonicalize adds on Windows
    let s = raw.to_string_lossy();
    let s = s.strip_prefix("\\\\?\\").unwrap_or(&s);
    PathBuf::from(s)
}
