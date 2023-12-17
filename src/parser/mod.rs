pub(crate) mod utils;
mod hosts;
mod versioning;
mod imports;
mod objects;

use crate::parser::hosts::{HostsParser};
use crate::parser::imports::detect;
use crate::parser::objects::{ObjectParser, ObjectsParser};
use crate::parser::utils::read_yaml;
use crate::parser::versioning::VersioningParser;
use crate::schema::{Schema};

pub fn parse(file_path: &str) -> Result<Schema, Box<dyn std::error::Error>> {
    let yaml = read_yaml(file_path)?;
    let main = &yaml[0];

    let hosts_parser = HostsParser { main };
    let hosts = hosts_parser.parse()?;

    let versioning_parser = VersioningParser { main };
    let versioning = versioning_parser.parse()?;

    let objects_imports = detect(&main["objects"], "example")?;
    let main = &objects_imports[0].as_ref().unwrap();
    let objects_parser = ObjectsParser { main: &main, parent_path: "example" };
    let object_decl_results = objects_parser.parse()?;
    let schema = Schema { hosts, versioning, object_decl_results };

    Ok(schema)
}
