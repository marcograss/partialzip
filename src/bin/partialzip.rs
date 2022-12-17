use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::{Arg, ArgAction, Command};
use partialzip::partzip::PartialZip;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, files_only: bool, check_range: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new_check_range(&url, check_range)?;
    let file_list = pz.list();
    for f in file_list {
        let descr = if files_only {
            f.name
        } else {
            format!(
                "{} - {} - Supported: {}",
                f.name,
                ByteSize(f.compressed_size),
                f.supported
            )
        };
        println!("{descr}");
    }
    Ok(())
}

/// Handler to download the file from command line
fn download(url: &str, filename: &str, outputfile: &str, check_range: bool) -> Result<()> {
    if Path::new(outputfile).exists() {
        return Err(anyhow!("The output file {outputfile} already exists"));
    }
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new_check_range(&url, check_range)?;
    #[cfg(feature = "progressbar")]
    let content = pz.download_with_progressbar(filename)?;
    #[cfg(not(feature = "progressbar"))]
    let content = pz.download(filename)?;
    let mut f = File::create(outputfile)?;
    f.write_all(&content)?;
    println!("{filename} extracted to {outputfile}");
    Ok(())
}

/// Handler to download the file and pipe it to stdout
fn pipe(url: &str, filename: &str, check_range: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new_check_range(&url, check_range)?;
    let content = pz.download(filename)?;
    std::io::stdout().write_all(&content)?;
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("check_range")
                .short('r')
                .action(ArgAction::SetTrue)
                .required(false)
                .help("Require using url with range support"),
        )
        .subcommand(
            Command::new("list")
                .about("list the files inside the zip")
                .arg(
                    Arg::new("files_only")
                        .short('f')
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .help("list files only, not size and support"),
                )
                .arg(Arg::new("url").required(true).help("url of the zip file")),
        )
        .subcommand(
            Command::new("download")
                .about("download a file from the zip")
                .arg(Arg::new("url").required(true).index(1))
                .arg(Arg::new("filename").required(true).index(2))
                .arg(Arg::new("outputfile").required(true).index(3)),
        )
        .subcommand(
            Command::new("pipe")
                .about("stream a file from the zip to stdout")
                .arg(Arg::new("url").required(true).index(1))
                .arg(Arg::new("filename").required(true).index(2)),
        )
        .get_matches();
    let check_range = matches.get_flag("check_range");
    match matches.subcommand() {
        Some(("list", matches)) => list(
            matches.get_one::<String>("url").unwrap(),
            matches.get_flag("files_only"),
            check_range,
        ),
        Some(("download", matches)) => download(
            matches.get_one::<String>("url").unwrap(),
            matches.get_one::<String>("filename").unwrap(),
            matches.get_one::<String>("outputfile").unwrap(),
            check_range,
        ),
        Some(("pipe", matches)) => pipe(
            matches.get_one::<String>("url").unwrap(),
            matches.get_one::<String>("filename").unwrap(),
            check_range,
        ),
        _ => Err(anyhow!("No command matched, try --help")),
    }
}
