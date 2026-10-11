//! Zip backup / restore of an instance game directory.
use crate::instance::{self, Instance};
use crate::paths::Paths;
use crate::tasks::Task;
use crate::util;
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

const SKIP: &[&str] = &[".cairn/logs", "crash-reports", "logs", "natives", ".DS_Store"];

fn should_skip(rel: &str) -> bool {
    SKIP.iter().any(|s| rel == *s || rel.starts_with(&format!("{s}/")))
}

pub fn backup(paths: &Paths, id: &str, dest: &Path, task: &Task) -> Result<PathBuf> {
    let inst = instance::get(paths, id)?;
    let src = inst.dir(paths);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    task.stage("Zipping instance", 0, 0);
    let file = File::create(dest).with_context(|| format!("creating {}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let meta = serde_json::json!({
        "cairnBackup": 1,
        "instance": inst,
    });
    zip.start_file("cairn-backup.json", opts)?;
    zip.write_all(serde_json::to_string_pretty(&meta)?.as_bytes())?;

    let mut files: Vec<PathBuf> = vec![];
    collect_files(&src, &src, &mut files)?;
    let total = files.len() as u64;
    for (i, path) in files.iter().enumerate() {
        let rel = path.strip_prefix(&src)?.to_string_lossy().replace('\\', "/");
        if should_skip(&rel) {
            continue;
        }
        task.stage("Zipping instance", i as u64, total);
        if path.is_dir() {
            continue;
        }
        zip.start_file(&rel, opts)?;
        let mut f = File::open(path)?;
        std::io::copy(&mut f, &mut zip)?;
    }
    zip.finish()?;
    task.stage("Zipping instance", total, total);
    Ok(dest.to_path_buf())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let rd = std::fs::read_dir(dir)?;
    for e in rd.flatten() {
        let p = e.path();
        let rel = p.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
        if should_skip(&rel) {
            continue;
        }
        if p.is_dir() {
            collect_files(root, &p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

pub fn restore(paths: &Paths, zip_path: &Path, task: &Task) -> Result<Instance> {
    task.stage("Reading backup", 0, 0);
    let file = File::open(zip_path).context("opening backup zip")?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut meta_bytes = Vec::new();
    {
        let mut entry = archive.by_name("cairn-backup.json").context("not a Cairn backup (missing cairn-backup.json)")?;
        entry.read_to_end(&mut meta_bytes)?;
    }
    let meta: serde_json::Value = serde_json::from_slice(&meta_bytes)?;
    let mut inst: Instance = serde_json::from_value(meta["instance"].clone()).context("backup instance metadata")?;
    if inst.name.trim().is_empty() {
        bail!("Backup has no instance name");
    }
    inst.id = String::new();
    inst = instance::create(paths, inst)?;
    let dest = inst.dir(paths);

    let total = archive.len() as u64;
    for i in 0..archive.len() {
        task.stage("Restoring files", i as u64, total);
        let mut entry = archive.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        if name == "cairn-backup.json" || name.ends_with('/') {
            continue;
        }
        if name.contains("..") {
            bail!("refusing unsafe path in backup: {name}");
        }
        let out = dest.join(&name);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
    }
    // Keep restored instance.json in sync with our new id/name.
    instance::save(paths, &inst)?;
    // Re-read in case zip overwrote instance.json with the old id.
    if let Ok(mut disk) = util::read_json::<Instance>(&dest.join("instance.json")) {
        disk.id = inst.id.clone();
        disk.name = inst.name.clone();
        instance::save(paths, &disk)?;
        inst = disk;
    }
    Ok(inst)
}
