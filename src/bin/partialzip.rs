use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::{Arg, ArgAction, Command};
use partialzip::partzip::PartialZip;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, files_only: bool, must_ranged: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url, must_ranged)?;
    let l = pz.list();
    for f in l {
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
fn download(url: &str, filename: &str, outputfile: &str, must_ranged: bool) -> Result<()> {
    if Path::new(outputfile).exists() {
        return Err(anyhow!("The output file {outputfile} already exists"));
    }
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url, must_ranged)?;
    let content = pz.download(filename)?;
    let mut f = File::create(outputfile)?;
    f.write_all(&content)?;
    println!("{filename} extracted to {outputfile}");
    Ok(())
}

/// Handler to download the file and pipe it to stdout
fn pipe(url: &str, filename: &str, must_ranged: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url, must_ranged)?;
    let content = pz.download(filename)?;
    std::io::stdout().write_all(&content)?;
    Ok(())
}

fn main() -> Result<()> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("must_ranged")
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
    let must_ranged = matches.get_flag("must_ranged");
    if let Some(matches) = matches.subcommand_matches("list") {
        list(
            matches.get_one::<String>("url").unwrap(),
            matches.get_flag("files_only"),
            must_ranged,
        )
    } else if let Some(matches) = matches.subcommand_matches("download") {
        download(
            matches.get_one::<String>("url").unwrap(),
            matches.get_one::<String>("filename").unwrap(),
            matches.get_one::<String>("outputfile").unwrap(),
            must_ranged,
        )
    } else if let Some(matches) = matches.subcommand_matches("pipe") {
        pipe(
            matches.get_one::<String>("url").unwrap(),
            matches.get_one::<String>("filename").unwrap(),
            must_ranged,
        )
    } else {
        Err(anyhow!("No command matched, try --help"))
    }
}
