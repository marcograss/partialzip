#[cfg(test)]
mod utils_tests {
    #[test]
    pub fn url_tests() {
        let ok = [
            "http://www.test.com",
            "https://sub.test.com",
            "ftp://ftp.test.com",
        ];
        let not_ok = [
            "file://repo.test.com",
            "asdasd://",
            "js:",
            "smb://storage.test.com",
            "not parsable URL",
        ];
        for s in ok {
            assert!(crate::utils::url_is_valid(s), "{s} should be a valid url");
        }
        for s in not_ok {
            assert!(
                !crate::utils::url_is_valid(s),
                "{s} should be a invalid url"
            )
        }
    }
}

#[cfg(test)]
mod partzip_tests {
    use actix_files as fs;
    use std::net::TcpListener;
    use zip::result::ZipError;

    use actix_web::{App, HttpServer};

    use crate::partzip::{PartialZip, PartialZipError, PartialZipFile};

    struct TestServer {
        pub address: String,
    }

    async fn spawn_server() -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
        let port = listener.local_addr().unwrap().port();
        let address = format!("http://127.0.0.1:{port}");
        let server = HttpServer::new(move || {
            App::new().service(fs::Files::new("/", "./testdata").show_files_listing())
        })
        .listen(listener)
        .unwrap()
        .run();
        let _ = tokio::spawn(server);
        TestServer { address }
    }

    #[tokio::test]
    async fn test_list() {
        let address = spawn_server().await.address;
        println!("listening on {address}");
        tokio::task::spawn_blocking(move || {
            let mut pz =
                PartialZip::new(&(address + "/test.zip")).expect("cannot create partialzip");
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
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_download() {
        let address = spawn_server().await.address;
        println!("listening on {address}");
        tokio::task::spawn_blocking(move || {
            let mut pz =
                PartialZip::new(&(address + "/test.zip")).expect("cannot create partialzip");
            let downloaded = pz.download("1.txt").expect("cannot download 1.txt");
            assert_eq!(downloaded, vec![0x41, 0x41, 0x41, 0x41, 0xa]);
            let downloaded = pz.download("2.txt").expect("cannot download 2.txt");
            assert_eq!(downloaded, vec![0x42, 0x42, 0x42, 0x42, 0xa]);
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_download_invalid_file() {
        let address = spawn_server().await.address;
        println!("listening on {address}");
        tokio::task::spawn_blocking(move || {
            let mut pz =
                PartialZip::new(&(address + "/test.zip")).expect("cannot create partialzip");
            println!("{:?}", pz);
            let downloaded = pz.download("414141.txt");
            assert!(
                matches!(
                    downloaded,
                    Err(PartialZipError::ZipRsError(ZipError::FileNotFound))
                ),
                "didn't throw an error when a file is not in the zip"
            );
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_invalid_header() {
        let address = spawn_server().await.address;
        println!("listening on {address}");
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&(address + "/invalid.zip"));
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
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let address = spawn_server().await.address;
        println!("listening on {address}");
        tokio::task::spawn_blocking(move || {
            let pz = PartialZip::new(&("invalid URL"));
            assert!(
                matches!(pz, Err(PartialZipError::InvalidUrl)),
                "didn't throw an error with invalid URL"
            );
            if let Err(e) = pz {
                println!("{:?}", e);
            }
        })
        .await
        .unwrap();
    }
}
