use anyhow::{bail, ensure, Context, Result};
use reqwest::{blocking::Client, Url};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Details {
    pub publishedfileid: String,
    pub result: u32,
    pub creator: String,
    pub title: String,
    pub file_url: String,
    pub consumer_app_id: u32,
    pub visibility: Option<u32>,
    pub banned: Option<u32>,
    pub file_size: String,
    pub time_updated: Option<u64>,
}
#[derive(Deserialize)]
struct Envelope {
    response: Response,
}
#[derive(Deserialize)]
struct Response {
    publishedfiledetails: Vec<Details>,
}

pub fn id(s: &str) -> Result<String> {
    let digits = if s.bytes().all(|b| b.is_ascii_digit()) {
        s.to_owned()
    } else {
        let url = Url::parse(s)?;
        ensure!(
            matches!(url.scheme(), "http" | "https")
                && url.host_str() == Some("steamcommunity.com")
                && matches!(
                    url.path().trim_end_matches('/'),
                    "/sharedfiles/filedetails" | "/workshop/filedetails"
                ),
            "Invalid Steam Workshop URL"
        );
        let mut ids = url.query_pairs().filter(|(k, _)| k == "id");
        let value = ids.next().context("Missing item ID")?.1.into_owned();
        ensure!(ids.next().is_none(), "Ambiguous item ID");
        value
    };
    ensure!(
        digits.bytes().all(|b| b.is_ascii_digit()) && digits.parse::<u64>()? > 0,
        "Invalid ID"
    );
    Ok(digits.parse::<u64>()?.to_string())
}

pub fn fetch(spec: &str, cache: &Path, metadata: &mut Option<Details>) -> Result<PathBuf> {
    let client = Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .build()?;
    if let Ok(url) = Url::parse(spec) {
        if url.host_str() == Some("reforger.armaplatform.com")
            && url.path().starts_with("/workshop/")
        {
            let mut page = String::new();
            client
                .get(url)
                .send()?
                .error_for_status()?
                .take(2_000_000)
                .read_to_string(&mut page)?;
            let title = page
                .split_once("<title>")
                .and_then(|(_, s)| s.split_once("</title>"))
                .map(|(s, _)| s)
                .unwrap_or("untitled page");
            bail!("Reforger page accessible ({title}). Reforger downloads are unsupported. Use --input with compatible Arma 3 files.");
        }
    }
    let id = id(spec)?;
    let net="Steam could not be reached. If the network blocks Steam, supply the archive with --input";
    let envelope: Envelope = client
        .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .form(&[("itemcount", "1"), ("publishedfileids[0]", &id)])
        .send()
        .context(net)?
        .error_for_status()
        .context("Steam API denied or unavailable. Supply the archive with --input")?
        .json()?;
    *metadata = envelope.response.publishedfiledetails.into_iter().next();
    let item = metadata.as_ref().context("Steam item details missing")?;
    ensure!(
        item.result == 1 && item.publishedfileid == id,
        "Item unavailable (Steam result {})",
        item.result
    );
    ensure!(
        item.visibility == Some(0) && item.banned == Some(0),
        "Mod is private or removed"
    );
    ensure!(
        item.consumer_app_id == 107410,
        "Not an Arma 3 Workshop item ({})",
        item.consumer_app_id
    );
    ensure!(!item.file_url.is_empty(),"Steam provides no public file_url for {}. Supply the archive or select the installed mod folder.",item.title);
    let dir = cache.join(id);
    fs::create_dir_all(&dir)?;
    let expected = item.file_size.parse::<u64>().ok();
    let versioned = item
        .time_updated
        .map(|time| dir.join(format!("archive-{time}")));
    if let Some(path) = &versioned {
        if path.exists() {
            let size = fs::metadata(path)?.len();
            ensure!(
                size <= crate::LIMIT && expected.is_none_or(|n| n == size),
                "Invalid cached file size {}",
                path.display()
            );
            return Ok(path.clone());
        }
    }
    let mut response = client
        .get(&item.file_url)
        .send()
        .context(net)?
        .error_for_status()
        .context("Steam download denied or unavailable. Use --input")?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    let n = io::copy(
        &mut (&mut response).take(crate::LIMIT + 1),
        tmp.as_file_mut(),
    )
    .context(net)?;
    ensure!(n <= crate::LIMIT, "Archive is too large");
    if let Some(expected) = expected {
        ensure!(expected == n, "Downloaded size mismatch");
    }
    if let Some(path) = versioned {
        match tmp.persist_noclobber(&path) {
            Ok(_) => (),
            Err(e)
                if e.error.kind() == io::ErrorKind::AlreadyExists
                    && fs::metadata(&path)?.len() == n =>
            {
                ()
            }
            Err(e) => return Err(e.into()),
        }
        return Ok(path);
    }
    let (_, path) = tmp.keep()?;
    Ok(path)
}
