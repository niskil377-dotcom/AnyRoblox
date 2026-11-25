use crate::{
    p3d::{Face, Mesh},
    report::Model,
    textures::{self, Assets},
};
use anyhow::Result;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub fn export(
    mesh: &Mesh,
    dir: &Path,
    parent: &Path,
    assets: &Assets,
    tool: Option<&Path>,
    report: &mut Model,
) -> Result<()> {
    export_with_textures(
        mesh,
        dir,
        parent,
        &mut textures::TextureBatch::new(assets, tool, None),
        report,
    )
}

pub(crate) fn export_with_textures(
    mesh: &Mesh,
    dir: &Path,
    parent: &Path,
    textures: &mut textures::TextureBatch<'_>,
    report: &mut Model,
) -> Result<()> {
    let mut obj = BufWriter::new(File::create(dir.join("model.obj"))?);
    let mut mtl = BufWriter::new(File::create(dir.join("model.mtl"))?);
    writeln!(obj, "mtllib model.mtl")?;
    for [x, y, z] in &mesh.vertices {
        writeln!(obj, "v {x} {y} {z}")?;
    }
    for [x, y, z] in &mesh.normals {
        writeln!(obj, "vn {x} {y} {z}")?;
    }
    let mut groups = BTreeMap::<_, Vec<&Face>>::new();
    for face in &mesh.faces {
        groups.entry(&face.material).or_default().push(face);
    }
    let mut uv = 0;
    for (i, ((texture, rvmat), faces)) in groups.into_iter().enumerate() {
        report.materials.insert(
            format!("material_{i}"),
            crate::report::Material {
                diffuse: texture.clone(),
                rvmat: rvmat.clone(),
            },
        );
        writeln!(obj, "g material_{i}\nusemtl material_{i}")?;
        writeln!(mtl, "newmtl material_{i}\nKd 1 1 1")?;
        if !texture.is_empty() {
            let png = format!("texture_{i}.png");
            match textures.export(texture, parent, &dir.join(&png), report) {
                Ok(transparent) => {
                    writeln!(mtl, "map_Kd {png}")?;
                    if transparent {
                        writeln!(mtl, "map_d -imfchan m {png}")?;
                    }
                }
                Err(e) => report.errors.push(format!("{texture}: {e:#}")),
            }
        }
        for f in faces {
            for c in &f.corners {
                writeln!(obj, "vt {} {}", c.uv[0], c.uv[1])?;
            }
            write!(obj, "f")?;
            for c in &f.corners {
                uv += 1;
                write!(obj, " {}/{}/{}", c.vertex + 1, uv, c.normal + 1)?;
            }
            writeln!(obj)?;
        }
    }
    obj.flush()?;
    mtl.flush()?;
    report.vertices = mesh.vertices.len();
    report.faces = mesh.faces.len();
    Ok(())
}
