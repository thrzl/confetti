use anyhow::{Result, anyhow, bail};
use dirs::download_dir;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::env::consts::{ARCH, OS};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir_in;

pub fn install_toolchain(global: bool) -> Result<()> {
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
    let version = latest_toolchain["tag_name"].as_str().unwrap();
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
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut buf = vec![0u8; 1024 * 128]; // 128 KB buffer
    loop {
        let bytes_read = res.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        destination.write_all(&buf[..bytes_read])?;

        progress.inc(bytes_read as u64);
    }

    progress.finish_with_message("roboRIO toolchain downloaded");

    info!("successfully downloaded toolchain archive");
    info!("extracting toolchain to ~/.confetti directory");
    #[cfg(target_os = "windows")]
    let extracted_path = extract_zip(&file_path)?.join("roborio-academic");
    #[cfg(not(target_os = "windows"))]
    let extracted_path = extract_tgz(&file_path)?.join("roborio-academic");
    info!(
        "setting up cargo to use roboRIO toolchain to {}",
        extracted_path.display()
    );
    setup_cargo_toolchain(&extracted_path, global)?;
    info!("roboRIO toolchain installation complete");
    Ok(())
}

fn extract_tgz(path: &PathBuf) -> Result<PathBuf> {
    let tar_file = flate2::read::GzDecoder::new(File::open(path)?);

    let destination_dir = dirs::home_dir().unwrap().join(".confetti");
    std::fs::create_dir_all(&destination_dir)?;
    let mut archive = tar::Archive::new(tar_file);
    let progress = ProgressBar::new_spinner()
        .with_message("extracting roboRIO toolchain")
        .with_style(ProgressStyle::with_template("{spinner:.green} {msg}")?)
        .with_finish(indicatif::ProgressFinish::Abandon);
    progress.enable_steady_tick(Duration::from_millis(100));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let output_path = destination_dir.join(entry.path()?);
        entry.unpack(&output_path)?;
    }
    progress.finish_using_style();
    Ok(destination_dir)
}

#[cfg(target_os = "windows")]
fn extract_zip(path: &PathBuf) -> Result<PathBuf> {
    let zip_file = File::open(path)?;
    let mut zip = zip::read::ZipArchive::new(zip_file)?;
    let destination_dir = dirs::home_dir().unwrap().join(".confetti");
    std::fs::create_dir_all(&destination_dir)?;

    let progress = ProgressBar::new(zip.len() as u64)
        .with_message("extracting roboRIO toolchain")
        .with_style(ProgressStyle::with_template("{spinner:.green} {msg} ({total_bytes}) [{wide_bar:.cyan/blue}] {percent}% extracted ({eta} left)")?)
        .with_finish(indicatif::ProgressFinish::Abandon);
    progress.enable_steady_tick(Duration::from_millis(100));
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;

        let output_path = destination_dir.join(file.mangled_name());
        if file.is_dir() {
            std::fs::create_dir_all(&output_path)?;
        } else {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&output_path)?;
            std::io::copy(&mut file, &mut output)?;
        }

        progress.inc(1);
    }
    progress.finish_with_message("roboRIO toolchain extracted");
    Ok(destination_dir)
}

pub fn setup_cargo_toolchain(path: &PathBuf, global: bool) -> Result<()> {
    let toolchain_bin = path.join("bin");
    // find file in bin that ends with gcc
    let gcc_path = std::fs::read_dir(&toolchain_bin)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.file_name().unwrap().to_str().unwrap().ends_with("gcc"))
        .ok_or(anyhow!("failed to find gcc in toolchain bin directory"))?;

    let cargo_config_dir = if global {
        dirs::home_dir().unwrap().join(".cargo")
    } else {
        std::env::current_dir()?.join(".cargo")
    };
    std::fs::create_dir_all(&cargo_config_dir)?;
    let cargo_config_path = cargo_config_dir.join("config.toml");
    let mut cargo_config_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&cargo_config_path)?;

    if cargo_config_path.exists() {
        let mut existing_config = String::default();
        File::open(&cargo_config_path)?.read_to_string(&mut existing_config)?;
        let mut config: toml::Value = toml::from_str(&existing_config)?;
        if !global {
            let table = config.as_table_mut().unwrap();
            table.insert(
                "build".to_string(),
                toml::Value::Table(toml::toml! {
                    target = "arm-unknown-linux-gnueabi"
                }),
            );
        }

        if let Some(target) = config.get_mut("target.arm-unknown-linux-gnueabi") {
            target["linker"] = toml::Value::String(gcc_path.to_str().unwrap().to_string());
            let updated_config = toml::to_string(&config)?;
            write!(cargo_config_file, "{}", updated_config)?;
        } else {
            let target_table: toml::Value = toml::from_str(&format!(
                r#"
                [target.arm-unknown-linux-gnueabi]
                linker = "{}"
                "#,
                gcc_path.to_str().unwrap()
            ))?;
            let full_toml_string = format!(
                "{}\n{}",
                toml::to_string_pretty(&config)?,
                toml::to_string_pretty(&target_table)?
            );
            write!(cargo_config_file, "{}", full_toml_string)?;
        }
        info!(
            "updated existing cargo config at {}",
            cargo_config_path.to_str().unwrap()
        );
        return Ok(());
    }

    writeln!(
        cargo_config_file,
        "[target.arm-unknown-linux-gnueabi]\nlinker = \"{}\"\n\n[build]\ntarget = \"arm-unknown-linux-gnueabi\"",
        gcc_path.to_str().unwrap()
    )?;
    Ok(())
}
