use std::{
    collections::HashMap,
    io::{stdout, Stdout},
};

use clap::{Parser, Subcommand};
use crossterm::{
    style::{Print, ResetColor, SetAttribute},
    ExecutableCommand,
};

use arc_isle::{
    parser,
    schema::{self, ApiSpec, HttpPayload, InterfaceSpec, Schema, StatusCode, TypeDecl},
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
    let (mut out, indent, separator) = prepare();
    let builder = section_decorator(&mut out, "Hosts", &indent, &separator)?;
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
    let (mut out, indent, separator) = prepare();
    section_decorator(&mut out, "Versioning", &indent, &separator)?
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
    let (mut out, indent, separator) = prepare();
    let builder = section_decorator(&mut out, "Types", &indent, &separator)?;
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
    let (mut out, indent, separator) = prepare();
    let builder = section_decorator(&mut out, "Interfaces", &indent, &separator)?;
    for interface in &parsed_schema.interfaces {
        match interface {
            Ok(val) => match &val.spec {
                InterfaceSpec::Api(api) => print_api_spec(&val.ident, &api, builder, &indent)?,
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

fn print_api_spec<'a>(
    ident: &str,
    api: &ApiSpec,
    builder: &'a mut Stdout,
    indent: &str,
) -> Result<&'a mut Stdout, Box<dyn std::error::Error>> {
    builder.execute(Print(format!("{}{} {}\n", &indent, api.method, ident)))?;
    if let Some(payload) = &api.payload {
        print_payload(&payload, builder, &indent)?;
    }
    if let Some(responses) = &api.responses {
        builder.execute(Print(format!(
            "{}|- Responses:\n{}",
            indent,
            displayable_responses(&responses, &indent)
        )))?;
    }
    Ok(builder)
}

fn print_payload<'a>(
    payload: &HttpPayload,
    builder: &'a mut Stdout,
    indent: &str,
) -> Result<&'a mut Stdout, Box<dyn std::error::Error>> {
    match payload {
        HttpPayload::Query(query) => {
            let mut output = String::new();
            displayable_propreties(query, &mut output, &indent, 1);
            Ok(builder.execute(Print(format!("{}|- Query:\n{}", indent, output)))?)
        }
        HttpPayload::Body(body) => {
            let mut output = String::new();
            displayable_propreties(body, &mut output, &indent, 1);
            Ok(builder.execute(Print(format!("{}|- Body:\n{}", indent, output)))?)
        }
    }
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

fn prepare() -> (Stdout, String, String) {
    let separator = (0..80).map(|_| "-").collect::<String>();
    let indent = (0..4).map(|_| " ").collect::<String>();
    (stdout(), indent, separator)
}

fn section_decorator<'a>(
    out: &'a mut Stdout,
    title: &str,
    indent: &str,
    separator: &str,
) -> Result<&'a mut Stdout, Box<dyn std::error::Error>> {
    Ok(out
        .execute(Print(&separator))?
        .execute(SetAttribute(crossterm::style::Attribute::Bold))?
        .execute(Print(format!("\n{}{}\n", &indent, title)))?
        .execute(Print(&separator))?
        .execute(Print("\n"))?
        .execute(ResetColor)?)
}
