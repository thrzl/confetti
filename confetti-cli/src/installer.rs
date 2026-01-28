use anyhow::{Result, anyhow, bail};
use dirs::download_dir;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use regex::Regex;
use reqwest::blocking::{Client, get};
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::env::consts::{ARCH, OS};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tempfile::{tempdir, tempdir_in};

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

pub fn install_toolchain() -> Result<()> {
    info!("detected system type {OS} {ARCH}");
    let (host_string, extension) = match (OS, ARCH) {
        ("linux", "aarch64") => ("aarch64-bookworm-linux-gnu", "tgz"),
        ("linux", "armv6") => ("armv6-bookworm-linux-gnueabihf", "tgz"),
        ("linux", "x86_64") => ("x86_64-linux-gnu", "tgz"),
        ("windows", "x86_64") => ("x86_64-w64-mingw32", "zip"),
        ("macos", "x86_64") => ("x86_64-apple-darwin", "tgz"),
        ("macos", "aarch64") => ("arm64-apple-darwin", "tgz"),
        _ => bail!("no installer is provided for this system"),
    };
    let file_regex = Regex::new(&format!(
        r#"cortexa9_vfpv3-roborio-academic-20\d\d-{}-Toolchain-.+\.{}$"#,
        host_string, extension
    ))?;

    let client = {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "github.com/thrzl/confetti".parse().unwrap(),
        );
        Client::builder().default_headers(headers).build()?
    };
    let res = client
        .get("https://api.github.com/repos/wpilibsuite/opensdk/releases?per_page=1")
        .header(reqwest::header::USER_AGENT, "github.com/thrzl/confetti")
        .send()?;
    res.error_for_status_ref()?;

    let release_data: Value = res.json()?;
    let latest_toolchain = release_data.as_array().unwrap().first().unwrap();
    let version = latest_toolchain["tag_name"].to_string();
    let asset = latest_toolchain["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| file_regex.is_match(asset["name"].as_str().unwrap()));
    let asset = match asset {
        Some(asset) => asset,
        None => bail!("failed to find appropriate toolchain for this system"),
    };
    let toolchain_name = asset["name"].as_str().unwrap();
    let toolchain_url = asset["browser_download_url"].as_str().unwrap();

    let downloads_dir = download_dir().ok_or(anyhow!("failed to find downloads directory"))?;
    let dir = tempdir_in(downloads_dir)?;
    let file_path = dir.path().join(&toolchain_name);
    let mut destination = File::create(&file_path)?;
    info!(
        "downloading roboRIO toolchain {version} to {}",
        dir.path().to_str().unwrap()
    );
    let mut res = client.get(toolchain_url).send()?;
    res.error_for_status_ref()?;
    let bytesize = res.content_length().unwrap();

    let progress = ProgressBar::new(bytesize).with_message("downloading roboRIO toolchain").with_style(ProgressStyle::with_template(
        "{spinner:.green} {msg} ({total_bytes}) [{wide_bar:.cyan/blue}] {percent}% downloaded ({eta} left)",
    )?);

    let mut buf = vec![0u8; 1024 * 64]; // 8 KB buffer
    loop {
        let bytes_read = res.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        destination.write_all(&buf[..bytes_read])?;

        progress.inc(bytes_read as u64);
    }

    info!(
        "successfully downloaded toolchain archive to {}",
        &file_path.to_str().unwrap()
    );
    Ok(())
}
