mod hosts;
mod imports;
mod interfaces;
mod types;
pub(crate) mod utils;
mod versioning;

use std::collections::HashSet;

use crate::parser::hosts::HostsParser;
use crate::parser::imports::detect;
use crate::parser::types::TypesParser;
use crate::parser::{utils::read_yaml, versioning::VersioningParser};
use crate::schema::{ImportError, Schema};

use self::interfaces::InterfacesParser;

pub fn parse(file_path: &str) -> Result<Schema, Box<dyn std::error::Error>> {
    let yaml = read_yaml(file_path)?;
    let main = &yaml[0];

    let hosts_parser = HostsParser { main };
    let hosts = hosts_parser.parse()?;

    let versioning_parser = VersioningParser { main };
    let versioning = versioning_parser.parse()?;

    let mut known_types: HashSet<String> = HashSet::new();
    let main_types_hash = main["types"]
        .as_hash()
        .ok_or(ImportError::InvalidInputSource)?;
    let types_imports = detect(&main_types_hash, "example");
    let mut types_parser = TypesParser {
        parent_path: "example",
        known_types: &mut known_types,
    };
    let mut types: Vec<_> = vec![];
    for import in types_imports {
        types.extend(types_parser.parse(import?)?);
    }

    let main_interfaces_hash = main["interfaces"]
        .as_hash()
        .ok_or(ImportError::InvalidInputSource)?;
    let interfaces_imports = detect(&main_interfaces_hash, "example");
    let interfaces_parser = InterfacesParser::new("example", &known_types, &types);
    let mut interfaces: Vec<_> = vec![];
    for import in interfaces_imports {
        interfaces.extend(interfaces_parser.parse(import?)?);
    }
    let schema = Schema {
        hosts,
        versioning,
        types,
        interfaces,
    };

    Ok(schema)
}
