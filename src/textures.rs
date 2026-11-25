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
                ensure!(
                    paths.len() == 1,
                    "Ambiguous texture {}",
                    candidate.display()
                );
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
    TextureBatch::new(assets, tool, None).export(name, parent, out, report)
}

pub(crate) struct TextureBatch<'a> {
    assets: &'a Assets,
    tool: Option<&'a Path>,
    workspace: Option<&'a Path>,
    converted: BTreeMap<PathBuf, (PathBuf, bool)>,
}

impl<'a> TextureBatch<'a> {
    pub(crate) fn new(
        assets: &'a Assets,
        tool: Option<&'a Path>,
        workspace: Option<&'a Path>,
    ) -> Self {
        Self {
            assets,
            tool,
            workspace,
            converted: BTreeMap::new(),
        }
    }

    pub(crate) fn export(
        &mut self,
        name: &str,
        parent: &Path,
        out: &Path,
        report: &mut Model,
    ) -> Result<bool> {
        let path = safe_path(name.trim_start_matches(['\\', '/']))?;
        let png = self.assets.find(&path.with_extension("png"), parent)?;
        let source = match png {
            Some(p) => p,
            None => match self.assets.find(&path, parent)? {
                Some(p) => p,
                None => {
                    report.missing_textures.push(name.to_owned());
                    anyhow::bail!("Missing texture: {name}")
                }
            },
        };
        if let Some((cached, transparent)) = self.converted.get(source) {
            fs::copy(cached, out)?;
            return Ok(*transparent);
        }
        let cached = self
            .workspace
            .map(|dir| dir.join(format!("texture-{}.png", self.converted.len())));
        let dest = cached.as_deref().unwrap_or(out);
        let transparent = if png.is_some() {
            write_png(source, dest)?
        } else {
            ensure!(
                path.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("paa")),
                "Unsupported texture format"
            );
            convert_paa(source, self.tool, dest)?
        };
        if let Some(cached) = cached {
            fs::copy(&cached, out)?;
            self.converted
                .insert(source.to_owned(), (cached, transparent));
        }
        Ok(transparent)
    }
}

fn convert_paa(paa: &Path, tool: Option<&Path>, out: &Path) -> Result<bool> {
    let exe = tool.context("No PAA converter configured")?;
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
    ensure!(status.success(), "PAA converter: {status}");
    write_png(&png, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_reuses_only_the_same_resolved_texture() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let dir = temp.path();
        let mut assets = Assets::default();
        for (name, alpha) in [("a", 0u8), ("b", 255u8)] {
            let rel = Path::new(name).join("skin.png");
            fs::create_dir(dir.join(name))?;
            image::RgbaImage::from_pixel(1, 1, image::Rgba([20, 30, 40, alpha]))
                .save(dir.join(&rel))?;
            assets.add(&rel, &dir.join(&rel));
        }
        let mut batch = TextureBatch::new(&assets, None, Some(dir));
        let mut report = Model::default();
        let mut export = |parent: &str, output: &str| {
            batch.export(
                "skin.paa",
                Path::new(parent),
                &dir.join(output),
                &mut report,
            )
        };
        assert!(export("a", "first.png")?);
        let first = fs::read(dir.join("first.png"))?;
        fs::remove_file(dir.join("a/skin.png"))?;
        fs::write(dir.join("first.png"), b"edited output")?;
        assert!(export("a", "second.png")?);
        assert_eq!(fs::read(dir.join("second.png"))?, first);
        assert!(!export("b", "third.png")?);
        assert_ne!(fs::read(dir.join("third.png"))?, first);
        Ok(())
    }
}
