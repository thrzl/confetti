use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use colored::Colorize;
use spinoff::{Color::Blue, Spinner, spinners::Dots};

mod cli;
use crate::cli::deploy;

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
        #[arg(short, long)]
        team: u32,
    },
}

fn run(cli: ConfettiCli) -> Result<()> {
    match &cli.command {
        Some(Commands::Deploy { team }) => {
            deploy::deploy(*team).map_err(|_| anyhow!("deployment failed"))
        }
        None => Ok(()),
    }
}

fn main() -> Result<()> {
    let cli = ConfettiCli::parse();

    fern::Dispatch::new()
        .level_for("ssh", log::LevelFilter::Error)
        .format(|out, message, record| {
            let category_text = match record.level() {
                log::Level::Info => "i".blue(),
                log::Level::Error => "✗".red(),
                _ => record.level().as_str().into(),
            }
            .bold();
            out.finish(format_args!("{category_text} {}", message.to_string()))
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
