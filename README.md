# PartialZip

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
