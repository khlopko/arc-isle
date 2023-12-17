use yaml_rust::{Yaml};
use std::fmt::{Debug, Display, Formatter};
use crate::schema::{Versioning, VersioningFormat};

pub struct VersioningParser<'a> {
    pub main: &'a Yaml
}

impl<'a> VersioningParser<'a> {
    pub fn parse(&self) -> Result<Versioning, VersioningError> {
        let raw_versioning: &Yaml = &self.main["versioning"];
        let raw_format: Option<&str> = raw_versioning["format"].as_str();
        let raw_format = raw_format.ok_or(VersioningError::NotFound)?;
        match raw_format {
            "headers" => {
                let header = match raw_versioning["header"].as_str() {
                    Some(header_name) => Some(header_name.to_string()),
                    None => return Err(VersioningError::MissingHeader)
                };
                Ok(Versioning { format: VersioningFormat::Headers, header })
            },
            other => Err(VersioningError::UnsupportedFormat(other.to_string()))
        }
    }
}

pub enum VersioningError {
    NotFound,
    UnsupportedFormat(String),
    MissingHeader
}

impl VersioningError {
    fn default_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VersioningError::NotFound =>
                write!(f, "Versioning info wasn't found in schema."),
            VersioningError::UnsupportedFormat(key) =>
                write!(f, "'{}' format is not supported for versioning", key),
            VersioningError::MissingHeader =>
                write!(f, "Missing 'header' key inside 'versioning'.")
        }
    }
}

impl std::error::Error for VersioningError {
}

impl Display for VersioningError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

impl Debug for VersioningError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}
