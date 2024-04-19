#[cfg(feature = "cmdline")]
/// Those are the tests for the CLI tool to see that everything works end to end and we didn't break the command line interface
mod cli_tests {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use std::process::Command;

    #[test]
    fn binary_exists() {
        Command::cargo_bin("partialzip").expect("binary exists");
    }

    #[test]
    fn cli_flags_work() -> Result<()> {
        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage"));
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Usage"));

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("list");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip list")
                .or(predicate::str::contains("partialzip.exe list")),
        );

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("download");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip download")
                .or(predicate::str::contains("partialzip.exe download")),
        );

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("pipe");
        cmd.assert().failure().stderr(
            predicate::str::contains("partialzip pipe")
                .or(predicate::str::contains("partialzip.exe pipe")),
        );

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn cli_works() -> Result<()> {
        use std::{fs, path::PathBuf};

        use tempfile::NamedTempFile;

        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("testdata/test.zip");
        let target_arg = format!("file://localhost{}", d.display());

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("list").arg("-d").arg(&target_arg);
        cmd.assert().success().stdout(
            predicate::str::is_match(
                "1.txt - 7 B - Supported: true\n2.txt - 7 B - Supported: true\n",
            )
            .unwrap(),
        );

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("list").arg(&target_arg);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt\n"));
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("2.txt\n"));

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("-r").arg("list").arg(&target_arg);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Range request not supported\n"));

        let output_file = NamedTempFile::new()?.path().display().to_string();

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("download")
            .arg(&target_arg)
            .arg("1.txt")
            .arg(&output_file);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("1.txt extracted to"));

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("download")
            .arg(&target_arg)
            .arg("1.txt")
            .arg(&output_file);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));

        fs::remove_file(&output_file)?;

        let mut cmd = Command::cargo_bin("partialzip")?;
        cmd.arg("pipe").arg(&target_arg).arg("1.txt");
        cmd.assert().success();

        Ok(())
    }
}
