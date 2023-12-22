use crate::parser::hosts::HostsParser;
use crate::parser::imports::detect;
use crate::parser::types::TypesParser;
use crate::parser::{utils::read_yaml, versioning::VersioningParser};
use crate::schema::Schema;

use self::interfaces::parse as parse_interfaces;

pub fn parse(file_path: &str) -> Result<Schema, Box<dyn std::error::Error>> {
    let yaml = read_yaml(file_path)?;
    let main = &yaml[0];

    let hosts_parser = HostsParser { main };
    let hosts = hosts_parser.parse()?;

    let versioning_parser = VersioningParser { main };
    let versioning = versioning_parser.parse()?;

    let types_imports = detect(&main["types"], "example")?;
    let types_main = &types_imports[0].as_ref().unwrap();
    let types_parser = TypesParser { main: &types_main, parent_path: "example" };
    let types = types_parser.parse()?;

    let interfaces_imports = detect(&main["interfaces"], "example")?;
    let interfaces_main = &interfaces_imports[0].as_ref().unwrap();
    let interfaces = parse_interfaces(&interfaces_main, "example");

    let schema = Schema { hosts, versioning, types, interfaces };

    Ok(schema)
}
