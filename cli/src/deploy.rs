use crate::with_spinner;
use anyhow::{Result, anyhow};
use log::{error, info};
use spinoff::{Color::Blue, Spinner, spinners::Dots};
use std::{
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};
use thiserror::Error;

const TARGET_TRIPLE: &str = "arm-unknown-linux-gnueabi";

#[derive(Error, Debug)]
enum DeployError {
    #[error("failed to find roboRIO")]
    RoboRIONotFound,

    #[error("deployment error")]
    Other(#[from] anyhow::Error),

    #[error("build failed")]
    BuildFailed,
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

pub fn build() -> Result<PathBuf> {
    todo!()
}

pub fn deploy(team_number: u32) -> Result<()> {
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

    todo!();
    let mut scp = connection.open_scp()?;
    scp.upload("target/", "/home/lvuser")?;
    scp.close();
    connection.close();

    Ok(())
}
