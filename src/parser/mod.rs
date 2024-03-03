mod hosts;
mod imports;
mod interfaces;
mod types;
pub(crate) mod utils;
mod versioning;

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;

use crate::parser::hosts::HostsParser;
use crate::parser::imports::detect;
use crate::parser::types::TypesParser;
use crate::parser::{utils::read_yaml, versioning::VersioningParser};
use crate::schema::{ImportError, Schema, TypeUsageMeta, UnknownType};

use self::interfaces::InterfacesParser;

#[derive(Debug)]
pub struct MissingTypeDeclError {
    pub list: Vec<UnknownType>
}

impl Error for MissingTypeDeclError {
}

impl Display for MissingTypeDeclError {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       f.write_str(&format!("{:?}", self.list))
   } 
}

pub fn parse(parent_path: &str) -> Result<Schema, Box<dyn std::error::Error>> {
    let file_path = &(parent_path.to_string() + "/main.yaml");
    let yaml = read_yaml(file_path)?;
    let main = &yaml[0];
    let hosts_parser = HostsParser { main };
    let hosts = hosts_parser.parse()?;
    let versioning_parser = VersioningParser { main };
    let versioning = versioning_parser.parse()?;
    let mut types_usage: HashMap<String, TypeUsageMeta> = HashMap::new();
    let main_types_hash = main["types"]
        .as_hash()
        .ok_or(ImportError::InvalidInputSource)?;
    let types_imports = detect(&main_types_hash, parent_path);
    let mut types_parser = TypesParser {
        parent_path,
        types_usage: &mut types_usage,
    };
    let mut types: Vec<_> = vec![];
    for import in types_imports {
        types.extend(types_parser.parse(import?)?);
    }
    let main_interfaces_hash = main["interfaces"]
        .as_hash()
        .ok_or(ImportError::InvalidInputSource)?;
    let interfaces_imports = detect(&main_interfaces_hash, parent_path);
    let mut interfaces_parser = InterfacesParser {
        parent_path,
        types_usage: &mut types_usage,
        types: &types,
    };
    let mut interfaces: Vec<_> = vec![];
    for import in interfaces_imports {
        interfaces.extend(interfaces_parser.parse(import?)?);
    }
    let mut missing_declations: Vec<UnknownType> = Vec::new();
    for (type_name, unknown) in &types_usage {
        if let Some(unknown) = unknown {
            for e in unknown {
                missing_declations.push(e.clone());
                match e {
                    UnknownType::InTypeDeclaration(ti, pi) => {
                        println!("Unknown type {} at {} in property at {}", type_name, ti, pi);
                    }
                    UnknownType::InPayload(ii, pi) => {
                        println!("Unknown type {} in interface (#{}) input {}", type_name, ii, pi);
                    }
                    UnknownType::InResponse(ii, code, pi) => {
                        println!(
                        "Unknown type {} in interface (#{}) output status code {} in property at {}",
                        type_name, ii, code, pi
                    );
                    }
                }
            }
        }
    }
    if !missing_declations.is_empty() {
        let err = MissingTypeDeclError{list: missing_declations};
        return Err(Box::new(err));
    }
    let schema = Schema {
        hosts,
        versioning,
        types,
        interfaces,
    };
    Ok(schema)
}
