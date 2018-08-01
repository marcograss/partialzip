# PartialZip

[![Build Status](https://travis-ci.org/marcograss/partialzip.svg?branch=master)](https://travis-ci.org/marcograss/partialzip)
[![Build status](https://ci.appveyor.com/api/projects/status/gi6poi45ds0lr9qi?svg=true)](https://ci.appveyor.com/project/marcograss/partialzip)


PartialZip is a very basic rewrite of https://github.com/planetbeing/partial-zip in Rust.

It allows you to download single files from inside online zip archives.

For now it supports almost only .IPSW (for iOS firmwares) over http/https URLs.

You are welcome to add more zip types and decompression methods and file sources.

## How to Use

```
cargo build --release
# listing files
./target/release/main list http://yoururl/file.ipsw
# download file
./target/release/main download http://yoururl/file.ipsw filename
# for example for kernelcache:
./target/release/main download http://yoururl/file.ipsw kernelcache.release.iphone10
```

## What is used for

sometimes zip archives are huge and you just need a couple of files, for example, a kernelcache from a ipsw
```
./target/release/main download "http://XXXXX/iPhone10,6_11.1.2_15B202_Restore.ipsw" kernelcache.release.iphone10b kernelcache.release.iphone10b
  [00:00:05] [########################################] 14.36MB/14.36MB (0s)
```

As you can see the time (and traffic) saved is significant. 

PartialZip only download the required chunks for your file, allowing you to download few MB instead of serveral GB of the original IPSW.

