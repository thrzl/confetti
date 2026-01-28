use anyhow::{Result, anyhow, bail};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use log::info;
use reqwest::blocking::get;
use serde_json::Value;
use std::env::consts::{ARCH, OS};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tempfile::tempdir;

/// get the installed version of the WPIHAL crate.
fn get_wpihal_version() -> Result<String> {
    let cmd = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .stdout(Stdio::piped())
        .output()?;
    let metadata_string = String::from_utf8(cmd.stdout)?;
    let metadata: Value = serde_json::from_str(&metadata_string)?;
    let package = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"].as_str().unwrap() == "wpihal-sys")
        .ok_or(anyhow!("failed to find wpihal crate"))?;
    Ok(package["version"].as_str().unwrap().to_string())
}

pub fn download_wpilib() -> Result<()> {
    let version = get_wpihal_version()?;
    info!("installed wpihal version: v{version}");

    info!("detected system type {OS} {ARCH}");
    let (directory, host_string, extension) = match (OS, ARCH) {
        ("linux", "aarch64") => ("LinuxArm64", "LinuxArm64", "tar.gz"),
        ("linux", "x86_64") => ("Linux", "Linux", "tar.gz"),
        ("windows", "x86_64") => ("Win64", "Windows", "iso"),
        ("macos", "x86_64") => ("macOS", "macOS-Intel", "dmg"),
        ("macos", "aarch64") => ("macOSArm", "macOS-Arm64", "dmg"),
        _ => bail!("no installer is provided for this system"),
    };

    let file_name = format!("WPILib_{host_string}-{version}.{extension}");
    let installer_url =
        format!("https://packages.wpilib.workers.dev/installer/v{version}/{directory}/{file_name}");
    let dir = tempdir()?;
    info!(
        "downloading {file_name} to {}",
        dir.path().to_str().unwrap()
    );
    let mut res = get(installer_url)?;
    res.error_for_status_ref()?;
    let bytesize = res.content_length().unwrap();

    let mut destination = {
        let path = dir.path().join(&file_name);
        File::create(path)?
    };
    let progress = ProgressBar::new(bytesize).with_message(format!("downloading {file_name}")).with_style(ProgressStyle::with_template(
        "{spinner:.green} {msg} [{wide_bar:.cyan/blue}] {percent}% of {total_bytes} downloaded ({eta} left)",
    )?);

    let mut buf = vec![0u8; 1024 * 8]; // 8 KB buffer
    loop {
        let bytes_read = res.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        destination.write_all(&buf[..bytes_read])?;

        progress.inc(bytes_read as u64);
    }

    Ok(())
}
