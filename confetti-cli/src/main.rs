use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use colored::Colorize;
use log::error;
use spinoff::{Color::Blue, Spinner, spinners::Dots};

mod deploy;
mod installer;
use crate::deploy::{build, deploy};

pub fn with_spinner<R>(
    message: String,
    f: impl FnOnce() -> Result<R>,
    on_success: impl FnOnce(&R, &mut Spinner),
    on_failure: impl FnOnce(&anyhow::Error, &mut Spinner),
) -> Result<R> {
    let mut spinner = Spinner::new(Dots, message, Blue);
    let result = f();
    match &result {
        Ok(value) => on_success(value, &mut spinner),
        Err(error) => on_failure(error, &mut spinner),
    };
    result
}

#[derive(Parser)]
#[command(version, about, arg_required_else_help = true)]
struct ConfettiCli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// deploy your code to the robot
    Deploy {
        /// the team number to deploy to
        #[arg(short, long)]
        team: u32,

        /// whether or not to run a debug build
        #[arg(long)]
        debug: bool,
    },

    /// build your robot code
    Build {
        /// whether or not to run a debug build
        #[arg(long)]
        debug: bool,
    },

    /// download and install roboRIO compilation toolchain
    Install {
        /// setup toolchain in global cargo config instead of the local project
        #[arg(long)]
        global: bool,
    },
}

fn check_cargo_toml() -> Result<()> {
    if !std::path::Path::new("Cargo.toml").exists() {
        bail!("no Cargo.toml found in current directory");
    }
    Ok(())
}

fn run(cli: ConfettiCli) -> Result<()> {
    match &cli.command {
        Some(Commands::Deploy { team, debug }) => {
            check_cargo_toml()?;
            deploy(*team, debug).map_err(|_| anyhow!("deployment failed"))
        }
        Some(Commands::Build { debug }) => {
            check_cargo_toml()?;
            let _ = with_spinner(
                "building robot code (this may take a minute)".to_string(),
                || build(debug),
                |binary, spinner| {
                    spinner.success(&format!(
                        "built robot code at {}",
                        binary
                            .to_str()
                            .unwrap()
                            .trim_start_matches('"')
                            .trim_end_matches('"')
                    ));
                },
                |error, spinner| {
                    spinner.clear();
                    error!("failed to build robot code: {error}")
                },
            )?;
            Ok(())
        }
        Some(Commands::Install { global }) => installer::install_toolchain(*global)
            .map_err(|e| anyhow!("installation failed").context(e)),
        None => Ok(()),
    }
}

fn main() -> Result<()> {
    let cli = ConfettiCli::parse();

    fern::Dispatch::new()
        .level_for("ssh", log::LevelFilter::Off)
        .level_for("tracing", log::LevelFilter::Off)
        .level_for("cargo", log::LevelFilter::Off)
        .format(|out, message, record| {
            let category_text = match record.level() {
                log::Level::Info => "i".blue(),
                log::Level::Error => "✗".red(),
                _ => record.level().as_str().into(),
            }
            .bold();
            out.finish(format_args!(
                "{category_text} {}: {}",
                record.target(),
                message.to_string()
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;

    let res = run(cli);
    if let Err(error) = res {
        log::error!("{}", error.to_string())
    };
    Ok(())
}
