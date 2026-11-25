use crate::{binary::Reader, safe_path};
use anyhow::{ensure, Result};
use std::{collections::HashSet, path::PathBuf};

pub struct Entry<'a> {
    pub path: PathBuf,
    pub data: &'a [u8],
    pub method: u32,
    pub original: u32,
}

impl Entry<'_> {
    pub fn contents(&self) -> Result<std::borrow::Cow<'_, [u8]>> {
        use std::borrow::Cow;
        if self.method == 0 {
            return Ok(Cow::Borrowed(self.data));
        }
        ensure!(
            self.method == 0x43707273,
            "Unknown PBO compression method 0x{:08x}",
            self.method
        );
        let length = self.original as usize;
        ensure!(length as u64 <= crate::LIMIT, "Cprs entry is too large");
        let mut r = Reader(self.data);
        let mut out = Vec::new();
        while out.len() < length {
            let flags = r.take(1)?[0];
            for bit in 0..8 {
                if out.len() == length {
                    break;
                }
                if flags & (1 << bit) != 0 {
                    out.push(r.take(1)?[0]);
                } else {
                    let pair = r.take(2)?;
                    let distance = usize::from(pair[0]) + (usize::from(pair[1] & 0xf0) << 4);
                    let count = usize::from(pair[1] & 15) + 3;
                    ensure!(
                        distance != 0 && count <= length - out.len(),
                        "Invalid Cprs reference"
                    );
                    for _ in 0..count {
                        let byte = out
                            .len()
                            .checked_sub(distance)
                            .map(|i| out[i])
                            .unwrap_or(0x20);
                        out.push(byte);
                    }
                }
            }
        }
        let checksum = out.iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)));
        ensure!(
            r.word()? == checksum && r.0.is_empty(),
            "Invalid Cprs checksum or length"
        );
        Ok(Cow::Owned(out))
    }
}
pub struct Archive<'a> {
    pub prefix: Option<PathBuf>,
    pub files: Vec<Entry<'a>>,
}

pub fn parse(bytes: &[u8]) -> Result<Archive<'_>> {
    let mut r = Reader(bytes);
    let mut prefix = None;
    let mut headers = Vec::new();
    let mut names = HashSet::new();
    let mut properties = false;
    loop {
        let name = r.string()?;
        let method = r.word()?;
        let original = r.word()?;
        let reserved = r.word()?;
        let timestamp = r.word()?;
        let size = r.word()?;
        if name.is_empty() {
            if method == 0x56657273 && headers.is_empty() && !properties {
                properties = true;
                loop {
                    let k = r.string()?;
                    if k.is_empty() {
                        break;
                    }
                    let v = r.string()?;
                    if k.eq_ignore_ascii_case("prefix") && !v.is_empty() {
                        prefix = Some(safe_path(&v)?);
                    }
                }
                continue;
            }
            ensure!(
                method | original | reserved | timestamp | size == 0,
                "Invalid PBO terminator"
            );
            break;
        }
        let path = safe_path(&name)?;
        ensure!(
            names.insert(path.to_string_lossy().to_ascii_lowercase()),
            "Duplicate PBO entry"
        );
        ensure!(headers.len() < 100_000, "Too many PBO entries");
        headers.push((path, method, size, original));
    }
    let mut files = Vec::new();
    for (path, method, size, original) in headers {
        files.push(Entry {
            path,
            method,
            original,
            data: r.take(size as usize)?,
        });
    }
    ensure!(
        r.0.is_empty() || (r.0.len() == 21 && r.0[0] == 0),
        "Unrecognized PBO trailer"
    );
    ensure!(properties || !files.is_empty(), "Empty PBO without signature");
    Ok(Archive { prefix, files })
}
