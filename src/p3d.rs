use crate::{binary::Reader, report::Model};
use anyhow::{bail, ensure, Context, Result};
use std::{collections::BTreeSet, fs, path::Path, process::Command};

pub type Vec3 = [f32; 3];
pub struct Corner {
    pub vertex: usize,
    pub normal: usize,
    pub uv: [f32; 2],
}
pub struct Face {
    pub corners: Vec<Corner>,
    pub material: (String, String),
}
#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub faces: Vec<Face>,
}

fn read_lod(r: &mut Reader, report: &mut Model) -> Result<(f32, Mesh)> {
    ensure!(
        r.take(4)? == b"P3DM" && r.word()? == 28 && r.word()? == 256,
        "LOD P3DM 28/256 attendu"
    );
    let nv = r.word()? as usize;
    let nn = r.word()? as usize;
    let nf = r.word()? as usize;
    r.word()?;
    ensure!(
        nv as u64 * 16 + nn as u64 * 12 + nf as u64 * 74 <= r.0.len() as u64,
        "tailles P3DM invalides"
    );
    let mut mesh = Mesh::default();
    for _ in 0..nv {
        mesh.vertices.push([r.float()?, r.float()?, -r.float()?]);
        r.word()?;
    }
    for _ in 0..nn {
        mesh.normals.push([-r.float()?, -r.float()?, r.float()?]);
    }
    for _ in 0..nf {
        let count = r.word()?;
        ensure!(matches!(count, 3 | 4), "face à {count} côtés");
        let mut corners = Vec::new();
        for _ in 0..count {
            let vertex = r.word()? as usize;
            let normal = r.word()? as usize;
            ensure!(
                vertex < nv && normal < nn,
                "indice de point ou normale hors limites"
            );
            corners.push(Corner {
                vertex,
                normal,
                uv: [r.float()?, 1.0 - r.float()?],
            });
        }
        if count == 3 {
            r.take(16)?;
        }
        r.word()?;
        mesh.faces.push(Face {
            corners,
            material: (r.string()?, r.string()?),
        });
    }
    ensure!(r.take(4)? == b"TAGG", "section TAGG absente");
    let mut proxy_vertices = BTreeSet::new();
    let mut proxy_faces = BTreeSet::new();
    loop {
        let active = r.take(1)?[0] != 0;
        let name = r.string()?;
        let len = r.word()? as usize;
        let data = r.take(len)?;
        if name == "#EndOfFile#" {
            ensure!(len == 0, "fin TAGG incorrecte");
            break;
        }
        if !active {
            continue;
        }
        if name.to_ascii_lowercase().starts_with("proxy:") {
            ensure!(len == nv + nf, "sélection proxy invalide");
            report.proxies.push(name);
            proxy_vertices.extend((0..nv).filter(|&i| data[i] != 0));
            proxy_faces.extend((0..nf).filter(|&i| data[nv + i] != 0));
        } else if name == "#Animation#" {
            let time = Reader(data).float()?;
            report.animations.push(format!("keyframe {time}"));
        } else if !name.starts_with('#') {
            report.named_selections.push(name);
        }
    }
    if !proxy_vertices.is_empty() || !proxy_faces.is_empty() {
        let mut idx = 0;
        mesh.faces.retain(|face| {
            let keep = !proxy_faces.contains(&idx)
                && !face
                    .corners
                    .iter()
                    .all(|c| proxy_vertices.contains(&c.vertex));
            idx += 1;
            keep
        });
        for face in &mesh.faces {
            for c in &face.corners {
                proxy_vertices.remove(&c.vertex);
            }
        }
        let mut map = vec![0; nv];
        let mut vertices = Vec::new();
        for (i, v) in mesh.vertices.into_iter().enumerate() {
            if !proxy_vertices.contains(&i) {
                map[i] = vertices.len();
                vertices.push(v);
            }
        }
        mesh.vertices = vertices;
        for face in &mut mesh.faces {
            for c in &mut face.corners {
                c.vertex = map[c.vertex];
            }
        }
    }
    Ok((r.float()?, mesh))
}

pub fn read_mlod(bytes: &[u8], report: &mut Model) -> Result<Mesh> {
    let mut r = Reader(bytes);
    ensure!(
        r.take(4)? == b"MLOD" && r.word()? == 257,
        "MLOD version 257 attendu"
    );
    let count = r.word()?;
    ensure!(count > 0 && count <= 1024, "nombre de LOD invalide");
    let mut best = None;
    for i in 0..count {
        let (resolution, mesh) = read_lod(&mut r, report).with_context(|| format!("LOD {i}"))?;
        report.lods.push(resolution);
        if resolution == 5e15 {
            report.hitbox_lods.push(resolution);
        }
        if (0.0..1000.0).contains(&resolution) && report.selected_lod.is_none_or(|v| resolution < v)
        {
            report.selected_lod = Some(resolution);
            best = Some(mesh);
        }
    }
    ensure!(r.0.is_empty(), "données après le dernier LOD");
    best.context("aucun LOD visuel")
}

pub fn load(
    path: &Path,
    tool: Option<&Path>,
    versions: &[u32],
    report: &mut Model,
) -> Result<Mesh> {
    let bytes = crate::read_limited(path)?;
    let mut r = Reader(&bytes);
    let signature = r.take(4)?;
    report.format = String::from_utf8_lossy(signature).into_owned();
    let version = r.word()?;
    report.version = Some(version);
    match signature {
        b"MLOD" => read_mlod(&bytes, report),
        b"ODOL" => {
            ensure!(
                versions.contains(&version),
                "version ODOL {version} non prise en charge (voir --odol-version)"
            );
            let exe = tool.context("débinariseur absent")?;
            let dir = tempfile::tempdir()?;
            let input = dir.path().join("input.p3d");
            fs::write(&input, bytes)?;
            let mut command = Command::new(exe);
            command.current_dir(dir.path());
            let output = if exe
                .file_stem()
                .is_some_and(|s| s.eq_ignore_ascii_case("P3DDebinarizer"))
            {
                command.arg(&input);
                dir.path().join("input_mlod.p3d")
            } else if exe
                .file_stem()
                .is_some_and(|s| s.eq_ignore_ascii_case("P3DDebin"))
            {
                command.arg("debin").arg(&input).arg("-o").arg(dir.path());
                dir.path().join("input_debin.p3d")
            } else {
                let path = dir.path().join("output.p3d");
                command.arg(input).arg(&path);
                path
            };
            let status = command.status()?;
            ensure!(status.success(), "échec du débinariseur ({status})");
            read_mlod(&crate::read_limited(&output)?, report)
        }
        _ => bail!("signature P3D inconnue"),
    }
}
