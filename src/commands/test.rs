use crate::{args::TestArgs, langs::check_output};
use anyhow::{bail, Context, Ok, Result};
use std::{path::Path, process::Command};

/// Run tests.
pub fn cmd_test(_args: &TestArgs) -> Result<()> {
    let venv_path = Path::new(".venv");
    let bin_path = venv_path.join("bin");
    let pytest_path = bin_path.join("pytest");

    // Ensure venv exists.
    if !venv_path.is_dir() {
        println!("⏳️ creating venv...");
        let mut cmd = Command::new("python3");
        let cmd = cmd.args(["-m", "venv", ".venv"]);
        let output = cmd.output().context("create venv")?;
        check_output(&output).context("create venv")?;
    }

    // Ensure pytest and firefly-test are installed.
    if !pytest_path.is_file() {
        println!("⏳️ installing dependencies...");
        let pip_path = bin_path.join("pip");
        let mut cmd = Command::new(&pip_path);
        let cmd = cmd.args(["install", "pytest", "firefly-test"]);
        let output = cmd.output().context("install firefly-test")?;
        check_output(&output).context("install firefly-test")?;
    }

    // Run pytest
    println!("⏳️ running pytest...");
    let mut cmd = Command::new(&pytest_path);
    let status = cmd.status().context("run pytest")?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }

    Ok(())
}
