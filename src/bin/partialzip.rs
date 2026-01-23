use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::{Parser, Subcommand};
use partialzip::partzip::{PartialZip, PartialZipOptions, DEFAULT_MAX_REDIRECTS};
use std::fs::File;
use url::Url;

/// Handler to list the files from command line
fn list(url: &str, detailed: bool, options: PartialZipOptions) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for listing")?;
    let pz = PartialZip::new_with_options(&url, options)
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
fn download(url: &str, filename: &str, outputfile: &str, options: PartialZipOptions) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for downloading")?;
    let pz = PartialZip::new_with_options(&url, options)
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
fn pipe(url: &str, filename: &str, options: PartialZipOptions) -> Result<()> {
    let url = Url::parse(url).context("invalid URL for piping")?;
    let pz = PartialZip::new_with_options(&url, options)
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
    /// Maximum number of HTTP redirects to follow
    #[arg(short = 'm', long, default_value_t = DEFAULT_MAX_REDIRECTS)]
    max_redirects: u32,
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
    let options = PartialZipOptions::new()
        .check_range(cli.check_range)
        .max_redirects(cli.max_redirects);

    match cli.command {
        Commands::List { detailed, url } => list(&url, detailed, options),
        Commands::Download {
            url,
            filename,
            outputfile,
        } => download(&url, &filename, &outputfile, options),
        Commands::Pipe { url, filename } => pipe(&url, &filename, options),
    }
}
