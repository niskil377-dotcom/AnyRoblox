use crate::{report::Model, safe_path};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Default)]
pub struct Assets(BTreeMap<String, Vec<PathBuf>>);
fn key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
}
impl Assets {
    pub fn add(&mut self, name: &Path, file: &Path) {
        let paths = self.0.entry(key(name)).or_default();
        if !paths.iter().any(|p| p == file) {
            paths.push(file.to_owned());
        }
    }
    fn find(&self, name: &Path, parent: &Path) -> Result<Option<&Path>> {
        for candidate in [name.to_owned(), parent.join(name)] {
            if let Some(paths) = self.0.get(&key(&candidate)) {
                ensure!(paths.len() == 1, "texture ambiguë {}", candidate.display());
                return Ok(paths.first().map(PathBuf::as_path));
            }
        }
        Ok(None)
    }
}

fn write_png(input: &Path, out: &Path) -> Result<bool> {
    let pixels = image::open(input)?.into_rgba8();
    let transparent = pixels.pixels().any(|p| p[3] != 255);
    pixels.save(out)?;
    Ok(transparent)
}

pub fn export(
    name: &str,
    parent: &Path,
    assets: &Assets,
    tool: Option<&Path>,
    out: &Path,
    report: &mut Model,
) -> Result<bool> {
    let path = safe_path(name.trim_start_matches(['\\', '/']))?;
    let fallback = assets.find(&path.with_extension("png"), parent)?;
    if let Some(png) = fallback {
        return write_png(png, out);
    }
    let Some(paa) = assets.find(&path, parent)? else {
        report.missing_textures.push(name.to_owned());
        anyhow::bail!("texture absente : {name}")
    };
    ensure!(
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("paa")),
        "format de texture non pris en charge"
    );
    let exe = tool.context("aucun convertisseur PAA configuré")?;
    let tmp = tempfile::tempdir()?;
    let input = tmp.path().join("input.paa");
    let png = tmp.path().join("output.png");
    fs::copy(paa, &input)?;
    let mut command = Command::new(exe);
    if exe
        .file_stem()
        .is_some_and(|s| s.eq_ignore_ascii_case("hemtt"))
    {
        command.args(["utils", "paa", "convert"]);
    }
    let status = command
        .current_dir(tmp.path())
        .arg(input)
        .arg(&png)
        .status()?;
    ensure!(status.success(), "convertisseur PAA : {status}");
    write_png(&png, out)
}
