extern crate clap;
extern crate partialzip;

use clap::{Arg, App, SubCommand};
use partialzip::{PartialZip};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;


fn list(url: &str) {
    let pz = PartialZip::new(url);
    match pz {
    	Ok(pz) => {
            let l = pz.list();
            for f in l {
                println!("{}", f);
            }
        },
    	Err(e) => println!("{}", e),
    }
}


fn download(url: &str, filename: &str, outputfile: &str) {
    if Path::new(outputfile).exists() {
        println!("The output file {} already exists", outputfile);
        return;
    }
    let pz = PartialZip::new(url);
    match pz {
    	Ok(pz) => {
            let content = pz.download(filename);
            match content {
                Ok(content) => {
                    let f = File::create(outputfile);
                    match f {
                        Ok(mut f) => {
                            if let Err(write_error) = f.write_all(&content) {
                                println!("{}", write_error);
                            }
                        },
                        Err(e) => println!("{}", e),
                    }
                },
                Err(e) => println!("{}", e),
            }
        },
    	Err(e) => println!("{}", e),
    }
    
}

fn main() {
    let matches = App::new("PartialZip")
        .version("0.1.0")
        .author("marcograss")
        .about("Download single files from online zip archives")
        .subcommand(
            SubCommand::with_name("list")
                .about("lists the file inside the zip")
                .arg(Arg::with_name("url").required(true)),
        )
        .subcommand(
            SubCommand::with_name("download")
                .about("download a file from the online zip")
                .arg(Arg::with_name("url").required(true).index(1))
                .arg(Arg::with_name("filename").required(true).index(2))
                .arg(Arg::with_name("outputfile").required(true).index(3)),
        )
        .get_matches();
    if let Some(matches) = matches.subcommand_matches("list") {
        let url = matches.value_of("url").unwrap();
        list(url);
    } else if let Some(matches) = matches.subcommand_matches("download") {
        let url = matches.value_of("url").unwrap();
        let filename = matches.value_of("filename").unwrap();
        let outputfile = matches.value_of("outputfile").unwrap();
        download(url, filename, outputfile);
    }
}