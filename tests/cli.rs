use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn cli_flags_work() -> Result<()> {
    let mut cmd = Command::cargo_bin("partialzip")?;
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No command matched"));
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("USAGE"));

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
    cmd.arg("list").arg(&target_arg);
    cmd.assert().success().stdout(
        predicate::str::is_match("1.txt - 7 B - Supported: true\n2.txt - 7 B - Supported: true\n")
            .unwrap(),
    );

    let mut cmd = Command::cargo_bin("partialzip")?;
    cmd.arg("list").arg("-f").arg(&target_arg);

    cmd.assert()
        .success()
        .stdout(predicate::str::is_match("1.txt\n2.txt\n").unwrap());

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

    Ok(())
}
