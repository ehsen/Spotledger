//! `spotledger pack-app` — package a DB-native app directory into a `.slpkg` archive.
//!
//! A `.slpkg` file is a gzip-compressed tar archive of the app directory.
//! The archive preserves the directory structure starting from `<app-name>/`
//! at the root so that `install_app` can unpack it and find `app.json`.
//!
//! ## Usage
//! ```
//! spotledger pack-app apps/my-app               # → my-app-1.0.0.slpkg (in cwd)
//! spotledger pack-app apps/my-app -o dist/      # → dist/my-app-1.0.0.slpkg
//! spotledger pack-app apps/my-app -o out.slpkg  # → out.slpkg (explicit name)
//! ```

use anyhow::{bail, Context, Result};
use flate2::{write::GzEncoder, Compression};
use std::path::{Path, PathBuf};

use crate::cli::PackAppArgs;

pub async fn pack_app(args: PackAppArgs) -> Result<()> {
    let app_dir = args
        .app_dir
        .canonicalize()
        .with_context(|| format!("App directory not found: {}", args.app_dir.display()))?;

    // ── 1. Read app.json ──────────────────────────────────────────────────────
    let manifest_path = app_dir.join("app.json");
    if !manifest_path.exists() {
        bail!("No app.json found in {}", app_dir.display());
    }
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Reading {}", manifest_path.display()))?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).context("Parsing app.json")?;

    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| app_dir.file_name().and_then(|n| n.to_str()).unwrap_or("app"));
    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0");

    // ── 2. Resolve output path ────────────────────────────────────────────────
    let archive_name = format!("{name}-{version}.slpkg");
    let output_path: PathBuf = match args.output {
        Some(ref p) if p.is_dir() => p.join(&archive_name),
        Some(ref p) => p.clone(),
        None => std::env::current_dir()?.join(&archive_name),
    };

    println!("Packaging '{}' v{} …", name, version);
    println!("  Source : {}", app_dir.display());
    println!("  Output : {}", output_path.display());

    // ── 3. Create the .slpkg (gzip'd tar) ────────────────────────────────────
    let file = std::fs::File::create(&output_path)
        .with_context(|| format!("Creating {}", output_path.display()))?;

    let gz = GzEncoder::new(file, Compression::best());
    let mut archive = tar::Builder::new(gz);

    // The archive root is the app directory name (e.g. "my-app/").
    // We walk the directory and add each entry with a path relative to the
    // parent of app_dir so that the archive starts with <app-name>/.
    let parent = app_dir
        .parent()
        .context("App dir has no parent")?;

    append_dir_recursive(&mut archive, &app_dir, parent)
        .context("Building archive")?;

    // Finish writing the archive
    let gz = archive.into_inner().context("Finalising tar archive")?;
    gz.finish().context("Finishing gzip stream")?;

    let meta = std::fs::metadata(&output_path)?;
    let size_kb = meta.len() / 1024;
    println!(
        "\n✓  Packed {} ({} KB)",
        output_path.display(),
        size_kb
    );
    println!("\nInstall with:");
    println!(
        "  spotledger install-app {} --site <site>",
        output_path.display()
    );
    Ok(())
}

/// Recursively append all files in `dir` to the archive.
/// Paths in the archive are relative to `base` so that the archive starts
/// with the app directory name (e.g. `my-app/`).
fn append_dir_recursive(
    archive: &mut tar::Builder<flate2::write::GzEncoder<std::fs::File>>,
    dir: &Path,
    base: &Path,
) -> Result<()> {
    for entry in walkdir(dir)? {
        let entry = entry?;
        let full_path = entry.path();

        if full_path.is_dir() {
            continue; // tar automatically creates parent dirs
        }

        // Strip the base prefix to get the archive-relative path
        let rel = full_path
            .strip_prefix(base)
            .with_context(|| format!("Stripping prefix from {}", full_path.display()))?;

        archive
            .append_path_with_name(full_path, rel)
            .with_context(|| format!("Adding {} to archive", full_path.display()))?;
    }
    Ok(())
}

/// Simple recursive directory walker that returns all entries.
fn walkdir(dir: &Path) -> Result<Vec<Result<walkdir_entry::Entry>>> {
    let mut entries = Vec::new();
    collect_entries(dir, &mut entries)?;
    Ok(entries.into_iter().map(Ok).collect())
}

fn collect_entries(dir: &Path, out: &mut Vec<walkdir_entry::Entry>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Reading dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_entries(&path, out)?;
        } else {
            out.push(walkdir_entry::Entry { path });
        }
    }
    Ok(())
}

mod walkdir_entry {
    use std::path::PathBuf;

    pub struct Entry {
        pub path: PathBuf,
    }

    impl Entry {
        pub fn path(&self) -> &std::path::Path {
            &self.path
        }
    }
}
