mod show;
mod modify;

use crate::cli::show::ShowCommands;
use arc_isle::parser;
use clap::{Parser, Subcommand};

use self::{modify::{run_modify, ModifyCommands}, show::run_show};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    path: String,
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Show {
        #[command(subcommand)]
        commands: ShowCommands,
    },
    Modify {
        #[command(subcommand)]
        commands: ModifyCommands,
    },
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let parsed_schema = parser::parse(&cli.path)?;
    match cli.commands {
        Commands::Show { commands } => run_show(&parsed_schema, commands)?,
        Commands::Modify { commands } => run_modify(&parsed_schema, commands)?,
    }
    Ok(())
}

