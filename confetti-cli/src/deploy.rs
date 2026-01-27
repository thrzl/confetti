use crate::with_spinner;
use anyhow::{Result, anyhow};
use log::{error, info};
use spinoff::{Color::Blue, Spinner, spinners::Dots};
use std::fs;
use std::io::Read;
use std::process::Command;
use std::{net::TcpStream, path::PathBuf};
use thiserror::Error;
use toml::Value;

const TARGET_TRIPLE: &str = "arm-unknown-linux-gnueabi";

#[derive(Error, Debug)]
enum DeployError {
    #[error("failed to find roboRIO")]
    RoboRIONotFound,

    #[error("deployment error")]
    Other(#[from] anyhow::Error),
}

fn find_roborio(team_number: u32) -> Result<String> {
    let (first_half, second_half): (String, String) = {
        let stringified_team_number = team_number.to_string();
        let chars = stringified_team_number.chars();
        (chars.clone().take(2).collect(), chars.skip(2).collect())
    };
    let mut addresses = vec![
        "172.22.11.2".to_string(),
        format!("roboRIO-{team_number}-frc.local"),
        format!("10.{first_half}.{second_half}.2"),
    ]
    .into_iter();

    let address = loop {
        let current_address = match addresses.next() {
            Some(current_address) => current_address,
            None => break None,
        };
        match TcpStream::connect((format!("lvuser@{current_address}"), 22)) {
            Ok(stream) => {
                stream.shutdown(std::net::Shutdown::Both)?;
                break Some(current_address);
            }
            _ => continue,
        }
    };

    // address.ok_or(DeployError::RoboRIONotFound.into())
    address.ok_or(DeployError::RoboRIONotFound.into())
}

pub fn build(debug: &bool) -> Result<PathBuf> {
    let mut manifest = fs::File::open(std::env::current_dir()?.join("Cargo.toml").as_path())?;
    let mut manifest_content = String::new();
    manifest.read_to_string(&mut manifest_content)?;
    let metadata: Value = toml::from_str(&manifest_content)?;
    let name = match metadata.get("bin") {
        Some(bins) => bins.as_array().unwrap()[0]["name"].as_str(),
        None => metadata["package"]["name"].as_str(),
    }
    .unwrap();
    // let name = metadata["package"]["name"].to_string();

    let mut args = vec!["build", "--quiet", "--target", TARGET_TRIPLE];
    if !debug {
        args.push("--release");
    }
    let err = Command::new("cargo")
        .args(args)
        .stdout(std::process::Stdio::null())
        // .stderr(std::process::Stdio::null())
        .status()?;
    if !err.success() {
        return Err(anyhow!(
            "cargo build failed with code {}",
            err.code().unwrap()
        ));
    };
    let binary_path = std::env::current_dir()?.join("target/release").join(name);
    Ok(binary_path)
}

pub fn deploy(team_number: u32, debug: &bool) -> Result<()> {
    let binary_path = with_spinner(
        "building robot code (this may take a minute)".to_string(),
        || build(debug),
        |binary, spinner| {
            spinner.success(&format!("built robot code at {}", binary.to_str().unwrap()));
        },
        |error, spinner| {
            spinner.clear();
            error!("failed to build robot code: {error}")
        },
    )?;
    let address = with_spinner(
        "looking for roboRIO".to_string(),
        || find_roborio(team_number),
        |address, spinner| spinner.success(&format!("found roboRIO @ {address}")),
        |_, spinner| {
            spinner.clear();
            error!("failed to find roboRIO")
        },
    )?;

    let mut connection = with_spinner(
        "establishing connection over SSH".to_string(),
        || {
            Ok(ssh::create_session()
                .username("lvuser")
                .password("")
                .connect(format!("{address}:22"))?
                .run_local())
        },
        |_, spinner| spinner.success("established connection!"),
        |error, spinner| {
            spinner.clear();
            error!("ssh connection failed: {error}")
        },
    )?;

    info!("beginning deployment");
    let mut spinner = Spinner::new(Dots, "killing current robot code", Blue);
    let mut shell = connection.open_exec()?;
    if shell
        .exec_command(". /etc/profile.d/frc-path.sh")
        .and(shell.exec_command(". /etc/profile.d/natinst-path.sh"))
        .and(shell.exec_command("/usr/local/bin/frcKillRobot.sh -t"))
        .is_err()
    {
        spinner.clear();
        error!("failed to kill robot code");
        return Err(anyhow!("failed to kill robot code"));
    };
    spinner.update_text("copying binaries");

    // todo!();
    let scp = connection.open_scp()?;
    scp.upload(binary_path.to_str().unwrap(), "/home/lvuser")?;
    connection.close();

    Ok(())
}
