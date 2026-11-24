#![forbid(unsafe_code)]
use anyhow::Result;
use arma2obj::{run, Options};
use clap::{ArgGroup, Parser};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(version, about="Convertit les modèles Arma 3 en OBJ/MTL",group(ArgGroup::new("source").required(true).args(["input","workshop"])))]
struct Cli {
    /// URL Steam Workshop ou publishedfileid
    #[arg(long)]
    workshop: Option<String>,
    /// Dossier local, archive ZIP ou PBO
    #[arg(long)]
    input: Option<PathBuf>,
    /// Dossier de sortie vide ou à créer
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "./cache")]
    cache: PathBuf,
    /// Débinariseur ODOL (P3DDebin, P3DDebinarizer ou adaptateur)
    #[arg(long, env = "ARMA2OBJ_DEBINARIZER")]
    debinarizer: Option<PathBuf>,
    /// Versions ODOL acceptées par cet exécutable (liste séparée par des virgules)
    #[arg(long, value_delimiter = ',', default_value = "75")]
    odol_version: Vec<u32>,
    /// Convertisseur de textures (HEMTT ou adaptateur)
    #[arg(long, env = "ARMA2OBJ_PAA_TOOL")]
    paa_tool: Option<PathBuf>,
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    fn executable(p: PathBuf) -> PathBuf {
        p.canonicalize().unwrap_or(p)
    }
    let ok = run(&Options {
        input: cli.input,
        workshop: cli.workshop,
        output: cli.output,
        cache: cli.cache,
        debinarizer: cli.debinarizer.map(executable),
        paa_tool: cli.paa_tool.map(executable),
        versions: cli.odol_version,
    })?;
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
