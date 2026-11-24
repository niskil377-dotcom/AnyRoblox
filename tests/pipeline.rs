use anyhow::Result;
use arma2obj::{pbo, run, workshop, Options};
use std::{fs, io::Write, path::Path};

fn words(buf: &mut Vec<u8>, values: &[u32]) {
    for n in values {
        buf.extend(n.to_le_bytes());
    }
}
fn header(buf: &mut Vec<u8>, name: &str, fields: [u32; 5]) {
    buf.extend(name.as_bytes());
    buf.push(0);
    words(buf, &fields);
}
fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut b = Vec::new();
    header(&mut b, "", [0x56657273, 0, 0, 0, 0]);
    b.extend(b"prefix\0test\0\0");
    for (path, bytes) in entries {
        header(&mut b, path, [0, 0, 0, 0, bytes.len() as u32]);
    }
    header(&mut b, "", [0; 5]);
    for (_, bytes) in entries {
        b.extend(*bytes);
    }
    b
}
fn lod(b: &mut Vec<u8>, resolution: f32, materials: &[&str]) {
    b.extend(b"P3DM");
    words(b, &[28, 256, 3, 1, materials.len() as u32, 0]);
    for v in [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        words(b, &v.map(f32::to_bits));
        words(b, &[0]);
    }
    words(b, &[0, 0, 1.0f32.to_bits()]);
    for tex in materials {
        words(b, &[3]);
        for i in 0..3 {
            words(b, &[i, 0, 0, 0]);
        }
        words(b, &[0; 5]);
        b.extend(tex.as_bytes());
        b.extend(b"\0\0");
    }
    b.extend(b"TAGG\x01#EndOfFile#\0");
    words(b, &[0, resolution.to_bits()]);
}
fn model() -> Vec<u8> {
    let mut b = b"MLOD".to_vec();
    words(&mut b, &[257, 3]);
    lod(&mut b, 5e15, &[]);
    lod(&mut b, 2., &[""]);
    lod(&mut b, 1., &["test\\skin.paa", "", "test\\skin.paa"]);
    b
}
fn options(input: &Path, output: &Path) -> Options {
    Options {
        input: Some(input.into()),
        output: output.into(),
        cache: "cache".into(),
        workshop: None,
        debinarizer: None,
        paa_tool: None,
        versions: vec![],
    }
}
fn json(path: &Path) -> Result<serde_json::Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[test]
fn pbo_headers_bounds_and_names() -> Result<()> {
    let data = archive(&[("models\\a.p3d", b"abc"), ("skin.paa", b"1234")]);
    let parsed = pbo::parse(&data)?;
    assert_eq!(parsed.prefix.as_deref(), Some(Path::new("test")));
    assert_eq!(parsed.files[1].contents()?.as_ref(), b"1234");
    assert!(pbo::parse(&data[..data.len() - 1]).is_err());
    for path in ["..\\escape", "/abs", "C:\\abs", "a:NUL", "aux.png"] {
        assert!(pbo::parse(&archive(&[(path, b"")])).is_err());
    }
    Ok(())
}
#[test]
fn compressed_pbo_checks_overlapping_references_and_checksum() -> Result<()> {
    let mut b = Vec::new();
    header(&mut b, "entry", [0x43707273, 6, 0, 0, 8]);
    header(&mut b, "", [0; 5]);
    b.extend([1, b'A', 1, 2]);
    words(&mut b, &[6 * 65]);
    assert_eq!(pbo::parse(&b)?.files[0].contents()?.as_ref(), b"AAAAAA");
    if let Some(last) = b.last_mut() {
        *last = 1;
    }
    assert!(pbo::parse(&b)?.files[0].contents().is_err());
    Ok(())
}
#[test]
fn zip_pbo_batch_reports_and_obj_materials() -> Result<()> {
    let root = tempfile::tempdir()?;
    let archive_path = root.path().join("no_extension");
    let out = root.path().join("out");
    let model = model();
    let pbo = archive(&[
        ("bad.p3d", b"ODOL\xff\0\0\0"),
        ("broken.p3d", &model[..16]),
        ("mesh.p3d", &model),
    ]);
    let mut zip = zip::ZipWriter::new(fs::File::create(&archive_path)?);
    zip.start_file("addons/item.pbo", zip::write::SimpleFileOptions::default())?;
    zip.write_all(&pbo)?;
    zip.finish()?;
    let original = fs::read(&archive_path)?;
    assert!(!run(&options(&archive_path, &out))?);
    assert_eq!(fs::read(archive_path)?, original);
    let summary = json(&out.join("report.json"))?;
    assert_eq!(summary["format"], "zip");
    assert_eq!(
        summary["archives"][0]["files"].as_array().map(Vec::len),
        Some(3)
    );
    let bad = json(&out.join("0001-bad/report.json"))?;
    assert_eq!(bad["status"], "échec");
    assert_eq!(bad["version"], 255);
    assert_eq!(
        json(&out.join("0002-broken/report.json"))?["status"],
        "échec"
    );
    let dir = out.join("0003-mesh");
    let report = json(&dir.join("report.json"))?;
    assert_eq!(report["status"], "partiel");
    assert_eq!(report["selected_lod"], 1.);
    assert_eq!(
        report["hitbox_lods"][0].as_f64().map(|v| v as f32),
        Some(5e15)
    );
    assert_eq!(report["missing_textures"][0], "test\\skin.paa");
    let obj = fs::read_to_string(dir.join("model.obj"))?;
    assert_eq!(obj.matches("\nf ").count(), 3);
    assert_eq!(obj.matches("usemtl ").count(), 2);
    assert!(obj.contains("f 1/7/1 2/8/1 3/9/1"));
    assert_eq!(
        fs::read_to_string(dir.join("model.mtl"))?
            .matches("newmtl")
            .count(),
        2
    );
    Ok(())
}
#[test]
fn png_fallback_and_local_inputs_are_preserved() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("input");
    fs::create_dir_all(dir.join("test"))?;
    let png = dir.join("test/skin.png");
    image::RgbaImage::new(2, 3).save(&png)?;
    fs::write(dir.join("mesh.p3d"), model())?;
    fs::write(dir.join("model.cfg"),"class CfgSkeletons {class Example {};}; class CfgModels {class mesh {class Animations {class Wheel {};};};};")?;
    let out = root.path().join("out");
    assert!(run(&options(&dir, &out))?);
    let mtl = fs::read_to_string(out.join("0001-mesh/model.mtl"))?;
    assert!(mtl.contains("map_Kd texture_1.png"));
    assert!(mtl.contains("map_d -imfchan m texture_1.png"));
    let cfg = json(&out.join("0001-mesh/report.json"))?;
    assert_eq!(cfg["rig_configs"][0]["skeletons"][0], "Example");
    assert_eq!(
        cfg["rig_configs"][0]["animations"][0],
        "CfgModels::mesh::Animations::Wheel"
    );
    assert_eq!(
        image::image_dimensions(out.join("0001-mesh/texture_1.png"))?,
        (2, 3)
    );
    assert!(run(&options(&dir, &dir.join("output"))).is_err());
    Ok(())
}
#[test]
fn workshop_identifiers() -> Result<()> {
    assert_eq!(workshop::id("12345")?, "12345");
    assert_eq!(
        workshop::id("https://steamcommunity.com/sharedfiles/filedetails/?id=12345&l=french")?,
        "12345"
    );
    for bad in [
        "0",
        "-1",
        "abc",
        "https://example.org/?id=12345",
        "https://steamcommunity.com/sharedfiles/filedetails/?id=1&id=2",
    ] {
        assert!(workshop::id(bad).is_err());
    }
    Ok(())
}

#[test]
fn proxy_faces_and_animation_tags_are_inventoried() -> Result<()> {
    let mut bytes = b"MLOD".to_vec();
    words(&mut bytes, &[257, 1]);
    lod(&mut bytes, 1.0, &[""]);
    let end = bytes.len() - 21; // active byte, EndOfFile name, size, resolution
    assert_eq!(bytes[end], 1);
    let mut tags = b"\x01proxy:test.001\0".to_vec();
    words(&mut tags, &[4]);
    tags.extend([1, 1, 1, 1]);
    tags.extend(b"\x01#Animation#\0");
    words(&mut tags, &[4, 0]);
    bytes.splice(end..end, tags);
    let mut report = arma2obj::report::Model::default();
    let mesh = arma2obj::p3d::read_mlod(&bytes, &mut report)?;
    assert!(mesh.faces.is_empty() && mesh.vertices.is_empty());
    assert_eq!(report.proxies, ["proxy:test.001"]);
    assert_eq!(report.animations, ["keyframe 0"]);
    Ok(())
}
