use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::{Parser, Subcommand};
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

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Require using url with range support
    #[arg(short = 'r', long)]
    check_range: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// list file size and support not only names
    List {
        /// list file size and support not only names
        #[arg(short = 'd', long)]
        detailed: bool,
        /// url of the zip file
        url: String,
    },
    /// download a file from the zip
    Download {
        url: String,
        filename: String,
        outputfile: String,
    },
    /// stream a file from the zip to stdout
    Pipe { url: String, filename: String },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::List { detailed, url } => list(&url, detailed, cli.check_range),
        Commands::Download {
            url,
            filename,
            outputfile,
        } => download(&url, &filename, &outputfile, cli.check_range),
        Commands::Pipe { url, filename } => pipe(&url, &filename, cli.check_range),
    }
}
