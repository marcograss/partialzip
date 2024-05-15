use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::{Parser, Subcommand};
use partialzip::partzip::PartialZip;
use std::fs::File;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, detailed: bool, check_range: bool) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for listing")?;
    let pz = PartialZip::new_check_range(&url, check_range)
        .context("Cannot create PartialZip instance for listing")?;
    if detailed {
        pz.list_detailed().into_iter().for_each(|f| {
            println!(
                "{} - {} - Supported: {}",
                f.name,
                ByteSize(f.compressed_size),
                f.supported
            );
        });
    } else {
        pz.list_names().into_iter().for_each(|f| println!("{f}"));
    }
    Ok(())
}

/// Handler to download the file from command line
fn download(url: &str, filename: &str, outputfile: &str, check_range: bool) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for downloading")?;
    let pz = PartialZip::new_check_range(&url, check_range)
        .context("Cannot create PartialZip instance for downloading")?;
    let mut f = File::create_new(outputfile).context("cannot create the output file")?;
    #[cfg(feature = "progressbar")]
    pz.download_to_write_with_progressbar(filename, &mut f)
        .context("download failed")?;
    #[cfg(not(feature = "progressbar"))]
    pz.download_to_write(filename, &mut f)
        .context("download failed")?;
    println!("{filename} extracted to {outputfile}");
    Ok(())
}

/// Handler to download the file and pipe it to stdout
fn pipe(url: &str, filename: &str, check_range: bool) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for piping")?;
    let pz = PartialZip::new_check_range(&url, check_range)
        .context("Cannot create PartialZip instance for piping")?;
    pz.download_to_write(filename, &mut std::io::stdout())
        .context("download failed")?;
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
