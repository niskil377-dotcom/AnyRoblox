use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Default, Serialize)]
pub struct Model {
    pub source: String,
    pub status: String,
    pub format: String,
    pub version: Option<u32>,
    pub vertices: usize,
    pub faces: usize,
    pub lods: Vec<f32>,
    pub selected_lod: Option<f32>,
    pub proxies: Vec<String>,
    pub animations: Vec<String>,
    pub named_selections: Vec<String>,
    pub rig: &'static str,
    pub rig_configs: Vec<crate::config::RigConfig>,
    pub materials: std::collections::BTreeMap<String, Material>,
    pub hitbox_lods: Vec<f32>,
    pub missing_textures: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct Material {
    pub diffuse: String,
    pub rvmat: String,
}

pub fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut f = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))?;
    serde_json::to_writer_pretty(f.as_file_mut(), value)?;
    f.persist_noclobber(path)?;
    Ok(())
}
