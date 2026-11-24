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
            "URL Workshop Steam invalide"
        );
        let mut ids = url.query_pairs().filter(|(k, _)| k == "id");
        let value = ids.next().context("id absent")?.1.into_owned();
        ensure!(ids.next().is_none(), "id ambigu");
        value
    };
    ensure!(
        digits.bytes().all(|b| b.is_ascii_digit()) && digits.parse::<u64>()? > 0,
        "identifiant invalide"
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
                .unwrap_or("sans titre HTML");
            bail!("fiche Reforger accessible ({title}). Aucun téléchargement Reforger implémenté. Utiliser --input avec des fichiers Arma 3 compatibles.");
        }
    }
    let id = id(spec)?;
    let net="accès réseau à Steam impossible ou interrompu ; si le réseau bloque Steam, joindre l'archive et utiliser --input";
    let envelope: Envelope = client
        .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .form(&[("itemcount", "1"), ("publishedfileids[0]", &id)])
        .send()
        .context(net)?
        .error_for_status()
        .context("API Steam refusée ou indisponible ; joindre l'archive pour --input")?
        .json()?;
    *metadata = envelope.response.publishedfiledetails.into_iter().next();
    let item = metadata.as_ref().context("fiche Steam absente")?;
    ensure!(
        item.result == 1 && item.publishedfileid == id,
        "fiche inaccessible (résultat Steam {})",
        item.result
    );
    ensure!(
        item.visibility == Some(0) && item.banned == Some(0),
        "mod non public ou retiré"
    );
    ensure!(
        item.consumer_app_id == 107410,
        "application différente d'Arma 3 ({})",
        item.consumer_app_id
    );
    ensure!(!item.file_url.is_empty(),"Steam ne fournit pas de file_url publique pour {}. Joindre l'archive ou utiliser --input avec le dossier local.",item.title);
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
                "taille incorrecte dans le cache {}",
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
        .context("téléchargement Steam refusé ou indisponible ; utiliser --input")?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    let n = io::copy(
        &mut (&mut response).take(crate::LIMIT + 1),
        tmp.as_file_mut(),
    )
    .context(net)?;
    ensure!(n <= crate::LIMIT, "archive trop volumineuse");
    if let Some(expected) = expected {
        ensure!(expected == n, "taille téléchargée incorrecte");
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
