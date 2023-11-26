use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::{Arg, ArgAction, Command};
use partialzip::partzip::PartialZip;
use std::fs::File;
use std::path::Path;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, detailed: bool, check_range: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new_check_range(&url, check_range)?;
    if detailed {
        let file_list = pz.list_detailed();
        for f in file_list {
            let descr = format!(
                "{} - {} - Supported: {}",
                f.name,
                ByteSize(f.compressed_size),
                f.supported
            );
            println!("{descr}");
        }
    } else {
        let file_list = pz.list_names();
        for f in file_list {
            println!("{f}");
        }
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
    let mut f = File::create(outputfile)?;
    #[cfg(feature = "progressbar")]
    pz.download_to_write_with_progressbar(filename, &mut f)?;
    #[cfg(not(feature = "progressbar"))]
    pz.download_to_write(filename, &mut f)?;
    println!("{filename} extracted to {outputfile}");
    Ok(())
}

/// Handler to download the file and pipe it to stdout
fn pipe(url: &str, filename: &str, check_range: bool) -> Result<()> {
    let url = Url::parse(url)?;
    let mut pz = PartialZip::new_check_range(&url, check_range)?;
    pz.download_to_write(filename, &mut std::io::stdout())?;
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
                    Arg::new("detailed")
                        .short('d')
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .help("list file size and support not only names"),
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
            matches.get_flag("detailed"),
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
