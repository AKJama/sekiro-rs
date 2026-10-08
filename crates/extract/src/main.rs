//! `sekiro-extract`: reads the player's Sekiro install and writes derived data to `cache/`.
//! The install is only ever read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use sekiro_formats::{bnd4, dvd::Vfs};

const DEFAULT_GAME: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Sekiro";

#[derive(Parser)]
#[command(about = "Extract data from your own Sekiro install into cache/")]
struct Cli {
    /// Sekiro install folder (read only).
    #[arg(long, default_value = DEFAULT_GAME)]
    game: PathBuf,
    /// Output cache folder.
    #[arg(long, default_value = "cache")]
    cache: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Unpack every archive entry to cache/raw, decompressed, with binders expanded alongside.
    Unpack {
        /// Only unpack paths containing this substring.
        #[arg(long)]
        filter: Option<String>,
    },
    /// List known archive paths.
    List {
        #[arg(long)]
        filter: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let vfs = open_vfs(&cli.game, &cli.cache)?;
    match cli.command {
        Command::List { filter } => {
            let mut paths: Vec<_> = vfs
                .paths()
                .filter(|p| filter.as_deref().is_none_or(|f| p.contains(f)))
                .collect();
            paths.sort_unstable();
            for p in &paths {
                println!("{p}");
            }
            eprintln!("{} of {} entries named", paths.len(), vfs.len());
        }
        Command::Unpack { filter } => unpack(&vfs, &cli.cache.join("raw"), filter.as_deref())?,
    }
    Ok(())
}

fn open_vfs(game: &Path, cache: &Path) -> Result<Vfs> {
    let refs = cache.join("refs");
    let keys_src = std::fs::read_to_string(refs.join("ArchiveKeys.cs"))
        .context("cache/refs/ArchiveKeys.cs missing: run tools/fetch-refs.ps1")?;
    let keys: HashMap<_, _> = sekiro_formats::dvd::parse_uxm_keys(&keys_src);
    let dictionary: Vec<String> = std::fs::read_to_string(refs.join("SekiroDictionary.txt"))?
        .lines()
        .map(|l| l.trim().to_string())
        .collect();
    Ok(Vfs::open(game, &keys, &dictionary)?)
}

fn unpack(vfs: &Vfs, out: &Path, filter: Option<&str>) -> Result<()> {
    let mut paths: Vec<String> = vfs
        .paths()
        .filter(|p| filter.is_none_or(|f| p.contains(f)))
        .map(str::to_string)
        .collect();
    paths.sort_unstable();
    let done = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let total = paths.len();
    paths.par_iter().for_each(|path| {
        if let Err(e) = unpack_one(vfs, out, path) {
            failed.fetch_add(1, Ordering::Relaxed);
            eprintln!("FAILED {path}: {e:#}");
        }
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_multiple_of(500) || n == total {
            eprintln!("{n}/{total}");
        }
    });
    let unnamed = vfs.len() - vfs.paths().count();
    eprintln!(
        "unpacked {} files ({} failed); {unnamed} archive entries have no known name",
        total,
        failed.load(Ordering::Relaxed)
    );
    Ok(())
}

fn unpack_one(vfs: &Vfs, out: &Path, path: &str) -> Result<()> {
    let data = vfs.read(path)?;
    let rel = path.trim_start_matches('/');
    let rel = rel.strip_suffix(".dcx").unwrap_or(rel);
    let dest = out.join(rel);
    std::fs::create_dir_all(dest.parent().unwrap())?;
    if bnd4::is_bnd4(&data) {
        expand_binder(
            vfs,
            &data,
            &dest.with_extension(format!(
                "{}.d",
                dest.extension().and_then(|e| e.to_str()).unwrap_or("")
            )),
        )?;
    }
    std::fs::write(&dest, &data)?;
    Ok(())
}

/// Writes each binder member into `dir`, recursing into nested binders.
fn expand_binder(vfs: &Vfs, data: &[u8], dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for file in bnd4::parse(data)? {
        let contents = file.contents(vfs.oodle())?;
        let name = file.file_name();
        let name = if name.is_empty() {
            format!("{}", file.id)
        } else {
            name.to_string()
        };
        let dest = dir.join(name.strip_suffix(".dcx").unwrap_or(&name));
        if bnd4::is_bnd4(&contents) {
            expand_binder(
                vfs,
                &contents,
                &dest.with_extension(format!(
                    "{}.d",
                    dest.extension().and_then(|e| e.to_str()).unwrap_or("")
                )),
            )?;
        }
        std::fs::write(&dest, &contents)?;
    }
    Ok(())
}
