use std::{collections::HashMap, io::stdout};

use clap::{Parser, Subcommand};
use crossterm::{
    style::{Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    ExecutableCommand,
};

use arc_isle::{
    parser,
    schema::{
        self, ApiSpec, HttpPayload, HttpResponses, InterfaceDecl, InterfaceSpec, Schema,
        StatusCode, TypeDecl,
    },
};

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
}

#[derive(Subcommand)]
enum ShowCommands {
    Hosts,
    Versioning,
    Types,
    Interfaces,
    All,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let parsed_schema = parser::parse(&cli.path)?;
    match cli.commands {
        Commands::Show { commands } => match commands {
            ShowCommands::Hosts => print_hosts(&parsed_schema)?,
            ShowCommands::Versioning => print_versioning(&parsed_schema)?,
            ShowCommands::Types => print_types(&parsed_schema)?,
            ShowCommands::Interfaces => print_interfaces(&parsed_schema)?,
            ShowCommands::All => {
                print_hosts(&parsed_schema)?;
                print_versioning(&parsed_schema)?;
                print_types(&parsed_schema)?;
                print_interfaces(&parsed_schema)?
            }
        },
    }
    Ok(())
}

fn print_hosts(parsed_schema: &Schema) -> Result<(), Box<dyn std::error::Error>> {
    let separator = (0..80).map(|_| "-").collect::<String>();
    let indent = (0..4).map(|_| " ").collect::<String>();
    let mut out = stdout();
    let builder = out
        .execute(Print(&separator))?
        .execute(SetAttribute(crossterm::style::Attribute::Bold))?
        .execute(Print(format!("\n{}Hosts\n", &indent)))?
        .execute(Print(&separator))?
        .execute(Print("\n"))?
        .execute(ResetColor)?;
    for host in &parsed_schema.hosts {
        builder.execute(Print(format!(
            "{}- {}: {}\n",
            &indent, host.env, host.address
        )))?;
    }
    builder
        .execute(Print(separator))?
        .execute(Print("\r\n"))
        .map(|_| Ok(()))?
}

fn print_versioning(parsed_schema: &Schema) -> Result<(), Box<dyn std::error::Error>> {
    let separator = (0..80).map(|_| "-").collect::<String>();
    let indent = (0..4).map(|_| " ").collect::<String>();
    stdout()
        .execute(Print(&separator))?
        .execute(SetAttribute(crossterm::style::Attribute::Bold))?
        .execute(Print(format!("\n{}Versioning\n", &indent)))?
        .execute(Print(&separator))?
        .execute(Print("\n"))?
        .execute(ResetColor)?
        .execute(Print(format!(
            "{}Format: {:?}\n",
            indent, parsed_schema.versioning.format
        )))?
        .execute(Print(format!(
            "{}Header: {:?}\n",
            indent, parsed_schema.versioning.header
        )))?
        .execute(Print(separator))?
        .execute(Print("\r\n"))
        .map(|_| Ok(()))?
}

fn print_types(parsed_schema: &Schema) -> Result<(), Box<dyn std::error::Error>> {
    let separator = (0..80).map(|_| "-").collect::<String>();
    let indent = (0..4).map(|_| " ").collect::<String>();
    let mut out = stdout();
    let builder = out
        .execute(Print(&separator))?
        .execute(SetAttribute(crossterm::style::Attribute::Bold))?
        .execute(Print(format!("\n{}Types\n", &indent)))?
        .execute(Print(&separator))?
        .execute(Print("\n"))?
        .execute(ResetColor)?;
    for type_ in &parsed_schema.types {
        match type_ {
            Ok(val) => builder
                .execute(Print(&indent))?
                .execute(Print(displayable_type(val, &indent, 1)))?
                .execute(Print("\n\n"))?,
            Err(err) => builder.execute(Print(format!("{}- {:?}\n", &indent, err)))?,
        };
    }
    builder
        .execute(Print(separator))?
        .execute(Print("\r\n"))
        .map(|_| Ok(()))?
}

fn displayable_type(decl: &schema::TypeDecl, indent: &str, level: usize) -> String {
    let mut output = format!("type `{}` {{\n", decl.name);
    let level_indent = indent.repeat(level);
    displayable_propreties(&decl.property_decls, &mut output, indent, level);
    output.push_str(&format!("{}}}", level_indent));
    output
}

fn displayable_propreties(
    property_decls: &Vec<schema::PropertyDecl>,
    output: &mut String,
    indent: &str,
    level: usize,
) {
    let level_indent = indent.repeat(level);
    for prop_decl in property_decls {
        match &prop_decl.data_type_decl {
            Ok(val) => match &val.data_type {
                schema::DataType::ObjectDecl(obj_decl) => output.push_str(&format!(
                    "{}{}{}: {}\n",
                    level_indent,
                    &indent,
                    prop_decl.name,
                    displayable_type(&obj_decl, &indent, level + 1)
                )),
                _ => output.push_str(&format!(
                    "{}{}{}: {}\n",
                    level_indent, &indent, prop_decl.name, val.data_type
                )),
            },
            Err(err) => output.push_str(&format!(
                "{}{}{}: {}\n",
                level_indent, &indent, prop_decl.name, err
            )),
        };
    }
}

fn print_interfaces(parsed_schema: &Schema) -> Result<(), Box<dyn std::error::Error>> {
    let separator = (0..80).map(|_| "-").collect::<String>();
    let indent = (0..4).map(|_| " ").collect::<String>();
    let mut out = stdout();
    let builder = out
        .execute(Print(&separator))?
        .execute(SetAttribute(crossterm::style::Attribute::Bold))?
        .execute(Print(format!("\n{}Interfaces\n", &indent)))?
        .execute(Print(&separator))?
        .execute(Print("\n"))?
        .execute(ResetColor)?;
    for interface in &parsed_schema.interfaces {
        match interface {
            Ok(val) => match &val.spec {
                InterfaceSpec::Api(api) => {
                    builder.execute(Print(format!("{}{} {}\n", &indent, api.method, val.ident)))?;
                    match &api.payload {
                        Some(HttpPayload::Query(query)) => {
                            let mut output = String::new();
                            displayable_propreties(query, &mut output, &indent, 1);
                            builder.execute(Print(format!("{}|- Query:\n{}", indent, output)))?
                        }
                        Some(HttpPayload::Body(body)) => {
                            let mut output = String::new();
                            displayable_propreties(body, &mut output, &indent, 1);
                            builder.execute(Print(format!("{}|- Body:\n{}", indent, output)))?
                        }
                        None => builder.execute(Print(""))?,
                    };
                    if let Some(responses) = &api.responses {
                        builder.execute(Print(format!(
                            "{}|- Responses:\n{}",
                            indent,
                            displayable_responses(&responses, &indent)
                        )))?
                    } else {
                        builder.execute(Print(""))?
                    }
                }
            },
            Err(err) => builder.execute(Print(format!("{}- {:?}\n", &indent, err)))?,
        };
        builder.execute(Print(&separator))?.execute(Print("\n"))?;
    }
    builder
        .execute(Print(&separator))?
        .execute(Print("\r\n"))
        .map(|_| Ok(()))?
}

fn displayable_responses(decl: &HashMap<StatusCode, TypeDecl>, indent: &str) -> String {
    let mut output = String::new();
    for (status, response) in decl {
        output.push_str(&format!(
            "{}{}{}: {}\n",
            indent,
            indent,
            status,
            displayable_type(response, indent, 2)
        ));
    }
    output
}
