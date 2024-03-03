mod hosts;
mod imports;
mod interfaces;
mod types;
pub(crate) mod utils;
mod versioning;

use std::collections::HashMap;

use crate::parser::hosts::HostsParser;
use crate::parser::imports::detect;
use crate::parser::types::TypesParser;
use crate::parser::{utils::read_yaml, versioning::VersioningParser};
use crate::schema::{ImportError, Schema, TypeUsageMeta, UnknownType};

use self::interfaces::InterfacesParser;

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
        types: &types
    };
    let mut interfaces: Vec<_> = vec![];
    for import in interfaces_imports {
        interfaces.extend(interfaces_parser.parse(import?)?);
    }
    for (_, unknown) in &types_usage {
        if let Some(unknown) = unknown {
            match unknown {
                UnknownType::InTypeDeclaration(ti, pi) => {
                },
                UnknownType::InPayload(ii) => {
                },
                UnknownType::InResponse(code, pi) => {
                },
            }
        }
    }
    let schema = Schema {
        hosts,
        versioning,
        types,
        interfaces,
    };
    Ok(schema)
}
