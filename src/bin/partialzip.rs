use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::{App, Arg, SubCommand};
use partialzip::partzip::PartialZip;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, files_only: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url)?;
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
fn download(url: &str, filename: &str, outputfile: &str) -> Result<()> {
    if Path::new(outputfile).exists() {
        return Err(anyhow!("The output file {outputfile} already exists"));
    }
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url)?;
    let content = pz.download(filename)?;
    let mut f = File::create(outputfile)?;
    f.write_all(&content)?;
    println!("{filename} extracted to {outputfile}");
    Ok(())
}

/// Handler to download the file and pipe it to stdout
fn pipe(url: &str, filename: &str) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new(&url)?;
    let content = pz.download(filename)?;
    std::io::stdout().write_all(&content)?;
    Ok(())
}

fn main() -> Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            SubCommand::with_name("list")
                .about("list the files inside the zip")
                .arg(
                    Arg::with_name("files_only")
                        .short('f')
                        .takes_value(false)
                        .required(false)
                        .help("list files only, not size and support"),
                )
                .arg(
                    Arg::with_name("url")
                        .required(true)
                        .help("url of the zip file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("download")
                .about("download a file from the zip")
                .arg(Arg::with_name("url").required(true).index(1))
                .arg(Arg::with_name("filename").required(true).index(2))
                .arg(Arg::with_name("outputfile").required(true).index(3)),
        )
        .subcommand(
            SubCommand::with_name("pipe")
                .about("stream a file from the zip to stdout")
                .arg(Arg::with_name("url").required(true).index(1))
                .arg(Arg::with_name("filename").required(true).index(2)),
        )
        .get_matches();
    if let Some(matches) = matches.subcommand_matches("list") {
        list(
            matches.value_of("url").unwrap(),
            matches.is_present("files_only"),
        )
    } else if let Some(matches) = matches.subcommand_matches("download") {
        download(
            matches.value_of("url").unwrap(),
            matches.value_of("filename").unwrap(),
            matches.value_of("outputfile").unwrap(),
        )
    } else if let Some(matches) = matches.subcommand_matches("pipe") {
        pipe(
            matches.value_of("url").unwrap(),
            matches.value_of("filename").unwrap(),
        )
    } else {
        Err(anyhow!("No command matched, try --help"))
    }
}
