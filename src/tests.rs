#[cfg(test)]
mod utils_tests {
    #[test]
    /// Test bad and good URLs
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
            );
        }
    }
}

#[cfg(test)]
mod partzip_tests {
    use actix_files as fs;
    use chrono::NaiveDateTime;
    use std::{net::TcpListener, path::PathBuf};
    use url::Url;
    use zip::result::ZipError;

    use actix_web::{App, HttpResponse, HttpServer};

    use std::time::Duration;

    use crate::partzip::{
        PartialZip, PartialZipError, PartialZipFileDetailed, PartialZipOptions,
        DEFAULT_CONNECT_TIMEOUT_SECS, DEFAULT_MAX_REDIRECTS, DEFAULT_TCP_KEEPIDLE_SECS,
        DEFAULT_TCP_KEEPINTVL_SECS,
    };

    use anyhow::Result;

    struct TestServer {
        address: Url,
    }

    /// Spawn the test server which hosts the test files
    fn spawn_server() -> Result<TestServer> {
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
    /// Test the list functionality of the library
    async fn test_list() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let list = pz.list_detailed();
            assert_eq!(
                list,
                vec![
                    PartialZipFileDetailed {
                        name: "1.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated.into(),
                        supported: true,
                        last_modified: NaiveDateTime::parse_from_str(
                            "2022-08-12T15:24:30",
                            "%Y-%m-%dT%H:%M:%S"
                        )
                        .ok(),
                    },
                    PartialZipFileDetailed {
                        name: "2.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated.into(),
                        supported: true,
                        last_modified: NaiveDateTime::parse_from_str(
                            "2022-08-12T15:24:36",
                            "%Y-%m-%dT%H:%M:%S"
                        )
                        .ok(),
                    }
                ]
            );
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    /// Test the download functionality of the library
    async fn test_download() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let downloaded = pz.download("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            let downloaded = pz.download("2.txt")?;
            assert_eq!(downloaded, vec![0x42, 0x42, 0x42, 0x42, 0xa]);
            Ok(())
        })
        .await?
    }

    #[cfg(feature = "progressbar")]
    #[tokio::test]
    /// See if the code with the progress bar at least run
    async fn test_download_progressbar() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&address.join("/files/test.zip")?)?;
            let downloaded = pz.download_with_progressbar("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            let downloaded = pz.download_with_progressbar("2.txt")?;
            assert_eq!(downloaded, vec![0x42, 0x42, 0x42, 0x42, 0xa]);
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    /// Test that downloading files that are not present in the archive throws an error
    async fn test_download_invalid_file() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&address.join("/files/test.zip")?)?;
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
    /// Test that invalid zip archives are rejected
    async fn test_invalid_header() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(
                &address
                    .join("/files/invalid.zip")
                    .expect("cannot join invalid URL"),
            );
            assert!(
                matches!(
                    pz,
                    Err(PartialZipError::ZipRsError(ZipError::InvalidArchive(_)))
                ),
                "didn't throw an error with invalid header"
            );
        })
        .await?;
        Ok(())
    }

    #[tokio::test]
    /// Test that invalid URLs don't get through
    async fn test_invalid_url() -> Result<()> {
        spawn_server()?;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&"invalid URL");
            assert!(
                matches!(pz, Err(PartialZipError::InvalidUrl)),
                "didn't throw an error with invalid URL"
            );
            if let Err(e) = pz {
                println!("{e:?}");
            }
        })
        .await?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    /// Test that we can open files over file:// not only http/https
    fn test_file_protocol() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let pz = PartialZip::new(&format!("file://localhost{}", d.display()))?;
        let list = pz.list_detailed();
        assert_eq!(
            list,
            vec![
                PartialZipFileDetailed {
                    name: "1.txt".to_string(),
                    compressed_size: 7,
                    compression_method: zip::CompressionMethod::Deflated.into(),
                    supported: true,
                    last_modified: NaiveDateTime::parse_from_str(
                        "2022-08-12T15:24:30",
                        "%Y-%m-%dT%H:%M:%S"
                    )
                    .ok(),
                },
                PartialZipFileDetailed {
                    name: "2.txt".to_string(),
                    compressed_size: 7,
                    compression_method: zip::CompressionMethod::Deflated.into(),
                    supported: true,
                    last_modified: NaiveDateTime::parse_from_str(
                        "2022-08-12T15:24:36",
                        "%Y-%m-%dT%H:%M:%S"
                    )
                    .ok(),
                }
            ]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    /// Test that the url getter works
    fn test_url_getter() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let url = format!("file://localhost{}", d.display());
        let pz = PartialZip::new(&url)?;
        assert_eq!(url, pz.url());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    /// Test that the file size getter works
    fn test_file_size_getter() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let expected_size = std::fs::metadata(&d)?.len();
        let url = format!("file://localhost{}", d.display());
        let pz = PartialZip::new(&url)?;
        assert_eq!(expected_size, pz.file_size());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    /// Test that it throws an error when the range protocol is not supported
    fn test_check_range_on_not_ranging_protocol() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let pz = PartialZip::new_check_range(&format!("file://localhost{}", d.display()), true);
        assert!(
            matches!(pz, Err(PartialZipError::RangeNotSupported)),
            "didn't throw an error with range not supported"
        );
    }

    #[tokio::test]
    /// Test that the range header is correctly detected
    async fn test_range_support() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new_check_range(&address.join("/files/test.zip")?, true)?;
            let downloaded = pz.download("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            Ok(())
        })
        .await?
    }

    #[tokio::test]
    /// Check if we follow redirects correctly
    async fn test_redirect() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&address.join("/redirect")?)?;
            let list = pz.list_detailed();
            assert_eq!(
                list,
                vec![
                    PartialZipFileDetailed {
                        name: "1.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated.into(),
                        supported: true,
                        last_modified: NaiveDateTime::parse_from_str(
                            "2022-08-12T15:24:30",
                            "%Y-%m-%dT%H:%M:%S"
                        )
                        .ok(),
                    },
                    PartialZipFileDetailed {
                        name: "2.txt".to_string(),
                        compressed_size: 7,
                        compression_method: zip::CompressionMethod::Deflated.into(),
                        supported: true,
                        last_modified: NaiveDateTime::parse_from_str(
                            "2022-08-12T15:24:36",
                            "%Y-%m-%dT%H:%M:%S"
                        )
                        .ok(),
                    }
                ]
            );
            Ok(())
        })
        .await?
    }

    #[test]
    /// Test that PartialZipOptions has correct default values
    fn test_options_defaults() {
        let options = PartialZipOptions::default();
        assert!(!options.check_range);
        assert_eq!(options.max_redirects, DEFAULT_MAX_REDIRECTS);
        assert_eq!(options.max_redirects, 10);
        assert_eq!(
            options.connect_timeout,
            Some(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        );
        assert_eq!(options.connect_timeout, Some(Duration::from_secs(30)));
        assert_eq!(
            options.tcp_keepidle,
            Duration::from_secs(DEFAULT_TCP_KEEPIDLE_SECS)
        );
        assert_eq!(options.tcp_keepidle, Duration::from_secs(120));
        assert_eq!(
            options.tcp_keepintvl,
            Duration::from_secs(DEFAULT_TCP_KEEPINTVL_SECS)
        );
        assert_eq!(options.tcp_keepintvl, Duration::from_secs(60));
    }

    #[test]
    /// Test that PartialZipOptions builder methods work correctly
    fn test_options_builder() {
        let options = PartialZipOptions::new()
            .check_range(true)
            .max_redirects(5)
            .connect_timeout(Some(Duration::from_secs(60)))
            .tcp_keepidle(Duration::from_secs(90))
            .tcp_keepintvl(Duration::from_secs(30));
        assert!(options.check_range);
        assert_eq!(options.max_redirects, 5);
        assert_eq!(options.connect_timeout, Some(Duration::from_secs(60)));
        assert_eq!(options.tcp_keepidle, Duration::from_secs(90));
        assert_eq!(options.tcp_keepintvl, Duration::from_secs(30));

        // Test None timeout
        let options = PartialZipOptions::new().connect_timeout(None);
        assert_eq!(options.connect_timeout, None);
    }

    #[cfg(unix)]
    #[test]
    /// Test that new_with_options works correctly
    fn test_new_with_options() -> Result<()> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let url = format!("file://localhost{}", d.display());
        let options = PartialZipOptions::new().max_redirects(5);
        let pz = PartialZip::new_with_options(&url, options)?;
        assert_eq!(url, pz.url());
        Ok(())
    }

    #[tokio::test]
    /// Test that max_redirects = 0 prevents following redirects
    async fn test_max_redirects_zero_blocks_redirect() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let options = PartialZipOptions::new().max_redirects(0);
            let redirect_url = address.join("/redirect").expect("valid URL");
            let pz = PartialZip::new_with_options(&redirect_url, options);
            // With max_redirects = 0, following the redirect should fail
            assert!(pz.is_err(), "should fail when max_redirects is 0 and URL redirects");
        })
        .await?;
        Ok(())
    }

    #[tokio::test]
    /// Test that new_with_options works with the test server
    async fn test_new_with_options_http() -> Result<()> {
        let address = spawn_server()?.address;
        tokio::task::spawn_blocking(move || {
            let options = PartialZipOptions::new()
                .check_range(true)
                .max_redirects(10);
            let pz = PartialZip::new_with_options(&address.join("/files/test.zip")?, options)?;
            let downloaded = pz.download("1.txt")?;
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            Ok(())
        })
        .await?
    }
}
