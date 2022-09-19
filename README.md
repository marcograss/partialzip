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

## What is used for

sometimes zip archives are huge and you just need a couple of files, for example, a kernelcache from a ipsw

```
./target/release/partialzip download "http://XXXXX/iPhone10,6_11.1.2_15B202_Restore.ipsw" kernelcache.release.iphone10b kernelcache.release.iphone10b
```

As you can see the time (and traffic) saved is significant.

PartialZip only download the required chunks for your file, allowing you to download few MB instead of several GB of the original IPSW.

## Showcases

- [Google Project Zero Blogpost: The curious tale of a fake Carrier.app](https://googleprojectzero.blogspot.com/2022/06/curious-case-carrier-app.html) - partialzip was used to efficiently download as many versions as possible of the DCP firmware from the iOS ipsws.

- [matteyeux's taco](https://github.com/matteyeux/taco) - partialzip is used as a crate to implement this tool to download and decrypt iOS firmware images.

