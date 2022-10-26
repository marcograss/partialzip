#[cfg(test)]
mod utils_tests {
    #[test]
    pub fn url_tests() {
        let valid_urls = [
            "http://www.test.com",
            "https://sub.test.com",
            "ftp://ftp.test.com",
            "file://localhost/home/test/1.zip",
        ];
        let invalid_urls = [
            "asdasd://",
            "js:",
            "smb://storage.test.com",
            "not parsable URL",
        ];
        for url in valid_urls {
            assert!(
                crate::utils::url_is_valid(url),
                "{url} should be a valid url"
            );
        }
        for url in invalid_urls {
            assert!(
                !crate::utils::url_is_valid(url),
                "{url} should be a invalid url"
            )
        }
    }
}

#[cfg(test)]
mod partzip_tests {
    use actix_files as fs;
    use std::{net::TcpListener, path::PathBuf};
    use url::Url;
    use zip::result::ZipError;

    use actix_web::{App, HttpResponse, HttpServer};

    use crate::partzip::{PartialZip, PartialZipError, PartialZipFile};

    use anyhow::Result;

    struct TestServer {
        address: Url,
    }

    async fn spawn_server() -> Result<TestServer> {
        // Bind to a random local port
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        // Local server address
        let address = Url::parse(&format!("http://127.0.0.1:{port}"))?;
        let server = HttpServer::new(move || {
            App::new()
                .service(fs::Files::new("/files/", "./testdata").show_files_listing())
                .service(actix_web::web::resource("/redirect").to(|| async {
                    HttpResponse::Found()
                        .append_header(("Location", "/files/test.zip"))
                        .finish()
                }))
        })
        .listen(listener)?
        .run();
        tokio::spawn(server);
        println!("listening on {address}");
        Ok(TestServer { address })
    }

    #[tokio::test]
    async fn test_list() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let mut pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let list = pz.list();
            assert_eq!(
                list,
                vec![
                    PartialZipFile {
                        name: "1.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated,
                        supported: true
                    },
                    PartialZipFile {
                        name: "2.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated,
                        supported: true
                    }
                ]
            );
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    async fn test_download() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let mut pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let downloaded = pz.download("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            let downloaded = pz.download("2.txt")?;
            assert_eq!(downloaded, vec![0x42, 0x42, 0x42, 0x42, 0xa]);
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    async fn test_download_invalid_file() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let mut pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let downloaded = pz.download("414141.txt");
            assert!(
                matches!(
                    downloaded,
                    Err(PartialZipError::ZipRsError(ZipError::FileNotFound))
                ),
                "didn't throw an error when a file is not in the zip"
            );
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    async fn test_invalid_header() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(
                &address
                    .join("/files/invalid.zip")
                    .expect("cannot join invalid URL"),
            );
            assert!(
                matches!(
                    pz,
                    Err(PartialZipError::ZipRsError(ZipError::InvalidArchive(
                        "Invalid zip header"
                    )))
                ),
                "didn't throw an error with invalid header"
            );
        })
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_url() -> Result<()> {
        spawn_server().await?;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&"invalid URL");
            assert!(
                matches!(pz, Err(PartialZipError::InvalidUrl)),
                "didn't throw an error with invalid URL"
            );
            if let Err(e) = pz {
                println!("{:?}", e);
            }
        })
        .await?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_file_protocol() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let mut pz = PartialZip::new(&format!("file://localhost{}", d.display()))?;
        let list = pz.list();
        assert_eq!(
            list,
            vec![
                PartialZipFile {
                    name: "1.txt".to_string(),
                    compressed_size: 7,
                    compression_method: zip::CompressionMethod::Deflated,
                    supported: true
                },
                PartialZipFile {
                    name: "2.txt".to_string(),
                    compressed_size: 7,
                    compression_method: zip::CompressionMethod::Deflated,
                    supported: true
                }
            ]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_must_ranged_on_not_ranging_protocol() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let pz = PartialZip::new_check_range(&format!("file://localhost{}", d.display()), true);
        assert!(
            matches!(pz, Err(PartialZipError::RangeNotSupported)),
            "didn't throw an error with range not supported"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_range_support() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let mut pz = PartialZip::new_check_range(&address.join("/files/test.zip")?, true)?;
            let downloaded = pz.download("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    async fn test_redirect() -> Result<()> {
        let address = spawn_server().await?.address;
        tokio::task::spawn_blocking(move || {
            let mut pz = PartialZip::new(&address.join("/redirect")?)?;
            let list = pz.list();
            assert_eq!(
                list,
                vec![
                    PartialZipFile {
                        name: "1.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated,
                        supported: true
                    },
                    PartialZipFile {
                        name: "2.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated,
                        supported: true
                    }
                ]
            );
            Ok(())
        })
        .await?
    }
}
