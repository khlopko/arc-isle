// main.rs

mod parser;
mod schema;

use std::io::stdout;

use crossterm::{
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    ExecutableCommand,
};
use clap::{Parser, Subcommand};

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
        commands: ShowCommands
    }
}

#[derive(Subcommand)]
enum ShowCommands {
    Hosts,
    Versioning,
    Types,
    Interfaces,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let parsed_schema = parser::parse(&cli.path)?;
    match cli.commands {
        Commands::Show{commands} => {
            match commands {
                ShowCommands::Hosts => {
                    for host in parsed_schema.hosts {
                        println!("Host: {:?}", host);
                    }
                }
                ShowCommands::Versioning => {
                    println!("Version: {:?}", parsed_schema.versioning);
                }
                ShowCommands::Types => {
                    for type_ in parsed_schema.types {
                        println!("Type: {:?}", type_);
                    }
                }
                ShowCommands::Interfaces => {
                    for interface in parsed_schema.interfaces {
                        println!("Interface: {:?}", interface);
                    }
                }
            }
        },
    }
    /*
    stdout()
        .execute(SetForegroundColor(Color::White))?
        .execute(SetBackgroundColor(Color::Red))?
        .execute(Print("Styled text here."))?
        .execute(ResetColor)?;
        */
    Ok(())
}

