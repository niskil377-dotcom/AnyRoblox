#![forbid(unsafe_code)]
mod binary;
mod config;
pub mod obj;
pub mod p3d;
pub mod pbo;
pub mod report;
pub mod textures;
pub mod workshop;

use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use textures::Assets;

pub const LIMIT: u64 = 512 * 1024 * 1024;

pub fn read_limited(path: &Path) -> Result<Vec<u8>> {
    let f = File::open(path)?;
    ensure!(f.metadata()?.len() <= LIMIT, "File exceeds 512 MiB");
    let mut bytes = Vec::new();
    f.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "File exceeds 512 MiB");
    Ok(bytes)
}

pub fn safe_path(raw: &str) -> Result<PathBuf> {
    let mut path = PathBuf::new();
    for part in raw.split(['\\', '/']) {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let device = matches!(
            base.as_str(),
            "CON" | "AUX" | "PRN" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.chars().count() == 4);
        ensure!(
            !part.is_empty()
                && !part.ends_with(['.', ' '])
                && !part.contains([':', '<', '>', '"', '|', '?', '*'])
                && !part.chars().any(char::is_control)
                && !device,
            "Invalid archive path : {raw:?}"
        );
        path.push(part);
    }
    Ok(path)
}

fn ext(p: &Path, s: &str) -> bool {
    p.extension().is_some_and(|x| x.eq_ignore_ascii_case(s))
}
fn files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![dir.to_owned()];
    let mut paths = Vec::new();
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                stack.push(entry.path());
            } else if ty.is_file() {
                paths.push(entry.path());
            }
        }
    }
    paths.sort();
    Ok(paths)
}
fn new_file(dir: &Path, rel: &Path) -> Result<File> {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().context("Missing parent directory")?)?;
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}
fn unzip(bytes: &[u8], dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    ensure!(zip.len() <= 100_000, "Too many ZIP entries");
    let mut remaining = 2 * 1024 * 1024 * 1024u64;
    for i in 0..zip.len() {
        let item = zip.by_index(i)?;
        let name = safe_path(item.name().trim_end_matches('/'))?;
        if item.is_dir() {
            continue;
        }
        let size = item.size();
        ensure!(
            size <= LIMIT && size <= remaining,
            "ZIP extraction limit exceeded"
        );
        let mut out = new_file(dir, &name)?;
        ensure!(
            std::io::copy(&mut item.take(size + 1), &mut out)? == size,
            "ZIP size mismatch"
        );
        remaining -= size;
    }
    Ok(())
}

#[derive(Default, Serialize)]
pub struct Run {
    pub workshop: Option<workshop::Details>,
    pub cached_file: Option<PathBuf>,
    pub format: String,
    pub archives: Vec<Inventory>,
    pub models: Vec<(String, String)>,
    pub errors: Vec<String>,
}
#[derive(Serialize)]
pub struct Inventory {
    source: String,
    prefix: Option<PathBuf>,
    files: Vec<PathBuf>,
}
pub struct Options {
    pub input: Option<PathBuf>,
    pub workshop: Option<String>,
    pub output: PathBuf,
    pub cache: PathBuf,
    pub debinarizer: Option<PathBuf>,
    pub paa_tool: Option<PathBuf>,
    pub versions: Vec<u32>,
}

#[derive(Default)]
struct Batch {
    assets: Assets,
    configs: Vec<(PathBuf, String)>,
    models: Vec<ModelSource>,
    extracted_bytes: u64,
}
struct ModelSource {
    file: PathBuf,
    virtual_path: PathBuf,
    source: String,
    error: Option<String>,
}
impl Batch {
    fn add(&mut self, file: &Path, virtual_path: &Path, source: String) {
        if ext(file, "p3d") {
            self.models.push(ModelSource {
                file: file.to_owned(),
                virtual_path: virtual_path.to_owned(),
                source,
                error: None,
            });
        } else if ext(file, "paa") || ext(file, "png") {
            self.assets.add(virtual_path, file);
        } else if file
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("model.cfg"))
        {
            self.configs.push((file.to_owned(), source));
        }
    }
    fn archive(&mut self, bytes: &[u8], source: &Path, work: &Path, run: &mut Run) -> Result<()> {
        let archive = pbo::parse(bytes)?;
        let dir = work.join(format!("pbo-{}", run.archives.len()));
        fs::create_dir(&dir)?;
        let prefix = archive
            .prefix
            .clone()
            .unwrap_or_else(|| PathBuf::from(source.file_stem().unwrap_or_default()));
        run.archives.push(Inventory {
            source: source.display().to_string(),
            prefix: archive.prefix,
            files: archive.files.iter().map(|f| f.path.clone()).collect(),
        });
        for entry in archive.files {
            let virtual_path = prefix.join(&entry.path);
            let display = format!("{}!{}", source.display(), entry.path.display());
            let file = dir.join(&entry.path);
            let extracted = (|| -> Result<()> {
                let size = if entry.method == 0 {
                    entry.data.len() as u64
                } else {
                    u64::from(entry.original)
                };
                ensure!(
                    self.extracted_bytes + size <= 2 * 1024 * 1024 * 1024,
                    "Batch exceeds the 2 GiB PBO limit"
                );
                let contents = entry.contents()?;
                new_file(&dir, &entry.path)?.write_all(&contents)?;
                self.extracted_bytes += size;
                Ok(())
            })();
            match extracted {
                Ok(()) => self.add(&file, &virtual_path, display),
                Err(e) => {
                    run.errors.push(format!("{display}: {e:#}"));
                    if ext(&file, "p3d") {
                        self.models.push(ModelSource {
                            file,
                            virtual_path,
                            source: display,
                            error: Some(format!("{e:#}")),
                        });
                    }
                }
            }
        }
        Ok(())
    }
    fn folder(&mut self, root: &Path, work: &Path, run: &mut Run) -> Result<()> {
        let paths = files(root)?;
        let mut prefixes = std::collections::BTreeMap::new();
        for marker in &paths {
            if marker.file_name().is_some_and(|n| {
                n.eq_ignore_ascii_case("$PBOPREFIX$") || n.eq_ignore_ascii_case("$PREFIX$")
            }) {
                let prefix = (|| -> Result<PathBuf> {
                    let f = File::open(marker)?;
                    let mut s = String::new();
                    f.take(4097).read_to_string(&mut s)?;
                    ensure!(s.len() < 4097, "Prefix is too long");
                    safe_path(s.lines().next().unwrap_or("").trim())
                })();
                match prefix {
                    Ok(prefix) => {
                        prefixes
                            .insert(marker.parent().context("Missing parent directory")?.to_owned(), prefix);
                    }
                    Err(e) => run.errors.push(format!("{}: {e:#}", marker.display())),
                }
            }
        }
        for file in paths {
            let rel = file.strip_prefix(root)?;
            if ext(&file, "pbo") {
                if let Err(e) = read_limited(&file).and_then(|b| self.archive(&b, rel, work, run)) {
                    run.errors.push(format!("{}: {e:#}", rel.display()));
                }
            } else {
                let virtual_path = file.ancestors().find_map(|ancestor| {
                    prefixes
                        .get(ancestor)
                        .and_then(|p| file.strip_prefix(ancestor).ok().map(|r| p.join(r)))
                });
                self.add(
                    &file,
                    virtual_path.as_deref().unwrap_or(rel),
                    rel.display().to_string(),
                );
            }
        }
        Ok(())
    }
}
fn pipeline(opt: &Options, run: &mut Run) -> Result<()> {
    let work = tempfile::tempdir()?;
    let path = match (&opt.input, &opt.workshop) {
        (Some(p), None) => p.clone(),
        (None, Some(s)) => {
            let path = workshop::fetch(s, &opt.cache, &mut run.workshop)?;
            run.cached_file = Some(path.clone());
            path
        }
        _ => bail!("Select one input source"),
    };
    let mut batch = Batch::default();
    if path.is_dir() {
        run.format = "dossier".into();
        batch.folder(&path, work.path(), run)?;
    } else {
        let data = read_limited(&path)?;
        if data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") {
            run.format = "zip".into();
            let dir = work.path().join("zip");
            fs::create_dir(&dir)?;
            unzip(&data, &dir)?;
            batch.folder(&dir, work.path(), run)?;
        } else {
            for (magic, format) in [
                (b"7z\xbc\xaf\x27\x1c".as_slice(), "7z"),
                (b"Rar!\x1a\x07", "rar"),
                (b"\x1f\x8b", "gzip"),
            ] {
                if data.starts_with(magic) {
                    run.format = format.to_owned();
                    bail!("Unsupported {format} archive. Extract locally and use --input");
                }
            }
            run.format = "inconnu".into();
            let source = path.file_name().context("Missing name")?;
            batch
                .archive(&data, Path::new(source), work.path(), run)
                .with_context(|| {
                    format!(
                        "Unknown format or invalid PBO (first bytes {:02x?})",
                        &data[..data.len().min(8)]
                    )
                })?;
            run.format = "pbo".into();
        }
    }
    ensure!(!batch.models.is_empty(), "No P3D models found");
    for (i, item) in batch.models.iter().enumerate() {
        let ModelSource {
            file,
            virtual_path: vpath,
            source,
            error,
        } = item;
        let name = format!(
            "{:04}-{}",
            i + 1,
            file.file_stem().unwrap_or_default().to_string_lossy()
        );
        let dir = opt.output.join(&name);
        let mut model = report::Model {
            source: source.clone(),
            rig: "Skeleton not interpreted. Observed MLOD selections are listed under named_selections.",
            rig_configs: batch.configs.iter().filter(|(cfg,_)|cfg.parent().is_some_and(|p|file.starts_with(p)))
                .map(|(cfg,name)|config::inventory(cfg,name.clone())).collect(),
            ..Default::default()
        };
        let result = (|| -> Result<()> {
            fs::create_dir(&dir)?;
            if let Some(message) = error {
                bail!("PBO extraction: {message}")
            }
            let mesh = p3d::load(file, opt.debinarizer.as_deref(), &opt.versions, &mut model)?;
            obj::export(
                &mesh,
                &dir,
                vpath.parent().unwrap_or(Path::new("")),
                &batch.assets,
                opt.paa_tool.as_deref(),
                &mut model,
            )
        })();
        model.status = match result {
            Err(e) => {
                model.errors.push(format!("{e:#}"));
                "échec"
            }
            Ok(()) if !model.errors.is_empty() => "partiel",
            Ok(()) => "ok",
        }
        .into();
        if let Err(e) = report::save(&dir.join("report.json"), &model) {
            run.errors.push(format!("{source}: {e:#}"));
        }
        run.models.push((name, model.status));
    }
    Ok(())
}

/// Resolve existing ancestors before creating a new destination.
fn destination(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let abs = std::path::absolute(path)?;
    let parent = abs.parent().context("Invalid output path")?;
    Ok(destination(parent)?.join(abs.file_name().context("Missing output name")?))
}

pub fn run(opt: &Options) -> Result<bool> {
    let out = destination(&opt.output)?;
    if let Some(input) = &opt.input {
        ensure!(
            !out.starts_with(input.canonicalize()?),
            "Choose an output folder outside the input"
        );
    } else {
        let cache = destination(&opt.cache)?;
        ensure!(
            !out.starts_with(&cache) && !cache.starts_with(&out),
            "Output and cache must use separate folders"
        );
    }
    fs::create_dir_all(&out)?;
    ensure!(
        fs::read_dir(&out)?.next().is_none(),
        "Choose an empty output folder"
    );
    let lock_path = out.join(".arma2obj-running");
    let lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)?;
    let _guard = tempfile::TempPath::try_from_path(&lock_path)?;
    drop(lock);
    let mut run = Run {
        format: "non déterminé".into(),
        ..Default::default()
    };
    if let Err(e) = pipeline(opt, &mut run) {
        run.errors.push(format!("{e:#}"));
    }
    report::save(&out.join("report.json"), &run)?;
    for error in &run.errors {
        eprintln!("{error}");
    }
    for (name, status) in &run.models {
        println!("{name}  {status}");
    }
    println!("{}", out.join("report.json").display());
    Ok(run.errors.is_empty() && run.models.iter().all(|(_, s)| s == "ok"))
}
