use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn cli_shows_help() -> Result<(), Box<dyn std::error::Error>> {
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
