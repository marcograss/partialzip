#[cfg(feature = "cmdline")]
/// Those are the tests for the CLI tool to see that everything works end to end and we didn't break the command line interface
mod cli_tests {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use std::process::Command;

    #[test]
    fn binary_exists() {
        assert!(
            assert_cmd::cargo_bin!("partialzip").exists(),
            "binary exists"
        );
    }

    #[test]
    fn cli_flags_work() {
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Usage"));

        // Verify --max-redirects flag is documented in help
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("--max-redirects"));

        // Verify --connect-timeout flag is documented in help
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("--connect-timeout"));

        // Verify authentication flags are documented in help
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("--username"))
            .stdout(predicate::str::contains("--password"));

        // Verify proxy flags are documented in help
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("--proxy"))
            .stdout(predicate::str::contains("--proxy-user"))
            .stdout(predicate::str::contains("--proxy-pass"));

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("list");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip list")
                .or(predicate::str::contains("partialzip.exe list")),
        );

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("download");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip download")
                .or(predicate::str::contains("partialzip.exe download")),
        );

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("pipe");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip pipe")
                .or(predicate::str::contains("partialzip.exe pipe")),
        );
    }

    #[cfg(unix)]
    #[test]
    fn cli_works() -> Result<()> {
        use std::{fs, path::PathBuf};

        use tempfile::NamedTempFile;

        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let target_arg = format!("file://localhost{}", d.display());

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("list").arg("-d").arg(&target_arg);
        cmd.assert().success().stdout(
            predicate::str::is_match(
                "1.txt - 7 B - Supported: true\n2.txt - 7 B - Supported: true\n",
            )
            .unwrap(),
        );

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("list").arg(&target_arg);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("2.txt\n"));

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-r").arg("list").arg(&target_arg);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Range request not supported\n"));

        let output_file = NamedTempFile::new()?.path().display().to_string();

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("download")
            .arg(&target_arg)
            .arg("1.txt")
            .arg(&output_file);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt extracted to"));

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("download")
            .arg(&target_arg)
            .arg("1.txt")
            .arg(&output_file);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("File exists"));

        fs::remove_file(&output_file)?;

        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("pipe").arg(&target_arg).arg("1.txt");
        cmd.assert().success();

        // Test --max-redirects flag with short form
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-m").arg("5").arg("list").arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        // Test --max-redirects flag with long form
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--max-redirects")
            .arg("20")
            .arg("list")
            .arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        // Test combining -r and -m flags
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-r")
            .arg("-m")
            .arg("5")
            .arg("list")
            .arg(&target_arg);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Range request not supported\n"));

        // Test --connect-timeout flag with short form
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-t").arg("60").arg("list").arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        // Test --connect-timeout flag with long form
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("--connect-timeout")
            .arg("45")
            .arg("list")
            .arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        // Test --connect-timeout with 0 (no timeout)
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-t").arg("0").arg("list").arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        // Test combining all flags
        let mut cmd = Command::new(assert_cmd::cargo_bin!("partialzip"));
        cmd.arg("-m")
            .arg("5")
            .arg("-t")
            .arg("30")
            .arg("list")
            .arg(&target_arg);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));

        Ok(())
    }
}
