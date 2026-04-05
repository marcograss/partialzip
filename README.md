# PartialZip

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
### Filtering and JSON output:
```
# filter files by glob pattern
partialzip list --filter "*.txt" http://yoururl/file.zip
# output as JSON (useful for scripting)
partialzip list --json http://yoururl/file.zip
# combine both
partialzip list -d --json --filter "kernel*" http://yoururl/file.ipsw
```

### Authentication and proxy:
```
# basic authentication
partialzip -u myuser -p mypass list http://yoururl/file.zip
# via HTTP proxy
partialzip --proxy http://proxy:8080 list http://yoururl/file.zip
# via SOCKS5 proxy with authentication
partialzip --proxy socks5://proxy:1080 --proxy-user user --proxy-pass pass list http://yoururl/file.zip
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

### Library features

**Streaming to disk** (avoids loading entire files into memory):
```rust
use partialzip::PartialZip;
use std::path::Path;

let pz = PartialZip::new("https://example.com/archive.zip")?;
pz.download_to_file("large_file.bin", Path::new("output.bin"))?;
```

**Glob pattern matching** (download files by pattern):
```rust
let pz = PartialZip::new("https://example.com/archive.zip")?;
let txt_files = pz.download_matching("*.txt")?;
// Or list matching filenames
let names = pz.list_names_matching("kernel*");
```

**Parallel downloads** (multiple connections for faster batch downloads):
```rust
let pz = PartialZip::new("https://example.com/archive.zip")?;
let results = pz.download_multiple_parallel(&["file1.txt", "file2.txt"], 4)?;
```

**Retry with exponential backoff** (resilience for flaky connections):
```rust
use partialzip::{PartialZip, PartialZipOptions};
use std::time::Duration;

let options = PartialZipOptions::new()
    .max_retries(3)
    .retry_base_delay(Duration::from_secs(1));
let pz = PartialZip::new_with_options("https://example.com/archive.zip", &options)?;
```

**Configuration** (timeouts, authentication, proxy):
```rust
use partialzip::{PartialZip, PartialZipOptions};
use std::time::Duration;

let options = PartialZipOptions::new()
    .max_redirects(5)
    .connect_timeout(Some(Duration::from_secs(60)))
    .check_range(true)
    .basic_auth("user", "pass")
    .proxy("http://proxy:8080");
let pz = PartialZip::new_with_options("https://example.com/archive.zip", &options)?;
```

## rustls
You can avoid using openssl by enabling the `rustls` feature to avoid the dependency

## Showcases

- [Google Project Zero Blogpost: The curious tale of a fake Carrier.app](https://googleprojectzero.blogspot.com/2022/06/curious-case-carrier-app.html) - partialzip was used to efficiently download as many versions as possible of the DCP firmware from the iOS ipsws.

- [matteyeux's taco](https://github.com/matteyeux/taco) - partialzip is used as a crate to implement this tool to download and decrypt iOS firmware images.

