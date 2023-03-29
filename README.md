# PartialZip

[![Travis Build Status](https://travis-ci.org/marcograss/partialzip.svg?branch=master)](https://travis-ci.org/marcograss/partialzip)
[![AppVeyor Build Status](https://ci.appveyor.com/api/projects/status/gi6poi45ds0lr9qi?svg=true)](https://ci.appveyor.com/project/marcograss/partialzip)
[![Github Build Status](https://github.com/marcograss/partialzip/actions/workflows/rust.yml/badge.svg)](https://github.com/marcograss/partialzip)

PartialZip is a rewrite of <https://github.com/planetbeing/partial-zip> in Rust.

It allows you to download single files from inside online zip archives.

You are welcome to add more zip types and decompression methods and file sources.

## How to Use

### As a command line tool:
```
cargo install partialzip
partialzip list http://yoururl/file.ipsw
partialzip download http://yoururl/file.ipsw kernelcache.release.iphone10 kernelcache.release.iphone10
```
### Or from git sources:
```
cargo build --release
# listing files
./target/release/partialzip list http://yoururl/file.ipsw
# download file
./target/release/partialzip download http://yoururl/file.ipsw filename
# for example for kernelcache:
./target/release/partialzip download http://yoururl/file.ipsw kernelcache.release.iphone10 kernelcache.release.iphone10
```
### Docker:
```
# build the container
docker build -t marcograss/partialzip .
# run it
# list files
docker run --rm marcograss/partialzip list http://yoururl/file.ipsw
# download piping to stdout and save it on the host
docker run --rm marcograss/partialzip pipe http://yoururl/file.ipsw kernelcache.release.iphone10 > kernelcache.release.iphone10
```
## What is used for

Sometimes zip archives are huge and you just need a couple of files, for example, a kernelcache from an ipsw

```
./target/release/partialzip download "http://XXXXX/iPhone10,6_11.1.2_15B202_Restore.ipsw" kernelcache.release.iphone10b kernelcache.release.iphone10b
```

As you can see the time (and traffic) saved is significant.

PartialZip only downloads the required chunks for your file, allowing you to download a few Mb instead of several Gb of the original archive.

## Prerequisites
One prerequisite to be able to partially download zips from http servers is that the server support the Range Header. In this way you can request specific parts of the archive.

Not all servers support this. You can check if this is supported using the `-r` flag

```
cargo run -- -r list http://yoururl/yourfile.zip
```

## How to use as a library
If you want to use partialzip as a library and you want to reduce the binary size, you can choose in your `Cargo.toml` the flag `default-features = false` in the partialzip dependency.
This will not build the command line of partialzip which is not required to use it as a library, and it will avoid including some unnecessary dependencies and save space.

## Showcases

- [Google Project Zero Blogpost: The curious tale of a fake Carrier.app](https://googleprojectzero.blogspot.com/2022/06/curious-case-carrier-app.html) - partialzip was used to efficiently download as many versions as possible of the DCP firmware from the iOS ipsws.

- [matteyeux's taco](https://github.com/matteyeux/taco) - partialzip is used as a crate to implement this tool to download and decrypt iOS firmware images.

