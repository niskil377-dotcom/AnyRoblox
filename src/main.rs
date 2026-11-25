#![forbid(unsafe_code)]
use anyhow::Result;
use arma2obj::{run, Options};
use clap::{ArgGroup, Parser};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(version, about="Convert Arma 3 models to OBJ/MTL",group(ArgGroup::new("source").required(true).args(["input","workshop"])))]
struct Cli {
    /// Steam Workshop URL or numeric item ID
    #[arg(long)]
    workshop: Option<String>,
    /// Local mod folder, ZIP archive or PBO file
    #[arg(long)]
    input: Option<PathBuf>,
    /// Empty or new output folder
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "./cache")]
    cache: PathBuf,
    /// ODOL debinarizer (P3DDebin, P3DDebinarizer or adapter)
    #[arg(long, env = "ARMA2OBJ_DEBINARIZER")]
    debinarizer: Option<PathBuf>,
    /// ODOL versions supported by the executable (comma-separated)
    #[arg(long, value_delimiter = ',', default_value = "75")]
    odol_version: Vec<u32>,
    /// Texture converter (HEMTT or adapter)
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
